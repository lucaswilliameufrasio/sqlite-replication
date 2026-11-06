use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::Connection;
use sqlite_replication_core::{
    AdmissionGate, ConservativeBudget, RestoreTarget, WriteGate, WritePermit, capture_once,
    capture_once_with_budget, restore_to_target, snapshot,
};
use sqlite_replication_spool::read_watermark;

struct TestGate {
    repository: PathBuf,
    acquired: Arc<AtomicBool>,
    released: Arc<AtomicBool>,
    verify_durable_txid: Option<u64>,
    release_error: Option<String>,
}

struct TestPermit {
    repository: PathBuf,
    released: Arc<AtomicBool>,
    verify_durable_txid: Option<u64>,
    release_error: Option<String>,
}

struct BudgetAwareGate(ConservativeBudget);
struct BudgetAwarePermit(ConservativeBudget);

impl WriteGate for BudgetAwareGate {
    type Permit = BudgetAwarePermit;

    fn acquire(&self) -> io::Result<Self::Permit> {
        self.0.reconcile(None)?;
        Ok(BudgetAwarePermit(self.0.clone()))
    }
}

impl WritePermit for BudgetAwarePermit {
    fn release(self) -> io::Result<()> {
        self.0.reconcile(None)
    }
}

#[test]
fn cooperative_public_capture_keeps_gate_callbacks_outside_budget_lock() {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return;
    }
    let directory = tempfile::tempdir().expect("temporary directory");
    let source = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    create_database(&source);
    snapshot(&source, &repository).expect("snapshot");
    let connection = Connection::open(&source).expect("open source");
    connection
        .pragma_update(None, "wal_autocheckpoint", "0")
        .expect("disable checkpoint");
    connection
        .execute("INSERT INTO records(value) VALUES ('cooperative')", [])
        .expect("commit");
    let budget = ConservativeBudget::new(AdmissionGate::new(), 1 << 20).expect("budget");
    budget
        .reconcile_with(|| Ok(Some(0)))
        .expect("test baseline");
    let gate = BudgetAwareGate(budget.clone());
    let result = capture_once_with_budget(&source, &repository, &gate, &budget).expect("capture");
    assert_eq!(result.txid, 2);
    let restored = directory.path().join("restored.sqlite");
    restore_to_target(&repository, &restored, RestoreTarget::Txid(2)).expect("restore");
    assert_eq!(row_count(&restored), 2);
    let error = capture_once_with_budget(
        directory.path().join("missing.sqlite"),
        &repository,
        &gate,
        &budget,
    )
    .expect_err("capture failure");
    assert_ne!(error.kind(), io::ErrorKind::Unsupported);
}

impl WriteGate for TestGate {
    type Permit = TestPermit;

    fn acquire(&self) -> io::Result<Self::Permit> {
        self.acquired.store(true, Ordering::Release);
        Ok(TestPermit {
            repository: self.repository.clone(),
            released: Arc::clone(&self.released),
            verify_durable_txid: self.verify_durable_txid,
            release_error: self.release_error.clone(),
        })
    }
}

impl WritePermit for TestPermit {
    fn release(self) -> io::Result<()> {
        if let Some(expected_txid) = self.verify_durable_txid {
            let watermark = read_watermark(&self.repository)?;
            if watermark.txid != expected_txid {
                return Err(io::Error::other(format!(
                    "write gate released at TXID {}, want {expected_txid}",
                    watermark.txid
                )));
            }
        }
        self.released.store(true, Ordering::Release);
        if let Some(message) = self.release_error {
            return Err(io::Error::other(message));
        }
        Ok(())
    }
}

#[test]
fn capture_requires_supported_platform_and_spools_before_releasing_gate() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let source_path = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    create_database(&source_path);
    snapshot(&source_path, &repository).expect("initial snapshot");

    let gate = TestGate {
        repository: repository.clone(),
        acquired: Arc::new(AtomicBool::new(false)),
        released: Arc::new(AtomicBool::new(false)),
        verify_durable_txid: Some(2),
        release_error: None,
    };

    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        let error = capture_once(&source_path, &repository, &gate)
            .expect_err("capture must fail closed outside Linux x86-64");
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
        assert!(
            !gate.acquired.load(Ordering::Acquire),
            "unsupported platform acquired write gate"
        );
        return;
    }

    let connection = Connection::open(&source_path).expect("open source SQLite database");
    connection
        .pragma_update(None, "wal_autocheckpoint", "0")
        .expect("disable autocheckpoint on test connection");
    connection
        .execute("INSERT INTO records(value) VALUES (?1)", ["captured"])
        .expect("commit source row");
    drop(connection);

    let result = capture_once(&source_path, &repository, &gate).expect("capture WAL");
    assert_eq!(result.txid, 2);
    assert!(result.transactions > 0);
    assert!(gate.acquired.load(Ordering::Acquire));
    assert!(gate.released.load(Ordering::Acquire));

    let by_txid = directory.path().join("restored-txid.sqlite");
    restore_to_target(&repository, &by_txid, RestoreTarget::Txid(2))
        .expect("restore incremental TXID");
    assert_eq!(row_count(&by_txid), 2);

    let by_timestamp = directory.path().join("restored-time.sqlite");
    restore_to_target(
        &repository,
        &by_timestamp,
        RestoreTarget::AtOrBeforeTimestampMillis(i64::MAX),
    )
    .expect("restore latest point by timestamp");
    assert_eq!(row_count(&by_timestamp), 2);

    let existing_contents = b"preserve existing destination";
    std::fs::write(&by_timestamp, existing_contents).expect("replace test destination");
    assert!(restore_to_target(&repository, &by_timestamp, RestoreTarget::Txid(2)).is_err());
    assert_eq!(
        std::fs::read(&by_timestamp).expect("read preserved destination"),
        existing_contents
    );
}

#[test]
fn capture_failure_still_releases_application_write_gate() {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return;
    }
    let directory = tempfile::tempdir().expect("temporary directory");
    let repository = directory.path().join("missing-repository");
    let source_path = directory.path().join("missing.sqlite");
    let gate = TestGate {
        repository: repository.clone(),
        acquired: Arc::new(AtomicBool::new(false)),
        released: Arc::new(AtomicBool::new(false)),
        verify_durable_txid: None,
        release_error: None,
    };

    let error = capture_once(&source_path, &repository, &gate)
        .expect_err("capture without an initialized repository must fail");
    assert_ne!(error.kind(), io::ErrorKind::Unsupported);
    assert!(gate.acquired.load(Ordering::Acquire));
    assert!(gate.released.load(Ordering::Acquire));
}

#[test]
fn release_error_does_not_hide_durable_capture() {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return;
    }
    let directory = tempfile::tempdir().expect("temporary directory");
    let source_path = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    create_database(&source_path);
    snapshot(&source_path, &repository).expect("initial snapshot");
    let connection = Connection::open(&source_path).expect("open source SQLite database");
    connection
        .execute("INSERT INTO records(value) VALUES (?1)", ["captured"])
        .expect("commit source row");
    drop(connection);

    let gate = TestGate {
        repository: repository.clone(),
        acquired: Arc::new(AtomicBool::new(false)),
        released: Arc::new(AtomicBool::new(false)),
        verify_durable_txid: Some(2),
        release_error: Some("write gate release failed".to_owned()),
    };
    let error = capture_once(&source_path, &repository, &gate)
        .expect_err("gate release failure is returned after durable capture");
    assert!(error.to_string().contains("release"));
    assert!(gate.released.load(Ordering::Acquire));
    assert_eq!(read_watermark(&repository).unwrap().txid, 2);
}

fn create_database(path: &Path) {
    let connection = Connection::open(path).expect("open source database");
    connection
        .execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             CREATE TABLE records (id INTEGER PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO records(value) VALUES ('base');",
        )
        .expect("initialize source database");
}

fn row_count(path: &Path) -> i64 {
    Connection::open(path)
        .expect("open restored database")
        .query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))
        .expect("count restored rows")
}
