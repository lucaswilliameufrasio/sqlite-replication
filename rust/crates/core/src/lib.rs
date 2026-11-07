//! Synchronous SQLite snapshot, local WAL capture, and offline restore APIs.

// Recovery implementation details remain private behind the checked public API.
mod admission;
mod conservative_budget;
mod coordinator;
#[allow(dead_code)]
mod recovery;
#[allow(dead_code)]
mod repository_lock;

pub use admission::{AcceptedWrite, AdmissionGate, CapacityGate, CapacityPermit};
pub use conservative_budget::{BudgetReservation, ConservativeBudget};
pub use coordinator::{
    ComponentState, Coordinator, CoordinatorAdapter, CoordinatorConfig, CoordinatorMetrics,
    ErrorCategory, FlushOutcome, Health, Limits, Position, RemoteConfig, Status,
};

pub(crate) mod capture;

use std::fs::{self, File};
use std::io::{self, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlite_replication_ltx::{decode_snapshot, encode_snapshot};
use sqlite_replication_sqlite::{create_online_backup, verify_database};
use sqlite_replication_storage_fs::{
    ensure_repository, ltx_path, promote_if_missing, sync_directory,
};

/// A caller-provided gate that pauses all application writers before capture.
pub trait WriteGate {
    /// Permit type held until local LTX and its watermark are durable.
    type Permit: WritePermit;

    /// Stops application writes and returns a permit, or leaves the gate closed
    /// only if it reports an error according to the caller's own contract.
    fn acquire(&self) -> io::Result<Self::Permit>;
}

/// A held application write gate. Release must resume writers and report failure.
pub trait WritePermit {
    /// Resumes application writes after the capture result is durable or failed.
    fn release(self) -> io::Result<()>;
}

/// Durable local progress produced by one WAL capture cycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CaptureResult {
    pub txid: u64,
    pub transactions: usize,
    pub wal_frames: u32,
}

/// Selects an exact TXID or the latest verified point at or before a UTC time.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestoreTarget {
    Txid(u64),
    AtOrBeforeTimestampMillis(i64),
}

/// Reports a validated chain position and objects confirmed by immutable puts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyncResult {
    pub txid: u64,
    pub database_checksum: u64,
    /// Includes matching immutable objects that were already present remotely.
    pub objects_confirmed: usize,
    /// Total source bytes whose immutable object writes were confirmed.
    pub bytes_confirmed: u64,
}

/// Captures the currently observable WAL and durably publishes local LTX files.
///
/// The repository must already contain an initial snapshot. The caller must
/// route every application writer through `gate`; its permit remains held until
/// all captured LTX files and the local watermark are synced. This operation is
/// supported only on Linux x86-64 with the SQLite runtime validated by CP-01.
/// It does not request a checkpoint, upload to remote storage, or promise
/// survival of physical power loss. Application code is resumed after the local
/// durability boundary even when the request's cancellation token was set late.
pub fn capture_once<G: WriteGate>(
    source_path: impl AsRef<Path>,
    repository_root: impl AsRef<Path>,
    gate: &G,
) -> io::Result<CaptureResult> {
    capture_once_inner(source_path.as_ref(), repository_root.as_ref(), gate, None)
}

/// Experimental cooperative capture using one process-local logical budget.
///
/// The gate is acquired before the budget mutex and released after that mutex
/// is unlocked. Mutation invalidates occupancy; a capture or gate-release
/// error also triggers invalidation after the release callback, including when
/// active reservations prevented mutation. A gate acquisition error likewise
/// triggers invalidation. Callers must reconcile before
/// accepting further reservations. Every relevant producer and
/// occupancy mutation must use the same budget; external writers and bypasses
/// are not protected. This is not a filesystem quota or a production adapter.
/// The caller's gate must stop writers before returning from `acquire`.
/// Invalidation cannot revoke writes already accepted or prevent a later
/// independent reconciliation. The caller must coordinate reconciliation and
/// treat an unsuccessful capture or release as unsafe.
pub fn capture_once_with_budget<G: WriteGate>(
    source_path: impl AsRef<Path>,
    repository_root: impl AsRef<Path>,
    gate: &G,
    budget: &ConservativeBudget,
) -> io::Result<CaptureResult> {
    capture_once_inner(
        source_path.as_ref(),
        repository_root.as_ref(),
        gate,
        Some(budget),
    )
}

fn capture_once_inner<G: WriteGate>(
    source_path: &Path,
    repository_root: &Path,
    gate: &G,
    budget: Option<&ConservativeBudget>,
) -> io::Result<CaptureResult> {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "incremental capture is supported only on Linux x86-64",
        ));
    }

    let timestamp_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_millis() as i64;
    let permit = match gate.acquire() {
        Ok(permit) => permit,
        Err(acquire_error) => {
            if let Some(budget) = budget
                && let Err(invalidate_error) = budget.reconcile(None)
            {
                return Err(io::Error::new(
                    acquire_error.kind(),
                    format!(
                        "{acquire_error}; invalidating the budget also failed: {invalidate_error}"
                    ),
                ));
            }
            return Err(acquire_error);
        }
    };
    let cancelled = AtomicBool::new(false);
    let capture = || {
        capture::capture_once_cancellable(
            source_path,
            repository_root,
            timestamp_millis,
            &cancelled,
            |_, _| Ok(()),
        )
    };
    let mut capture_result = match budget {
        Some(budget) => budget.with_occupancy_mutation(capture),
        None => capture(),
    };
    let mut release_result = permit.release();
    if (capture_result.is_err() || release_result.is_err())
        && let Some(budget) = budget
        && let Err(invalidate_error) = budget.reconcile(None)
    {
        if let Err(release_error) = &release_result {
            release_result = Err(io::Error::other(format!(
                "{release_error}; invalidating the budget also failed: {invalidate_error}"
            )));
        } else if let Err(capture_error) = &capture_result {
            capture_result = Err(io::Error::new(
                capture_error.kind(),
                format!("{capture_error}; invalidating the budget also failed: {invalidate_error}"),
            ));
        }
    }

    let result = match (capture_result, release_result) {
        (Ok(result), Ok(())) => result,
        (Err(capture_error), Ok(())) => return Err(capture_error),
        (Ok(_), Err(release_error)) => {
            return Err(io::Error::other(format!(
                "capture completed durably but releasing the application write gate failed: {release_error}"
            )));
        }
        (Err(capture_error), Err(release_error)) => {
            return Err(io::Error::other(format!(
                "capture failed: {capture_error}; releasing the application write gate also failed: {release_error}"
            )));
        }
    };

    Ok(CaptureResult {
        txid: result.state.database.txid,
        transactions: result.transactions.len(),
        wal_frames: result.wal_frames,
    })
}

/// Creates and publishes the initial immutable LTX v2 snapshot.
pub fn snapshot(
    database_path: impl AsRef<Path>,
    repository_root: impl AsRef<Path>,
) -> io::Result<PathBuf> {
    let database_path = database_path.as_ref();
    let repository_root = repository_root.as_ref();
    ensure_repository(repository_root)?;

    let staging_directory = repository_root.join(".staging");
    fs::create_dir_all(&staging_directory)?;
    let database_stage = tempfile::Builder::new()
        .prefix(".snapshot-")
        .suffix(".sqlite")
        .tempfile_in(&staging_directory)?;
    create_online_backup(database_path, database_stage.path())?;
    database_stage.as_file().sync_all()?;

    let mut database_image = File::open(database_stage.path())?;
    let destination = ltx_path(repository_root, 1, 1);
    let parent = destination
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "LTX path has no parent"))?;
    fs::create_dir_all(parent)?;
    let ltx_stage = tempfile::Builder::new()
        .prefix(".snapshot-")
        .suffix(".ltx")
        .tempfile_in(parent)?;
    let mut output = ltx_stage.reopen()?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_millis() as i64;
    encode_snapshot(&mut database_image, &mut output, timestamp)?;
    output.sync_all()?;
    drop(output);
    let (stage_file, stage_path) = ltx_stage.keep().map_err(|error| error.error)?;
    stage_file.sync_all()?;
    drop(stage_file);
    sqlite_replication_storage_fs::publish(&stage_path, &destination)?;
    Ok(destination)
}

/// Restores the initial snapshot to a path that must not already exist.
pub fn restore_if_missing(
    repository_root: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> io::Result<()> {
    let repository_root = repository_root.as_ref();
    let destination = destination.as_ref();
    sqlite_replication_storage_fs::read_repository(repository_root)?;
    let source = File::open(ltx_path(repository_root, 1, 1))?;
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    reject_existing_sidecars(destination)?;
    let mut restored = tempfile::Builder::new()
        .prefix(".restore-")
        .suffix(".sqlite")
        .tempfile_in(parent)?;
    let header = decode_snapshot(source, &mut restored)?;
    restored.as_file().sync_all()?;
    let expected_size = u64::from(header.page_size) * u64::from(header.commit);
    if restored.as_file().metadata()?.len() != expected_size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "restored SQLite image has an invalid size",
        ));
    }
    verify_database(restored.path())?;
    remove_sqlite_sidecars(restored.path())?;
    restored.as_file_mut().seek(SeekFrom::Start(0))?;
    promote_if_missing(restored, destination)?;
    sync_directory(parent)
}

/// Restores and verifies an incremental snapshot-plus-delta chain to a new path.
/// Existing database files and stale SQLite sidecars are never overwritten.
pub fn restore_to_target(
    repository_root: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    target: RestoreTarget,
) -> io::Result<()> {
    let target = match target {
        RestoreTarget::Txid(txid) => recovery::RestoreTarget::Txid(txid),
        RestoreTarget::AtOrBeforeTimestampMillis(timestamp) => {
            recovery::RestoreTarget::AtOrBeforeTimestampMillis(timestamp)
        }
    };
    recovery::restore_to_target(repository_root.as_ref(), destination.as_ref(), target)
}

/// Stages and verifies an S3-compatible repository before restoring a point.
/// Floci is the only tested implementation; other providers are not implied to
/// be compatible. Local temporary space proportional to the remote repository
/// is required. Existing database files and sidecars are never overwritten.
pub async fn restore_from_s3(
    store: &sqlite_replication_storage_s3::S3Store,
    prefix: &str,
    destination: impl AsRef<Path>,
    target: RestoreTarget,
) -> io::Result<()> {
    let target = match target {
        RestoreTarget::Txid(txid) => recovery::RestoreTarget::Txid(txid),
        RestoreTarget::AtOrBeforeTimestampMillis(timestamp) => {
            recovery::RestoreTarget::AtOrBeforeTimestampMillis(timestamp)
        }
    };
    recovery::restore_s3_to_target(store, prefix, destination.as_ref(), target).await
}

/// Uploads a validated local LTX chain with immutable keys and persists a
/// destination-scoped acknowledgement after all object writes are confirmed.
/// `remote_id` must be stable, non-secret, and unique to the configured
/// endpoint. Retries upload idempotently and never delete local or remote data.
pub async fn sync_to_s3(
    store: &sqlite_replication_storage_s3::S3Store,
    repository_root: impl AsRef<Path>,
    prefix: &str,
    remote_id: &str,
) -> io::Result<SyncResult> {
    let result =
        recovery::sync_filesystem_to_s3(store, repository_root.as_ref(), prefix, remote_id).await?;
    Ok(SyncResult {
        txid: result.txid,
        database_checksum: result.database_checksum,
        objects_confirmed: result.objects_confirmed,
        bytes_confirmed: result.bytes_confirmed,
    })
}

/// Returns the filesystem LTX path for the initial snapshot.
pub fn initial_snapshot_path(repository_root: impl AsRef<Path>) -> PathBuf {
    ltx_path(repository_root.as_ref(), 1, 1)
}

fn remove_sqlite_sidecars(database_path: &Path) -> io::Result<()> {
    for suffix in ["-wal", "-shm"] {
        let mut sidecar = database_path.as_os_str().to_os_string();
        sidecar.push(suffix);
        let sidecar = PathBuf::from(sidecar);
        match fs::remove_file(sidecar) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn reject_existing_sidecars(database_path: &Path) -> io::Result<()> {
    for suffix in ["-wal", "-shm"] {
        let mut sidecar = database_path.as_os_str().to_os_string();
        sidecar.push(suffix);
        match fs::symlink_metadata(PathBuf::from(sidecar)) {
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("restore destination has a stale SQLite {suffix} sidecar"),
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::Connection;

    use super::{restore_if_missing, snapshot};

    #[test]
    fn snapshot_and_restore_verify_sqlite_and_do_not_overwrite() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source_path = directory.path().join("source.sqlite");
        let repository_path = directory.path().join("repository");
        let restored_path = directory.path().join("restored.sqlite");
        create_database(&source_path);

        let ltx_path = snapshot(&source_path, &repository_path).expect("create snapshot");
        assert!(ltx_path.exists());
        restore_if_missing(&repository_path, &restored_path).expect("restore snapshot");
        let restored = Connection::open(&restored_path).expect("open restored database");
        let count: i64 = restored
            .query_row("SELECT count(*) FROM records", [], |row| row.get(0))
            .expect("count restored rows");
        assert_eq!(count, 128);

        assert!(restore_if_missing(&repository_path, &restored_path).is_err());
        assert_eq!(
            Connection::open(&restored_path)
                .expect("reopen restored database")
                .query_row("SELECT count(*) FROM records", [], |row| row
                    .get::<_, i64>(0))
                .expect("count rows after rejected overwrite"),
            128
        );
    }

    #[test]
    fn corrupt_snapshot_does_not_promote_partial_database() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source_path = directory.path().join("source.sqlite");
        let repository_path = directory.path().join("repository");
        let destination = directory.path().join("not-created.sqlite");
        create_database(&source_path);
        let ltx_path = snapshot(&source_path, &repository_path).expect("create snapshot");
        let mut content = fs::read(&ltx_path).expect("read snapshot");
        *content.last_mut().expect("LTX trailer exists") ^= 1;
        fs::write(ltx_path, content).expect("corrupt snapshot");

        assert!(restore_if_missing(&repository_path, &destination).is_err());
        assert!(!destination.exists());
    }

    #[test]
    fn stale_destination_sidecars_reject_restore_without_modification() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source_path = directory.path().join("source.sqlite");
        let repository_path = directory.path().join("repository");
        let destination = directory.path().join("restored.sqlite");
        let stale_wal = directory.path().join("restored.sqlite-wal");
        create_database(&source_path);
        snapshot(&source_path, &repository_path).expect("create snapshot");
        fs::write(&stale_wal, b"stale sidecar").expect("write stale sidecar");

        assert!(restore_if_missing(&repository_path, &destination).is_err());
        assert!(!destination.exists());
        assert_eq!(
            fs::read(stale_wal).expect("read stale sidecar"),
            b"stale sidecar"
        );
    }

    fn create_database(path: &std::path::Path) {
        let connection = Connection::open(path).expect("open source database");
        connection
            .execute_batch(
                "PRAGMA journal_mode=WAL;
                 CREATE TABLE records (id INTEGER PRIMARY KEY, payload TEXT NOT NULL);",
            )
            .expect("create source table");
        let transaction = connection
            .unchecked_transaction()
            .expect("begin transaction");
        for id in 1..=128 {
            let payload = format!("value-{id:04}-{}", "x".repeat(1024));
            transaction
                .execute(
                    "INSERT INTO records(id, payload) VALUES (?1, ?2)",
                    (id, payload),
                )
                .expect("insert source row");
        }
        transaction.commit().expect("commit source rows");
    }
}
