use std::fs;
use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};
use sqlite_replication_sqlite::{begin_capture_lease, create_online_backup, verify_database};
use sqlite_replication_wal::scan;

#[derive(Debug)]
struct CheckpointResult {
    busy: i64,
    log_frames: i64,
    checkpointed: i64,
}

#[test]
fn capture_lease_holds_locks_and_reads_stable_wal() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("capture.sqlite");
    let initializer = open_test_connection(&database_path);
    initializer
        .execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE records (id INTEGER PRIMARY KEY);
             INSERT INTO records VALUES (1);",
        )
        .expect("initialize source database");

    let busy_writer = open_test_connection(&database_path);
    busy_writer
        .execute_batch("BEGIN IMMEDIATE;")
        .expect("prepare external writer lock");
    assert!(
        begin_capture_lease(&database_path).is_err(),
        "capture lease succeeded while an external writer held BEGIN IMMEDIATE"
    );
    busy_writer
        .execute_batch("ROLLBACK;")
        .expect("release prepared external writer");

    let lease = begin_capture_lease(&database_path).expect("begin capture lease");
    let stable = lease.read_stable_wal().expect("read stable WAL");
    let scanned = scan(&stable.bytes).expect("validate captured WAL");
    assert_eq!(scanned.frames.len() as i64, stable.observation.log_frames);
    let wal_path = sidecar_path(&database_path, "-wal");
    let before_checkpoint = fs::read(&wal_path).expect("read WAL before checkpoint");

    let writer = open_test_connection(&database_path);
    assert!(writer.execute_batch("BEGIN IMMEDIATE;").is_err());
    let checkpointer = open_test_connection(&database_path);
    let truncate = checkpoint_status(&checkpointer, "TRUNCATE").expect("attempt TRUNCATE");
    assert_ne!(truncate.busy, 0);
    assert_eq!(
        fs::read(&wal_path).expect("read WAL after checkpoint"),
        before_checkpoint,
        "TRUNCATE changed WAL bytes while the lease was held"
    );

    lease.release().expect("release capture lease");
    writer
        .execute_batch("BEGIN IMMEDIATE;")
        .expect("external writer acquires lock after lease release");
    writer
        .execute("INSERT INTO records VALUES (2)", [])
        .expect("external writer writes after lease release");
    writer
        .execute_batch("COMMIT;")
        .expect("external writer commits after lease release");
}

#[test]
fn immediate_writer_lock_and_noop_checkpoint_boundary() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    let initializer = open_test_connection(&database_path);
    let runtime_version: String = initializer
        .query_row("SELECT sqlite_version()", [], |row| row.get(0))
        .expect("read SQLite runtime version");
    assert!(
        runtime_version_at_least(&runtime_version, (3, 51, 3)),
        "unsupported SQLite runtime: {runtime_version}"
    );
    println!("CP-01 lock experiment SQLite runtime: {runtime_version}");

    let journal_mode: String = initializer
        .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
        .expect("enable WAL mode");
    assert_eq!(journal_mode, "wal");
    initializer
        .execute_batch(
            "CREATE TABLE records (id INTEGER PRIMARY KEY);
             INSERT INTO records(id) VALUES (1);",
        )
        .expect("create and seed records table");

    let reader = open_test_connection(&database_path);
    reader
        .execute_batch("BEGIN;")
        .expect("begin reader transaction");
    let reader_count: i64 = reader
        .query_row("SELECT count(*) FROM records", [], |row| row.get(0))
        .expect("pin reader snapshot");
    assert_eq!(reader_count, 1);

    let guard = open_test_connection(&database_path);
    guard
        .execute_batch("BEGIN IMMEDIATE;")
        .expect("acquire immediate writer lock");
    let backup_path = directory.path().join("guarded-backup.sqlite");
    create_online_backup(&database_path, &backup_path)
        .expect("create online backup while guard holds writer lock");
    verify_database(&backup_path).expect("verify online backup under writer guard");

    let checkpointer = open_test_connection(&database_path);
    let baseline_status = checkpoint_status(&checkpointer, "NOOP").expect("NOOP checkpoint");
    assert!(
        baseline_status.log_frames > 0,
        "NOOP returned no committed WAL frames: {baseline_status:?}"
    );
    let wal_path = sidecar_path(&database_path, "-wal");
    let wal_before_checkpoints = fs::read(&wal_path).expect("read WAL before checkpoints");
    assert!(
        wal_before_checkpoints.len() >= 32,
        "WAL header is incomplete"
    );

    for mode in ["PASSIVE", "FULL", "RESTART", "TRUNCATE"] {
        let status = checkpoint_status(&checkpointer, mode)
            .unwrap_or_else(|error| panic!("{mode} checkpoint attempt failed: {error}"));
        assert!(
            mode == "PASSIVE" || status.busy != 0,
            "{mode} checkpoint did not report that the writer lock prevented completion: {status:?}"
        );
        let wal_after_checkpoint = fs::read(&wal_path)
            .unwrap_or_else(|error| panic!("read WAL after {mode} checkpoint attempt: {error}"));
        assert_eq!(
            wal_after_checkpoint, wal_before_checkpoints,
            "{mode} checkpoint changed WAL bytes while BEGIN IMMEDIATE was held"
        );
        let status_after_checkpoint =
            checkpoint_status(&checkpointer, "NOOP").expect("NOOP after checkpoint attempt");
        assert_eq!(
            status_after_checkpoint.log_frames, baseline_status.log_frames,
            "NOOP boundary changed during {mode} checkpoint attempt"
        );
    }

    let writer = open_test_connection(&database_path);
    assert!(
        writer.execute_batch("BEGIN IMMEDIATE;").is_err(),
        "second writer acquired BEGIN IMMEDIATE while the guard held the writer lock"
    );

    guard
        .execute_batch("ROLLBACK;")
        .expect("release immediate writer lock");
    writer
        .execute_batch(
            "BEGIN IMMEDIATE;
             INSERT INTO records(id) VALUES (2);
             COMMIT;",
        )
        .expect("writer commit after guard release");

    let pinned_count: i64 = reader
        .query_row("SELECT count(*) FROM records", [], |row| row.get(0))
        .expect("read from pinned snapshot after writer commit");
    assert_eq!(pinned_count, 1);

    let status_after_writer = checkpoint_status(&checkpointer, "NOOP").expect("NOOP after writer");
    assert!(
        status_after_writer.log_frames > baseline_status.log_frames,
        "NOOP boundary did not advance after writer commit: before={baseline_status:?}, after={status_after_writer:?}"
    );

    let truncate_status = checkpoint_status(&checkpointer, "TRUNCATE").expect("blocked TRUNCATE");
    assert!(
        truncate_status.busy != 0 && truncate_status.checkpointed < truncate_status.log_frames,
        "TRUNCATE did not report an incomplete checkpoint behind the pinned reader: {truncate_status:?}"
    );
    let wal_after_blocked_truncate = fs::read(&wal_path).expect("read WAL after blocked TRUNCATE");
    assert!(
        !wal_after_blocked_truncate.is_empty(),
        "TRUNCATE removed WAL bytes newer than the pinned reader"
    );

    reader
        .execute_batch("ROLLBACK;")
        .expect("release reader snapshot");
}

fn open_test_connection(path: &Path) -> Connection {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )
    .expect("open test database connection");
    connection
        .busy_timeout(Duration::ZERO)
        .expect("disable test busy timeout");
    connection
        .execute_batch("PRAGMA wal_autocheckpoint=0;")
        .expect("disable autocheckpoint for test connection");
    connection
}

fn checkpoint_status(connection: &Connection, mode: &str) -> rusqlite::Result<CheckpointResult> {
    connection.query_row(&format!("PRAGMA wal_checkpoint({mode})"), [], |row| {
        Ok(CheckpointResult {
            busy: row.get(0)?,
            log_frames: row.get(1)?,
            checkpointed: row.get(2)?,
        })
    })
}

fn sidecar_path(database_path: &Path, suffix: &str) -> std::path::PathBuf {
    let mut path = database_path.as_os_str().to_os_string();
    path.push(suffix);
    path.into()
}

fn runtime_version_at_least(version: &str, minimum: (u32, u32, u32)) -> bool {
    let mut components = version.split('.');
    let Some(major) = components
        .next()
        .and_then(|value| value.parse::<u32>().ok())
    else {
        return false;
    };
    let Some(minor) = components
        .next()
        .and_then(|value| value.parse::<u32>().ok())
    else {
        return false;
    };
    let Some(patch) = components
        .next()
        .and_then(|value| value.parse::<u32>().ok())
    else {
        return false;
    };
    if components.next().is_some() {
        return false;
    }
    (major, minor, patch) >= minimum
}
