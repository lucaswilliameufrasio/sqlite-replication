use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::Connection;
use sqlite_replication_core::{
    AdmissionGate, ConservativeBudget, RestoreTarget, WriteGate, WritePermit, capture_once,
    capture_once_with_budget, capture_once_with_image_limits,
    capture_once_with_recovery_image_limit, capture_once_with_reserved_budget, restore_to_target,
    snapshot,
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

#[test]
fn batch_limit_capture_rejects_and_restores_after_retry() {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return;
    }
    let directory = tempfile::tempdir().expect("temporary directory");
    let source = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    create_database(&source);
    snapshot(&source, &repository).expect("snapshot");
    let connection = Connection::open(&source).expect("source");
    connection
        .execute("INSERT INTO records(value) VALUES ('batch retry')", [])
        .expect("commit");
    let budget = ConservativeBudget::new(AdmissionGate::new(), 10).expect("budget");
    budget.reconcile_with(|| Ok(Some(0))).expect("baseline");
    let gate = TestGate {
        repository: repository.clone(),
        acquired: Arc::new(AtomicBool::new(false)),
        released: Arc::new(AtomicBool::new(false)),
        verify_durable_txid: None,
        release_error: None,
    };
    let limits = |ltx_batch| sqlite_replication_core::CaptureLimits {
        recovery_image: 1 << 20,
        backup_image: 1 << 20,
        ltx_batch,
    };
    let error = sqlite_replication_core::capture_once_with_batch_limit(
        &source,
        &repository,
        &gate,
        &budget,
        1,
        limits(1),
    )
    .expect_err("limit");
    assert!(error.to_string().contains("LTX batch size limit exceeded"));
    assert_eq!(read_watermark(&repository).expect("watermark").txid, 1);
    budget.reconcile_with(|| Ok(Some(0))).expect("reconcile");
    let result = sqlite_replication_core::capture_once_with_batch_limit(
        &source,
        &repository,
        &gate,
        &budget,
        1,
        limits(1 << 20),
    )
    .expect("retry");
    assert_eq!(result.txid, 2);
    let restored = directory.path().join("restored.sqlite");
    restore_to_target(&repository, &restored, RestoreTarget::Txid(2)).expect("restore");
    let restored = Connection::open(restored).expect("open restore");
    let count: i64 = restored
        .query_row(
            "SELECT count(*) FROM records WHERE value='batch retry'",
            [],
            |row| row.get(0),
        )
        .expect("query");
    assert_eq!(count, 1);
}

#[cfg(unix)]
#[test]
fn repository_logical_measurement_fails_closed() {
    let dir = tempfile::tempdir().expect("repository");
    assert!(sqlite_replication_core::measure_repository_logical_bytes(dir.path()).is_err());
    std::fs::create_dir(dir.path().join("ltx")).expect("ltx");
    std::fs::write(dir.path().join("ltx/data"), b"1234").expect("data");
    assert_eq!(
        sqlite_replication_core::measure_repository_logical_bytes(dir.path()).expect("measure"),
        4
    );
    std::os::unix::fs::symlink("missing", dir.path().join("ltx/link")).expect("symlink");
    assert!(sqlite_replication_core::measure_repository_logical_bytes(dir.path()).is_err());
}

#[test]
fn derived_reservation_with_measured_spool_and_retry() {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return;
    }
    let dir = tempfile::tempdir().expect("temporary directory");
    let source = dir.path().join("source.sqlite");
    let repository = dir.path().join("repository");
    create_database(&source);
    snapshot(&source, &repository).expect("snapshot");
    std::fs::create_dir_all(repository.join(".staging")).expect("staging");
    std::fs::write(repository.join(".staging/residual-test"), b"backup residue")
        .expect("residual image");
    std::fs::write(repository.join("ltx/residual.stage"), b"ltx residue").expect("residual ltx");
    let connection = Connection::open(&source).expect("source");
    connection
        .execute("INSERT INTO records(value) VALUES ('derived retry')", [])
        .expect("commit");
    let measure =
        || sqlite_replication_core::measure_repository_logical_bytes(&repository).map(Some);
    let baseline = measure().expect("measure spool").expect("known");
    assert!(baseline > 0);
    let limits = || sqlite_replication_core::CaptureLimits {
        recovery_image: 1 << 20,
        backup_image: 1 << 20,
        ltx_batch: 1 << 20,
    };
    let gate = TestGate {
        repository: repository.clone(),
        acquired: Arc::new(AtomicBool::new(false)),
        released: Arc::new(AtomicBool::new(false)),
        verify_durable_txid: None,
        release_error: None,
    };
    let held = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(repository.join(".repository-coordination.lock"))
        .expect("lock file");
    held.try_lock().expect("hold repository lock");
    let blocked =
        ConservativeBudget::new(AdmissionGate::new(), baseline + (3 << 20)).expect("budget");
    assert!(
        sqlite_replication_core::capture_once_with_measured_reservation(
            &source,
            &repository,
            &gate,
            &blocked,
            limits(),
            4 << 20
        )
        .is_err()
    );
    drop(held);
    let small =
        ConservativeBudget::new(AdmissionGate::new(), baseline + (3 << 20) - 1).expect("budget");
    assert!(
        sqlite_replication_core::capture_once_with_measured_reservation(
            &source,
            &repository,
            &gate,
            &small,
            limits(),
            4 << 20
        )
        .is_err()
    );
    assert!(read_watermark(&repository).is_err());
    let budget =
        ConservativeBudget::new(AdmissionGate::new(), baseline + (3 << 20)).expect("budget");
    let result = sqlite_replication_core::capture_once_with_measured_reservation(
        &source,
        &repository,
        &gate,
        &budget,
        limits(),
        4 << 20,
    )
    .expect("retry");
    assert_eq!(result.txid, 2);
    let restored = dir.path().join("restored.sqlite");
    restore_to_target(&repository, &restored, RestoreTarget::Txid(2)).expect("restore");
    let restored = Connection::open(restored).expect("open restore");
    let count: i64 = restored
        .query_row(
            "SELECT count(*) FROM records WHERE value='derived retry'",
            [],
            |row| row.get(0),
        )
        .expect("query");
    assert_eq!(count, 1);
}

#[test]
fn derived_reservation_rejects_overflow_and_insufficient_budget() {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return;
    }
    let budget = ConservativeBudget::new(AdmissionGate::new(), 10).expect("budget");
    budget.reconcile_with(|| Ok(Some(5))).expect("baseline");
    let gate = TestGate {
        repository: "missing".into(),
        acquired: Arc::new(AtomicBool::new(false)),
        released: Arc::new(AtomicBool::new(false)),
        verify_durable_txid: None,
        release_error: None,
    };
    let overflow = sqlite_replication_core::CaptureLimits {
        recovery_image: u64::MAX,
        backup_image: 1,
        ltx_batch: 1,
    };
    let error = sqlite_replication_core::capture_once_with_derived_reservation(
        "missing", "missing", &gate, &budget, overflow, 10,
    )
    .expect_err("overflow");
    assert!(error.to_string().contains("overflow"));
    assert!(!gate.acquired.load(Ordering::SeqCst));
    let limits = sqlite_replication_core::CaptureLimits {
        recovery_image: 2,
        backup_image: 2,
        ltx_batch: 2,
    };
    let error = sqlite_replication_core::capture_once_with_derived_reservation(
        "missing", "missing", &gate, &budget, limits, 10,
    )
    .expect_err("insufficient budget");
    assert!(
        error.to_string().contains("capacity") || error.kind() == std::io::ErrorKind::WouldBlock,
        "{error}"
    );
}

#[test]
fn spool_limit_capture_rejects_and_restores_after_retry() {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return;
    }
    let dir = tempfile::tempdir().expect("temporary directory");
    let source = dir.path().join("source.sqlite");
    let repository = dir.path().join("repository");
    create_database(&source);
    snapshot(&source, &repository).expect("snapshot");
    let connection = Connection::open(&source).expect("source");
    connection
        .execute("INSERT INTO records(value) VALUES ('spool retry')", [])
        .expect("commit");
    let budget = ConservativeBudget::new(AdmissionGate::new(), 10).expect("budget");
    budget.reconcile_with(|| Ok(Some(0))).expect("baseline");
    let gate = TestGate {
        repository: repository.clone(),
        acquired: Arc::new(AtomicBool::new(false)),
        released: Arc::new(AtomicBool::new(false)),
        verify_durable_txid: None,
        release_error: None,
    };
    let limits = || sqlite_replication_core::CaptureLimits {
        recovery_image: 1 << 20,
        backup_image: 1 << 20,
        ltx_batch: 1 << 20,
    };
    let mut small_batch = limits();
    small_batch.ltx_batch = 1;
    let error = sqlite_replication_core::capture_once_with_spool_limit(
        &source,
        &repository,
        &gate,
        &budget,
        1,
        small_batch,
        1 << 20,
    )
    .expect_err("independent batch limit");
    assert!(error.to_string().contains("LTX batch size limit exceeded"));
    budget
        .reconcile_with(|| Ok(Some(0)))
        .expect("reconcile batch failure");
    let error = sqlite_replication_core::capture_once_with_spool_limit(
        &source,
        &repository,
        &gate,
        &budget,
        1,
        limits(),
        1,
    )
    .expect_err("spool limit");
    assert!(error.to_string().contains("spool size limit exceeded"));
    assert_eq!(read_watermark(&repository).expect("watermark").txid, 1);
    budget.reconcile_with(|| Ok(Some(0))).expect("reconcile");
    let result = sqlite_replication_core::capture_once_with_spool_limit(
        &source,
        &repository,
        &gate,
        &budget,
        1,
        limits(),
        1 << 20,
    )
    .expect("retry");
    assert_eq!(result.txid, 2);
    let restored = dir.path().join("restored.sqlite");
    restore_to_target(&repository, &restored, RestoreTarget::Txid(2)).expect("restore");
    let restored = Connection::open(restored).expect("open restore");
    let count: i64 = restored
        .query_row(
            "SELECT count(*) FROM records WHERE value='spool retry'",
            [],
            |row| row.get(0),
        )
        .expect("query");
    assert_eq!(count, 1);
}

#[test]
fn limited_backup_capture_fails_closed_and_retries() {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return;
    }
    let directory = tempfile::tempdir().expect("temporary directory");
    let source = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    create_database(&source);
    snapshot(&source, &repository).expect("snapshot");
    let connection = Connection::open(&source).expect("source");
    connection
        .execute("INSERT INTO records(value) VALUES (zeroblob(131072))", [])
        .expect("commit");
    let admission = AdmissionGate::new();
    let budget = ConservativeBudget::new(admission, 10).expect("budget");
    budget.reconcile_with(|| Ok(Some(0))).expect("baseline");
    let gate = TestGate {
        repository: repository.clone(),
        acquired: Arc::new(AtomicBool::new(false)),
        released: Arc::new(AtomicBool::new(false)),
        verify_durable_txid: None,
        release_error: None,
    };
    assert!(
        capture_once_with_image_limits(&source, &repository, &gate, &budget, 1, 1 << 20, 8192)
            .is_err()
    );
    assert_eq!(read_watermark(&repository).expect("base watermark").txid, 1);
    assert!(budget.try_reserve(Some(1)).is_err());
    budget.reconcile_with(|| Ok(Some(0))).expect("reconcile");
    let result =
        capture_once_with_image_limits(&source, &repository, &gate, &budget, 1, 1 << 20, 1 << 20)
            .expect("retry");
    assert_eq!(result.txid, 2);
    let restored = directory.path().join("restored.sqlite");
    restore_to_target(&repository, &restored, RestoreTarget::Txid(2)).expect("restore");
    assert_eq!(row_count(&restored), 2);
}

#[test]
fn limited_recovery_image_capture_fails_and_retries() {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return;
    }
    let directory = tempfile::tempdir().expect("temporary directory");
    let source = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    create_database(&source);
    snapshot(&source, &repository).expect("snapshot");
    let connection = Connection::open(&source).expect("source");
    connection
        .execute("INSERT INTO records(value) VALUES ('limited')", [])
        .expect("commit");
    let budget = ConservativeBudget::new(AdmissionGate::new(), 10).expect("budget");
    budget.reconcile_with(|| Ok(Some(0))).expect("baseline");
    let gate = TestGate {
        repository: repository.clone(),
        acquired: Arc::new(AtomicBool::new(false)),
        released: Arc::new(AtomicBool::new(false)),
        verify_durable_txid: None,
        release_error: None,
    };
    assert!(
        capture_once_with_recovery_image_limit(&source, &repository, &gate, &budget, 1, 1).is_err()
    );
    assert!(read_watermark(&repository).is_err());
    budget.reconcile_with(|| Ok(Some(0))).expect("reconcile");
    let result =
        capture_once_with_recovery_image_limit(&source, &repository, &gate, &budget, 1, 1 << 20)
            .expect("retry");
    assert_eq!(result.txid, 2);
    let restored = directory.path().join("restored.sqlite");
    restore_to_target(&repository, &restored, RestoreTarget::Txid(2)).expect("restore");
    assert_eq!(row_count(&restored), 2);
}

#[test]
fn reserved_budget_rejects_before_capture_and_retries() {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return;
    }
    let directory = tempfile::tempdir().expect("temporary directory");
    let source = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    create_database(&source);
    snapshot(&source, &repository).expect("snapshot");
    let connection = Connection::open(&source).expect("source");
    connection
        .execute("INSERT INTO records(value) VALUES ('reserved')", [])
        .expect("commit");
    let budget = ConservativeBudget::new(AdmissionGate::new(), 10).expect("budget");
    budget
        .reconcile_with(|| Ok(Some(5)))
        .expect("initial measurement");
    let gate = TestGate {
        repository: repository.clone(),
        acquired: Arc::new(AtomicBool::new(false)),
        released: Arc::new(AtomicBool::new(false)),
        verify_durable_txid: None,
        release_error: None,
    };
    let error = capture_once_with_reserved_budget(&source, &repository, &gate, &budget, 6)
        .expect_err("over budget");
    assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
    assert!(
        read_watermark(&repository).is_err(),
        "rejected capture must not publish watermark"
    );
    budget.reconcile_with(|| Ok(Some(4))).expect("reconcile");
    let result =
        capture_once_with_reserved_budget(&source, &repository, &gate, &budget, 6).expect("retry");
    assert_eq!(result.txid, 2);
    assert!(budget.try_reserve(Some(1)).is_err());
    let restored = directory.path().join("restored.sqlite");
    restore_to_target(&repository, &restored, RestoreTarget::Txid(2)).expect("restore");
    assert_eq!(row_count(&restored), 2);
}

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
