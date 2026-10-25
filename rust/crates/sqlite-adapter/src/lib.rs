//! SQLite Online Backup API adapter supported by the initial M1 vertical.

use std::fs::{self, File, Metadata};
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use rusqlite::backup::{Backup, StepResult};
use rusqlite::{Connection, OpenFlags};

const MINIMUM_SQLITE_VERSION: (u32, u32, u32) = (3, 51, 3);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CaptureObservation {
    pub busy: i64,
    pub log_frames: i64,
    pub checkpointed: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StableWal {
    pub bytes: Vec<u8>,
    pub observation: CaptureObservation,
}

/// Holds a SQLite writer reservation and a read snapshot while a capture is staged.
pub struct CaptureLease {
    source_path: PathBuf,
    writer: Option<Connection>,
    reader: Option<Connection>,
}

impl CaptureLease {
    /// Reads a stable WAL sidecar while both SQLite transactions remain open.
    pub fn read_stable_wal(&self) -> io::Result<StableWal> {
        if self.writer.is_none() || self.reader.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "SQLite capture lease is released",
            ));
        }
        let before = observe_checkpoint(&self.source_path)?;
        let wal_path = sidecar_path(&self.source_path, "-wal");
        let (bytes, wal_file_metadata) = read_stable_wal_file(&wal_path, || Ok(()))?;
        let after = observe_checkpoint(&self.source_path)?;
        if before.log_frames != after.log_frames || before.checkpointed != after.checkpointed {
            return Err(io::Error::other(
                "SQLite WAL checkpoint observation changed during capture",
            ));
        }
        // ctime is checked around the file read; this later check is path identity only.
        verify_stable_wal_path(&wal_path, wal_file_metadata.as_ref(), false)?;
        if after.log_frames > 0 && wal_file_metadata.is_none() {
            return Err(io::Error::other(
                "SQLite reports WAL frames but the sidecar is absent",
            ));
        }
        Ok(StableWal {
            bytes,
            observation: after,
        })
    }

    /// Releases the reader pin before the writer reservation.
    pub fn release(self) -> io::Result<()> {
        self.release_with_hook(|| {})
    }

    fn release_with_hook<F>(mut self, after_reader_release: F) -> io::Result<()>
    where
        F: FnOnce(),
    {
        let mut failures = Vec::new();
        if let Some(reader) = self.reader.take() {
            if let Err(error) = reader.execute_batch("ROLLBACK;") {
                failures.push(format!("release SQLite read pin: {error}"));
            }
            drop(reader);
        }
        after_reader_release();
        if let Some(writer) = self.writer.take() {
            if let Err(error) = writer.execute_batch("ROLLBACK;") {
                failures.push(format!("release SQLite writer guard: {error}"));
            }
            drop(writer);
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(io::Error::other(failures.join("; ")))
        }
    }
}

fn read_stable_wal_file<F>(
    wal_path: &Path,
    after_open: F,
) -> io::Result<(Vec<u8>, Option<Metadata>)>
where
    F: FnOnce() -> io::Result<()>,
{
    let mut file = match File::open(wal_path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok((Vec::new(), None));
        }
        Err(error) => {
            return Err(io::Error::other(format!(
                "open SQLite WAL for capture: {error}"
            )));
        }
    };
    let before_metadata = file.metadata()?;
    after_open()?;

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let after_metadata = file.metadata()?;
    if before_metadata.len() != after_metadata.len()
        || before_metadata.modified().ok() != after_metadata.modified().ok()
        || after_metadata.len() != bytes.len() as u64
    {
        return Err(io::Error::other("SQLite WAL changed while it was read"));
    }
    if !same_file_change_time(&before_metadata, &after_metadata)? {
        return Err(io::Error::other(
            "SQLite WAL change time changed while it was read",
        ));
    }
    verify_stable_wal_path(wal_path, Some(&before_metadata), true)?;
    Ok((bytes, Some(before_metadata)))
}

fn verify_stable_wal_path(
    wal_path: &Path,
    expected: Option<&Metadata>,
    verify_change_time: bool,
) -> io::Result<()> {
    let current = match fs::metadata(wal_path) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(io::Error::other(format!(
                "stat SQLite WAL after capture: {error}"
            )));
        }
    };
    match (expected, current.as_ref()) {
        (None, None) => Ok(()),
        (None, Some(_)) => Err(io::Error::other("SQLite WAL appeared while it was read")),
        (Some(_), None) => Err(io::Error::other(
            "SQLite WAL path disappeared while it was read",
        )),
        (Some(expected), Some(current)) => {
            if !same_file_identity(expected, current)? {
                return Err(io::Error::other(
                    "SQLite WAL path changed while it was read",
                ));
            }
            if verify_change_time && !same_file_change_time(expected, current)? {
                return Err(io::Error::other(
                    "SQLite WAL change time changed after it was read",
                ));
            }
            Ok(())
        }
    }
}

#[cfg(unix)]
fn same_file_identity(left: &Metadata, right: &Metadata) -> io::Result<bool> {
    use std::os::unix::fs::MetadataExt;

    Ok(left.dev() == right.dev() && left.ino() == right.ino())
}

#[cfg(unix)]
fn same_file_change_time(left: &Metadata, right: &Metadata) -> io::Result<bool> {
    use std::os::unix::fs::MetadataExt;

    Ok(left.ctime() == right.ctime() && left.ctime_nsec() == right.ctime_nsec())
}

#[cfg(not(unix))]
fn same_file_identity(_left: &Metadata, _right: &Metadata) -> io::Result<bool> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "SQLite WAL identity validation is only supported on Unix",
    ))
}

#[cfg(not(unix))]
fn same_file_change_time(_left: &Metadata, _right: &Metadata) -> io::Result<bool> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "SQLite WAL change-time validation is only supported on Unix",
    ))
}

/// Acquires a managed BEGIN IMMEDIATE writer guard followed by a pinned reader.
pub fn begin_capture_lease(path: &Path) -> io::Result<CaptureLease> {
    let writer = open_capture_connection(path)?;
    writer
        .execute_batch("BEGIN IMMEDIATE;")
        .map_err(sqlite_error)?;
    let reader = open_capture_connection(path)?;
    reader.execute_batch("BEGIN;").map_err(sqlite_error)?;
    let _schema_entries: i64 = reader
        .query_row("SELECT count(*) FROM sqlite_schema", [], |row| row.get(0))
        .map_err(|error| io::Error::other(format!("pin SQLite read snapshot: {error}")))?;
    Ok(CaptureLease {
        source_path: path.to_path_buf(),
        writer: Some(writer),
        reader: Some(reader),
    })
}

fn open_capture_connection(path: &Path) -> io::Result<Connection> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(sqlite_error)?;
    connection
        .busy_timeout(Duration::ZERO)
        .map_err(sqlite_error)?;
    ensure_supported_version(&connection)?;
    let journal_mode: String = connection
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .map_err(sqlite_error)?;
    if !journal_mode.eq_ignore_ascii_case("wal") {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("SQLite journal_mode must be WAL, got {journal_mode:?}"),
        ));
    }
    connection
        .execute_batch("PRAGMA wal_autocheckpoint=0;")
        .map_err(sqlite_error)?;
    let autocheckpoint: i64 = connection
        .query_row("PRAGMA wal_autocheckpoint", [], |row| row.get(0))
        .map_err(sqlite_error)?;
    if autocheckpoint != 0 {
        return Err(io::Error::other(format!(
            "managed SQLite connection has autocheckpoint={autocheckpoint}"
        )));
    }
    Ok(connection)
}

fn observe_checkpoint(path: &Path) -> io::Result<CaptureObservation> {
    let connection = open_capture_connection(path)?;
    let observation = connection
        .query_row("PRAGMA wal_checkpoint(NOOP)", [], |row| {
            Ok(CaptureObservation {
                busy: row.get(0)?,
                log_frames: row.get(1)?,
                checkpointed: row.get(2)?,
            })
        })
        .map_err(|error| io::Error::other(format!("observe SQLite WAL boundary: {error}")))?;
    if observation.busy != 0 {
        return Err(io::Error::other(format!(
            "SQLite NOOP checkpoint reported busy={}",
            observation.busy
        )));
    }
    Ok(observation)
}

fn sidecar_path(database_path: &Path, suffix: &str) -> PathBuf {
    let mut path = database_path.as_os_str().to_os_string();
    path.push(suffix);
    path.into()
}

/// Creates a consistent offline image using SQLite's Online Backup API.
pub fn create_online_backup(source_path: &Path, destination_path: &Path) -> io::Result<()> {
    create_online_backup_inner(source_path, destination_path, || Ok(()))
}

/// Creates a consistent image and cooperatively checks cancellation between
/// SQLite Online Backup steps. A step already in progress is not preempted.
pub fn create_online_backup_cancellable(
    source_path: &Path,
    destination_path: &Path,
    cancelled: &AtomicBool,
) -> io::Result<()> {
    create_online_backup_inner(source_path, destination_path, || {
        if cancelled.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "SQLite Online Backup was cancelled",
            ));
        }
        Ok(())
    })
}

fn create_online_backup_inner<F>(
    source_path: &Path,
    destination_path: &Path,
    mut check_cancelled: F,
) -> io::Result<()>
where
    F: FnMut() -> io::Result<()>,
{
    check_cancelled()?;
    let source = Connection::open_with_flags(
        source_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(sqlite_error)?;
    ensure_supported_version(&source)?;
    let journal_mode: String = source
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .map_err(sqlite_error)?;
    if !journal_mode.eq_ignore_ascii_case("wal") {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("SQLite journal_mode must be WAL, got {journal_mode:?}"),
        ));
    }

    let mut destination = Connection::open(destination_path).map_err(sqlite_error)?;
    let backup = Backup::new(&source, &mut destination).map_err(sqlite_error)?;
    loop {
        check_cancelled()?;
        match backup.step(128).map_err(sqlite_error)? {
            StepResult::Done => break,
            StepResult::More | StepResult::Busy | StepResult::Locked => {
                thread::sleep(Duration::from_millis(10));
            }
            _ => {
                return Err(io::Error::other(
                    "SQLite Online Backup API returned unknown state",
                ));
            }
        }
    }
    check_cancelled()?;
    drop(backup);
    destination
        .execute_batch("PRAGMA journal_mode=DELETE;")
        .map_err(sqlite_error)?;
    drop(destination);
    source.close().map_err(|(_, error)| sqlite_error(error))?;
    Ok(())
}

/// Verifies runtime compatibility and executes both SQLite integrity checks.
pub fn verify_database(path: &Path) -> io::Result<()> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(sqlite_error)?;
    ensure_supported_version(&connection)?;
    check_pragma(&connection, "quick_check")?;
    check_pragma(&connection, "integrity_check")?;
    connection
        .close()
        .map_err(|(_, error)| sqlite_error(error))?;
    Ok(())
}

fn ensure_supported_version(connection: &Connection) -> io::Result<()> {
    let version: String = connection
        .query_row("SELECT sqlite_version()", [], |row| row.get(0))
        .map_err(sqlite_error)?;
    let parsed = parse_version(&version).ok_or_else(|| {
        io::Error::new(io::ErrorKind::Unsupported, "invalid SQLite runtime version")
    })?;
    if parsed < MINIMUM_SQLITE_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("SQLite {version} is unsupported; require >= 3.51.3"),
        ));
    }
    Ok(())
}

fn parse_version(version: &str) -> Option<(u32, u32, u32)> {
    let mut components = version.split('.');
    let major = components.next()?.parse().ok()?;
    let minor = components.next()?.parse().ok()?;
    let patch = components.next()?.parse().ok()?;
    if components.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn check_pragma(connection: &Connection, pragma: &str) -> io::Result<()> {
    let mut statement = connection
        .prepare(&format!("PRAGMA {pragma}"))
        .map_err(sqlite_error)?;
    let mut rows = statement.query([]).map_err(sqlite_error)?;
    let mut seen = false;
    while let Some(row) = rows.next().map_err(sqlite_error)? {
        let value: String = row.get(0).map_err(sqlite_error)?;
        seen = true;
        if !value.eq_ignore_ascii_case("ok") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("SQLite {pragma} failed: {value}"),
            ));
        }
    }
    if !seen {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("SQLite {pragma} returned no result"),
        ));
    }
    Ok(())
}

fn sqlite_error(error: rusqlite::Error) -> io::Error {
    io::Error::other(error)
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File, FileTimes};
    use std::path::Path;
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, SystemTime};

    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;

    use rusqlite::{Connection, ErrorCode, OpenFlags};

    use super::{
        begin_capture_lease, create_online_backup, create_online_backup_inner,
        read_stable_wal_file, verify_database, verify_stable_wal_path,
    };

    #[cfg(unix)]
    #[test]
    fn stable_wal_read_rejects_truncation_during_read() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let wal_path = directory.path().join("source.sqlite-wal");
        fs::write(&wal_path, b"0123456789abcdef").expect("create WAL sidecar");

        let error = read_stable_wal_file(&wal_path, || {
            let file = fs::OpenOptions::new().write(true).open(&wal_path)?;
            file.set_len(8)
        })
        .expect_err("truncation during the read should be rejected");
        assert!(error.to_string().contains("changed while it was read"));
    }

    #[cfg(unix)]
    #[test]
    fn stable_wal_read_rejects_same_metadata_path_replacement() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let wal_path = directory.path().join("source.sqlite-wal");
        let original_target = directory.path().join("original-wal-data");
        let replacement_target = directory.path().join("replacement-wal-data");
        let replacement_link = directory.path().join("replacement-wal-link");
        let original = b"0123456789abcdef";
        let replacement = b"fedcba9876543210";
        fs::write(&original_target, original).expect("create original WAL target");
        fs::write(&replacement_target, replacement).expect("create replacement WAL target");
        std::os::unix::fs::symlink(&original_target, &wal_path)
            .expect("create original WAL sidecar link");
        let original_metadata = fs::metadata(&wal_path).expect("stat original WAL sidecar");
        let original_modified = original_metadata
            .modified()
            .expect("read original modification time");

        let error = read_stable_wal_file(&wal_path, || {
            File::open(&replacement_target)?
                .set_times(FileTimes::new().set_modified(original_modified))?;
            std::os::unix::fs::symlink(&replacement_target, &replacement_link)?;
            fs::rename(&replacement_link, &wal_path)?;
            let replacement_metadata = fs::metadata(&wal_path)?;
            if replacement_metadata.len() != original_metadata.len()
                || replacement_metadata.modified().ok() != Some(original_modified)
            {
                return Err(std::io::Error::other(
                    "replacement fixture did not preserve size and modification time",
                ));
            }
            Ok(())
        })
        .expect_err("same-size, same-time replacement should be rejected");
        assert!(error.to_string().contains("path changed while it was read"));
    }

    #[cfg(unix)]
    #[test]
    fn stable_wal_read_rejects_in_place_rewrite_with_restored_mod_time() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let wal_path = directory.path().join("source.sqlite-wal");
        let original = b"original-frame".repeat(8);
        let replacement = b"replaced-frame".repeat(8);
        assert_eq!(original.len(), replacement.len());
        fs::write(&wal_path, &original).expect("create original WAL sidecar");
        let known_modified = SystemTime::UNIX_EPOCH + Duration::from_secs(978_307_200);
        File::open(&wal_path)
            .expect("open original WAL sidecar")
            .set_times(FileTimes::new().set_modified(known_modified))
            .expect("set original WAL timestamps");
        let original_metadata = fs::metadata(&wal_path).expect("stat original WAL sidecar");
        let original_change_time = (original_metadata.ctime(), original_metadata.ctime_nsec());

        let error = read_stable_wal_file(&wal_path, || {
            fs::write(&wal_path, &replacement)?;
            File::open(&wal_path)?.set_times(FileTimes::new().set_modified(known_modified))?;
            let replacement_metadata = fs::metadata(&wal_path)?;
            if replacement_metadata.len() != original_metadata.len()
                || replacement_metadata.modified()? != known_modified
            {
                return Err(std::io::Error::other(
                    "in-place rewrite fixture did not preserve size and modification time",
                ));
            }
            if (
                replacement_metadata.ctime(),
                replacement_metadata.ctime_nsec(),
            ) == original_change_time
            {
                return Err(std::io::Error::other(
                    "in-place rewrite fixture did not advance change time",
                ));
            }
            Ok(())
        })
        .expect_err("same-size in-place rewrite with restored mtime should be rejected");
        assert!(
            error
                .to_string()
                .contains("change time changed while it was read")
        );
    }

    #[cfg(unix)]
    #[test]
    fn stable_wal_read_rejects_sidecar_appearance_after_missing_check() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let wal_path = directory.path().join("source.sqlite-wal");
        verify_stable_wal_path(&wal_path, None, false).expect("verify absent WAL sidecar");
        fs::write(&wal_path, b"new WAL generation")
            .expect("create WAL sidecar after absence check");

        let error = verify_stable_wal_path(&wal_path, None, false)
            .expect_err("WAL sidecar appearance after absent read should be rejected");
        assert!(error.to_string().contains("WAL appeared"));
    }

    #[cfg(unix)]
    #[test]
    fn stable_wal_read_rejects_sidecar_disappearance_after_open() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let wal_path = directory.path().join("source.sqlite-wal");
        fs::write(&wal_path, b"captured WAL generation").expect("create WAL sidecar");
        let wal_metadata = fs::metadata(&wal_path).expect("stat WAL sidecar");
        fs::remove_file(&wal_path).expect("remove WAL sidecar");

        let error = verify_stable_wal_path(&wal_path, Some(&wal_metadata), false)
            .expect_err("WAL sidecar disappearance after open should be rejected");
        assert!(error.to_string().contains("path disappeared"));
    }

    #[test]
    fn online_backup_creates_verified_database() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source_path = directory.path().join("source.sqlite");
        let backup_path = directory.path().join("backup.sqlite");
        let connection = Connection::open(&source_path).expect("open source");
        connection
            .execute_batch(
                "PRAGMA journal_mode=WAL;
                 CREATE TABLE records (id INTEGER PRIMARY KEY, value TEXT NOT NULL);
                 INSERT INTO records(value) VALUES ('one'), ('two');",
            )
            .expect("create source data");
        drop(connection);

        create_online_backup(&source_path, &backup_path).expect("online backup");
        verify_database(&backup_path).expect("verify backup");
        let restored = Connection::open(Path::new(&backup_path)).expect("open backup");
        let count: i64 = restored
            .query_row("SELECT count(*) FROM records", [], |row| row.get(0))
            .expect("count rows");
        assert_eq!(count, 2);
    }

    #[test]
    fn online_backup_checks_cancellation_between_sqlite_steps() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source_path = directory.path().join("source.sqlite");
        let backup_path = directory.path().join("backup.sqlite");
        let connection = Connection::open(&source_path).expect("open source");
        connection
            .execute_batch(
                "PRAGMA page_size=512;
                 PRAGMA journal_mode=WAL;
                 CREATE TABLE payloads (value BLOB NOT NULL);
                 INSERT INTO payloads VALUES (zeroblob(262144));",
            )
            .expect("create multi-page source database");
        drop(connection);

        let mut cancellation_checks = 0;
        let result = create_online_backup_inner(&source_path, &backup_path, || {
            cancellation_checks += 1;
            if cancellation_checks == 3 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "test cancellation between online backup steps",
                ));
            }
            Ok(())
        });
        let error = result.expect_err("backup should stop at the second step boundary");
        assert_eq!(error.kind(), std::io::ErrorKind::Interrupted);
        assert_eq!(cancellation_checks, 3);
    }

    #[test]
    fn capture_lease_keeps_writer_guard_after_reader_pin_release() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database_path = directory.path().join("capture.sqlite");
        let initializer = Connection::open(&database_path).expect("open SQLite source");
        initializer
            .execute_batch(
                "PRAGMA journal_mode=WAL;
                 CREATE TABLE records (id INTEGER PRIMARY KEY);
                 INSERT INTO records VALUES (1);",
            )
            .expect("initialize source database");
        drop(initializer);

        let lease = begin_capture_lease(&database_path).expect("begin capture lease");
        let writer = open_test_connection(&database_path);
        let (reader_released_sender, reader_released_receiver) = mpsc::channel();
        let (continue_sender, continue_receiver) = mpsc::channel();
        let release_thread = thread::spawn(move || {
            lease.release_with_hook(|| {
                reader_released_sender
                    .send(())
                    .expect("signal reader-pin release");
                continue_receiver
                    .recv()
                    .expect("wait before writer-guard release");
            })
        });

        let reader_released = reader_released_receiver
            .recv_timeout(Duration::from_secs(5))
            .is_ok();
        let mut writer_begin_error = None;
        let mut checkpoint_result = None;
        if reader_released {
            match writer.execute_batch("BEGIN IMMEDIATE;") {
                Ok(()) => {
                    writer
                        .execute_batch("ROLLBACK;")
                        .expect("rollback unexpectedly acquired writer transaction");
                }
                Err(error) => {
                    writer_begin_error = Some(error);
                }
            }
            checkpoint_result =
                Some(
                    writer.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, i64>(2)?,
                        ))
                    }),
                );
        }
        continue_sender
            .send(())
            .expect("continue capture lease release");
        let release_result = release_thread
            .join()
            .expect("capture lease release thread did not panic");
        release_result.expect("release capture lease");

        assert!(
            reader_released,
            "reader-pin release barrier was not reached"
        );
        let writer_begin_error =
            writer_begin_error.expect("writer acquired its lock before writer-guard release");
        assert_eq!(
            writer_begin_error.sqlite_error_code(),
            Some(ErrorCode::DatabaseBusy)
        );
        let (checkpoint_busy, checkpoint_frames, checkpointed): (i64, i64, i64) = checkpoint_result
            .expect("checkpoint result was not collected")
            .expect("checkpoint during release barrier");
        assert_ne!(
            checkpoint_busy, 0,
            "TRUNCATE completed before writer-guard release: log={checkpoint_frames}, checkpointed={checkpointed}"
        );
        writer
            .execute_batch("BEGIN IMMEDIATE;")
            .expect("writer acquires lock after writer-guard release");
        writer
            .execute_batch("ROLLBACK;")
            .expect("release writer acquired after capture lease");
    }

    fn open_test_connection(path: &Path) -> Connection {
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )
        .expect("open test connection");
        connection
            .busy_timeout(Duration::ZERO)
            .expect("disable test busy timeout");
        connection
            .execute_batch("PRAGMA wal_autocheckpoint=0;")
            .expect("disable test autocheckpoint");
        connection
    }
}
