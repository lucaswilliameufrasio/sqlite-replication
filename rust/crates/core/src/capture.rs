//! Internal WAL capture orchestration. This remains unavailable as public API
//! until the full CP-01 matrix is reviewed and approved.

use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use sqlite_replication_ltx::{DatabaseState, decode_delta, encode_delta};
use sqlite_replication_spool::{self, FaultPoint as SpoolFaultPoint};
use sqlite_replication_sqlite::{
    CaptureLease, begin_capture_lease, create_online_backup, create_online_backup_cancellable,
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

pub(crate) fn capture_once(
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

fn capture_once_with_hook<F>(
    source_path: &Path,
    repository_root: &Path,
    timestamp_millis: i64,
    hook: F,
) -> io::Result<Result>
where
    F: FnMut(FaultPoint, usize) -> io::Result<()>,
{
    capture_once_with_cancellation(source_path, repository_root, timestamp_millis, None, hook)
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
        hook,
    )
}

fn capture_once_with_cancellation<F>(
    source_path: &Path,
    repository_root: &Path,
    timestamp_millis: i64,
    cancelled: Option<&AtomicBool>,
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

    let state = sqlite_replication_spool::recover(repository_root, &stable_wal.bytes)?;
    check_cancelled(cancelled)?;
    let staging_directory = repository_root.join(".staging");
    fs::create_dir_all(&staging_directory)?;
    let source_image = tempfile::Builder::new()
        .prefix(".capture-source-")
        .suffix(".sqlite")
        .tempfile_in(&staging_directory)?;
    match cancelled {
        Some(token) => {
            create_online_backup_cancellable(source_path, source_image.path(), token)?;
        }
        None => {
            create_online_backup(source_path, source_image.path())?;
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

    sqlite_replication_spool::publish_transactions(
        repository_root,
        &transactions,
        &next_state,
        |point, index| {
            if point != SpoolFaultPoint::AfterWatermark {
                check_cancelled(cancelled)?;
            }
            hook(FaultPoint::Spool(point), index)?;
            if point == SpoolFaultPoint::AfterWatermark {
                return Ok(());
            }
            check_cancelled(cancelled)
        },
    )?;
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
