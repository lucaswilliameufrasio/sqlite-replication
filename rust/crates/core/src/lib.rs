//! Synchronous bootstrap and offline restore API for the initial M1 vertical.
//! This crate deliberately does not expose WAL replication.

// Recovery planning remains private until the restore contract and CP-01 gate
// permit it to be wired into public restore operations.
#[allow(dead_code)]
mod recovery;

// CP-01 keeps this orchestration compiled and testable, but not wired into the
// public API until the required crash and platform matrix is complete.
#[allow(dead_code)]
pub(crate) mod capture;

use std::fs::{self, File};
use std::io::{self, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlite_replication_ltx::{decode_snapshot, encode_snapshot};
use sqlite_replication_sqlite::{create_online_backup, verify_database};
use sqlite_replication_storage_fs::{
    ensure_repository, ltx_path, promote_if_missing, sync_directory,
};

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
