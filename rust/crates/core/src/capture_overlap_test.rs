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
    RestoreTarget, WriteGate, WritePermit,
};

struct ObservedGate(Arc<AtomicUsize>);

#[derive(Default)]
struct TestApplicationState {
    closed: bool,
    writers: usize,
}

#[derive(Clone, Default)]
struct DrainingApplicationGate(Arc<(Mutex<TestApplicationState>, Condvar)>);

impl DrainingApplicationGate {
    fn try_enter(&self) -> io::Result<()> {
        let mut state = self.0.0.lock().expect("application gate");
        if state.closed {
            return Err(io::Error::new(io::ErrorKind::WouldBlock, "gate closed"));
        }
        state.writers += 1;
        Ok(())
    }

    fn leave(&self) {
        let mut state = self.0.0.lock().expect("application gate");
        state.writers -= 1;
        self.0.1.notify_all();
    }
}

impl WriteGate for DrainingApplicationGate {
    type Permit = Self;

    fn acquire(&self) -> io::Result<Self::Permit> {
        let mut state = self.0.0.lock().expect("application gate");
        state.closed = true;
        self.0.1.notify_all();
        let (state, timeout) = self
            .0
            .1
            .wait_timeout_while(state, Duration::from_secs(5), |state| state.writers != 0)
            .expect("drain");
        if timeout.timed_out() && state.writers != 0 {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "writers did not drain",
            ));
        }
        Ok(self.clone())
    }
}

impl WritePermit for DrainingApplicationGate {
    fn release(self) -> io::Result<()> {
        let mut state = self.0.0.lock().expect("application gate");
        state.closed = false;
        self.0.1.notify_all();
        Ok(())
    }
}

#[test]
fn budgeted_public_capture_drains_application_writer_before_capture() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    let connection = Connection::open(&database).expect("source");
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE data(value TEXT); INSERT INTO data VALUES ('base');").expect("initialize");
    crate::snapshot(&database, &repository).expect("snapshot");
    let admission = crate::AdmissionGate::new();
    admission.open().expect("open admission");
    let budget = ConservativeBudget::new(admission, 100).expect("budget");
    budget
        .reconcile_with(|| Ok(Some(0)))
        .expect("logical baseline");
    let gate = DrainingApplicationGate::default();
    gate.try_enter().expect("writer admitted");
    let reservation = budget.try_reserve(Some(1)).expect("reserve");
    let (result_tx, result_rx) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        let gate_for_capture = gate.clone();
        let budget_for_capture = budget.clone();
        let database = &database;
        let repository = &repository;
        scope.spawn(move || {
            let result = crate::capture_once_with_budget(
                database,
                repository,
                &gate_for_capture,
                &budget_for_capture,
            );
            result_tx.send(result).expect("capture result");
        });
        let state = gate.0.0.lock().expect("application gate");
        let (state, timeout) = gate
            .0
            .1
            .wait_timeout_while(state, Duration::from_secs(5), |state| !state.closed)
            .expect("wait for close");
        assert!(!timeout.timed_out(), "capture closed application gate");
        drop(state);
        assert_eq!(
            gate.try_enter().expect_err("new writer rejected").kind(),
            io::ErrorKind::WouldBlock
        );
        assert!(result_rx.try_recv().is_err());
        connection
            .execute("INSERT INTO data VALUES ('committed')", [])
            .expect("commit");
        reservation.settle(Some(0)).expect("logical settlement");
        gate.leave();
        let result = result_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("capture completed")
            .expect("capture");
        assert_eq!(result.txid, 2);
    });
    gate.try_enter().expect("application gate reopened");
    assert!(
        budget.try_reserve(Some(1)).is_err(),
        "capture invalidates occupancy"
    );
    gate.leave();
    budget
        .reconcile_with(|| Ok(Some(1)))
        .expect("logical reconciliation");
    gate.try_enter().expect("second writer admitted");
    let second = budget
        .try_reserve(Some(1))
        .expect("reserve after reconciliation");
    connection
        .execute("INSERT INTO data VALUES ('second')", [])
        .expect("second commit");
    second.settle(Some(0)).expect("logical settlement");
    gate.leave();
    let next = crate::capture_once_with_budget(&database, &repository, &gate, &budget)
        .expect("capture second writer");
    assert_eq!(next.txid, 3);
    let restored = directory.path().join("restored.sqlite");
    crate::restore_to_target(&repository, &restored, RestoreTarget::Txid(3)).expect("restore");
    let count: i64 = Connection::open(restored)
        .expect("restored")
        .query_row("SELECT COUNT(*) FROM data", [], |row| row.get(0))
        .expect("count");
    assert_eq!(count, 3);
}

impl WriteGate for ObservedGate {
    type Permit = Self;

    fn acquire(&self) -> io::Result<Self::Permit> {
        Ok(Self(self.0.clone()))
    }
}

impl WritePermit for ObservedGate {
    fn release(self) -> io::Result<()> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

struct FailingReleaseGate;

struct FailingAcquireGate;

impl WriteGate for FailingAcquireGate {
    type Permit = Self;

    fn acquire(&self) -> io::Result<Self::Permit> {
        Err(io::Error::other("injected acquire failure"))
    }
}

impl WritePermit for FailingAcquireGate {
    fn release(self) -> io::Result<()> {
        panic!("no permit was acquired");
    }
}

#[test]
fn failed_gate_acquire_invalidates_budget_even_with_active_writer() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let admission = crate::AdmissionGate::new();
    admission.open().expect("open admission");
    let budget = ConservativeBudget::new(admission, 100).expect("budget");
    budget.reconcile_with(|| Ok(Some(0))).expect("baseline");
    let reservation = budget.try_reserve(Some(1)).expect("active writer");
    let error = crate::capture_once_with_budget(
        directory.path().join("source.sqlite"),
        directory.path().join("repository"),
        &FailingAcquireGate,
        &budget,
    )
    .expect_err("gate acquisition fails");
    assert!(error.to_string().contains("injected acquire failure"));
    assert!(
        budget.try_reserve(Some(1)).is_err(),
        "acquire failure must invalidate occupancy"
    );
    assert!(
        reservation.settle(Some(0)).is_err(),
        "settlement must not restore unknown occupancy"
    );
}

struct ReconcileThenFailGate(ConservativeBudget);

struct ReconcileOnReleaseGate(ConservativeBudget);

impl WriteGate for ReconcileOnReleaseGate {
    type Permit = Self;

    fn acquire(&self) -> io::Result<Self::Permit> {
        Ok(Self(self.0.clone()))
    }
}

impl WritePermit for ReconcileOnReleaseGate {
    fn release(self) -> io::Result<()> {
        self.0.reconcile_with(|| Ok(Some(0)))
    }
}

#[test]
fn failed_capture_invalidates_release_callback_reconciliation() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let admission = crate::AdmissionGate::new();
    admission.open().expect("open admission");
    let budget = ConservativeBudget::new(admission, 100).expect("budget");
    budget.reconcile_with(|| Ok(Some(0))).expect("baseline");
    let gate = ReconcileOnReleaseGate(budget.clone());
    crate::capture_once_with_budget(
        directory.path().join("missing.sqlite"),
        directory.path().join("missing-repository"),
        &gate,
        &budget,
    )
    .expect_err("capture fails");
    assert!(
        budget.try_reserve(Some(1)).is_err(),
        "capture failure must invalidate callback reconciliation"
    );
}

impl WriteGate for ReconcileThenFailGate {
    type Permit = Self;

    fn acquire(&self) -> io::Result<Self::Permit> {
        Ok(Self(self.0.clone()))
    }
}

impl WritePermit for ReconcileThenFailGate {
    fn release(self) -> io::Result<()> {
        self.0.reconcile_with(|| Ok(Some(0)))?;
        Err(io::Error::other("injected release failure"))
    }
}

#[test]
fn failed_gate_release_invalidates_callback_reconciliation() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    let connection = Connection::open(&database).expect("source");
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE data(value TEXT); INSERT INTO data VALUES ('base');").expect("initialize");
    crate::snapshot(&database, &repository).expect("snapshot");
    let admission = crate::AdmissionGate::new();
    admission.open().expect("open admission");
    let budget = ConservativeBudget::new(admission, 100).expect("budget");
    budget.reconcile_with(|| Ok(Some(0))).expect("baseline");
    let reservation = budget.try_reserve(Some(1)).expect("reserve");
    connection
        .execute("INSERT INTO data VALUES ('committed')", [])
        .expect("commit");
    reservation.settle(Some(0)).expect("settle logical cost");
    let error = crate::capture_once_with_budget(
        &database,
        &repository,
        &ReconcileThenFailGate(budget.clone()),
        &budget,
    )
    .expect_err("release must fail");
    assert!(error.to_string().contains("injected release failure"));
    assert!(
        budget.try_reserve(Some(1)).is_err(),
        "release failure must invalidate reconciliation"
    );
    let restored = directory.path().join("restored.sqlite");
    crate::restore_to_target(&repository, &restored, RestoreTarget::Txid(2))
        .expect("durable capture");
}

impl WriteGate for FailingReleaseGate {
    type Permit = Self;

    fn acquire(&self) -> io::Result<Self::Permit> {
        Ok(Self)
    }
}

impl WritePermit for FailingReleaseGate {
    fn release(self) -> io::Result<()> {
        Err(io::Error::other("injected gate release failure"))
    }
}

#[test]
fn public_budgeted_capture_release_error_preserves_durable_data_and_unknown_occupancy() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    let connection = Connection::open(&database).expect("source");
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE data(value TEXT); INSERT INTO data VALUES ('base');").expect("initialize");
    crate::snapshot(&database, &repository).expect("snapshot");
    let admission = crate::AdmissionGate::new();
    admission.open().expect("open admission");
    let budget = ConservativeBudget::new(admission, 100).expect("budget");
    budget
        .reconcile_with(|| Ok(Some(0)))
        .expect("logical baseline");
    let producer = budget.try_reserve(Some(1)).expect("reserve");
    connection
        .execute("INSERT INTO data VALUES ('committed')", [])
        .expect("commit");
    producer.settle(Some(0)).expect("logical settlement");
    let error =
        crate::capture_once_with_budget(&database, &repository, &FailingReleaseGate, &budget)
            .expect_err("release failure is reported");
    assert!(error.to_string().contains("capture completed durably"));
    assert!(error.to_string().contains("injected gate release failure"));
    assert!(
        budget.try_reserve(Some(1)).is_err(),
        "capture invalidates occupancy"
    );
    let restored = directory.path().join("restored.sqlite");
    crate::restore_to_target(&repository, &restored, RestoreTarget::Txid(2))
        .expect("restore published capture");
    let value: String = Connection::open(restored)
        .expect("restored")
        .query_row(
            "SELECT value FROM data ORDER BY rowid DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("value");
    assert_eq!(value, "committed");
    budget
        .reconcile_with(|| Ok(Some(1)))
        .expect("test-only logical reconciliation");
    budget
        .try_reserve(Some(1))
        .expect("budget reconciled")
        .settle(Some(0))
        .expect("settle");
}

#[test]
fn public_capture_rejects_active_producer_then_recovers() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    let connection = Connection::open(&database).expect("source");
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE data(value TEXT); INSERT INTO data VALUES ('base');").expect("initialize");
    crate::snapshot(&database, &repository).expect("snapshot");
    let admission = crate::AdmissionGate::new();
    admission.open().expect("open admission");
    let budget = ConservativeBudget::new(admission, 100).expect("budget");
    budget
        .reconcile_with(|| Ok(Some(0)))
        .expect("logical baseline");
    let releases = Arc::new(AtomicUsize::new(0));
    let gate = ObservedGate(releases.clone());
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (settle_tx, settle_rx) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        let producer_budget = budget.clone();
        let producer_database = &database;
        scope.spawn(move || {
            let reservation = producer_budget.try_reserve(Some(1)).expect("reserve");
            let writer = Connection::open(producer_database).expect("writer");
            writer
                .execute("INSERT INTO data VALUES ('committed')", [])
                .expect("commit");
            ready_tx.send(()).expect("producer ready");
            settle_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("settle requested");
            reservation
                .settle(Some(0))
                .expect_err("failed capture invalidated occupancy");
        });
        ready_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("producer committed");
        let error = crate::capture_once_with_budget(&database, &repository, &gate, &budget)
            .expect_err("active reservation rejects mutation");
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        assert_eq!(releases.load(Ordering::SeqCst), 1);
        assert!(
            crate::restore_to_target(
                &repository,
                directory.path().join("premature.sqlite"),
                RestoreTarget::Txid(2),
            )
            .is_err()
        );
        settle_tx.send(()).expect("allow settlement");
    });
    let result = crate::capture_once_with_budget(&database, &repository, &gate, &budget)
        .expect("retry capture");
    assert_eq!(result.txid, 2);
    assert_eq!(releases.load(Ordering::SeqCst), 2);
    assert!(
        budget.try_reserve(Some(1)).is_err(),
        "capture invalidates occupancy"
    );
    let restored = directory.path().join("restored.sqlite");
    crate::restore_to_target(&repository, &restored, RestoreTarget::Txid(2)).expect("restore");
    let value: String = Connection::open(restored)
        .expect("restored")
        .query_row(
            "SELECT value FROM data ORDER BY rowid DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("value");
    assert_eq!(value, "committed");
}

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
async fn fresh_budget_requires_measurement_after_coordinator_recreation() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    let connection = Connection::open(&database).expect("open source");
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE data(value TEXT); INSERT INTO data VALUES ('base');").expect("create source");
    crate::snapshot(&database, &repository).expect("snapshot");
    let make_adapter = || {
        Arc::new(ReconcilingAdapter {
            inner: BudgetCaptureAdapter {
                database: database.clone(),
                repository: repository.clone(),
                target: AtomicUsize::new(1),
                fail: AtomicBool::new(false),
                panic_capture: AtomicBool::new(false),
                admission: Mutex::new(Vec::new()),
                budget: Mutex::new(None),
            },
            measurement_fails: AtomicBool::new(false),
        })
    };
    let make_coordinator = |adapter: Arc<ReconcilingAdapter>| {
        Coordinator::new(CoordinatorConfig {
            repository_root: repository.clone(),
            database_path: database.clone(),
            adapter,
            limits: Limits {
                max_spool_bytes: 1 << 30,
                max_wal_bytes: 1 << 30,
                max_unprotected_age: Duration::from_secs(3600),
                max_uploads_in_flight: 1,
            },
            remote: None,
        })
        .expect("coordinator")
    };
    let first_adapter = make_adapter();
    let first = make_coordinator(first_adapter.clone());
    let first_budget = ConservativeBudget::new(first.admission_gate(), 1 << 30).expect("budget");
    *first_adapter.inner.budget.lock().expect("budget reference") = Some(first_budget.clone());
    first.flush().await.expect("initial reconcile");
    first_budget
        .try_reserve(Some(1))
        .expect("first reservation")
        .settle(Some(0))
        .expect("settle");
    drop(first);
    drop(first_adapter);
    drop(first_budget);

    let second_adapter = make_adapter();
    let second = make_coordinator(second_adapter.clone());
    let second_budget =
        ConservativeBudget::new(second.admission_gate(), 1 << 30).expect("fresh budget");
    *second_adapter
        .inner
        .budget
        .lock()
        .expect("budget reference") = Some(second_budget.clone());
    assert!(second_budget.try_reserve(Some(1)).is_err());
    second_adapter
        .measurement_fails
        .store(true, Ordering::SeqCst);
    second
        .flush()
        .await
        .expect_err("failed reconciliation keeps fresh coordinator closed");
    assert!(second.admission_gate().try_accept().is_err());
    assert!(second_budget.try_reserve(Some(1)).is_err());
    second_adapter
        .measurement_fails
        .store(false, Ordering::SeqCst);
    second.flush().await.expect("reconcile fresh coordinator");
    let reservation = second_budget
        .try_reserve(Some(1))
        .expect("reserve after reconciliation");
    connection
        .execute("UPDATE data SET value='after recreation'", [])
        .expect("commit");
    reservation.settle(Some(0)).expect("logical settlement");
    second_adapter.inner.target.store(2, Ordering::SeqCst);
    second.flush().await.expect("capture after recreation");
    let restored = directory.path().join("restored.sqlite");
    crate::restore_to_target(&repository, &restored, RestoreTarget::Txid(2)).expect("restore");
    let value: String = Connection::open(restored)
        .expect("open restored")
        .query_row("SELECT value FROM data", [], |row| row.get(0))
        .expect("value");
    assert_eq!(value, "after recreation");
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
