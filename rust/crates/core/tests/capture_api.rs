use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::Connection;
use sqlite_replication_core::{WriteGate, WritePermit, capture_once, snapshot};
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
