use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sqlite_replication_spool::{read_remote_ack, recover};
use sqlite_replication_storage_s3::S3Store;
use tokio::sync::{Mutex, mpsc, watch};

use crate::recovery::{RestoreTarget, build_restore_plan_from_filesystem};
use crate::{CaptureResult, sync_to_s3};

pub trait CoordinatorAdapter: Send + Sync + 'static {
    fn observe_txid(&self) -> io::Result<Option<u64>>;
    fn capture(&self) -> io::Result<CaptureResult>;
    /// Applies limits atomically with write admission. Enabling must reserve
    /// enough capacity to prevent writes exceeding `limits` and leave admission
    /// closed if it returns an error; disabling must stop new writes before
    /// returning successfully.
    fn set_write_admission(&self, enabled: bool, limits: Limits) -> io::Result<()>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    pub max_spool_bytes: u64,
    pub max_wal_bytes: u64,
    pub max_unprotected_age: Duration,
    pub max_uploads_in_flight: usize,
}

#[derive(Clone)]
pub struct RemoteConfig {
    pub store: S3Store,
    pub prefix: String,
    pub remote_id: String,
}

pub struct CoordinatorConfig {
    pub repository_root: PathBuf,
    pub database_path: PathBuf,
    pub adapter: Arc<dyn CoordinatorAdapter>,
    pub limits: Limits,
    pub remote: Option<RemoteConfig>,
}

pub struct Coordinator {
    config: CoordinatorConfig,
    flush_lock: Mutex<()>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Position {
    pub txid: u64,
    pub database_checksum: u64,
    pub timestamp_millis: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Status {
    pub observed_txid: Option<u64>,
    pub local_durable: Option<Position>,
    pub remote_confirmed: Option<Position>,
    pub remote_ack_matches: bool,
    pub spool_bytes: u64,
    pub wal_bytes: u64,
    pub unprotected_age: Duration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComponentState {
    Unknown,
    Healthy,
    Degraded,
    NeedsOperator,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCategory {
    None,
    Adapter,
    LocalStorage,
    RemoteStorage,
    Backpressure,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Health {
    pub local: ComponentState,
    pub remote: ComponentState,
    pub local_error: ErrorCategory,
    pub remote_error: ErrorCategory,
}

impl Coordinator {
    pub fn new(config: CoordinatorConfig) -> io::Result<Self> {
        if config.repository_root.as_os_str().is_empty()
            || config.database_path.as_os_str().is_empty()
        {
            return Err(invalid_input(
                "repository root and database path are required",
            ));
        }
        if config.limits.max_spool_bytes == 0
            || config.limits.max_wal_bytes == 0
            || config.limits.max_unprotected_age.is_zero()
            || config.limits.max_uploads_in_flight == 0
        {
            return Err(invalid_input(
                "all replication limits must be explicitly positive",
            ));
        }
        if let Some(remote) = &config.remote {
            if remote.remote_id.trim().is_empty() {
                return Err(invalid_input("a stable remote ID is required"));
            }
        }
        Ok(Self {
            config,
            flush_lock: Mutex::new(()),
        })
    }

    /// Reports available positions separately; absent values remain `None`.
    pub fn status(&self) -> io::Result<Status> {
        self.status_internal().map_err(|(_, error)| error)
    }

    fn status_internal(&self) -> Result<Status, (ErrorCategory, io::Error)> {
        let observed_txid = self
            .config
            .adapter
            .observe_txid()
            .map_err(|error| (ErrorCategory::Adapter, error))?;
        let state = recover(&self.config.repository_root, &[])
            .map_err(|error| (ErrorCategory::LocalStorage, error))?;
        let plan = build_restore_plan_from_filesystem(
            &self.config.repository_root,
            RestoreTarget::Txid(state.database.txid),
        )
        .map_err(|error| (ErrorCategory::LocalStorage, error))?;
        let local_durable = Some(Position {
            txid: state.database.txid,
            database_checksum: state.database.checksum,
            timestamp_millis: Some(plan.timestamp_millis),
        });
        let spool_bytes = spool_bytes(&self.config.repository_root.join("ltx"))
            .map_err(|error| (ErrorCategory::LocalStorage, error))?;
        let wal_bytes = file_size_if_present(&wal_path(&self.config.database_path))
            .map_err(|error| (ErrorCategory::LocalStorage, error))?;
        let unprotected_age = match (observed_txid, local_durable) {
            (Some(observed), Some(local)) if observed <= local.txid => Duration::ZERO,
            (_, Some(local)) => age_since_millis(local.timestamp_millis),
            _ => Duration::MAX,
        };
        let (remote_confirmed, remote_ack_matches) = match &self.config.remote {
            Some(remote) => match read_remote_ack(&self.config.repository_root) {
                Ok(ack) if ack.remote_id == remote.remote_id && ack.prefix == remote.prefix => (
                    Some(Position {
                        txid: ack.txid,
                        database_checksum: ack.database_checksum,
                        timestamp_millis: None,
                    }),
                    true,
                ),
                _ => (None, false),
            },
            None => (None, false),
        };
        Ok(Status {
            observed_txid,
            local_durable,
            remote_confirmed,
            remote_ack_matches,
            spool_bytes,
            wal_bytes,
            unprotected_age,
        })
    }

    /// Probes component availability without deriving application liveness.
    pub async fn health(&self) -> Health {
        let status = match self.status_internal() {
            Ok(status) => status,
            Err((category, _)) => {
                return Health {
                    local: match category {
                        ErrorCategory::Adapter => ComponentState::Degraded,
                        ErrorCategory::Cancelled => ComponentState::Unknown,
                        _ => ComponentState::NeedsOperator,
                    },
                    remote: ComponentState::Unknown,
                    local_error: category,
                    remote_error: ErrorCategory::None,
                };
            }
        };
        let mut health = Health {
            local: ComponentState::Healthy,
            remote: ComponentState::Unknown,
            local_error: ErrorCategory::None,
            remote_error: ErrorCategory::None,
        };
        if status.spool_bytes > self.config.limits.max_spool_bytes
            || status.wal_bytes > self.config.limits.max_wal_bytes
            || status.unprotected_age > self.config.limits.max_unprotected_age
        {
            health.local = ComponentState::Degraded;
            health.local_error = ErrorCategory::Backpressure;
        }
        if let Some(remote) = &self.config.remote {
            let key = remote_key(&remote.prefix, "_meta/repository.json");
            if remote.store.head(&key).await.is_err() {
                health.remote = ComponentState::Degraded;
                health.remote_error = ErrorCategory::RemoteStorage;
            } else {
                health.remote = ComponentState::Healthy;
            }
        }
        health
    }

    /// Flushes the TXID observed at call start. Any error leaves write admission
    /// closed; success reopens it only after configured limits are satisfied.
    pub async fn flush(&self) -> io::Result<()> {
        let _guard = self.flush_lock.lock().await;
        let target = match self.config.adapter.observe_txid() {
            Ok(Some(target)) => target,
            Ok(None) => {
                let close_result = self
                    .config
                    .adapter
                    .set_write_admission(false, self.config.limits);
                close_result?;
                return Err(invalid_data("Flush target is unknown"));
            }
            Err(error) => {
                let close_result = self
                    .config
                    .adapter
                    .set_write_admission(false, self.config.limits);
                return Err(join_io(error, close_result));
            }
        };
        self.config
            .adapter
            .set_write_admission(false, self.config.limits)?;
        let mut status = self.status()?;
        let mut local_txid = status
            .local_durable
            .map(|position| position.txid)
            .unwrap_or(0);
        while local_txid < target {
            let adapter = Arc::clone(&self.config.adapter);
            let captured = tokio::task::spawn_blocking(move || adapter.capture())
                .await
                .map_err(io::Error::other)??;
            if captured.txid <= local_txid {
                return Err(invalid_data("capture did not advance the durable position"));
            }
            local_txid = captured.txid;
        }
        if let Some(remote) = &self.config.remote {
            let result = sync_to_s3(
                &remote.store,
                &self.config.repository_root,
                &remote.prefix,
                &remote.remote_id,
            )
            .await?;
            if result.txid < target {
                return Err(invalid_data(
                    "remote acknowledgement did not reach Flush target",
                ));
            }
        }
        status = self.status()?;
        if status
            .local_durable
            .is_none_or(|position| position.txid < target)
        {
            return Err(invalid_data(
                "local durable position did not reach Flush target",
            ));
        }
        if status.spool_bytes > self.config.limits.max_spool_bytes
            || status.wal_bytes > self.config.limits.max_wal_bytes
            || status.unprotected_age > self.config.limits.max_unprotected_age
        {
            return Err(io::Error::other(
                "replication limits exceeded; writes remain blocked",
            ));
        }
        if self.config.remote.is_some()
            && status
                .remote_confirmed
                .is_none_or(|position| position.txid < target)
        {
            return Err(invalid_data("matching remote acknowledgement is missing"));
        }
        self.config
            .adapter
            .set_write_admission(true, self.config.limits)
    }

    /// Runs an initial reconciliation, then flushes on coalescible adapter
    /// events. The caller must start with writes denied; cancellation or a
    /// closed event stream leaves admission closed.
    pub async fn run(
        &self,
        events: &mut mpsc::Receiver<()>,
        cancellation: &mut watch::Receiver<bool>,
    ) -> io::Result<()> {
        self.config
            .adapter
            .set_write_admission(false, self.config.limits)?;
        self.flush().await?;
        loop {
            tokio::select! {
                changed = cancellation.changed() => {
                    if changed.is_err() || *cancellation.borrow() {
                        self.config.adapter.set_write_admission(false, self.config.limits)?;
                        return Ok(());
                    }
                }
                event = events.recv() => {
                    if event.is_none() {
                        self.config.adapter.set_write_admission(false, self.config.limits)?;
                        return Ok(());
                    }
                    self.flush().await?;
                }
            }
        }
    }
}

fn spool_bytes(root: &Path) -> io::Result<u64> {
    if !root.exists() {
        return Ok(0);
    }
    let mut total = 0_u64;
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file() {
                total = total
                    .checked_add(entry.metadata()?.len())
                    .ok_or_else(|| invalid_data("spool byte count overflow"))?;
            } else {
                return Err(invalid_data("spool contains a non-regular file"));
            }
        }
    }
    Ok(total)
}

fn file_size_if_present(path: &Path) -> io::Result<u64> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(metadata.len()),
        Ok(_) => Err(invalid_data("SQLite WAL path is not a regular file")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error),
    }
}

fn wal_path(database_path: &Path) -> PathBuf {
    let mut value = database_path.as_os_str().to_os_string();
    value.push("-wal");
    PathBuf::from(value)
}

fn age_since_millis(timestamp_millis: Option<i64>) -> Duration {
    let Some(timestamp_millis) = timestamp_millis else {
        return Duration::MAX;
    };
    let Ok(timestamp) = u64::try_from(timestamp_millis) else {
        return Duration::MAX;
    };
    let timestamp = UNIX_EPOCH + Duration::from_millis(timestamp);
    SystemTime::now()
        .duration_since(timestamp)
        .unwrap_or(Duration::ZERO)
}

fn remote_key(prefix: &str, suffix: &str) -> String {
    if prefix.is_empty() {
        suffix.to_owned()
    } else {
        format!("{prefix}/{suffix}")
    }
}

fn invalid_input(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn invalid_data(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn join_io(primary: io::Error, secondary: io::Result<()>) -> io::Error {
    match secondary {
        Ok(()) => primary,
        Err(secondary) => io::Error::other(format!(
            "{primary}; write admission closure also failed: {secondary}"
        )),
    }
}
