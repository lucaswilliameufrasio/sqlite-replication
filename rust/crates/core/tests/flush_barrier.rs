use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use rusqlite::Connection;
use sqlite_replication_core::{
    CaptureResult, Coordinator, CoordinatorAdapter, CoordinatorConfig, Limits, RestoreTarget,
    WriteGate, WritePermit, capture_once, restore_to_target, snapshot,
};

struct IsolatedGate;

impl WriteGate for IsolatedGate {
    type Permit = Self;

    fn acquire(&self) -> io::Result<Self::Permit> {
        Ok(Self)
    }
}

impl WritePermit for IsolatedGate {
    fn release(self) -> io::Result<()> {
        Ok(())
    }
}

struct SqliteAdapter {
    source: PathBuf,
    repository: PathBuf,
}

impl CoordinatorAdapter for SqliteAdapter {
    fn observe_txid(&self) -> io::Result<Option<u64>> {
        Ok(None)
    }

    fn capture(&self) -> io::Result<CaptureResult> {
        capture_once(&self.source, &self.repository, &IsolatedGate)
    }

    fn set_write_admission(&self, _enabled: bool, _limits: Limits) -> io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn barrier_captures_checkpointed_changes_without_observed_txid() {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return;
    }
    let directory = tempfile::tempdir().expect("temporary directory");
    let source = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    let connection = Connection::open(&source).expect("open source");
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE records(value TEXT);").expect("initialize");
    snapshot(&source, &repository).expect("snapshot");
    connection
        .execute("INSERT INTO records VALUES ('first')", [])
        .expect("first commit");
    connection
        .execute("INSERT INTO records VALUES ('second')", [])
        .expect("second commit");
    let busy: i64 = connection
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))
        .expect("checkpoint");
    assert_eq!(busy, 0);
    let coordinator = Coordinator::new(CoordinatorConfig {
        repository_root: repository.clone(),
        database_path: source.clone(),
        adapter: Arc::new(SqliteAdapter {
            source,
            repository: repository.clone(),
        }),
        limits: Limits {
            max_spool_bytes: 1 << 30,
            max_wal_bytes: 1 << 30,
            max_unprotected_age: Duration::from_secs(60),
            max_uploads_in_flight: 1,
        },
        remote: None,
    })
    .expect("coordinator");
    coordinator.flush().await.expect("barrier capture");
    let restored = directory.path().join("restored.sqlite");
    restore_to_target(&repository, &restored, RestoreTarget::Txid(2)).expect("restore");
    let count: i64 = Connection::open(restored)
        .expect("open restore")
        .query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))
        .expect("count");
    assert_eq!(count, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn barrier_drains_accepted_writer_before_capturing_its_commit() {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return;
    }
    let directory = tempfile::tempdir().expect("temporary directory");
    let source = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    let connection = Connection::open(&source).expect("open source");
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE records(value TEXT);").expect("initialize");
    snapshot(&source, &repository).expect("snapshot");
    let coordinator = Arc::new(
        Coordinator::new(CoordinatorConfig {
            repository_root: repository.clone(),
            database_path: source.clone(),
            adapter: Arc::new(SqliteAdapter {
                source,
                repository: repository.clone(),
            }),
            limits: Limits {
                max_spool_bytes: 1 << 30,
                max_wal_bytes: 1 << 30,
                max_unprotected_age: Duration::from_secs(60),
                max_uploads_in_flight: 1,
            },
            remote: None,
        })
        .expect("coordinator"),
    );
    coordinator.flush().await.expect("initial barrier");
    let permit = coordinator
        .admission_gate()
        .try_accept()
        .expect("accept writer");
    let task_coordinator = coordinator.clone();
    let mut flush = tokio::spawn(async move { task_coordinator.flush().await });
    assert!(
        tokio::time::timeout(Duration::from_millis(150), &mut flush)
            .await
            .is_err()
    );
    connection
        .execute("INSERT INTO records VALUES ('accepted')", [])
        .expect("commit accepted write");
    let busy: i64 = connection
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))
        .expect("checkpoint");
    assert_eq!(busy, 0);
    drop(permit);
    tokio::time::timeout(Duration::from_secs(5), flush)
        .await
        .expect("barrier timeout")
        .expect("task")
        .expect("barrier");
    let restored = directory.path().join("restored.sqlite");
    restore_to_target(&repository, &restored, RestoreTarget::Txid(2)).expect("restore");
    let value: String = Connection::open(restored)
        .expect("open restore")
        .query_row("SELECT value FROM records", [], |row| row.get(0))
        .expect("value");
    assert_eq!(value, "accepted");
}
