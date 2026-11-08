//! Internal WAL capture orchestration. This remains unavailable as public API
//! until the full CP-01 matrix is reviewed and approved.

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
#[path = "capture_overlap_test.rs"]
mod capture_overlap_test;

use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use sqlite_replication_ltx::{DatabaseState, decode_delta, encode_delta};
use sqlite_replication_spool::{self, FaultPoint as SpoolFaultPoint};
use sqlite_replication_sqlite::{
    CaptureLease, begin_capture_lease, create_online_backup, create_online_backup_cancellable,
    create_online_backup_cancellable_with_image_limit,
};
use sqlite_replication_wal::scan;
use sqlite_replication_wal::translate::{State, Transaction, position_from_wal, translate_wal};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FaultPoint {
    AfterLeaseAcquire,
    AfterWalRead,
    AfterSourceSnapshot,
    AfterWalTranslation,
    Spool(SpoolFaultPoint),
    AfterSpoolPublication,
}

pub(crate) struct Result {
    pub transactions: Vec<Transaction>,
    pub state: State,
    pub wal_frames: u32,
}

#[cfg(test)]
fn capture_once(
    source_path: &Path,
    repository_root: &Path,
    timestamp_millis: i64,
) -> io::Result<Result> {
    capture_once_with_hook(
        source_path,
        repository_root,
        timestamp_millis,
        |_, _| Ok(()),
    )
}

#[cfg(test)]
fn capture_once_with_hook<F>(
    source_path: &Path,
    repository_root: &Path,
    timestamp_millis: i64,
    hook: F,
) -> io::Result<Result>
where
    F: FnMut(FaultPoint, usize) -> io::Result<()>,
{
    capture_once_with_cancellation(
        source_path,
        repository_root,
        timestamp_millis,
        None,
        (None, None, None, None),
        hook,
    )
}

pub(crate) fn capture_once_cancellable<F>(
    source_path: &Path,
    repository_root: &Path,
    timestamp_millis: i64,
    cancelled: &AtomicBool,
    hook: F,
) -> io::Result<Result>
where
    F: FnMut(FaultPoint, usize) -> io::Result<()>,
{
    capture_once_with_cancellation(
        source_path,
        repository_root,
        timestamp_millis,
        Some(cancelled),
        (None, None, None, None),
        hook,
    )
}

pub(crate) fn capture_once_cancellable_with_recovery_limit<F>(
    source_path: &Path,
    repository_root: &Path,
    timestamp_millis: i64,
    cancelled: &AtomicBool,
    recovery_limit: u64,
    limits: (Option<u64>, Option<u64>, Option<u64>),
    hook: F,
) -> io::Result<Result>
where
    F: FnMut(FaultPoint, usize) -> io::Result<()>,
{
    capture_once_with_cancellation(
        source_path,
        repository_root,
        timestamp_millis,
        Some(cancelled),
        (Some(recovery_limit), limits.0, limits.1, limits.2),
        hook,
    )
}

fn capture_once_with_cancellation<F>(
    source_path: &Path,
    repository_root: &Path,
    timestamp_millis: i64,
    cancelled: Option<&AtomicBool>,
    image_limits: (Option<u64>, Option<u64>, Option<u64>, Option<u64>),
    hook: F,
) -> io::Result<Result>
where
    F: FnMut(FaultPoint, usize) -> io::Result<()>,
{
    check_cancelled(cancelled)?;
    let _repository_lock = crate::repository_lock::acquire(repository_root)?;
    capture_once_with_cancellation_locked(
        source_path,
        repository_root,
        timestamp_millis,
        cancelled,
        image_limits,
        hook,
    )
}

// The caller must hold the matching repository coordination lock.
pub(crate) fn capture_once_with_cancellation_locked<F>(
    source_path: &Path,
    repository_root: &Path,
    timestamp_millis: i64,
    cancelled: Option<&AtomicBool>,
    image_limits: (Option<u64>, Option<u64>, Option<u64>, Option<u64>),
    mut hook: F,
) -> io::Result<Result>
where
    F: FnMut(FaultPoint, usize) -> io::Result<()>,
{
    check_cancelled(cancelled)?;
    let lease = begin_capture_lease(source_path)?;
    let capture_result = capture_under_lease(
        &lease,
        source_path,
        repository_root,
        timestamp_millis,
        cancelled,
        image_limits,
        &mut hook,
    );
    let release_result = lease.release();
    match (capture_result, release_result) {
        (Ok(result), Ok(())) => Ok(result),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(io::Error::other(format!(
            "capture succeeded but SQLite lease release failed: {error}"
        ))),
        (Err(capture_error), Err(release_error)) => Err(io::Error::other(format!(
            "capture failed: {capture_error}; lease release also failed: {release_error}"
        ))),
    }
}

fn capture_under_lease<F>(
    lease: &CaptureLease,
    source_path: &Path,
    repository_root: &Path,
    timestamp_millis: i64,
    cancelled: Option<&AtomicBool>,
    image_limits: (Option<u64>, Option<u64>, Option<u64>, Option<u64>),
    hook: &mut F,
) -> io::Result<Result>
where
    F: FnMut(FaultPoint, usize) -> io::Result<()>,
{
    check_cancelled(cancelled)?;
    hook(FaultPoint::AfterLeaseAcquire, 0)?;
    check_cancelled(cancelled)?;
    let stable_wal = lease.read_stable_wal()?;
    check_cancelled(cancelled)?;
    validate_checkpoint_observation(&stable_wal.bytes, stable_wal.observation.log_frames)?;
    hook(FaultPoint::AfterWalRead, 0)?;
    check_cancelled(cancelled)?;

    let state = match image_limits.0 {
        Some(limit) => sqlite_replication_spool::recover_with_image_limit(
            repository_root,
            &stable_wal.bytes,
            limit,
        )?,
        None => sqlite_replication_spool::recover(repository_root, &stable_wal.bytes)?,
    };
    check_cancelled(cancelled)?;
    let staging_directory = repository_root.join(".staging");
    fs::create_dir_all(&staging_directory)?;
    let source_image = tempfile::Builder::new()
        .prefix(".capture-source-")
        .suffix(".sqlite")
        .tempfile_in(&staging_directory)?;
    match (cancelled, image_limits.1) {
        (Some(token), Some(limit)) => {
            create_online_backup_cancellable_with_image_limit(
                source_path,
                source_image.path(),
                token,
                limit,
            )?;
        }
        (Some(token), None) => {
            create_online_backup_cancellable(source_path, source_image.path(), token)?;
        }
        (None, None) => {
            create_online_backup(source_path, source_image.path())?;
        }
        (None, Some(_)) => {
            return Err(io::Error::other(
                "bounded backup requires cancellation token",
            ));
        }
    }
    source_image.as_file().sync_all()?;
    check_cancelled(cancelled)?;
    hook(FaultPoint::AfterSourceSnapshot, 0)?;
    check_cancelled(cancelled)?;

    let mut image = File::open(source_image.path())?;
    let target_state = DatabaseState::load(&mut image, state.database.txid)?;
    let (mut transactions, mut next_state) =
        translate_wal(&stable_wal.bytes, &state, timestamp_millis)?;
    if next_state.database.checksum != target_state.checksum
        || next_state.database.commit != target_state.commit
    {
        let delta = state
            .database
            .delta_to_database_image(&mut image, timestamp_millis)?;
        transactions.clear();
        next_state.database = state.database.clone();
        if let Some((header, pages, reconciled_database)) = delta {
            let mut encoded = Vec::new();
            encode_delta(header, &pages, reconciled_database.checksum, &mut encoded)?;
            let (_, decoded_pages, decoded_checksum) =
                decode_delta(std::io::Cursor::new(&encoded))?;
            if decoded_pages != pages || decoded_checksum != reconciled_database.checksum {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "encoded reconciliation delta differs from its source pages",
                ));
            }
            transactions.push(Transaction {
                start_frame: 0,
                end_frame: 0,
                header,
                pages,
                post_checksum: reconciled_database.checksum,
                encoded,
            });
            next_state.database = reconciled_database;
        }
        next_state.wal = position_from_wal(&stable_wal.bytes)?;
    }
    if next_state.database.checksum != target_state.checksum
        || next_state.database.commit != target_state.commit
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "captured LTX state does not match consistent SQLite source image",
        ));
    }
    hook(FaultPoint::AfterWalTranslation, transactions.len())?;
    check_cancelled(cancelled)?;

    if let (Some(_), Some(batch_limit)) = (image_limits.3, image_limits.2) {
        let mut remaining = batch_limit;
        for transaction in &transactions {
            let size = u64::try_from(transaction.encoded.len()).map_err(io::Error::other)?;
            remaining = remaining.checked_sub(size).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "LTX batch size limit exceeded")
            })?;
        }
    }

    let spool_hook = |point, index| {
        if point != SpoolFaultPoint::AfterWatermark {
            check_cancelled(cancelled)?;
        }
        hook(FaultPoint::Spool(point), index)?;
        if point == SpoolFaultPoint::AfterWatermark {
            return Ok(());
        }
        check_cancelled(cancelled)
    };
    match (image_limits.3, image_limits.2) {
        (Some(limit), _) => sqlite_replication_spool::publish_transactions_with_spool_limit(
            repository_root,
            &transactions,
            &next_state,
            spool_hook,
            limit,
        )?,
        (None, Some(limit)) => sqlite_replication_spool::publish_transactions_with_batch_limit(
            repository_root,
            &transactions,
            &next_state,
            spool_hook,
            limit,
        )?,
        (None, None) => sqlite_replication_spool::publish_transactions(
            repository_root,
            &transactions,
            &next_state,
            spool_hook,
        )?,
    }
    hook(FaultPoint::AfterSpoolPublication, transactions.len())?;
    Ok(Result {
        transactions,
        wal_frames: next_state.wal.frame_count,
        state: next_state,
    })
}

fn check_cancelled(cancelled: Option<&AtomicBool>) -> io::Result<()> {
    if cancelled.is_some_and(|token| token.load(Ordering::Acquire)) {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "WAL capture was cancelled",
        ));
    }
    Ok(())
}

fn validate_checkpoint_observation(wal_bytes: &[u8], observed_log_frames: i64) -> io::Result<()> {
    if wal_bytes.is_empty() {
        if observed_log_frames > 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("SQLite reports {observed_log_frames} WAL frames but sidecar is empty"),
            ));
        }
        return Ok(());
    }
    let scanned = scan(wal_bytes)?;
    if observed_log_frames >= 0 && observed_log_frames != scanned.frames.len() as i64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "SQLite NOOP reports {observed_log_frames} WAL frames, file contains {} complete frames",
                scanned.frames.len()
            ),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::{Connection, ErrorCode, OpenFlags};
    use std::fs::File;
    use std::io;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::{Duration, Instant};

    use sqlite_replication_spool::{FaultPoint as SpoolFaultPoint, read_watermark};

    use super::{FaultPoint, capture_once, capture_once_cancellable, capture_once_with_hook};

    #[test]
    fn locked_capture_excludes_other_repository_operation_and_retries() {
        struct Gate;
        struct Permit;
        impl crate::WriteGate for Gate {
            type Permit = Permit;
            fn acquire(&self) -> io::Result<Permit> {
                Ok(Permit)
            }
        }
        impl crate::WritePermit for Permit {
            fn release(self) -> io::Result<()> {
                Ok(())
            }
        }
        if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            return;
        }
        let dir = tempfile::tempdir().expect("directory");
        let source = dir.path().join("source.sqlite");
        let repository = dir.path().join("repository");
        let db = Connection::open(&source).expect("database");
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE data(value TEXT); INSERT INTO data VALUES ('base')").expect("initialize");
        crate::snapshot(&source, &repository).expect("snapshot");
        db.execute("UPDATE data SET value='retry'", [])
            .expect("commit");
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let thread_source = source.clone();
        let thread_repository = repository.clone();
        let worker = thread::spawn(move || {
            let _guard = crate::repository_lock::acquire(&thread_repository).expect("lock");
            super::capture_once_with_cancellation_locked(
                &thread_source,
                &thread_repository,
                100,
                None,
                (Some(1 << 20), Some(1 << 20), Some(1 << 20), Some(4 << 20)),
                |point, _| {
                    if point == FaultPoint::AfterLeaseAcquire {
                        entered_tx.send(()).expect("signal");
                        release_rx.recv().expect("release");
                        return Err(io::Error::other("injected capture failure"));
                    }
                    Ok(())
                },
            )
        });
        entered_rx.recv().expect("capture entered");
        assert!(crate::repository_lock::acquire(&repository).is_err());
        let budget = crate::ConservativeBudget::new(crate::AdmissionGate::new(), 8 << 20)
            .expect("independent budget");
        let limits = || crate::CaptureLimits {
            recovery_image: 1 << 20,
            backup_image: 1 << 20,
            ltx_batch: 1 << 20,
        };
        let blocked = crate::capture_once_with_measured_reservation(
            &source,
            &repository,
            &Gate,
            &budget,
            limits(),
            4 << 20,
        );
        assert!(blocked.is_err(), "public capture ignored the held lock");
        release_tx.send(()).expect("release");
        assert!(worker.join().expect("thread").is_err());
        let result = crate::capture_once_with_measured_reservation(
            &source,
            &repository,
            &Gate,
            &budget,
            limits(),
            4 << 20,
        )
        .expect("public retry");
        assert_eq!(result.txid, 2);
    }

    #[test]
    fn characterize_capture_does_not_exclude_budget_reservations() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("source.sqlite");
        let repository = directory.path().join("repository");
        let database = Connection::open(&source).expect("open database");
        database.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE records(value TEXT)").expect("initialize");
        crate::snapshot(&source, &repository).expect("snapshot");
        let gate = crate::AdmissionGate::new();
        gate.open().expect("open gate");
        let budget = crate::ConservativeBudget::new(gate, 10).expect("budget");
        budget
            .reconcile_with(|| Ok(Some(0)))
            .expect("logical baseline");
        let reservation = budget.try_reserve(Some(1)).expect("producer reservation");
        database
            .execute("INSERT INTO records VALUES ('committed')", [])
            .expect("commit");
        reservation.settle(Some(1)).expect("logical settlement");
        let mut observed = false;
        let result = capture_once_with_hook(&source, &repository, 123, |point, _| {
            if point == FaultPoint::AfterSourceSnapshot {
                thread::scope(|scope| {
                    let producer = budget.clone();
                    scope
                        .spawn(move || {
                            producer
                                .try_reserve(Some(1))
                                .expect("capture does not close budget")
                                .settle(Some(0))
                                .expect("settle");
                        })
                        .join()
                        .expect("producer thread");
                });
                observed = true;
            }
            Ok(())
        })
        .expect("capture");
        assert!(observed);
        assert!(!result.transactions.is_empty());
    }

    #[test]
    fn failed_capture_keeps_unknown_budget_closed_until_reconciled() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("source.sqlite");
        let repository = directory.path().join("repository");
        let database = Connection::open(&source).expect("open database");
        database.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE records(value TEXT)").expect("initialize");
        crate::snapshot(&source, &repository).expect("snapshot");
        let gate = crate::AdmissionGate::new();
        gate.open().expect("open gate");
        let budget = crate::ConservativeBudget::new(gate, 1 << 20).expect("budget");
        budget
            .reconcile_with(|| Ok(Some(0)))
            .expect("logical test baseline");
        let reservation = budget.try_reserve(Some(4096)).expect("reservation");
        database
            .execute("INSERT INTO records VALUES ('committed')", [])
            .expect("commit");
        assert!(reservation.settle(None).is_err());
        let error = capture_once_with_hook(&source, &repository, 123, |point, _| {
            if point == FaultPoint::AfterSourceSnapshot {
                return Err(io::Error::other("injected capture error"));
            }
            Ok(())
        })
        .err()
        .expect("capture fails at injected boundary");
        assert!(error.to_string().contains("injected capture error"));
        assert!(
            budget.try_reserve(Some(1)).is_err(),
            "failure must not restore occupancy"
        );
        let result = capture_once(&source, &repository, 124).expect("retry capture");
        assert!(!result.transactions.is_empty());
        assert!(
            budget.try_reserve(Some(1)).is_err(),
            "capture does not automatically reconcile"
        );
        budget
            .reconcile_with(|| Ok(Some(1)))
            .expect("test-only logical occupancy");
        budget
            .try_reserve(Some(1))
            .expect("explicit reconciliation")
            .settle(Some(0))
            .expect("settle");
    }

    #[test]
    fn cooperative_capture_panic_poison_fails_closed() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("source.sqlite");
        let repository = directory.path().join("repository");
        let database = Connection::open(&source).expect("open database");
        database.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE records(value TEXT)").expect("initialize");
        crate::snapshot(&source, &repository).expect("snapshot");
        database
            .execute("INSERT INTO records VALUES ('data')", [])
            .expect("commit");
        let gate = crate::AdmissionGate::new();
        gate.open().expect("open gate");
        let budget = crate::ConservativeBudget::new(gate, 100).expect("budget");
        budget
            .reconcile_with(|| Ok(Some(0)))
            .expect("logical baseline");
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = budget.with_occupancy_mutation(|| {
                capture_once_with_hook(&source, &repository, 123, |point, _| {
                    if point == FaultPoint::AfterSourceSnapshot {
                        panic!("injected capture panic");
                    }
                    Ok(())
                })
            });
        }));
        assert!(panic.is_err());
        assert!(budget.try_reserve(Some(1)).is_err());
        assert!(budget.reconcile_with(|| Ok(Some(0))).is_err());
        assert!(budget.with_occupancy_mutation(|| Ok(())).is_err());
    }

    #[test]
    fn cooperative_capture_waits_for_real_ltx_measurement() {
        struct NotifyGate(std::sync::mpsc::Sender<()>);

        impl crate::WriteGate for NotifyGate {
            type Permit = Self;

            fn acquire(&self) -> io::Result<Self::Permit> {
                self.0.send(()).map_err(io::Error::other)?;
                Ok(Self(self.0.clone()))
            }
        }

        impl crate::WritePermit for NotifyGate {
            fn release(self) -> io::Result<()> {
                Ok(())
            }
        }

        fn ltx_bytes(path: &std::path::Path) -> io::Result<u64> {
            if path.is_file() {
                return Ok(path.metadata()?.len());
            }
            let mut total = 0;
            for entry in std::fs::read_dir(path)? {
                total += ltx_bytes(&entry?.path())?;
            }
            Ok(total)
        }

        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("source.sqlite");
        let repository = directory.path().join("repository");
        let database = Connection::open(&source).expect("open database");
        database.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE records(value TEXT)").expect("initialize");
        crate::snapshot(&source, &repository).expect("snapshot");
        database
            .execute("INSERT INTO records VALUES ('data')", [])
            .expect("commit");
        let gate = crate::AdmissionGate::new();
        gate.open().expect("open gate");
        let budget = crate::ConservativeBudget::new(gate, 1 << 20).expect("budget");
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let (gate_tx, gate_rx) = std::sync::mpsc::channel();
        thread::scope(|scope| {
            let measurement = budget.clone();
            let ltx = repository.join("ltx");
            scope.spawn(move || {
                measurement
                    .reconcile_with(|| {
                        let size = ltx_bytes(&ltx)?;
                        entered_tx.send(size).expect("report measurement");
                        release_rx
                            .recv_timeout(Duration::from_secs(5))
                            .expect("release measurement");
                        Ok(Some(size))
                    })
                    .expect("reconcile");
            });
            let before = entered_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("measurement started");
            let capture_budget = budget.clone();
            let source = &source;
            let repository = &repository;
            scope.spawn(move || {
                let result = crate::capture_once_with_budget(
                    source,
                    repository,
                    &NotifyGate(gate_tx),
                    &capture_budget,
                );
                done_tx
                    .send(result.map(|r| r.transactions))
                    .expect("report capture");
            });
            gate_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("capture acquired application gate");
            assert!(
                done_rx.recv_timeout(Duration::from_millis(100)).is_err(),
                "capture waits for measurement"
            );
            release_tx.send(()).expect("release measurement");
            assert!(
                done_rx
                    .recv_timeout(Duration::from_secs(5))
                    .expect("capture finished")
                    .expect("capture")
                    > 0
            );
            let after = ltx_bytes(&repository.join("ltx")).expect("measure captured LTX");
            assert!(after > before);
        });
        assert!(
            budget.try_reserve(Some(1)).is_err(),
            "capture invalidated occupancy"
        );
        budget
            .reconcile_with(|| Ok(Some(ltx_bytes(&repository.join("ltx"))?)))
            .expect("fresh measurement");
        budget
            .try_reserve(Some(1))
            .expect("reconciled")
            .settle(Some(0))
            .expect("settle");
        let restored = directory.path().join("restored.sqlite");
        crate::restore_to_target(&repository, &restored, crate::RestoreTarget::Txid(2))
            .expect("restore");
        let count: i64 = Connection::open(restored)
            .expect("restored")
            .query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))
            .expect("count");
        assert_eq!(count, 1);
    }

    #[test]
    fn cooperative_capture_invalidates_budget_and_requires_reconciliation() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("source.sqlite");
        let repository = directory.path().join("repository");
        let database = Connection::open(&source).expect("open database");
        database.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE records(value TEXT)").expect("initialize");
        crate::snapshot(&source, &repository).expect("snapshot");
        let gate = crate::AdmissionGate::new();
        gate.open().expect("open gate");
        let budget = crate::ConservativeBudget::new(gate, 100).expect("budget");
        budget.reconcile_with(|| Ok(Some(0))).expect("baseline");
        let permit = budget.try_reserve(Some(1)).expect("reserve");
        database
            .execute("INSERT INTO records VALUES ('data')", [])
            .expect("commit");
        let err = budget
            .with_occupancy_mutation(|| Ok(()))
            .expect_err("active producer excludes capture");
        assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
        permit.settle(Some(0)).expect("settle logical cost");
        let failed = budget.with_occupancy_mutation(|| {
            capture_once_with_hook(&source, &repository, 123, |point, _| {
                if point == FaultPoint::AfterSourceSnapshot {
                    return Err(io::Error::other("injected failure"));
                }
                Ok(())
            })
        });
        assert!(failed.is_err());
        assert!(budget.try_reserve(Some(1)).is_err());
        let captured = budget
            .with_occupancy_mutation(|| capture_once(&source, &repository, 124))
            .expect("capture retry");
        assert!(!captured.transactions.is_empty());
        assert!(budget.try_reserve(Some(1)).is_err());
        budget
            .reconcile_with(|| Ok(Some(1)))
            .expect("logical test measurement");
        budget
            .try_reserve(Some(1))
            .expect("reconciled")
            .settle(Some(0))
            .expect("settle");
    }

    #[test]
    fn characterize_capture_can_stale_reconciliation_measurement() {
        fn file_bytes(path: &std::path::Path) -> u64 {
            if path.is_file() {
                return path.metadata().expect("file metadata").len();
            }
            std::fs::read_dir(path)
                .expect("read directory")
                .map(|entry| file_bytes(&entry.expect("entry").path()))
                .sum()
        }

        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("source.sqlite");
        let repository = directory.path().join("repository");
        let database = Connection::open(&source).expect("open database");
        database.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE records(value TEXT)").expect("initialize");
        crate::snapshot(&source, &repository).expect("snapshot");
        let ltx = repository.join("ltx");
        let initial = file_bytes(&ltx);
        let gate = crate::AdmissionGate::new();
        gate.open().expect("open gate");
        let budget = crate::ConservativeBudget::new(gate, initial + 1).expect("budget");
        budget
            .reconcile_with(|| Ok(Some(file_bytes(&ltx))))
            .expect("baseline");
        let permit = budget
            .try_reserve(Some(1))
            .expect("logical producer reservation");
        database
            .execute("INSERT INTO records VALUES ('data')", [])
            .expect("commit");
        permit.settle(Some(0)).expect("logical settlement");
        let mut after = 0;
        budget
            .reconcile_with(|| {
                let measured = file_bytes(&ltx);
                let captured = capture_once(&source, &repository, 123)?;
                assert!(!captured.transactions.is_empty());
                after = file_bytes(&ltx);
                Ok(Some(measured))
            })
            .expect("install stale observation");
        assert!(after > initial + 1);
        budget
            .try_reserve(Some(1))
            .expect("characterization: stale occupancy admits reservation")
            .settle(Some(0))
            .expect("settle");
        println!(
            "ltx_before={initial} ltx_after={after} budget_limit={}",
            initial + 1
        );
    }

    #[test]
    fn conservative_budget_reconciles_after_real_capture() {
        fn ltx_bytes(path: &std::path::Path) -> u64 {
            if path.is_file() {
                return path.metadata().expect("metadata").len();
            }
            std::fs::read_dir(path)
                .expect("read ltx directory")
                .map(|entry| ltx_bytes(&entry.expect("entry").path()))
                .sum()
        }

        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("source.sqlite");
        let repository = directory.path().join("repository");
        let database = Connection::open(&source).expect("open database");
        database.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE records(value TEXT)").expect("initialize");
        crate::snapshot(&source, &repository).expect("snapshot");
        let gate = crate::AdmissionGate::new();
        gate.open().expect("open gate");
        let budget = crate::ConservativeBudget::new(gate, 1 << 20).expect("budget");
        let mut baseline = 0;
        budget
            .reconcile_with(|| {
                baseline = ltx_bytes(&repository.join("ltx"));
                Ok(Some(baseline))
            })
            .expect("baseline");
        let permit = budget.try_reserve(Some(4096)).expect("logical reservation");
        database
            .execute("INSERT INTO records VALUES ('data')", [])
            .expect("commit");
        assert!(permit.settle(None).is_err());
        assert!(budget.try_reserve(Some(1)).is_err());
        let result = capture_once(&source, &repository, 123).expect("capture");
        let mut measured = 0;
        assert!(!result.transactions.is_empty());
        assert!(
            budget.try_reserve(Some(1)).is_err(),
            "capture alone does not reconcile budget"
        );
        budget
            .reconcile_with(|| {
                measured = ltx_bytes(&repository.join("ltx"));
                Ok(Some(measured))
            })
            .expect("exclusive test measurement");
        assert!(measured > baseline);
        budget
            .try_reserve(Some(1))
            .expect("reconciled")
            .settle(Some(0))
            .expect("settle");
        println!("baseline_ltx={baseline} captured_ltx={measured}");
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn characterize_wal_spool_and_staging_for_controlled_writes() {
        fn file_bytes(path: &std::path::Path) -> u64 {
            if !path.exists() {
                return 0;
            }
            if path.is_file() {
                return path.metadata().expect("file metadata").len();
            }
            std::fs::read_dir(path)
                .expect("read directory")
                .map(|entry| file_bytes(&entry.expect("entry").path()))
                .sum()
        }

        for mode in ["batch", "individual", "checkpointed"] {
            let directory = tempfile::tempdir().expect("temporary directory");
            let source = directory.path().join("source.sqlite");
            let repository = directory.path().join("repository");
            let database = Connection::open(&source).expect("open source");
            database.execute_batch(
                "PRAGMA page_size=4096; PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE records(id INTEGER PRIMARY KEY, value BLOB NOT NULL);",
            ).expect("initialize");
            crate::snapshot(&source, &repository).expect("snapshot");
            database
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
                .expect("reset WAL");
            let baseline_ltx = file_bytes(&repository.join("ltx"));
            if mode == "batch" {
                database.execute_batch("BEGIN").expect("begin batch");
            }
            for id in 1..=20 {
                database
                    .execute(
                        "INSERT INTO records VALUES (?1, ?2)",
                        rusqlite::params![id, vec![b'x'; 100]],
                    )
                    .expect("insert");
            }
            if mode == "batch" {
                database.execute_batch("COMMIT").expect("commit batch");
            }
            if mode == "checkpointed" {
                database
                    .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
                    .expect("checkpoint");
            }
            let wal = std::path::PathBuf::from(format!("{}-wal", source.display()));
            let wal_bytes = file_bytes(&wal);
            let mut staging_at_snapshot = 0;
            let mut staging_after_publication = 0;
            let result = capture_once_with_hook(&source, &repository, 100, |point, _| {
                if point == FaultPoint::AfterSourceSnapshot {
                    staging_at_snapshot = file_bytes(&repository.join(".staging"));
                }
                if point == FaultPoint::AfterSpoolPublication {
                    staging_after_publication = file_bytes(&repository.join(".staging"));
                }
                Ok(())
            })
            .expect("capture");
            let added_ltx = file_bytes(&repository.join("ltx")) - baseline_ltx;
            println!(
                "mode={mode} wal_bytes={wal_bytes} wal_frames={} ltx_added={added_ltx} staging_snapshot={staging_at_snapshot} staging_publication={staging_after_publication} transactions={} first_wal_offset={:?}",
                result.state.wal.frame_count,
                result.transactions.len(),
                result.transactions.first().map(|tx| tx.header.wal_offset)
            );
            assert!(added_ltx > 0);
            assert!(staging_at_snapshot > 0);
            if mode == "checkpointed" {
                assert_eq!(wal_bytes, 0);
                assert_eq!(result.transactions[0].header.wal_offset, 0);
            }
        }
    }

    #[test]
    fn capture_cycle_publishes_and_recovers_checkpointed_changes() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source_path = directory.path().join("source.sqlite");
        let repository_path = directory.path().join("repository");
        let database = Connection::open(&source_path).expect("open SQLite source");
        database
            .busy_timeout(std::time::Duration::ZERO)
            .expect("disable busy timeout");
        database
            .execute_batch(
                "PRAGMA page_size=512;
                 PRAGMA journal_mode=WAL;
                 PRAGMA wal_autocheckpoint=0;
                 CREATE TABLE records (id INTEGER PRIMARY KEY, value TEXT);
                 INSERT INTO records VALUES (1, 'base');",
            )
            .expect("initialize database");
        let (busy, log_frames, checkpointed): (i64, i64, i64) = database
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("checkpoint baseline");
        assert_eq!(busy, 0);
        assert!(log_frames < 0 || checkpointed == log_frames);
        crate::snapshot(&source_path, &repository_path).expect("create base snapshot");

        database
            .execute("UPDATE records SET value='first' WHERE id=1", [])
            .expect("commit first change");
        let first = capture_once(&source_path, &repository_path, 100).expect("capture first WAL");
        assert_eq!(first.transactions.len(), 1);
        assert_eq!(first.state.database.txid, 2);

        let repeated = capture_once(&source_path, &repository_path, 101)
            .expect("repeat capture without changes");
        assert!(repeated.transactions.is_empty());
        assert_eq!(repeated.state.database.txid, first.state.database.txid);

        database
            .execute("UPDATE records SET value='checkpointed' WHERE id=1", [])
            .expect("commit change before checkpoint");
        let (busy, log_frames, checkpointed): (i64, i64, i64) = database
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("checkpoint between capture cycles");
        assert_eq!(busy, 0);
        assert!(log_frames < 0 || checkpointed == log_frames);
        let checkpointed_capture =
            capture_once(&source_path, &repository_path, 102).expect("capture checkpointed data");
        assert_eq!(checkpointed_capture.transactions.len(), 1);
        assert_eq!(checkpointed_capture.state.database.txid, 3);
        assert_eq!(checkpointed_capture.transactions[0].header.wal_offset, 0);

        database
            .execute("UPDATE records SET value='orphan-stage' WHERE id=1", [])
            .expect("commit change for publication fault");
        let failed = capture_once_with_hook(&source_path, &repository_path, 103, |point, _| {
            if point == FaultPoint::Spool(SpoolFaultPoint::AfterLtxPublish) {
                return Err(std::io::Error::other("simulated crash before watermark"));
            }
            Ok(())
        });
        assert!(failed.is_err());

        let recovered = capture_once(&source_path, &repository_path, 104)
            .expect("recover published LTX before watermark");
        assert!(recovered.transactions.is_empty());
        assert_eq!(recovered.state.database.txid, 4);
        let watermark = read_watermark(&repository_path).expect("read recovered watermark");
        assert_eq!(watermark.txid, 4);
        assert_eq!(
            watermark.database_checksum,
            recovered.state.database.checksum
        );

        for value in ["batch-a", "batch-b", "batch-c"] {
            database
                .execute("UPDATE records SET value=?1 WHERE id=1", [value])
                .unwrap_or_else(|error| panic!("commit batched change {value:?}: {error}"));
        }
        let partial_publication =
            capture_once_with_hook(&source_path, &repository_path, 105, |point, index| {
                if point == FaultPoint::Spool(SpoolFaultPoint::AfterLtxPublish) && index == 0 {
                    return Err(std::io::Error::other(
                        "crash after first of several LTX publications",
                    ));
                }
                Ok(())
            });
        assert!(partial_publication.is_err());

        let remaining = capture_once(&source_path, &repository_path, 106)
            .expect("resume after partial-batch publication");
        assert_eq!(remaining.transactions.len(), 2);
        assert_eq!(remaining.state.database.txid, 7);
        assert_eq!(read_watermark(&repository_path).unwrap().txid, 7);
    }

    #[test]
    fn capture_reopens_after_busy_writer_lease_failure() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source_path = directory.path().join("source.sqlite");
        let repository_path = directory.path().join("repository");
        let database = Connection::open(&source_path).expect("open SQLite source");
        database
            .busy_timeout(Duration::ZERO)
            .expect("disable source busy timeout");
        database
            .execute_batch(
                "PRAGMA page_size=512;
                 PRAGMA journal_mode=WAL;
                 PRAGMA wal_autocheckpoint=0;
                 CREATE TABLE records (id INTEGER PRIMARY KEY, value TEXT);
                 INSERT INTO records VALUES (1, 'base');",
            )
            .expect("initialize source database");
        let (busy, log_frames, checkpointed): (i64, i64, i64) = database
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("checkpoint baseline database");
        assert_eq!(busy, 0);
        assert!(log_frames < 0 || checkpointed == log_frames);
        crate::snapshot(&source_path, &repository_path).expect("create base snapshot");
        database
            .execute(
                "UPDATE records SET value='captured-before-busy' WHERE id=1",
                [],
            )
            .expect("commit source value before busy lease attempt");
        let before_busy = capture_once(&source_path, &repository_path, 599)
            .expect("create watermark before busy lease attempt");
        assert_eq!(before_busy.transactions.len(), 1);
        assert_eq!(before_busy.state.database.txid, 2);
        database
            .execute("UPDATE records SET value='pending-capture' WHERE id=1", [])
            .expect("commit source value for capture");

        let external = Connection::open(&source_path).expect("open external writer");
        external
            .busy_timeout(Duration::ZERO)
            .expect("disable external writer busy timeout");
        external
            .execute_batch("BEGIN IMMEDIATE;")
            .expect("acquire external writer lock");
        let error = match capture_once(&source_path, &repository_path, 600) {
            Ok(_) => panic!("capture succeeded while the external writer held the lock"),
            Err(error) => error,
        };
        let sqlite_error = error
            .get_ref()
            .and_then(|source| source.downcast_ref::<rusqlite::Error>())
            .expect("busy lease error should retain the SQLite cause");
        assert_eq!(
            sqlite_error.sqlite_error_code(),
            Some(ErrorCode::DatabaseBusy)
        );
        let watermark =
            read_watermark(&repository_path).expect("read watermark after busy lease failure");
        assert_eq!(watermark.txid, before_busy.state.database.txid);
        assert_eq!(
            watermark.database_checksum,
            before_busy.state.database.checksum
        );

        external
            .execute_batch("ROLLBACK;")
            .expect("release external writer lock");
        let retried = capture_once(&source_path, &repository_path, 601)
            .expect("retry capture after releasing external writer");
        assert_eq!(retried.transactions.len(), 1);
        assert_eq!(retried.state.database.txid, 3);
        let watermark = read_watermark(&repository_path).expect("read watermark after retry");
        assert_eq!(watermark.txid, 3);
        assert_eq!(watermark.database_checksum, retried.state.database.checksum);
    }

    #[test]
    fn capture_cancellation_after_partial_publication_releases_lease_and_recovers() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source_path = directory.path().join("source.sqlite");
        let repository_path = directory.path().join("repository");
        let database = Connection::open(&source_path).expect("open SQLite source");
        database
            .busy_timeout(Duration::ZERO)
            .expect("disable busy timeout");
        database
            .execute_batch(
                "PRAGMA page_size=512;
                 PRAGMA journal_mode=WAL;
                 PRAGMA wal_autocheckpoint=0;
                 CREATE TABLE records (id INTEGER PRIMARY KEY, value TEXT);
                 INSERT INTO records VALUES (1, 'base');",
            )
            .expect("initialize source database");
        let (busy, log_frames, checkpointed): (i64, i64, i64) = database
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("checkpoint baseline database");
        assert_eq!(busy, 0);
        assert!(log_frames < 0 || checkpointed == log_frames);
        crate::snapshot(&source_path, &repository_path).expect("create base snapshot");
        for value in ["first", "second", "third"] {
            database
                .execute("UPDATE records SET value=?1 WHERE id=1", [value])
                .unwrap_or_else(|error| panic!("commit source value {value:?}: {error}"));
        }

        let cancelled = AtomicBool::new(false);
        let mut reached_cancellation_boundary = false;
        let capture_error = match capture_once_cancellable(
            &source_path,
            &repository_path,
            400,
            &cancelled,
            |point, index| {
                if point == FaultPoint::Spool(SpoolFaultPoint::AfterLtxPublish) && index == 0 {
                    reached_cancellation_boundary = true;
                    cancelled.store(true, Ordering::Release);
                }
                Ok(())
            },
        ) {
            Ok(_) => panic!("capture succeeded after cancellation was requested"),
            Err(error) => error,
        };
        assert!(reached_cancellation_boundary);
        assert_eq!(capture_error.kind(), io::ErrorKind::Interrupted);

        let first_ltx = sqlite_replication_storage_fs::ltx_path(&repository_path, 2, 2);
        assert!(
            first_ltx.is_file(),
            "published immutable LTX was not retained"
        );
        let second_ltx = sqlite_replication_storage_fs::ltx_path(&repository_path, 3, 3);
        assert!(
            !second_ltx.exists(),
            "capture published a later LTX after cancellation"
        );
        let watermark =
            read_watermark(&repository_path).expect("read watermark after cancellation");
        assert_eq!(
            watermark.txid, 1,
            "watermark advanced across partial cancellation"
        );

        let writer = open_capture_test_connection(&source_path);
        writer
            .execute_batch("BEGIN IMMEDIATE;")
            .expect("capture released writer guard after cancellation");
        writer
            .execute_batch("ROLLBACK;")
            .expect("release writer acquired after cancellation");

        let recovered = capture_once(&source_path, &repository_path, 401)
            .expect("recover partial publication after cancellation");
        assert_eq!(recovered.state.database.txid, 4);
        assert_eq!(recovered.transactions.len(), 2);
        let latest_value: String = database
            .query_row("SELECT value FROM records WHERE id=1", [], |row| row.get(0))
            .expect("read final source value");
        assert_eq!(latest_value, "third");
        let watermark = read_watermark(&repository_path).expect("read recovered watermark");
        assert_eq!(watermark.txid, 4);
        assert_eq!(
            watermark.database_checksum,
            recovered.state.database.checksum
        );

        database
            .execute("UPDATE records SET value='fourth' WHERE id=1", [])
            .expect("commit source value before watermark cancellation");
        let cancel_before_watermark = AtomicBool::new(false);
        let watermark_error = match capture_once_cancellable(
            &source_path,
            &repository_path,
            402,
            &cancel_before_watermark,
            |point, _| {
                if point == FaultPoint::Spool(SpoolFaultPoint::AfterWatermarkSync) {
                    cancel_before_watermark.store(true, Ordering::Release);
                }
                Ok(())
            },
        ) {
            Ok(_) => panic!("capture succeeded after cancellation before watermark promotion"),
            Err(error) => error,
        };
        assert_eq!(watermark_error.kind(), io::ErrorKind::Interrupted);
        let watermark = read_watermark(&repository_path)
            .expect("read watermark after pre-promotion cancellation");
        assert_eq!(watermark.txid, 4, "watermark advanced before promotion");
        let final_ltx = sqlite_replication_storage_fs::ltx_path(&repository_path, 5, 5);
        assert!(final_ltx.is_file(), "final immutable LTX was not retained");
        writer
            .execute_batch("BEGIN IMMEDIATE;")
            .expect("capture released writer guard before watermark promotion");
        writer
            .execute_batch("ROLLBACK;")
            .expect("release writer acquired after pre-promotion cancellation");

        let resumed = capture_once(&source_path, &repository_path, 403)
            .expect("recover after cancellation before watermark promotion");
        assert_eq!(resumed.state.database.txid, 5);
        assert!(resumed.transactions.is_empty());
        let latest_value: String = database
            .query_row("SELECT value FROM records WHERE id=1", [], |row| row.get(0))
            .expect("read latest source value after watermark recovery");
        assert_eq!(latest_value, "fourth");
        let watermark =
            read_watermark(&repository_path).expect("read watermark after promotion recovery");
        assert_eq!(watermark.txid, 5);
        assert_eq!(watermark.database_checksum, resumed.state.database.checksum);

        let cancel_after_watermark = AtomicBool::new(false);
        let completed = capture_once_cancellable(
            &source_path,
            &repository_path,
            404,
            &cancel_after_watermark,
            |point, _| {
                if point == FaultPoint::Spool(SpoolFaultPoint::AfterWatermark) {
                    cancel_after_watermark.store(true, Ordering::Release);
                }
                Ok(())
            },
        )
        .expect("cancellation after durable watermark should report completion");
        assert_eq!(completed.state.database.txid, 5);
        assert!(cancel_after_watermark.load(Ordering::Acquire));
    }

    #[test]
    fn checkpointed_commits_can_collapse_into_one_reconciliation_txid() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("source.sqlite");
        let repository = directory.path().join("repository");
        let connection = Connection::open(&source).expect("open source");
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE records(value TEXT);").expect("initialize");
        crate::snapshot(&source, &repository).expect("snapshot");
        connection
            .execute("INSERT INTO records VALUES ('first')", [])
            .expect("first commit");
        connection
            .execute("INSERT INTO records VALUES ('second')", [])
            .expect("second commit");
        let (busy, _, _): (i64, i64, i64) = connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("checkpoint");
        assert_eq!(busy, 0);
        let captured = capture_once(&source, &repository, 123).expect("capture reconciled image");
        assert_eq!(captured.state.database.txid, 2);
        assert_eq!(captured.transactions.len(), 1);
        let restored = directory.path().join("restored.sqlite");
        crate::restore_to_target(&repository, &restored, crate::RestoreTarget::Txid(2))
            .expect("restore");
        let count: i64 = Connection::open(restored)
            .expect("open restore")
            .query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))
            .expect("count");
        assert_eq!(count, 2);
    }

    #[test]
    fn capture_serializes_unmanaged_writer_and_reconciles_autocheckpoint() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source_path = directory.path().join("source.sqlite");
        let repository_path = directory.path().join("repository");
        let database = Connection::open(&source_path).expect("open SQLite source");
        database
            .busy_timeout(Duration::ZERO)
            .expect("disable managed test busy timeout");
        database
            .execute_batch(
                "PRAGMA page_size=512;
                 PRAGMA journal_mode=WAL;
                 PRAGMA wal_autocheckpoint=0;
                 CREATE TABLE records (id INTEGER PRIMARY KEY, value TEXT);
                 INSERT INTO records VALUES (1, 'base');",
            )
            .expect("initialize source database");
        let (busy, log_frames, checkpointed): (i64, i64, i64) = database
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("checkpoint baseline database");
        assert_eq!(busy, 0);
        assert!(log_frames < 0 || checkpointed == log_frames);
        crate::snapshot(&source_path, &repository_path).expect("create base snapshot");
        database
            .execute("UPDATE records SET value='managed-capture' WHERE id=1", [])
            .expect("commit source value for capture");

        let external = Connection::open(&source_path).expect("open unmanaged app connection");
        external
            .busy_timeout(Duration::from_millis(250))
            .expect("configure external busy timeout");
        external
            .execute_batch("PRAGMA wal_autocheckpoint=1;")
            .expect("enable external autocheckpoint");
        let external_busy_timeout: i64 = external
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .expect("read external busy timeout");
        let external_autocheckpoint: i64 = external
            .query_row("PRAGMA wal_autocheckpoint", [], |row| row.get(0))
            .expect("read external autocheckpoint");
        assert_eq!(external_busy_timeout, 250);
        assert_eq!(external_autocheckpoint, 1);

        let mut observed_busy = false;
        let captured = capture_once_with_hook(&source_path, &repository_path, 500, |point, _| {
            if point != FaultPoint::AfterWalRead {
                return Ok(());
            }
            let begin_error = match external.execute_batch("BEGIN IMMEDIATE;") {
                Ok(()) => {
                    let _ = external.execute_batch("ROLLBACK;");
                    return Err(io::Error::other(
                        "unmanaged writer acquired its lock during capture",
                    ));
                }
                Err(error) => error,
            };
            if begin_error.sqlite_error_code() != Some(ErrorCode::DatabaseBusy) {
                return Err(io::Error::other(format!(
                    "unmanaged writer returned {begin_error}, want SQLITE_BUSY"
                )));
            }
            observed_busy = true;
            Ok(())
        })
        .expect("capture while unmanaged writer attempts its lock");
        assert!(observed_busy);
        assert_eq!(captured.state.database.txid, 2);

        external
            .execute_batch(
                "BEGIN IMMEDIATE;
                 UPDATE records SET value='external-after-capture' WHERE id=1;
                 COMMIT;",
            )
            .expect("external writer commits after capture release");
        let (busy, log_frames, checkpointed): (i64, i64, i64) = external
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("external connection truncates WAL after commit");
        assert_eq!(busy, 0);
        assert!(log_frames < 0 || checkpointed == log_frames);

        let reconciled = capture_once(&source_path, &repository_path, 501)
            .expect("reconcile external checkpointed commit");
        assert_eq!(reconciled.state.database.txid, 3);
        assert_eq!(reconciled.transactions.len(), 1);
        assert_eq!(reconciled.transactions[0].header.wal_offset, 0);
        let watermark = read_watermark(&repository_path).expect("read final watermark");
        assert_eq!(watermark.txid, 3);
        assert_eq!(
            watermark.database_checksum,
            reconciled.state.database.checksum
        );
    }

    #[test]
    fn capture_release_window_survives_writer_and_checkpoint() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source_path = directory.path().join("source.sqlite");
        let repository_path = directory.path().join("repository");
        let database = Connection::open(&source_path).expect("open SQLite source");
        database
            .busy_timeout(Duration::ZERO)
            .expect("disable busy timeout");
        database
            .execute_batch(
                "PRAGMA page_size=512;
                 PRAGMA journal_mode=WAL;
                 PRAGMA wal_autocheckpoint=0;
                 CREATE TABLE records (id INTEGER PRIMARY KEY, value TEXT);
                 INSERT INTO records VALUES (1, 'base');",
            )
            .expect("initialize database");
        let (busy, log_frames, checkpointed_frames): (i64, i64, i64) = database
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("checkpoint baseline database");
        assert_eq!(busy, 0);
        assert!(log_frames < 0 || checkpointed_frames == log_frames);
        crate::snapshot(&source_path, &repository_path).expect("create base snapshot");
        database
            .execute("UPDATE records SET value='before-capture' WHERE id=1", [])
            .expect("commit change before capture");

        let writer = open_capture_test_connection(&source_path);
        let boundaries = [
            FaultPoint::AfterWalRead,
            FaultPoint::AfterSourceSnapshot,
            FaultPoint::AfterWalTranslation,
            FaultPoint::AfterSpoolPublication,
        ];
        let mut observed_boundaries = Vec::new();
        let captured = capture_once_with_hook(
            &source_path,
            &repository_path,
            200,
            |point, _| {
                if !boundaries.contains(&point) {
                    return Ok(());
                }
                let begin_error = match writer.execute_batch("BEGIN IMMEDIATE;") {
                    Ok(()) => {
                        let _ = writer.execute_batch("ROLLBACK;");
                        return Err(io::Error::other(format!(
                            "writer acquired BEGIN IMMEDIATE at capture boundary {point:?}"
                        )));
                    }
                    Err(error) => error,
                };
                if begin_error.sqlite_error_code() != Some(ErrorCode::DatabaseBusy) {
                    return Err(io::Error::other(format!(
                        "writer at capture boundary {point:?} returned {begin_error}, want SQLITE_BUSY"
                    )));
                }
                observed_boundaries.push(point);

                if point == FaultPoint::AfterSpoolPublication {
                    let watermark = read_watermark(&repository_path).map_err(io::Error::other)?;
                    if watermark.txid != 2 {
                        return Err(io::Error::other(format!(
                            "published watermark TXID = {}, want 2 before checkpoint",
                            watermark.txid
                        )));
                    }
                    let (checkpoint_busy, checkpoint_frames, checkpointed): (i64, i64, i64) =
                        writer
                            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                            })
                            .map_err(io::Error::other)?;
                    if checkpoint_busy == 0 {
                        return Err(io::Error::other(format!(
                            "TRUNCATE completed while capture lease was active: busy={checkpoint_busy} log={checkpoint_frames} checkpointed={checkpointed}"
                        )));
                    }
                }
                Ok(())
            },
        )
        .expect("capture under deterministic writer/checkpoint barriers");
        assert_eq!(
            observed_boundaries, boundaries,
            "writer was not tested at each capture boundary"
        );
        assert_eq!(captured.state.database.txid, 2);
        assert_eq!(captured.transactions.len(), 1);

        writer
            .execute_batch(
                "BEGIN IMMEDIATE;
                 INSERT INTO records VALUES (2, 'after-release');
                 COMMIT;",
            )
            .expect("writer commit immediately after capture lease release");
        let (busy, log_frames, checkpointed_frames): (i64, i64, i64) = writer
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("checkpoint durable capture and post-release writer");
        assert_eq!(busy, 0);
        assert!(log_frames < 0 || checkpointed_frames == log_frames);

        let reconciled = capture_once(&source_path, &repository_path, 201)
            .expect("capture commit checkpointed before the next pin");
        assert_eq!(reconciled.transactions.len(), 1);
        assert_eq!(reconciled.state.database.txid, 3);
        assert_eq!(reconciled.transactions[0].header.wal_offset, 0);
        let value: String = database
            .query_row("SELECT value FROM records WHERE id=2", [], |row| row.get(0))
            .expect("read post-release committed row");
        assert_eq!(value, "after-release");
        let watermark = read_watermark(&repository_path).expect("read final capture watermark");
        assert_eq!(watermark.txid, 3);
        assert_eq!(
            watermark.database_checksum,
            reconciled.state.database.checksum
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires the bounded Linux tmpfs provided by make test-cp01-enospc"]
    fn capture_recovers_after_real_enospc_on_bounded_tmpfs() {
        use std::ffi::CString;
        use std::fs;
        use std::mem::MaybeUninit;
        use std::os::unix::ffi::OsStrExt;
        use std::path::PathBuf;

        let tmpfs_root = PathBuf::from(
            std::env::var_os("SQLITE_REPLICATION_ENOSPC_ROOT")
                .expect("bounded tmpfs root environment variable"),
        );
        let encoded_path = CString::new(tmpfs_root.as_os_str().as_bytes())
            .expect("tmpfs root path contains no NUL byte");
        let mut filesystem_info = MaybeUninit::<libc::statfs>::uninit();
        let stat_result =
            unsafe { libc::statfs(encoded_path.as_ptr(), filesystem_info.as_mut_ptr()) };
        assert_eq!(stat_result, 0, "inspect bounded tmpfs");
        let filesystem_info = unsafe { filesystem_info.assume_init() };
        assert_eq!(
            filesystem_info.f_type as u64, 0x01021994,
            "filesystem is not tmpfs"
        );
        let block_count = filesystem_info.f_blocks;
        let block_size =
            u64::try_from(filesystem_info.f_bsize).expect("tmpfs block size is nonnegative");
        let capacity = block_count
            .checked_mul(block_size)
            .expect("tmpfs capacity fits in u64");
        assert!(
            capacity > 0 && capacity <= 16 << 20,
            "tmpfs capacity is {capacity} bytes"
        );

        let repository = tempfile::Builder::new()
            .prefix("rust-cp01-enospc-")
            .tempdir_in(&tmpfs_root)
            .expect("create repository on bounded tmpfs");
        let source_directory = tempfile::tempdir().expect("temporary source directory");
        let source_path = source_directory.path().join("source.sqlite");
        let database = Connection::open(&source_path).expect("open SQLite source");
        database
            .busy_timeout(Duration::ZERO)
            .expect("disable busy timeout");
        database
            .execute_batch(
                "PRAGMA page_size=512;
                 PRAGMA journal_mode=WAL;
                 PRAGMA wal_autocheckpoint=0;
                 CREATE TABLE records (id INTEGER PRIMARY KEY, value TEXT);
                 INSERT INTO records VALUES (1, 'base');",
            )
            .expect("initialize source database");
        let (busy, log_frames, checkpointed): (i64, i64, i64) = database
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("checkpoint baseline database");
        assert_eq!(busy, 0);
        assert!(log_frames < 0 || checkpointed == log_frames);
        crate::snapshot(&source_path, repository.path()).expect("create base snapshot on tmpfs");

        const COMMIT_COUNT: usize = 1000;
        for index in 0..COMMIT_COUNT {
            let value = format!("value-{index:05}");
            database
                .execute("UPDATE records SET value=?1 WHERE id=1", [&value])
                .unwrap_or_else(|error| panic!("commit WAL transaction {}: {error}", index + 1));
        }

        let filler_path = fill_tmpfs_except(&tmpfs_root, 128 << 10);
        let capture_error = match capture_once(&source_path, repository.path(), 300) {
            Ok(_) => panic!("capture unexpectedly succeeded with a nearly full tmpfs"),
            Err(error) => error,
        };
        // std adds path context to filesystem errors, so raw_os_error is not on the outer error.
        assert_eq!(
            capture_error.kind(),
            io::ErrorKind::StorageFull,
            "capture error: {capture_error:?}"
        );
        let published_ltx =
            count_published_ltx(repository.path()).expect("count partial LTX chain");
        assert!(
            published_ltx > 1 && published_ltx < COMMIT_COUNT + 1,
            "tmpfs failure published {published_ltx} LTX files; expected a partial chain"
        );
        let watermark = read_watermark(repository.path()).expect("read watermark after ENOSPC");
        assert_eq!(
            watermark.txid, 1,
            "watermark advanced across partial publication"
        );

        let writer = open_capture_test_connection(&source_path);
        writer
            .execute_batch("BEGIN IMMEDIATE;")
            .expect("capture released writer guard after ENOSPC");
        writer
            .execute_batch("ROLLBACK;")
            .expect("release writer acquired after ENOSPC");
        fs::remove_file(&filler_path).expect("free bounded tmpfs after ENOSPC");

        let recovered = capture_once(&source_path, repository.path(), 301)
            .expect("recover partial chain after freeing tmpfs");
        assert_eq!(recovered.state.database.txid, COMMIT_COUNT as u64 + 1);
        let latest_value: String = database
            .query_row("SELECT value FROM records WHERE id=1", [], |row| row.get(0))
            .expect("read latest source value");
        assert_eq!(latest_value, format!("value-{:05}", COMMIT_COUNT - 1));
        let watermark = read_watermark(repository.path()).expect("read recovered watermark");
        assert_eq!(watermark.txid, COMMIT_COUNT as u64 + 1);
        assert_eq!(
            watermark.database_checksum,
            recovered.state.database.checksum
        );
    }

    #[cfg(target_os = "linux")]
    fn fill_tmpfs_except(root: &std::path::Path, reserve: u64) -> std::path::PathBuf {
        use std::ffi::CString;
        use std::fs::OpenOptions;
        use std::io::Write;
        use std::mem::MaybeUninit;
        use std::os::unix::ffi::OsStrExt;

        let encoded_path = CString::new(root.as_os_str().as_bytes()).expect("tmpfs path");
        let mut filesystem_info = MaybeUninit::<libc::statfs>::uninit();
        let stat_result =
            unsafe { libc::statfs(encoded_path.as_ptr(), filesystem_info.as_mut_ptr()) };
        assert_eq!(stat_result, 0, "inspect tmpfs before filling");
        let filesystem_info = unsafe { filesystem_info.assume_init() };
        let available_blocks = filesystem_info.f_bavail;
        let block_size =
            u64::try_from(filesystem_info.f_bsize).expect("tmpfs block size is nonnegative");
        let available = available_blocks
            .checked_mul(block_size)
            .expect("available tmpfs capacity fits in u64");
        assert!(
            available > reserve,
            "tmpfs has {available} bytes, reserve is {reserve}"
        );

        let filler_path = root.join("rust-cp01-enospc-filler");
        let mut filler = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&filler_path)
            .expect("create tmpfs filler");
        let buffer = vec![0xa5; 64 << 10];
        let mut remaining = available - reserve;
        while remaining > 0 {
            let write_size = remaining.min(buffer.len() as u64) as usize;
            filler
                .write_all(&buffer[..write_size])
                .expect("fill bounded tmpfs while preserving reserve");
            remaining -= write_size as u64;
        }
        filler.sync_all().expect("sync tmpfs filler");
        drop(filler);
        filler_path
    }

    #[cfg(target_os = "linux")]
    fn count_published_ltx(root: &std::path::Path) -> std::io::Result<usize> {
        let mut count = 0;
        for entry in std::fs::read_dir(root.join("ltx").join("0"))? {
            let entry = entry?;
            if entry.file_type()?.is_file()
                && entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension.to_str() == Some("ltx"))
            {
                count += 1;
            }
        }
        Ok(count)
    }

    fn open_capture_test_connection(path: &std::path::Path) -> Connection {
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )
        .expect("open capture test connection");
        connection
            .busy_timeout(Duration::ZERO)
            .expect("disable capture test busy timeout");
        connection
            .execute_batch("PRAGMA wal_autocheckpoint=0;")
            .expect("disable capture test autocheckpoint");
        connection
    }

    #[cfg(unix)]
    #[test]
    fn recovers_after_sigkill_at_durability_boundaries() {
        for (name, ltx_already_published) in [
            ("before-stage-sync", false),
            ("after-stage-sync", false),
            ("before-ltx-publish", false),
            ("after-ltx-publish", true),
            ("before-watermark-write", true),
            ("before-watermark-sync", true),
            ("after-watermark-sync", true),
            ("after-watermark", true),
        ] {
            let directory = tempfile::tempdir().expect("temporary directory");
            let source_path = directory.path().join("source.sqlite");
            let repository_path = directory.path().join("repository");
            let database = Connection::open(&source_path).expect("open source database");
            database
                .busy_timeout(Duration::ZERO)
                .expect("disable busy timeout");
            database
                .execute_batch(
                    "PRAGMA page_size=512;
                     PRAGMA journal_mode=WAL;
                     PRAGMA wal_autocheckpoint=0;
                     CREATE TABLE records (id INTEGER PRIMARY KEY, value TEXT);
                     INSERT INTO records VALUES (1, 'base');",
                )
                .expect("initialize source database");
            crate::snapshot(&source_path, &repository_path).expect("create base snapshot");
            database
                .execute("UPDATE records SET value='survives-kill' WHERE id=1", [])
                .expect("commit WAL transaction");
            drop(database);

            let marker_path = directory.path().join("reached-boundary");
            let mut child = Command::new(std::env::current_exe().expect("current test executable"))
                .args([
                    "--exact",
                    "capture::tests::capture_crash_child",
                    "--nocapture",
                ])
                .env("SQLITE_REPLICATION_CAPTURE_CRASH_CHILD", "1")
                .env("SQLITE_REPLICATION_CAPTURE_CRASH_POINT", name)
                .env("SQLITE_REPLICATION_CAPTURE_SOURCE", source_path.as_os_str())
                .env(
                    "SQLITE_REPLICATION_CAPTURE_REPOSITORY",
                    repository_path.as_os_str(),
                )
                .env("SQLITE_REPLICATION_CAPTURE_MARKER", marker_path.as_os_str())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("start child capture");
            let deadline = Instant::now() + Duration::from_secs(15);
            while !marker_path.exists() {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("child did not reach durability boundary {name}");
                }
                thread::sleep(Duration::from_millis(5));
            }
            child.kill().expect("send SIGKILL to child capture");
            let status = child.wait().expect("wait for killed child");
            assert!(
                !status.success(),
                "child unexpectedly exited cleanly at {name}"
            );

            let recovered = capture_once(&source_path, &repository_path, 200)
                .unwrap_or_else(|error| panic!("recover after SIGKILL at {name}: {error}"));
            assert_eq!(recovered.state.database.txid, 2);
            if ltx_already_published {
                assert!(recovered.transactions.is_empty());
            } else {
                assert_eq!(recovered.transactions.len(), 1);
            }
        }
    }

    #[test]
    fn capture_crash_child() {
        if std::env::var("SQLITE_REPLICATION_CAPTURE_CRASH_CHILD").as_deref() != Ok("1") {
            return;
        }
        let source_path = std::path::PathBuf::from(
            std::env::var_os("SQLITE_REPLICATION_CAPTURE_SOURCE").expect("source path env"),
        );
        let repository_path = std::path::PathBuf::from(
            std::env::var_os("SQLITE_REPLICATION_CAPTURE_REPOSITORY").expect("repository path env"),
        );
        let marker_path = std::path::PathBuf::from(
            std::env::var_os("SQLITE_REPLICATION_CAPTURE_MARKER").expect("marker path env"),
        );
        let want_point =
            std::env::var("SQLITE_REPLICATION_CAPTURE_CRASH_POINT").expect("crash point env");
        let result = capture_once_with_hook(&source_path, &repository_path, 199, |point, _| {
            let matches = match want_point.as_str() {
                "before-stage-sync" => point == FaultPoint::Spool(SpoolFaultPoint::BeforeStageSync),
                "after-stage-sync" => point == FaultPoint::Spool(SpoolFaultPoint::AfterStageSync),
                "before-ltx-publish" => {
                    point == FaultPoint::Spool(SpoolFaultPoint::BeforeLtxPublish)
                }
                "after-ltx-publish" => point == FaultPoint::Spool(SpoolFaultPoint::AfterLtxPublish),
                "before-watermark-write" => {
                    point == FaultPoint::Spool(SpoolFaultPoint::BeforeWatermarkWrite)
                }
                "before-watermark-sync" => {
                    point == FaultPoint::Spool(SpoolFaultPoint::BeforeWatermarkSync)
                }
                "after-watermark-sync" => {
                    point == FaultPoint::Spool(SpoolFaultPoint::AfterWatermarkSync)
                }
                "after-watermark" => point == FaultPoint::Spool(SpoolFaultPoint::AfterWatermark),
                _ => false,
            };
            if matches {
                File::create(&marker_path)?.sync_all()?;
                loop {
                    thread::sleep(Duration::from_secs(1));
                }
            }
            Ok(())
        });
        match result {
            Ok(_) => panic!("child capture completed without requested fault"),
            Err(error) => panic!("child capture failed before requested fault: {error}"),
        }
    }
}
