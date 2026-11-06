use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use rusqlite::{Connection, ErrorCode};
use tokio::sync::oneshot;

use super::{FaultPoint, capture_once_with_hook};
use crate::{
    CaptureResult, ConservativeBudget, Coordinator, CoordinatorAdapter, CoordinatorConfig, Limits,
    RestoreTarget,
};

struct BudgetCaptureAdapter {
    database: PathBuf,
    repository: PathBuf,
    target: AtomicUsize,
    fail: AtomicBool,
    panic_capture: AtomicBool,
    admission: Mutex<Vec<bool>>,
    budget: Mutex<Option<ConservativeBudget>>,
}

impl CoordinatorAdapter for BudgetCaptureAdapter {
    fn observe_txid(&self) -> io::Result<Option<u64>> {
        Ok(Some(self.target.load(Ordering::SeqCst) as u64))
    }

    fn capture(&self) -> io::Result<CaptureResult> {
        let budget = self
            .budget
            .lock()
            .map_err(|_| io::Error::other("budget reference poisoned"))?
            .clone()
            .ok_or_else(|| io::Error::other("budget not configured"))?;
        let result = budget.with_occupancy_mutation(|| {
            capture_once_with_hook(&self.database, &self.repository, 100, |point, _| {
                if point == FaultPoint::AfterSourceSnapshot
                    && self.panic_capture.swap(false, Ordering::SeqCst)
                {
                    panic!("injected capture panic");
                }
                if point == FaultPoint::AfterSourceSnapshot
                    && self.fail.swap(false, Ordering::SeqCst)
                {
                    return Err(io::Error::other("injected capture failure"));
                }
                Ok(())
            })
        })?;
        Ok(CaptureResult {
            txid: result.state.database.txid,
            transactions: result.transactions.len(),
            wal_frames: result.wal_frames,
        })
    }

    fn set_write_admission(&self, enabled: bool, _limits: Limits) -> io::Result<()> {
        self.admission.lock().expect("admission lock").push(enabled);
        Ok(())
    }
}

// Test-only policy: a WAL length is an observation, not a conservative bound.
struct ReconcilingAdapter {
    inner: BudgetCaptureAdapter,
    measurement_fails: AtomicBool,
}

impl CoordinatorAdapter for ReconcilingAdapter {
    fn observe_txid(&self) -> io::Result<Option<u64>> {
        self.inner.observe_txid()
    }

    fn capture(&self) -> io::Result<CaptureResult> {
        self.inner.capture()
    }

    fn set_write_admission(&self, enabled: bool, limits: Limits) -> io::Result<()> {
        if enabled {
            let budget = self
                .inner
                .budget
                .lock()
                .map_err(|_| io::Error::other("budget reference poisoned"))?
                .clone()
                .ok_or_else(|| io::Error::other("budget not configured"))?;
            budget.reconcile_with(|| {
                if self.measurement_fails.load(Ordering::SeqCst) {
                    return Err(io::Error::other("injected WAL measurement failure"));
                }
                let mut wal = self.inner.database.as_os_str().to_os_string();
                wal.push("-wal");
                Ok(Some(std::fs::metadata(PathBuf::from(wal))?.len()))
            })?;
        }
        self.inner.set_write_admission(enabled, limits)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_reconciliation_keeps_admission_closed_after_real_capture() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    let connection = Connection::open(&database).expect("open source");
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE data(value TEXT); INSERT INTO data VALUES ('base');").expect("create source");
    crate::snapshot(&database, &repository).expect("snapshot");
    let adapter = Arc::new(ReconcilingAdapter {
        inner: BudgetCaptureAdapter {
            database: database.clone(),
            repository: repository.clone(),
            target: AtomicUsize::new(1),
            fail: AtomicBool::new(false),
            panic_capture: AtomicBool::new(false),
            admission: Mutex::new(Vec::new()),
            budget: Mutex::new(None),
        },
        measurement_fails: AtomicBool::new(true),
    });
    let coordinator = Coordinator::new(CoordinatorConfig {
        repository_root: repository.clone(),
        database_path: database,
        adapter: adapter.clone(),
        limits: Limits {
            max_spool_bytes: 1 << 30,
            max_wal_bytes: 1 << 30,
            max_unprotected_age: Duration::from_secs(3600),
            max_uploads_in_flight: 1,
        },
        remote: None,
    })
    .expect("coordinator");
    let budget = ConservativeBudget::new(coordinator.admission_gate(), 1 << 30).expect("budget");
    *adapter.inner.budget.lock().expect("budget reference") = Some(budget.clone());
    let initial_error = coordinator
        .flush()
        .await
        .expect_err("initial measurement failure prevents admission");
    assert!(
        initial_error
            .to_string()
            .contains("injected WAL measurement failure")
    );
    assert!(coordinator.admission_gate().try_accept().is_err());
    assert!(budget.try_reserve(Some(1)).is_err());
    adapter.measurement_fails.store(false, Ordering::SeqCst);
    coordinator
        .flush()
        .await
        .expect("initial reconciliation and admission");
    let permit = budget.try_reserve(Some(1)).expect("logical reservation");
    connection
        .execute("UPDATE data SET value='captured'", [])
        .expect("commit");
    permit.settle(Some(0)).expect("logical settlement");
    adapter.inner.target.store(2, Ordering::SeqCst);
    adapter.measurement_fails.store(true, Ordering::SeqCst);
    let error = coordinator
        .flush()
        .await
        .expect_err("measurement failure prevents reopening");
    assert!(
        error
            .to_string()
            .contains("injected WAL measurement failure")
    );
    assert!(coordinator.admission_gate().try_accept().is_err());
    assert!(budget.try_reserve(Some(1)).is_err());
    let restored = directory.path().join("restored.sqlite");
    crate::restore_to_target(&repository, &restored, RestoreTarget::Txid(2))
        .expect("capture durable despite measurement error");
    let value: String = Connection::open(restored)
        .expect("reader")
        .query_row("SELECT value FROM data", [], |row| row.get(0))
        .expect("value");
    assert_eq!(value, "captured");
    adapter.measurement_fails.store(false, Ordering::SeqCst);
    coordinator
        .flush()
        .await
        .expect("retry measurement and reopen");
    budget
        .try_reserve(Some(1))
        .expect("reconciled admission")
        .settle(Some(0))
        .expect("settle");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cooperative_budget_flush_real_capture_failure_and_retry() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    let connection = Connection::open(&database).expect("open source");
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE data(value TEXT); INSERT INTO data VALUES ('base');").expect("create source");
    crate::snapshot(&database, &repository).expect("snapshot");
    let adapter = Arc::new(BudgetCaptureAdapter {
        database: database.clone(),
        repository: repository.clone(),
        target: AtomicUsize::new(1),
        fail: AtomicBool::new(true),
        panic_capture: AtomicBool::new(false),
        admission: Mutex::new(Vec::new()),
        budget: Mutex::new(None),
    });
    let coordinator = Arc::new(
        Coordinator::new(CoordinatorConfig {
            repository_root: repository.clone(),
            database_path: database,
            adapter: adapter.clone(),
            limits: Limits {
                max_spool_bytes: 1 << 30,
                max_wal_bytes: 1 << 30,
                max_unprotected_age: Duration::from_secs(3600),
                max_uploads_in_flight: 1,
            },
            remote: None,
        })
        .expect("coordinator"),
    );
    let budget = ConservativeBudget::new(coordinator.admission_gate(), 1 << 30).expect("budget");
    *adapter.budget.lock().expect("budget reference") = Some(budget.clone());
    coordinator.flush().await.expect("open admission");
    budget
        .reconcile_with(|| Ok(Some(0)))
        .expect("logical baseline");
    let permit = budget.try_reserve(Some(2)).expect("reserve producer");
    connection
        .execute("UPDATE data SET value='captured'", [])
        .expect("commit");
    adapter.target.store(2, Ordering::SeqCst);
    let running = coordinator.clone();
    let mut flush = tokio::spawn(async move { running.flush().await });
    tokio::time::timeout(Duration::from_secs(5), async {
        while adapter.admission.lock().expect("admission lock").len() < 3 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("admission closed");
    assert!(budget.try_reserve(Some(1)).is_err());
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut flush)
            .await
            .is_err()
    );
    permit.settle(Some(0)).expect("settle logical reservation");
    let error = flush.await.expect("task").expect_err("injected failure");
    assert!(error.to_string().contains("injected capture failure"));
    assert!(budget.try_reserve(Some(1)).is_err());
    assert!(
        budget
            .reconcile_with(|| Err(io::Error::other("injected measurement failure")))
            .is_err()
    );
    assert!(
        budget.try_reserve(Some(1)).is_err(),
        "failed measurement cannot restore occupancy"
    );
    budget
        .reconcile_with(|| Ok(Some(0)))
        .expect("test-only baseline before retry");
    coordinator.flush().await.expect("retry captures real WAL");
    assert!(
        budget.try_reserve(Some(1)).is_err(),
        "capture does not reconcile budget"
    );
    let mut observed_wal = 0;
    budget
        .reconcile_with(|| {
            observed_wal = coordinator.status()?.wal_bytes;
            Ok(Some(observed_wal))
        })
        .expect("reconcile observed WAL length");
    assert!(observed_wal > 0);
    budget
        .try_reserve(Some(1))
        .expect("reconciled")
        .settle(Some(0))
        .expect("settle");
    let restored = directory.path().join("restored.sqlite");
    crate::restore_to_target(&repository, &restored, RestoreTarget::Txid(2)).expect("restore");
    let value: String = Connection::open(restored)
        .expect("reader")
        .query_row("SELECT value FROM data", [], |row| row.get(0))
        .expect("value");
    assert_eq!(value, "captured");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn coordinator_capture_panic_closes_gate_and_poisons_budget() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    let connection = Connection::open(&database).expect("open source");
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE data(value TEXT); INSERT INTO data VALUES ('base');").expect("create source");
    crate::snapshot(&database, &repository).expect("snapshot");
    let adapter = Arc::new(BudgetCaptureAdapter {
        database: database.clone(),
        repository: repository.clone(),
        target: AtomicUsize::new(1),
        fail: AtomicBool::new(false),
        panic_capture: AtomicBool::new(true),
        admission: Mutex::new(Vec::new()),
        budget: Mutex::new(None),
    });
    let coordinator = Coordinator::new(CoordinatorConfig {
        repository_root: repository,
        database_path: database,
        adapter: adapter.clone(),
        limits: Limits {
            max_spool_bytes: 1 << 30,
            max_wal_bytes: 1 << 30,
            max_unprotected_age: Duration::from_secs(3600),
            max_uploads_in_flight: 1,
        },
        remote: None,
    })
    .expect("coordinator");
    let budget = ConservativeBudget::new(coordinator.admission_gate(), 100).expect("budget");
    *adapter.budget.lock().expect("budget reference") = Some(budget.clone());
    coordinator.flush().await.expect("open admission");
    budget
        .reconcile_with(|| Ok(Some(0)))
        .expect("test baseline");
    let reservation = budget.try_reserve(Some(1)).expect("reserve");
    connection
        .execute("UPDATE data SET value='committed'", [])
        .expect("commit");
    reservation.settle(Some(0)).expect("logical settlement");
    adapter.target.store(2, Ordering::SeqCst);
    let error = coordinator.flush().await.expect_err("capture task panics");
    assert!(error.to_string().contains("panicked"));
    assert!(coordinator.admission_gate().try_accept().is_err());
    assert!(budget.try_reserve(Some(1)).is_err());
    assert!(budget.reconcile_with(|| Ok(Some(0))).is_err());
    let value: String = connection
        .query_row("SELECT value FROM data", [], |row| row.get(0))
        .expect("read committed row");
    assert_eq!(value, "committed");
}

struct LeaseAdapter {
    database: PathBuf,
    repository: PathBuf,
    attempts: AtomicUsize,
    started: Mutex<Option<oneshot::Sender<()>>>,
    finished: Mutex<Option<oneshot::Sender<Result<u64, String>>>>,
    release: (Mutex<bool>, Condvar),
    admission: Mutex<Vec<bool>>,
}

impl CoordinatorAdapter for LeaseAdapter {
    fn observe_txid(&self) -> io::Result<Option<u64>> {
        Ok(Some(2))
    }

    fn capture(&self) -> io::Result<CaptureResult> {
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
        let result = capture_once_with_hook(&self.database, &self.repository, 100, |point, _| {
            if attempt == 0 && point == FaultPoint::AfterLeaseAcquire {
                self.started
                    .lock()
                    .expect("start lock")
                    .take()
                    .expect("start sender")
                    .send(())
                    .expect("report held SQLite lease");
                let (lock, wake) = &self.release;
                let released = lock.lock().expect("release lock");
                let (released, _) = wake
                    .wait_timeout_while(released, Duration::from_secs(10), |released| !*released)
                    .expect("wait for release");
                if !*released {
                    return Err(io::Error::other("test timed out waiting for release"));
                }
            }
            Ok(())
        });
        if attempt == 0 {
            let completion = result
                .as_ref()
                .map(|result| result.state.database.txid)
                .map_err(ToString::to_string);
            self.finished
                .lock()
                .expect("finish lock")
                .take()
                .expect("finish sender")
                .send(completion)
                .expect("report first capture completion");
        }
        result.map(|result| CaptureResult {
            txid: result.state.database.txid,
            transactions: result.transactions.len(),
            wal_frames: result.wal_frames,
        })
    }

    fn set_write_admission(&self, enabled: bool, _limits: Limits) -> io::Result<()> {
        self.admission.lock().expect("admission lock").push(enabled);
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropped_flush_sqlite_lease_rejects_second_capture_until_first_finishes() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    let connection = Connection::open(&database).expect("open source");
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE data(value TEXT); INSERT INTO data VALUES ('base');").expect("create source");
    crate::snapshot(&database, &repository).expect("initial snapshot");
    connection
        .execute("UPDATE data SET value='captured'", [])
        .expect("commit source change");
    let (started_tx, started_rx) = oneshot::channel();
    let (finished_tx, finished_rx) = oneshot::channel();
    let adapter = Arc::new(LeaseAdapter {
        database: database.clone(),
        repository: repository.clone(),
        attempts: AtomicUsize::new(0),
        started: Mutex::new(Some(started_tx)),
        finished: Mutex::new(Some(finished_tx)),
        release: (Mutex::new(false), Condvar::new()),
        admission: Mutex::new(Vec::new()),
    });
    let coordinator = Arc::new(
        Coordinator::new(CoordinatorConfig {
            repository_root: repository.clone(),
            database_path: database,
            adapter: adapter.clone(),
            limits: Limits {
                max_spool_bytes: 1 << 30,
                max_wal_bytes: 1 << 30,
                max_unprotected_age: Duration::from_secs(3600),
                max_uploads_in_flight: 1,
            },
            remote: None,
        })
        .expect("coordinator"),
    );
    let first_coordinator = coordinator.clone();
    let first = tokio::spawn(async move { first_coordinator.flush().await });
    tokio::time::timeout(Duration::from_secs(5), started_rx)
        .await
        .expect("first lease acquired")
        .expect("start signal");
    first.abort();
    assert!(
        first
            .await
            .expect_err("first future aborted")
            .is_cancelled()
    );
    let second = tokio::time::timeout(Duration::from_secs(5), coordinator.flush()).await;
    let (lock, wake) = &adapter.release;
    *lock.lock().expect("release lock") = true;
    wake.notify_all();
    let error = second
        .expect("second flush returns before release")
        .expect_err("SQLite lease must reject overlap");
    let sqlite = error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<rusqlite::Error>())
        .expect("SQLite error preserved");
    assert!(
        matches!(sqlite, rusqlite::Error::SqliteFailure(code, _) if code.code == ErrorCode::DatabaseBusy)
    );
    assert_eq!(adapter.attempts.load(Ordering::SeqCst), 2);
    assert_eq!(
        *adapter.admission.lock().expect("admission lock"),
        [false, false]
    );
    let txid = tokio::time::timeout(Duration::from_secs(5), finished_rx)
        .await
        .expect("first capture completes")
        .expect("finish signal")
        .expect("first capture succeeds");
    assert_eq!(txid, 2);
    let restored = directory.path().join("restored.sqlite");
    crate::restore_to_target(&repository, &restored, RestoreTarget::Txid(2))
        .expect("restore first capture");
    let value: String = Connection::open(restored)
        .expect("open restored database")
        .query_row("SELECT value FROM data", [], |row| row.get(0))
        .expect("read restored data");
    assert_eq!(value, "captured");
}
