use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sqlite_replication_spool::{read_remote_ack, recover};
use sqlite_replication_storage_s3::S3Store;
use tokio::sync::{Mutex, mpsc, watch};

use crate::recovery::{RestoreTarget, build_restore_plan_from_filesystem};
use crate::{AdmissionGate, CaptureResult, sync_to_s3};

pub trait CoordinatorAdapter: Send + Sync + 'static {
    /// Returns the adapter's observed position, or `None` if unknown.
    /// A known TXID must be nonzero.
    /// An adapter unable to establish a trustworthy position must return `None`.
    fn observe_txid(&self) -> io::Result<Option<u64>>;
    /// Durably captures the consistent state of all drained cooperative writers.
    /// A no-op capture returns the current nonzero durable TXID. Implementations
    /// must not report a no-op when accepted changes remain uncaptured: matching
    /// the returned TXID to the spool alone cannot verify the source state.
    fn capture(&self) -> io::Result<CaptureResult>;
    /// Applies limits atomically with write admission. Enabling must reserve
    /// enough capacity to prevent writes exceeding `limits` and leave admission
    /// closed if it returns an error; disabling must stop new writes before
    /// returning successfully.
    fn set_write_admission(&self, enabled: bool, limits: Limits) -> io::Result<()>;
}

/// Bounded result label for a completed flush future. `ReturnedOk` does not
/// imply admission reopened: a worker can return `Ok` after observing cancellation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlushOutcome {
    ReturnedOk,
    ReturnedError,
}

/// Called synchronously after the flush lock is released. Implementations must
/// be fast, concurrency-safe, and must not panic. Duration includes lock wait,
/// but excludes this callback. Dropped futures do not generate an observation.
pub trait CoordinatorMetrics: Send + Sync + 'static {
    fn observe_flush(&self, duration: Duration, outcome: FlushOutcome);
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
    flush_lock: Arc<Mutex<()>>,
    metrics: Option<Arc<dyn CoordinatorMetrics>>,
    admission: AdmissionGate,
    run_active: AtomicBool,
}

struct RunCancellationGuard<'a> {
    admission: AdmissionGate,
    active: &'a AtomicBool,
}

impl Drop for RunCancellationGuard<'_> {
    fn drop(&mut self) {
        let _ = self.admission.cancel();
        self.active.store(false, Ordering::Release);
    }
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
    fn observe_txid(&self) -> io::Result<Option<u64>> {
        match self.config.adapter.observe_txid()? {
            Some(0) => Err(invalid_data("observed TXID must be nonzero")),
            observed => Ok(observed),
        }
    }

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
        if let Some(remote) = &config.remote
            && remote.remote_id.trim().is_empty()
        {
            return Err(invalid_input("a stable remote ID is required"));
        }
        Ok(Self {
            config,
            flush_lock: Arc::new(Mutex::new(())),
            metrics: None,
            admission: AdmissionGate::new(),
            run_active: AtomicBool::new(false),
        })
    }

    /// Gate shared by the cancellation controller and every cooperating writer.
    /// Writers must use this same instance at their actual acceptance boundary.
    pub fn admission_gate(&self) -> AdmissionGate {
        self.admission.clone()
    }

    /// Installs an optional observer without requiring an exporter or server.
    pub fn with_metrics(mut self, metrics: Arc<dyn CoordinatorMetrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Reports available positions separately; absent values remain `None`.
    pub fn status(&self) -> io::Result<Status> {
        self.status_internal().map_err(|(_, error)| error)
    }

    fn status_internal(&self) -> Result<Status, (ErrorCategory, io::Error)> {
        let observed_txid = self
            .observe_txid()
            .map_err(|error| (ErrorCategory::Adapter, error))?;
        let _repository_lock = crate::repository_lock::acquire(&self.config.repository_root)
            .map_err(|error| (ErrorCategory::LocalStorage, error))?;
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
        if status.spool_bytes >= self.config.limits.max_spool_bytes
            || status.wal_bytes >= self.config.limits.max_wal_bytes
            || status.unprotected_age > self.config.limits.max_unprotected_age
        {
            health.local = ComponentState::Degraded;
            health.local_error = ErrorCategory::Backpressure;
        }
        if status.observed_txid.is_none() {
            health.local = ComponentState::Degraded;
            health.local_error = ErrorCategory::Adapter;
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

    /// Drains accepted writers and captures a consistent state. Any error leaves write admission
    /// closed; success reopens it only after configured limits are satisfied.
    pub async fn flush(&self) -> io::Result<()> {
        self.flush_internal(None).await
    }

    async fn flush_internal(&self, cancellation: Option<&watch::Receiver<bool>>) -> io::Result<()> {
        let started = Instant::now();
        let result = self.flush_locked(cancellation).await;
        if let Some(metrics) = &self.metrics {
            let outcome = if result.is_ok() {
                FlushOutcome::ReturnedOk
            } else {
                FlushOutcome::ReturnedError
            };
            metrics.observe_flush(started.elapsed(), outcome);
        }
        result
    }

    async fn flush_locked(&self, cancellation: Option<&watch::Receiver<bool>>) -> io::Result<()> {
        let guard = Arc::clone(&self.flush_lock).lock_owned().await;
        self.admission.close()?;
        self.config
            .adapter
            .set_write_admission(false, self.config.limits)?;
        if let Some(cancellation) = cancellation {
            let mut cancellation = cancellation.clone();
            loop {
                if *cancellation.borrow() || cancellation.has_changed().is_err() {
                    return Ok(());
                }
                tokio::select! {
                    result = self.admission.drain() => {
                        result?;
                        break;
                    }
                    changed = cancellation.changed() => {
                        if changed.is_err() || *cancellation.borrow() {
                            return Ok(());
                        }
                    }
                }
            }
        } else {
            self.admission.drain().await?;
        }
        let previous_txid = {
            let _repository_lock = crate::repository_lock::acquire(&self.config.repository_root)?;
            recover(&self.config.repository_root, &[])?.database.txid
        };
        let adapter = Arc::clone(&self.config.adapter);
        let (captured, _guard) = tokio::task::spawn_blocking(move || {
            let captured = adapter.capture();
            (captured, guard)
        })
        .await
        .map_err(io::Error::other)?;
        let captured = captured?;
        let target = captured.txid;
        if target == 0 {
            return Err(invalid_data("capture returned a zero durable position"));
        }
        let status = self.status()?;
        if target < previous_txid
            || status
                .local_durable
                .is_none_or(|position| position.txid != target)
        {
            return Err(invalid_data(
                "capture position does not match durable spool",
            ));
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
        let status = self.status()?;
        if status
            .local_durable
            .is_none_or(|position| position.txid < target)
        {
            return Err(invalid_data(
                "local durable position did not reach Flush target",
            ));
        }
        if status.spool_bytes >= self.config.limits.max_spool_bytes
            || status.wal_bytes >= self.config.limits.max_wal_bytes
            || (status.observed_txid.is_some()
                && status.unprotected_age > self.config.limits.max_unprotected_age)
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
        if let Some(cancellation) = cancellation
            && (*cancellation.borrow() || cancellation.has_changed().is_err())
        {
            return Ok(());
        }
        self.config
            .adapter
            .set_write_admission(true, self.config.limits)?;
        if let Some(cancellation) = cancellation
            && (*cancellation.borrow() || cancellation.has_changed().is_err())
        {
            self.admission.cancel()?;
            self.config
                .adapter
                .set_write_admission(false, self.config.limits)?;
            return Ok(());
        }
        if self.admission.is_cancelled()? {
            self.config
                .adapter
                .set_write_admission(false, self.config.limits)?;
            return Ok(());
        }
        match self.admission.open() {
            Ok(()) => Ok(()),
            Err(error) => {
                let close_result = self
                    .config
                    .adapter
                    .set_write_admission(false, self.config.limits);
                if self.admission.is_cancelled()? {
                    close_result
                } else {
                    Err(join_io(error, close_result))
                }
            }
        }
    }

    /// Runs with cancellation and cooperating writers using the shared gate.
    /// A write accepted before cancellation may still complete afterwards.
    pub async fn run(&self, events: &mut mpsc::Receiver<()>) -> io::Result<()> {
        let _guard = self.begin_run()?;
        let mut cancellation = self.admission.subscribe();
        let result = self.run_inner(events, &mut cancellation).await;
        self.admission.cancel()?;
        result
    }

    /// Runs an initial reconciliation with a legacy independent cancellation
    /// receiver. This receiver alone cannot guarantee strict write admission;
    /// use `run` and the shared gate for cooperating writers.
    #[deprecated(note = "use run with AdmissionGate::cancel for coordinated cancellation")]
    pub async fn run_legacy(
        &self,
        events: &mut mpsc::Receiver<()>,
        cancellation: &mut watch::Receiver<bool>,
    ) -> io::Result<()> {
        let _guard = self.begin_run()?;
        self.run_inner(events, cancellation).await
    }

    fn begin_run(&self) -> io::Result<RunCancellationGuard<'_>> {
        if self
            .run_active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "a coordinator run is already active",
            ));
        }
        Ok(RunCancellationGuard {
            admission: self.admission.clone(),
            active: &self.run_active,
        })
    }

    async fn run_inner(
        &self,
        events: &mut mpsc::Receiver<()>,
        cancellation: &mut watch::Receiver<bool>,
    ) -> io::Result<()> {
        self.admission.close()?;
        self.config
            .adapter
            .set_write_admission(false, self.config.limits)?;
        if *cancellation.borrow() {
            self.admission.cancel()?;
            return Ok(());
        }
        self.flush_internal(Some(cancellation)).await?;
        loop {
            tokio::select! {
                changed = cancellation.changed() => {
                    if changed.is_err() || *cancellation.borrow() {
                        self.admission.cancel()?;
                        self.config.adapter.set_write_admission(false, self.config.limits)?;
                        return Ok(());
                    }
                }
                event = events.recv() => {
                    if event.is_none() {
                        self.admission.cancel()?;
                        self.config.adapter.set_write_admission(false, self.config.limits)?;
                        return Ok(());
                    }
                    self.flush_internal(Some(cancellation)).await?;
                }
            }
        }
    }
}

fn spool_bytes(root: &Path) -> io::Result<u64> {
    match fs::metadata(root) {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
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

#[cfg(all(test, unix))]
mod spool_measurement_tests {
    use super::spool_bytes;
    use std::os::unix::fs::symlink;

    #[test]
    fn spool_measurement_does_not_treat_metadata_errors_as_empty() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let root = directory.path().join("loop");
        symlink("loop", &root).expect("create symlink loop");
        assert!(spool_bytes(&root).is_err());
        assert_eq!(spool_bytes(&directory.path().join("absent")).unwrap(), 0);
    }
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
