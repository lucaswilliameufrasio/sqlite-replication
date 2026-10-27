//! S3-compatible storage adapter backed by the AWS SDK for Rust.
//!
//! The adapter is intentionally independent from the replication core and does
//! not expose WAL capture. Callers own the SDK client, credential chain, and a
//! bounded `aws_sdk_s3::config::retry::RetryConfig::standard()` policy.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::Path as FilePath;
use std::sync::Arc;
use std::time::{Duration, Instant};

use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

const BUFFER_SIZE: usize = 1024 * 1024;
const MULTIPART_THRESHOLD: u64 = 8 * 1024 * 1024;
const PART_SIZE: usize = 8 * 1024 * 1024;
const MAX_PARTS: u64 = 10_000;
const MAX_OBJECT_SIZE: u64 = 5 * 1024 * 1024 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    Conflict,
    NotFound,
    Retryable,
    Permanent,
}

#[derive(Debug)]
pub struct StorageError {
    pub operation: &'static str,
    pub kind: ErrorKind,
}

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "S3 {} failed ({:?})", self.operation, self.kind)
    }
}

impl std::error::Error for StorageError {}

fn error(operation: &'static str, kind: ErrorKind, _message: impl fmt::Display) -> StorageError {
    StorageError { operation, kind }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Capabilities {
    pub conditional_put: bool,
    pub conditional_multipart_complete: bool,
    pub byte_ranges: bool,
    pub multipart_upload: bool,
    pub delete_batch_limit: usize,
    pub max_object_size_bytes: u64,
}

/// Receives low-cardinality observations without object keys or metadata.
pub trait Metrics: Send + Sync {
    fn observe_storage(
        &self,
        operation: &'static str,
        duration: Duration,
        bytes: u64,
        error: Option<ErrorKind>,
    );
}

#[derive(Clone)]
pub struct S3Store {
    client: Client,
    bucket: String,
    metrics: Option<Arc<dyn Metrics>>,
}

impl S3Store {
    pub fn new(client: Client, bucket: impl Into<String>) -> Result<Self, StorageError> {
        let bucket = bucket.into();
        if bucket.trim().is_empty() {
            return Err(error("configure", ErrorKind::Permanent, "empty bucket"));
        }
        Ok(Self {
            client,
            bucket,
            metrics: None,
        })
    }

    pub fn with_metrics(mut self, metrics: Arc<dyn Metrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    fn observe(
        &self,
        operation: &'static str,
        started: Instant,
        bytes: u64,
        failure: Option<ErrorKind>,
    ) {
        if let Some(metrics) = &self.metrics {
            metrics.observe_storage(operation, started.elapsed(), bytes, failure);
        }
    }

    /// Reports protocol features requested by this adapter, not endpoint
    /// discovery; deployments must verify provider semantics independently.
    pub fn capabilities(&self) -> Capabilities {
        Capabilities {
            conditional_put: true,
            conditional_multipart_complete: true,
            byte_ranges: true,
            multipart_upload: true,
            delete_batch_limit: 1000,
            max_object_size_bytes: MAX_OBJECT_SIZE,
        }
    }

    /// Spools input to a temporary file to compute size and SHA-256 with bounded
    /// memory, then uses a conditional single PUT or conditional multipart
    /// completion. The temporary file is removed when this method returns.
    pub async fn put_immutable<R>(&self, key: &str, input: R) -> Result<PutReceipt, StorageError>
    where
        R: AsyncRead + Unpin,
    {
        self.put_immutable_with_metadata(key, input, HashMap::new())
            .await
    }

    pub async fn put_immutable_with_metadata<R>(
        &self,
        key: &str,
        input: R,
        user_metadata: HashMap<String, String>,
    ) -> Result<PutReceipt, StorageError>
    where
        R: AsyncRead + Unpin,
    {
        let started = Instant::now();
        let result = self.put_immutable_inner(key, input, user_metadata).await;
        let (size, failure) = match &result {
            Ok(receipt) => (receipt.size, None),
            Err(error) => (0, Some(error.kind)),
        };
        self.observe("put", started, size, failure);
        result
    }

    async fn put_immutable_inner<R>(
        &self,
        key: &str,
        mut input: R,
        user_metadata: HashMap<String, String>,
    ) -> Result<PutReceipt, StorageError>
    where
        R: AsyncRead + Unpin,
    {
        validate_key(key)?;
        let user_metadata = normalize_metadata(user_metadata)?;
        let stage =
            tempfile::NamedTempFile::new().map_err(|e| error("put", ErrorKind::Permanent, e))?;
        let stage_path = stage.path().to_path_buf();
        let mut file = tokio::fs::File::from_std(
            stage
                .reopen()
                .map_err(|e| error("put", ErrorKind::Permanent, e))?,
        );
        let mut hasher = Sha256::new();
        let mut size = 0_u64;
        let mut buffer = vec![0_u8; BUFFER_SIZE];
        loop {
            let count = input
                .read(&mut buffer)
                .await
                .map_err(|e| error("put", ErrorKind::Permanent, e))?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
            size = size
                .checked_add(count as u64)
                .ok_or_else(|| error("put", ErrorKind::Permanent, "object size overflow"))?;
            file.write_all(&buffer[..count])
                .await
                .map_err(|e| error("put", ErrorKind::Permanent, e))?;
        }
        file.sync_all()
            .await
            .map_err(|e| error("put", ErrorKind::Permanent, e))?;
        drop(file);
        if size > MAX_OBJECT_SIZE {
            return Err(error(
                "put",
                ErrorKind::Permanent,
                "object exceeds S3 limit",
            ));
        }
        let sha256 = format!("{:x}", hasher.finalize());
        let mut metadata = user_metadata.clone();
        metadata.insert("content-sha256".to_owned(), sha256.clone());
        metadata.insert("content-length".to_owned(), size.to_string());

        let outcome = if size < MULTIPART_THRESHOLD {
            self.put_single(key, &stage_path, size, &metadata).await
        } else {
            self.put_multipart(key, &stage_path, size, &metadata).await
        };

        match outcome {
            Ok(()) => Ok(PutReceipt { size, sha256 }),
            Err(upload_error) => match self
                .verify_existing(key, size, &sha256, &user_metadata)
                .await
            {
                Ok(true) => Ok(PutReceipt { size, sha256 }),
                Ok(false) => Err(error(
                    "put",
                    ErrorKind::Conflict,
                    "immutable key already contains different bytes",
                )),
                Err(verify_error) if verify_error.kind == ErrorKind::NotFound => Err(upload_error),
                Err(verify_error) => Err(verify_error),
            },
        }
    }

    async fn put_single(
        &self,
        key: &str,
        path: &FilePath,
        size: u64,
        metadata: &HashMap<String, String>,
    ) -> Result<(), StorageError> {
        let body = ByteStream::from_path(path)
            .await
            .map_err(|e| error("put", ErrorKind::Permanent, e))?;
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .content_length(size as i64)
            .if_none_match("*")
            .set_metadata(Some(metadata.clone()))
            .send()
            .await
            .map(|_| ())
            .map_err(|e| classify("put", e))
    }

    async fn put_multipart(
        &self,
        key: &str,
        path: &FilePath,
        size: u64,
        metadata: &HashMap<String, String>,
    ) -> Result<(), StorageError> {
        let upload = self
            .client
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .set_metadata(Some(metadata.clone()))
            .send()
            .await
            .map_err(|e| classify("multipart-create", e))?;
        let upload_id = upload
            .upload_id()
            .ok_or_else(|| {
                error(
                    "multipart-create",
                    ErrorKind::Permanent,
                    "missing upload id",
                )
            })?
            .to_owned();
        let mut parts = Vec::new();
        let mut part_number = 1_i32;
        let size_for_part = size.div_ceil(MAX_PARTS);
        let part_size = (PART_SIZE as u64).max(size_for_part);
        let mut offset = 0_u64;

        while offset < size {
            let count = part_size.min(size - offset);
            let body = ByteStream::read_from()
                .path(path)
                .offset(offset)
                .length(aws_sdk_s3::primitives::Length::Exact(count))
                .build()
                .await;
            let body = match body {
                Ok(body) => body,
                Err(failure) => {
                    self.abort_multipart(key, &upload_id).await;
                    return Err(error("multipart-read", ErrorKind::Permanent, failure));
                }
            };
            let uploaded = self
                .client
                .upload_part()
                .bucket(&self.bucket)
                .key(key)
                .upload_id(&upload_id)
                .part_number(part_number)
                .content_length(count as i64)
                .body(body)
                .send()
                .await
                .map_err(|e| classify("multipart-part", e));
            match uploaded {
                Ok(output) => {
                    parts.push(
                        CompletedPart::builder()
                            .part_number(part_number)
                            .set_e_tag(output.e_tag().map(ToOwned::to_owned))
                            .build(),
                    );
                }
                Err(upload_error) => {
                    self.abort_multipart(key, &upload_id).await;
                    return Err(upload_error);
                }
            }
            part_number += 1;
            offset += count;
        }

        let complete = CompletedMultipartUpload::builder()
            .set_parts(Some(parts))
            .build();
        let completed = self
            .client
            .complete_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(&upload_id)
            .multipart_upload(complete)
            .if_none_match("*")
            .send()
            .await;
        match completed {
            Ok(_) => Ok(()),
            Err(failure) => {
                self.abort_multipart(key, &upload_id).await;
                Err(classify("multipart-complete", failure))
            }
        }
    }

    async fn abort_multipart(&self, key: &str, upload_id: &str) {
        let _ = self
            .client
            .abort_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(upload_id)
            .send()
            .await;
    }

    async fn verify_existing(
        &self,
        key: &str,
        expected_size: u64,
        expected_sha256: &str,
        user_metadata: &HashMap<String, String>,
    ) -> Result<bool, StorageError> {
        let object = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| classify("verify", e))?;
        if object.content_length().unwrap_or_default() as u64 != expected_size {
            return Ok(false);
        }
        if user_metadata.iter().any(|(key, value)| {
            object
                .metadata()
                .and_then(|metadata| metadata.get(key))
                .is_none_or(|remote| remote != value)
        }) {
            return Ok(false);
        }
        if let Some(remote_hash) = object
            .metadata()
            .and_then(|metadata| metadata.get("content-sha256"))
        {
            if remote_hash != expected_sha256 {
                return Ok(false);
            }
        }
        let mut body = object.body.into_async_read();
        let mut hasher = Sha256::new();
        let mut buffer = vec![0_u8; BUFFER_SIZE];
        loop {
            let count = body
                .read(&mut buffer)
                .await
                .map_err(|e| error("verify", ErrorKind::Retryable, e))?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        Ok(format!("{:x}", hasher.finalize()) == expected_sha256)
    }

    pub async fn get(
        &self,
        key: &str,
    ) -> Result<aws_sdk_s3::operation::get_object::GetObjectOutput, StorageError> {
        let started = Instant::now();
        let result = self.get_inner(key).await;
        let (size, failure) = match &result {
            Ok(output) => (
                output.content_length().unwrap_or_default().max(0) as u64,
                None,
            ),
            Err(error) => (0, Some(error.kind)),
        };
        self.observe("get", started, size, failure);
        result
    }

    async fn get_inner(
        &self,
        key: &str,
    ) -> Result<aws_sdk_s3::operation::get_object::GetObjectOutput, StorageError> {
        validate_key(key)?;
        self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| classify("get", e))
    }

    pub async fn get_range(
        &self,
        key: &str,
        start: u64,
        end_inclusive: u64,
    ) -> Result<aws_sdk_s3::operation::get_object::GetObjectOutput, StorageError> {
        let started = Instant::now();
        let result = self.get_range_inner(key, start, end_inclusive).await;
        let (size, failure) = match &result {
            Ok(output) => (
                output.content_length().unwrap_or_default().max(0) as u64,
                None,
            ),
            Err(error) => (0, Some(error.kind)),
        };
        self.observe("get-range", started, size, failure);
        result
    }

    async fn get_range_inner(
        &self,
        key: &str,
        start: u64,
        end_inclusive: u64,
    ) -> Result<aws_sdk_s3::operation::get_object::GetObjectOutput, StorageError> {
        validate_key(key)?;
        if end_inclusive < start {
            return Err(error(
                "get-range",
                ErrorKind::Permanent,
                "invalid byte range",
            ));
        }
        self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .range(format!("bytes={start}-{end_inclusive}"))
            .send()
            .await
            .map_err(|e| classify("get-range", e))
    }

    pub async fn head(
        &self,
        key: &str,
    ) -> Result<aws_sdk_s3::operation::head_object::HeadObjectOutput, StorageError> {
        let started = Instant::now();
        let result = self.head_inner(key).await;
        let failure = result.as_ref().err().map(|error| error.kind);
        self.observe("head", started, 0, failure);
        result
    }

    async fn head_inner(
        &self,
        key: &str,
    ) -> Result<aws_sdk_s3::operation::head_object::HeadObjectOutput, StorageError> {
        validate_key(key)?;
        self.client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| classify("head", e))
    }

    pub async fn list_page(
        &self,
        prefix: &str,
        cursor: Option<&str>,
        page_size: i32,
    ) -> Result<aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Output, StorageError> {
        let started = Instant::now();
        let result = self.list_page_inner(prefix, cursor, page_size).await;
        let failure = result.as_ref().err().map(|error| error.kind);
        self.observe("list", started, 0, failure);
        result
    }

    async fn list_page_inner(
        &self,
        prefix: &str,
        cursor: Option<&str>,
        page_size: i32,
    ) -> Result<aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Output, StorageError> {
        if page_size <= 0 || page_size > 1000 {
            return Err(error("list", ErrorKind::Permanent, "invalid page size"));
        }
        self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .set_continuation_token(cursor.map(ToOwned::to_owned))
            .max_keys(page_size)
            .send()
            .await
            .map_err(|e| classify("list", e))
    }

    pub async fn delete_batch(&self, keys: &[String]) -> Result<(), StorageError> {
        let started = Instant::now();
        let result = self.delete_batch_inner(keys).await;
        let failure = result.as_ref().err().map(|error| error.kind);
        self.observe("delete", started, 0, failure);
        result
    }

    async fn delete_batch_inner(&self, keys: &[String]) -> Result<(), StorageError> {
        if keys.len() > 1000 {
            return Err(error(
                "delete",
                ErrorKind::Permanent,
                "batch exceeds 1000 keys",
            ));
        }
        if keys.is_empty() {
            return Ok(());
        }
        let objects = keys
            .iter()
            .map(|key| {
                aws_sdk_s3::types::ObjectIdentifier::builder()
                    .key(key)
                    .build()
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| error("delete", ErrorKind::Permanent, e))?;
        let delete = aws_sdk_s3::types::Delete::builder()
            .set_objects(Some(objects))
            .build()
            .map_err(|e| error("delete", ErrorKind::Permanent, e))?;
        let response = self
            .client
            .delete_objects()
            .bucket(&self.bucket)
            .delete(delete)
            .send()
            .await
            .map_err(|e| classify("delete", e))?;
        if !response.errors().is_empty() {
            return Err(error(
                "delete",
                ErrorKind::Retryable,
                "one or more keys failed",
            ));
        }
        Ok(())
    }

    /// Compare every listed object under `prefix` with the caller's local
    /// inventory. Matching content is read and SHA-256 checked; no object is
    /// removed and no caller acknowledgement is advanced.
    pub async fn reconcile(
        &self,
        prefix: &str,
        expected: &[ObjectDescriptor],
    ) -> Result<ReconcileReport, StorageError> {
        let started = Instant::now();
        let result = self.reconcile_inner(prefix, expected).await;
        let failure = result.as_ref().err().map(|error| error.kind);
        self.observe("reconcile", started, 0, failure);
        result
    }

    async fn reconcile_inner(
        &self,
        prefix: &str,
        expected: &[ObjectDescriptor],
    ) -> Result<ReconcileReport, StorageError> {
        let mut by_key = HashMap::with_capacity(expected.len());
        for descriptor in expected {
            validate_key(&descriptor.key)?;
            if !descriptor.key.starts_with(prefix)
                || descriptor.sha256.len() != 64
                || !descriptor
                    .sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(error(
                    "reconcile",
                    ErrorKind::Permanent,
                    "invalid expected object descriptor",
                ));
            }
            let normalized = ObjectDescriptor {
                key: descriptor.key.clone(),
                size: descriptor.size,
                sha256: descriptor.sha256.to_ascii_lowercase(),
            };
            if by_key.insert(descriptor.key.clone(), normalized).is_some() {
                return Err(error(
                    "reconcile",
                    ErrorKind::Permanent,
                    "duplicate expected object key",
                ));
            }
        }

        let mut report = ReconcileReport::default();
        let mut seen_remote = HashSet::new();
        let mut seen_cursors = HashSet::new();
        let mut cursor: Option<String> = None;
        loop {
            let page = self.list_page(prefix, cursor.as_deref(), 1000).await?;
            for object in page.contents() {
                let key = object.key().ok_or_else(|| {
                    error(
                        "reconcile",
                        ErrorKind::Permanent,
                        "list returned an object without a key",
                    )
                })?;
                if !seen_remote.insert(key.to_owned()) {
                    continue;
                }
                let Some(descriptor) = by_key.get(key) else {
                    report.unexpected.push(key.to_owned());
                    continue;
                };
                if !self
                    .verify_existing(key, descriptor.size, &descriptor.sha256, &HashMap::new())
                    .await?
                {
                    report.divergent.push(key.to_owned());
                }
            }
            let next_cursor = page.next_continuation_token();
            let Some(next_cursor) = next_cursor.filter(|value| !value.is_empty()) else {
                if page.is_truncated().unwrap_or(false) {
                    return Err(error(
                        "reconcile",
                        ErrorKind::Permanent,
                        "truncated list page has no continuation token",
                    ));
                }
                break;
            };
            if !seen_cursors.insert(next_cursor.to_owned()) {
                return Err(error(
                    "reconcile",
                    ErrorKind::Permanent,
                    "list returned a repeated continuation token",
                ));
            }
            cursor = Some(next_cursor.to_owned());
        }

        for key in by_key.keys() {
            if !seen_remote.contains(key) {
                report.missing.push(key.clone());
            }
        }
        report.missing.sort_unstable();
        report.divergent.sort_unstable();
        report.unexpected.sort_unstable();
        Ok(report)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PutReceipt {
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectDescriptor {
    pub key: String,
    pub size: u64,
    pub sha256: String,
}

/// Read-only comparison of the local immutable set with remote objects.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReconcileReport {
    pub missing: Vec<String>,
    pub divergent: Vec<String>,
    pub unexpected: Vec<String>,
}

fn validate_key(key: &str) -> Result<(), StorageError> {
    if key.is_empty() || key.starts_with('/') || key.contains('\0') {
        return Err(error(
            "validate-key",
            ErrorKind::Permanent,
            "invalid object key",
        ));
    }
    Ok(())
}

fn normalize_metadata(
    metadata: HashMap<String, String>,
) -> Result<HashMap<String, String>, StorageError> {
    let mut normalized = HashMap::with_capacity(metadata.len());
    for (key, value) in metadata {
        let key = key.trim().to_ascii_lowercase();
        if key.is_empty()
            || key.chars().any(char::is_whitespace)
            || key == "content-sha256"
            || key == "content-length"
            || normalized.insert(key, value).is_some()
        {
            return Err(error(
                "metadata",
                ErrorKind::Permanent,
                "invalid, reserved, or duplicate metadata key",
            ));
        }
    }
    Ok(normalized)
}

fn classify<E>(operation: &'static str, failure: aws_sdk_s3::error::SdkError<E>) -> StorageError
where
    E: std::error::Error + 'static,
{
    let kind = match &failure {
        aws_sdk_s3::error::SdkError::ServiceError(service) => {
            let status = service.raw().status().as_u16();
            classify_status(status)
        }
        aws_sdk_s3::error::SdkError::TimeoutError(_)
        | aws_sdk_s3::error::SdkError::DispatchFailure(_) => ErrorKind::Retryable,
        _ => ErrorKind::Permanent,
    };
    error(operation, kind, failure)
}

fn classify_status(status: u16) -> ErrorKind {
    match status {
        404 => ErrorKind::NotFound,
        409 | 412 => ErrorKind::Conflict,
        408 | 429 | 500..=599 => ErrorKind::Retryable,
        _ => ErrorKind::Permanent,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::env;
    use std::io::{self, Cursor, Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use aws_credential_types::Credentials;
    use aws_sdk_s3::config::Region;
    use tokio::io::AsyncReadExt;

    use super::{ErrorKind, Metrics, ObjectDescriptor, S3Store, classify_status};
    use sha2::{Digest, Sha256};

    #[derive(Default)]
    struct RecordingMetrics(Mutex<Vec<(&'static str, u64, Option<ErrorKind>)>>);

    impl Metrics for RecordingMetrics {
        fn observe_storage(
            &self,
            operation: &'static str,
            duration: Duration,
            bytes: u64,
            error: Option<ErrorKind>,
        ) {
            assert!(duration >= Duration::ZERO);
            self.0
                .lock()
                .expect("metrics mutex")
                .push((operation, bytes, error));
        }
    }

    #[test]
    fn rejects_invalid_bucket_and_key() {
        let client = aws_sdk_s3::Client::from_conf(
            aws_sdk_s3::Config::builder()
                .behavior_version_latest()
                .region(Region::new("us-east-1"))
                .credentials_provider(Credentials::new("test", "test", None, None, "unit-test"))
                .build(),
        );
        assert!(S3Store::new(client.clone(), " ").is_err());
        assert!(super::validate_key("").is_err());
        assert!(super::validate_key("/absolute").is_err());
        assert!(super::validate_key("nested/key.ltx").is_ok());
        assert!(
            S3Store::new(client, "bucket")
                .unwrap()
                .capabilities()
                .conditional_multipart_complete
        );
    }

    #[test]
    fn classifies_s3_http_statuses() {
        let cases = [
            (403, ErrorKind::Permanent),
            (404, ErrorKind::NotFound),
            (409, ErrorKind::Conflict),
            (412, ErrorKind::Conflict),
            (429, ErrorKind::Retryable),
            (500, ErrorKind::Retryable),
            (503, ErrorKind::Retryable),
        ];
        for (status, expected) in cases {
            assert_eq!(classify_status(status), expected, "HTTP status {status}");
        }
    }

    #[tokio::test]
    async fn reconcile_rejects_invalid_inventory_before_remote_io() {
        let config = aws_sdk_s3::Config::builder()
            .behavior_version_latest()
            .region(Region::new("us-east-1"))
            .credentials_provider(Credentials::new("test", "test", None, None, "unit-test"))
            .build();
        let store = S3Store::new(aws_sdk_s3::Client::from_conf(config), "bucket")
            .expect("construct adapter");
        let cases = [
            ObjectDescriptor {
                key: "test/object".into(),
                size: 1,
                sha256: "not-hex".into(),
            },
            ObjectDescriptor {
                key: "other/object".into(),
                size: 1,
                sha256: format!("{:x}", Sha256::digest(b"x")),
            },
        ];
        assert!(store.reconcile("test/", &cases[..1]).await.is_err());
        assert!(store.reconcile("test/", &cases[1..]).await.is_err());
        assert!(
            store
                .reconcile(
                    "test/",
                    &[
                        ObjectDescriptor {
                            key: "test/object".into(),
                            size: 1,
                            sha256: format!("{:x}", Sha256::digest(b"x")),
                        },
                        ObjectDescriptor {
                            key: "test/object".into(),
                            size: 1,
                            sha256: format!("{:x}", Sha256::digest(b"x")),
                        },
                    ],
                )
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn metrics_use_low_cardinality_operations() {
        let client = aws_sdk_s3::Client::from_conf(
            aws_sdk_s3::Config::builder()
                .behavior_version_latest()
                .region(Region::new("us-east-1"))
                .credentials_provider(Credentials::new("test", "test", None, None, "unit-test"))
                .build(),
        );
        let metrics = Arc::new(RecordingMetrics::default());
        let store = S3Store::new(client, "bucket")
            .expect("construct store")
            .with_metrics(metrics.clone());

        assert!(store.head("/private-object-key").await.is_err());
        let observations = metrics.0.lock().expect("metrics mutex");
        assert_eq!(
            observations.as_slice(),
            [("head", 0, Some(ErrorKind::Permanent))]
        );
    }

    #[tokio::test]
    async fn sdk_retries_transient_head_failure() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
        let endpoint = format!("http://{}", listener.local_addr().expect("local address"));
        let server = thread::spawn(move || {
            for attempt in 0..2 {
                let (mut stream, _) = listener.accept().expect("accept SDK request");
                stream
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .expect("set server request timeout");
                let mut request = Vec::new();
                let mut buffer = [0; 1024];
                loop {
                    let count = stream.read(&mut buffer).expect("read SDK request");
                    request.extend_from_slice(&buffer[..count]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                assert!(request.starts_with(b"HEAD "), "expected a HEAD request");
                let response = if attempt == 0 {
                    let body = b"<Error><Code>ServiceUnavailable</Code></Error>";
                    format!(
                        "HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        String::from_utf8_lossy(body)
                    )
                } else {
                    "HTTP/1.1 200 OK\r\nContent-Length: 17\r\nETag: \"test-etag\"\r\nConnection: close\r\n\r\n".to_owned()
                };
                stream
                    .write_all(response.as_bytes())
                    .expect("write S3 test response");
            }
            2
        });
        let config = aws_sdk_s3::Config::builder()
            .behavior_version_latest()
            .region(Region::new("us-east-1"))
            .endpoint_url(endpoint)
            .force_path_style(true)
            .credentials_provider(Credentials::new("test", "test", None, None, "retry-test"))
            .retry_config(
                aws_sdk_s3::config::retry::RetryConfig::standard()
                    .with_max_attempts(3)
                    .with_initial_backoff(Duration::from_millis(1))
                    .with_max_backoff(Duration::from_millis(5)),
            )
            .build();
        let client = aws_sdk_s3::Client::from_conf(config);
        let store = S3Store::new(client, "bucket").expect("construct adapter");
        let output = store.head("retry-key").await.expect("HEAD should retry");
        assert_eq!(output.content_length(), Some(17));
        assert_eq!(server.join().expect("test server thread"), 2);
    }

    #[tokio::test]
    async fn sdk_retry_attempts_are_bounded() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
        let endpoint = format!("http://{}", listener.local_addr().expect("local address"));
        let server = thread::spawn(move || {
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().expect("accept SDK request");
                let (request_line, _, _) = read_test_http_request(&mut stream);
                assert!(request_line.starts_with("HEAD "));
                write_test_http_response(
                    &mut stream,
                    503,
                    "<Error><Code>ServiceUnavailable</Code></Error>",
                );
            }
            3
        });
        let config = aws_sdk_s3::Config::builder()
            .behavior_version_latest()
            .region(Region::new("us-east-1"))
            .endpoint_url(endpoint)
            .force_path_style(true)
            .credentials_provider(Credentials::new(
                "test",
                "test",
                None,
                None,
                "retry-exhaustion-test",
            ))
            .retry_config(
                aws_sdk_s3::config::retry::RetryConfig::standard()
                    .with_max_attempts(3)
                    .with_initial_backoff(Duration::from_millis(1))
                    .with_max_backoff(Duration::from_millis(5)),
            )
            .build();
        let store = S3Store::new(aws_sdk_s3::Client::from_conf(config), "bucket")
            .expect("construct adapter");
        let error = store
            .head("unavailable-key")
            .await
            .expect_err("HEAD must fail after exhausting retries");
        assert_eq!(error.kind, ErrorKind::Retryable);
        assert_eq!(server.join().expect("test server thread"), 3);
    }

    #[tokio::test]
    async fn put_reconciles_lost_response_after_remote_commit() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
        let endpoint = format!("http://{}", listener.local_addr().expect("local address"));
        let server = thread::spawn(move || {
            let mut stored = Vec::new();
            let mut first_put = true;
            loop {
                let (mut stream, _) = listener.accept().expect("accept SDK request");
                let (request_line, headers, body) = read_test_http_request(&mut stream);
                if request_line.starts_with("PUT ") {
                    if first_put {
                        first_put = false;
                        stored = body;
                        // Model the server committing the object and losing its response.
                        drop(stream);
                        continue;
                    }
                    write_test_http_response(
                        &mut stream,
                        412,
                        "<Error><Code>PreconditionFailed</Code></Error>",
                    );
                    continue;
                }
                assert!(
                    request_line.starts_with("GET "),
                    "unexpected request: {request_line}"
                );
                assert!(!headers.is_empty(), "request headers should be present");
                write_test_http_response(&mut stream, 200, &String::from_utf8_lossy(&stored));
                break;
            }
            stored
        });
        let config = aws_sdk_s3::Config::builder()
            .behavior_version_latest()
            .region(Region::new("us-east-1"))
            .endpoint_url(endpoint)
            .force_path_style(true)
            .credentials_provider(Credentials::new(
                "test",
                "test",
                None,
                None,
                "lost-reply-test",
            ))
            .retry_config(
                aws_sdk_s3::config::retry::RetryConfig::standard()
                    .with_max_attempts(2)
                    .with_initial_backoff(Duration::from_millis(1))
                    .with_max_backoff(Duration::from_millis(5)),
            )
            .build();
        let store = S3Store::new(aws_sdk_s3::Client::from_conf(config), "bucket")
            .expect("construct adapter");
        let content = b"committed before reply was lost";
        let receipt = store
            .put_immutable("lost-reply.ltx", Cursor::new(content))
            .await
            .expect("PUT should verify remote content after a lost response");
        assert_eq!(receipt.size, content.len() as u64);
        assert_eq!(receipt.sha256, format!("{:x}", Sha256::digest(content)));
        assert_eq!(server.join().expect("test server thread"), content);
    }

    fn read_test_http_request(stream: &mut std::net::TcpStream) -> (String, Vec<String>, Vec<u8>) {
        let mut head = Vec::new();
        let mut byte = [0_u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).expect("read request headers");
            head.push(byte[0]);
        }
        let head = String::from_utf8(head).expect("HTTP headers are UTF-8");
        let mut lines = head.split("\r\n");
        let request_line = lines.next().expect("request line").to_owned();
        let headers = lines.map(str::to_owned).collect::<Vec<_>>();
        let content_length = headers
            .iter()
            .find_map(|header| {
                let (name, value) = header.split_once(':')?;
                if name.eq_ignore_ascii_case("x-amz-decoded-content-length")
                    || name.eq_ignore_ascii_case("content-length")
                {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            })
            .unwrap_or_default();
        let is_chunked = headers.iter().any(|header| {
            header.split_once(':').is_some_and(|(name, value)| {
                (name.eq_ignore_ascii_case("transfer-encoding")
                    && value.to_ascii_lowercase().contains("chunked"))
                    || (name.eq_ignore_ascii_case("content-encoding")
                        && value.to_ascii_lowercase().contains("aws-chunked"))
            })
        });
        let expect_continue = headers.iter().any(|header| {
            header.split_once(':').is_some_and(|(name, value)| {
                name.eq_ignore_ascii_case("expect")
                    && value.trim().eq_ignore_ascii_case("100-continue")
            })
        });
        if expect_continue {
            stream
                .write_all(b"HTTP/1.1 100 Continue\r\n\r\n")
                .expect("write continue response");
        }
        let mut body = Vec::with_capacity(content_length);
        if is_chunked {
            loop {
                let chunk_header = read_test_http_line(stream);
                let chunk_size = usize::from_str_radix(
                    chunk_header.split(';').next().expect("chunk size").trim(),
                    16,
                )
                .expect("valid chunk size");
                if chunk_size == 0 {
                    loop {
                        if read_test_http_line(stream).is_empty() {
                            break;
                        }
                    }
                    break;
                }
                let mut chunk = vec![0; chunk_size];
                stream.read_exact(&mut chunk).expect("read request chunk");
                body.extend_from_slice(&chunk);
                let mut chunk_terminator = [0; 2];
                stream
                    .read_exact(&mut chunk_terminator)
                    .expect("read chunk terminator");
            }
        } else {
            body.resize(content_length, 0);
            stream.read_exact(&mut body).expect("read request body");
        }
        (request_line, headers, body)
    }

    fn read_test_http_line(stream: &mut std::net::TcpStream) -> String {
        let mut line = Vec::new();
        let mut byte = [0_u8; 1];
        loop {
            stream.read_exact(&mut byte).expect("read HTTP line");
            if byte[0] == b'\n' {
                break;
            }
            if byte[0] != b'\r' {
                line.push(byte[0]);
            }
        }
        String::from_utf8(line).expect("HTTP line is UTF-8")
    }

    fn write_test_http_response(stream: &mut std::net::TcpStream, status: u16, body: &str) {
        let reason = match status {
            200 => "OK",
            412 => "Precondition Failed",
            _ => "Test Response",
        };
        let response = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("write test HTTP response");
    }

    struct PendingReader(Arc<AtomicBool>);

    impl tokio::io::AsyncRead for PendingReader {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            _context: &mut std::task::Context<'_>,
            _buffer: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<io::Result<()>> {
            self.0.store(true, Ordering::Release);
            std::task::Poll::Pending
        }
    }

    #[tokio::test]
    async fn dropping_put_future_cancels_staging_read() {
        let config = aws_sdk_s3::Config::builder()
            .behavior_version_latest()
            .region(Region::new("us-east-1"))
            .credentials_provider(Credentials::new("test", "test", None, None, "cancel-test"))
            .build();
        let client = aws_sdk_s3::Client::from_conf(config);
        let store = S3Store::new(client, "bucket").expect("construct adapter");
        let polled = Arc::new(AtomicBool::new(false));
        let task_polled = polled.clone();
        let task = tokio::spawn(async move {
            store
                .put_immutable("cancel-key", PendingReader(task_polled))
                .await
        });
        for _ in 0..100 {
            if polled.load(Ordering::Acquire) {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(
            polled.load(Ordering::Acquire),
            "staging read must be polled"
        );
        task.abort();
        let error = task.await.expect_err("aborted PUT task must be cancelled");
        assert!(error.is_cancelled());
    }

    #[tokio::test]
    async fn sdk_honors_operation_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
        let endpoint = format!("http://{}", listener.local_addr().expect("local address"));
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept SDK request");
            let (request_line, _, _) = read_test_http_request(&mut stream);
            assert!(request_line.starts_with("HEAD "));
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .expect("set server timeout");
            let mut byte = [0; 1];
            let _ = stream.read(&mut byte);
        });
        let config = aws_sdk_s3::Config::builder()
            .behavior_version_latest()
            .region(Region::new("us-east-1"))
            .endpoint_url(endpoint)
            .force_path_style(true)
            .credentials_provider(Credentials::new("test", "test", None, None, "timeout-test"))
            .timeout_config(
                aws_sdk_s3::config::timeout::TimeoutConfig::builder()
                    .operation_timeout(Duration::from_millis(100))
                    .build(),
            )
            .retry_config(aws_sdk_s3::config::retry::RetryConfig::disabled())
            .build();
        let store = S3Store::new(aws_sdk_s3::Client::from_conf(config), "bucket")
            .expect("construct adapter");
        let error = store
            .head("timeout-key")
            .await
            .expect_err("HEAD should time out against stalled server");
        assert_eq!(error.kind, ErrorKind::Retryable);
        server.join().expect("test server thread");
    }

    #[tokio::test]
    #[ignore = "requires the isolated Floci service from make test-floci"]
    async fn floci_adapter_conformance() {
        let Ok(endpoint) = env::var("SQLITE_REPLICATION_FLOCI_ENDPOINT") else {
            eprintln!("skipping Floci adapter test: endpoint not configured");
            return;
        };
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_nanos();
        let bucket = format!("sqlite-replication-rust-{suffix}");
        let config = aws_sdk_s3::Config::builder()
            .behavior_version_latest()
            .region(Region::new("us-east-1"))
            .endpoint_url(endpoint)
            .force_path_style(true)
            .credentials_provider(Credentials::new("test", "test", None, None, "floci-test"))
            .build();
        let client = aws_sdk_s3::Client::from_conf(config);
        client
            .create_bucket()
            .bucket(&bucket)
            .send()
            .await
            .expect("create isolated Floci bucket");
        let store = S3Store::new(client.clone(), &bucket).expect("construct adapter");
        let result = run_floci_contract(&store).await;
        if let (Ok(run_id), Ok(fixture_path)) = (
            env::var("SQLITE_REPLICATION_FLOCI_RUN_ID"),
            env::var("SQLITE_REPLICATION_FLOCI_FIXTURE"),
        ) {
            let shared_bucket = format!("sqlite-replication-adapter-{run_id}");
            let shared_store =
                S3Store::new(client.clone(), shared_bucket).expect("construct shared adapter");
            let fixture = std::fs::read(fixture_path).expect("read shared LTX fixture");
            let mut go_object = shared_store
                .get("go-marker.ltx")
                .await
                .expect("read Go-published object")
                .body
                .into_async_read();
            let mut go_bytes = Vec::new();
            go_object
                .read_to_end(&mut go_bytes)
                .await
                .expect("read Go-published bytes");
            assert_eq!(go_bytes, fixture, "Go object must survive Floci restart");
            let mut snapshot_input = std::io::Cursor::new(&go_bytes);
            let mut restored = tempfile::NamedTempFile::new().expect("create Rust restore image");
            sqlite_replication_ltx::decode_snapshot(&mut snapshot_input, &mut restored)
                .expect("decode Go-published LTX with Rust");
            restored
                .as_file()
                .sync_all()
                .expect("sync restored SQLite image");
            sqlite_replication_sqlite::verify_database(restored.path())
                .expect("verify Go-published SQLite snapshot");
            shared_store
                .put_immutable("rust-marker.ltx", Cursor::new(fixture))
                .await
                .expect("publish Rust cross-language marker");
        }
        let _ = client.delete_bucket().bucket(&bucket).send().await;
        result.expect("Floci adapter contract");
    }

    async fn run_floci_contract(store: &S3Store) -> Result<(), String> {
        let content = b"immutable Floci object\n";
        let metadata = HashMap::from([("source".to_owned(), "conformance".to_owned())]);
        let receipt = store
            .put_immutable_with_metadata("test/small.ltx", Cursor::new(content), metadata.clone())
            .await
            .map_err(|error| error.to_string())?;
        if receipt.size != content.len() as u64 {
            return Err("single PUT returned incorrect object size".into());
        }
        store
            .put_immutable_with_metadata("test/small.ltx", Cursor::new(content), metadata.clone())
            .await
            .map_err(|error| format!("identical retry failed: {error}"))?;
        let metadata_conflict = store
            .put_immutable_with_metadata(
                "test/small.ltx",
                Cursor::new(content),
                HashMap::from([("source".to_owned(), "different".to_owned())]),
            )
            .await
            .expect_err("metadata differences must conflict");
        if metadata_conflict.kind != ErrorKind::Conflict {
            return Err("metadata conflict returned the wrong error class".into());
        }
        let conflict = store
            .put_immutable("test/small.ltx", Cursor::new(b"different"))
            .await
            .expect_err("different bytes must conflict");
        if conflict.kind != ErrorKind::Conflict {
            return Err("immutable conflict returned the wrong error class".into());
        }

        let head = store
            .head("test/small.ltx")
            .await
            .map_err(|error| error.to_string())?;
        if head
            .metadata()
            .and_then(|metadata| metadata.get("source"))
            .map(String::as_str)
            != Some("conformance")
        {
            return Err("HEAD did not preserve caller metadata".into());
        }
        if head.content_length() != Some(content.len() as i64) {
            return Err("HEAD returned incorrect content length".into());
        }
        let empty_receipt = store
            .put_immutable("test/empty.ltx", Cursor::new(&[]))
            .await
            .map_err(|error| format!("empty object PUT failed: {error}"))?;
        if empty_receipt.size != 0 || empty_receipt.sha256 != format!("{:x}", Sha256::digest([])) {
            return Err("empty object returned incorrect size or SHA-256".into());
        }
        let range = store
            .get_range("test/small.ltx", 0, 8)
            .await
            .map_err(|error| error.to_string())?;
        let mut range_body = range.body.into_async_read();
        let mut range_bytes = Vec::new();
        range_body
            .read_to_end(&mut range_bytes)
            .await
            .map_err(|error| error.to_string())?;
        if range_bytes != content[..9] {
            return Err("byte-range GET returned incorrect bytes".into());
        }

        let large = vec![0x5a; 8 * 1024 * 1024 + 128];
        let multipart_receipt = store
            .put_immutable("test/large.ltx", Cursor::new(&large))
            .await
            .map_err(|error| format!("multipart PUT failed: {error}"))?;
        if multipart_receipt.size != large.len() as u64 {
            return Err("multipart PUT returned incorrect size".into());
        }

        store
            .client
            .put_object()
            .bucket(&store.bucket)
            .key("test/divergent.ltx")
            .body(aws_sdk_s3::primitives::ByteStream::from(b"evil".to_vec()))
            .send()
            .await
            .map_err(|error| format!("create divergent reconciliation fixture: {error}"))?;
        store
            .client
            .put_object()
            .bucket(&store.bucket)
            .key("test/unexpected.ltx")
            .body(aws_sdk_s3::primitives::ByteStream::from(
                b"untracked".to_vec(),
            ))
            .send()
            .await
            .map_err(|error| format!("create unexpected reconciliation fixture: {error}"))?;
        let missing_hash = format!("{:x}", Sha256::digest(b"x"));
        let divergent_hash = format!("{:x}", Sha256::digest(b"safe"));
        let report = store
            .reconcile(
                "test/",
                &[
                    ObjectDescriptor {
                        key: "test/small.ltx".into(),
                        size: receipt.size,
                        sha256: receipt.sha256.clone(),
                    },
                    ObjectDescriptor {
                        key: "test/large.ltx".into(),
                        size: multipart_receipt.size,
                        sha256: multipart_receipt.sha256.clone(),
                    },
                    ObjectDescriptor {
                        key: "test/empty.ltx".into(),
                        size: empty_receipt.size,
                        sha256: empty_receipt.sha256.clone(),
                    },
                    ObjectDescriptor {
                        key: "test/missing.ltx".into(),
                        size: 1,
                        sha256: missing_hash,
                    },
                    ObjectDescriptor {
                        key: "test/divergent.ltx".into(),
                        size: 4,
                        sha256: divergent_hash,
                    },
                ],
            )
            .await
            .map_err(|error| format!("reconcile remote inventory: {error}"))?;
        if report.missing != ["test/missing.ltx"]
            || report.divergent != ["test/divergent.ltx"]
            || report.unexpected != ["test/unexpected.ltx"]
        {
            return Err(format!("unexpected reconciliation report: {report:?}"));
        }

        let first = store
            .list_page("test/", None, 1)
            .await
            .map_err(|error| error.to_string())?;
        if first.contents().len() != 1 || first.next_continuation_token().is_none() {
            return Err("first list page did not contain a cursor".into());
        }
        let second = store
            .list_page("test/", first.next_continuation_token(), 1)
            .await
            .map_err(|error| error.to_string())?;
        if second.contents().len() != 1 {
            return Err("second list page did not contain the remaining key".into());
        }
        store
            .delete_batch(&[
                "test/small.ltx".into(),
                "test/large.ltx".into(),
                "test/empty.ltx".into(),
                "test/divergent.ltx".into(),
                "test/unexpected.ltx".into(),
            ])
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}
