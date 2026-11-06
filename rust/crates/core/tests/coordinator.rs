use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use rusqlite::Connection;
use sqlite_replication_core::{
    AdmissionGate, CapacityGate, CaptureResult, ComponentState, ConservativeBudget, Coordinator,
    CoordinatorAdapter, CoordinatorConfig, CoordinatorMetrics, ErrorCategory, FlushOutcome, Limits,
    RestoreTarget, WriteGate, WritePermit,
};

struct BlockingCaptureAdapter {
    database_path: PathBuf,
    repository_root: PathBuf,
    admission: Mutex<Vec<bool>>,
    started: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    release: Arc<(Mutex<bool>, Condvar)>,
}

#[tokio::test]
async fn conservative_budget_reconciles_observed_wal_after_unknown_growth() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let connection = Connection::open(&database_path).expect("open database");
    connection
        .execute_batch("PRAGMA wal_autocheckpoint=0; PRAGMA wal_checkpoint(TRUNCATE)")
        .expect("reset WAL");
    let coordinator = coordinator(&repository_root, &database_path, Some(1), None);
    coordinator.flush().await.expect("open admission");
    let limit = 65_536;
    let budget = ConservativeBudget::new(coordinator.admission_gate(), limit).expect("budget");
    let mut before = 0;
    budget
        .reconcile_with(|| {
            before = coordinator.status()?.wal_bytes;
            Ok(Some(before))
        })
        .expect("baseline");
    let reservation = budget.try_reserve(Some(16_384)).expect("test reservation");
    connection
        .execute("INSERT INTO records(value) VALUES ('committed')", [])
        .expect("commit");
    assert!(reservation.settle(None).is_err());
    assert!(budget.try_reserve(Some(1)).is_err());
    let mut measured = 0;
    budget
        .reconcile_with(|| {
            measured = coordinator.status()?.wal_bytes;
            Ok(Some(measured))
        })
        .expect("exclusive test measurement");
    assert!(measured > before && measured < limit);
    assert!(budget.try_reserve(Some(limit - measured + 1)).is_err());
    budget
        .try_reserve(Some(1))
        .expect("remaining logical capacity")
        .settle(Some(0))
        .expect("settlement");
    println!("wal_before={before} wal_after={measured} logical_limit={limit}");
}

#[tokio::test]
async fn conservative_budget_sqlite_unknown_settlement_requires_reconciliation() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let coordinator = coordinator(&repository_root, &database_path, Some(1), None);
    coordinator.flush().await.expect("open admission");
    let budget = ConservativeBudget::new(coordinator.admission_gate(), 3).expect("budget");
    assert!(budget.try_reserve(Some(1)).is_err());
    budget
        .reconcile_with(|| Ok(Some(0)))
        .expect("test baseline");
    let reservation = budget.try_reserve(Some(2)).expect("reserve transaction");
    let connection = Connection::open(&database_path).expect("open database");
    connection
        .execute("INSERT INTO records(value) VALUES ('committed')", [])
        .expect("commit write");
    assert!(
        budget.reconcile_with(|| Ok(Some(0))).is_err(),
        "cannot reconcile with an active reservation"
    );
    assert!(
        reservation.settle(None).is_err(),
        "unknown committed growth fails closed"
    );
    assert!(budget.try_reserve(Some(1)).is_err());
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))
        .expect("count rows");
    assert_eq!(count, 1, "failed settlement does not undo SQLite commit");
    budget
        .reconcile_with(|| Ok(Some(2)))
        .expect("test supplies a conservative logical occupancy");
    assert!(budget.try_reserve(Some(2)).is_err());
    budget
        .try_reserve(Some(1))
        .expect("remaining capacity")
        .settle(Some(0))
        .expect("settle");
}

struct BlockingCaptureGate<'a>(&'a BlockingCaptureAdapter);
struct CapturePermit;

struct CancelOnOpenAdapter {
    cancellation: tokio::sync::watch::Sender<bool>,
    admission: Mutex<Vec<bool>>,
}

struct AdmissionObservation {
    gate: Mutex<Option<AdmissionGate>>,
    accepted: Mutex<Option<bool>>,
}

impl CoordinatorMetrics for AdmissionObservation {
    fn observe_flush(&self, _duration: Duration, _outcome: FlushOutcome) {
        let gate = self.gate.lock().expect("gate lock");
        let accepted = gate.as_ref().expect("shared gate").try_accept().is_ok();
        *self.accepted.lock().expect("observation lock") = Some(accepted);
    }
}

struct LegacyCancellationObservation {
    gate: Mutex<Option<AdmissionGate>>,
    cancellation: tokio::sync::watch::Sender<bool>,
    accepted: Mutex<Option<bool>>,
}

impl CoordinatorMetrics for LegacyCancellationObservation {
    fn observe_flush(&self, _duration: Duration, _outcome: FlushOutcome) {
        self.cancellation.send(true).expect("cancel receiver");
        let gate = self.gate.lock().expect("gate lock");
        let accepted = gate.as_ref().expect("shared gate").try_accept().is_ok();
        *self.accepted.lock().expect("observation lock") = Some(accepted);
    }
}

#[tokio::test]
async fn legacy_receiver_cancellation_does_not_atomically_close_gate() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let (sender, mut cancellation) = tokio::sync::watch::channel(false);
    let observation = Arc::new(LegacyCancellationObservation {
        gate: Mutex::new(None),
        cancellation: sender,
        accepted: Mutex::new(None),
    });
    let coordinator = Coordinator::new(CoordinatorConfig {
        repository_root,
        database_path,
        adapter: coordinator_adapter(Some(1)),
        limits: limits(1 << 30),
        remote: None,
    })
    .expect("coordinator")
    .with_metrics(observation.clone());
    let gate = coordinator.admission_gate();
    *observation.gate.lock().expect("gate lock") = Some(gate.clone());
    let (_sender, mut events) = tokio::sync::mpsc::channel(1);
    tokio::time::timeout(
        Duration::from_secs(5),
        coordinator.run_legacy(&mut events, &mut cancellation),
    )
    .await
    .expect("worker exits")
    .expect("clean cancellation");
    assert_eq!(
        *observation.accepted.lock().expect("observation lock"),
        Some(true),
        "characterize legacy receiver race"
    );
    assert!(gate.try_accept().is_err(), "worker eventually cancels gate");
}

struct StrictCancelOnOpenAdapter {
    gate: Mutex<Option<AdmissionGate>>,
    accepted_after_cancel: Mutex<bool>,
}

impl CoordinatorAdapter for StrictCancelOnOpenAdapter {
    fn observe_txid(&self) -> io::Result<Option<u64>> {
        Ok(Some(1))
    }

    fn capture(&self) -> io::Result<CaptureResult> {
        Err(io::Error::other("capture should not be needed"))
    }

    fn set_write_admission(&self, enabled: bool, _limits: Limits) -> io::Result<()> {
        if enabled {
            let gate = self.gate.lock().expect("gate lock");
            let gate = gate.as_ref().expect("shared gate");
            gate.cancel()?;
            *self.accepted_after_cancel.lock().expect("acceptance lock") =
                gate.try_accept().is_ok();
        }
        Ok(())
    }
}

#[tokio::test]
async fn cancellation_inside_admission_callback_never_reopens_writes() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let adapter = Arc::new(StrictCancelOnOpenAdapter {
        gate: Mutex::new(None),
        accepted_after_cancel: Mutex::new(false),
    });
    let coordinator = Coordinator::new(CoordinatorConfig {
        repository_root,
        database_path,
        adapter: Arc::clone(&adapter) as Arc<dyn CoordinatorAdapter>,
        limits: limits(1 << 30),
        remote: None,
    })
    .expect("coordinator");
    let gate = coordinator.admission_gate();
    *adapter.gate.lock().expect("gate lock") = Some(gate.clone());
    let (_sender, mut events) = tokio::sync::mpsc::channel(1);
    tokio::time::timeout(Duration::from_secs(5), coordinator.run(&mut events))
        .await
        .expect("no callback deadlock")
        .expect("clean cancellation");
    assert!(
        !*adapter
            .accepted_after_cancel
            .lock()
            .expect("acceptance lock")
    );
    assert!(gate.try_accept().is_err());
}

impl CoordinatorAdapter for CancelOnOpenAdapter {
    fn observe_txid(&self) -> io::Result<Option<u64>> {
        Ok(Some(1))
    }

    fn capture(&self) -> io::Result<CaptureResult> {
        Err(io::Error::other("capture should not be needed"))
    }

    fn set_write_admission(&self, enabled: bool, _limits: Limits) -> io::Result<()> {
        if enabled {
            self.cancellation
                .send(true)
                .map_err(|error| io::Error::other(error.to_string()))?;
        }
        self.admission.lock().expect("admission lock").push(enabled);
        Ok(())
    }
}

impl WriteGate for BlockingCaptureGate<'_> {
    type Permit = CapturePermit;

    fn acquire(&self) -> io::Result<Self::Permit> {
        if let Some(started) = self.0.started.lock().expect("started lock").take() {
            let _ = started.send(());
        }
        let (lock, wake) = &*self.0.release;
        let released = lock.lock().expect("release lock");
        let (released, _) = wake
            .wait_timeout_while(released, Duration::from_secs(10), |released| !*released)
            .expect("wait for release");
        if !*released {
            return Err(io::Error::other("test capture release timed out"));
        }
        Ok(CapturePermit)
    }
}

impl WritePermit for CapturePermit {
    fn release(self) -> io::Result<()> {
        Ok(())
    }
}

impl CoordinatorAdapter for BlockingCaptureAdapter {
    fn observe_txid(&self) -> io::Result<Option<u64>> {
        Ok(Some(2))
    }

    fn capture(&self) -> io::Result<CaptureResult> {
        sqlite_replication_core::capture_once(
            &self.database_path,
            &self.repository_root,
            &BlockingCaptureGate(self),
        )
    }

    fn set_write_admission(&self, enabled: bool, _limits: Limits) -> io::Result<()> {
        self.admission.lock().expect("admission lock").push(enabled);
        Ok(())
    }
}

struct TestAdapter {
    observed_txid: Option<u64>,
    admission: Mutex<Vec<bool>>,
    applied_limits: Mutex<Vec<Limits>>,
}

// Test-only admission policy: availability of independently established
// conservative bounds is simulated, not measured or enforced by this adapter.
struct UnknownBudgetAdapter {
    known: std::sync::atomic::AtomicBool,
}

struct VanishingTargetAdapter {
    observations: std::sync::atomic::AtomicUsize,
    unknown_at: usize,
    admissions: Mutex<Vec<bool>>,
}

impl CoordinatorAdapter for VanishingTargetAdapter {
    fn observe_txid(&self) -> io::Result<Option<u64>> {
        let call = self
            .observations
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if call < self.unknown_at {
            return Ok(Some(1));
        }
        Ok(None)
    }

    fn capture(&self) -> io::Result<CaptureResult> {
        Err(io::Error::other("unexpected capture"))
    }

    fn set_write_admission(&self, enabled: bool, _limits: Limits) -> io::Result<()> {
        self.admissions
            .lock()
            .expect("admission lock")
            .push(enabled);
        Ok(())
    }
}

#[tokio::test]
async fn target_becoming_unknown_before_reopen_keeps_admission_closed() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let adapter = Arc::new(VanishingTargetAdapter {
        observations: std::sync::atomic::AtomicUsize::new(0),
        unknown_at: 2,
        admissions: Mutex::new(Vec::new()),
    });
    let coordinator = Coordinator::new(CoordinatorConfig {
        repository_root,
        database_path,
        adapter: adapter.clone(),
        limits: limits(1 << 30),
        remote: None,
    })
    .expect("coordinator");
    let error = coordinator
        .flush()
        .await
        .expect_err("unknown final observation must fail closed");
    assert!(error.to_string().contains("unknown"));
    assert!(coordinator.admission_gate().try_accept().is_err());
    assert!(
        !adapter
            .admissions
            .lock()
            .expect("admission lock")
            .contains(&true)
    );
}

#[tokio::test]
async fn target_becoming_unknown_after_drain_keeps_admission_closed() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let adapter = Arc::new(VanishingTargetAdapter {
        observations: std::sync::atomic::AtomicUsize::new(0),
        unknown_at: 1,
        admissions: Mutex::new(Vec::new()),
    });
    let coordinator = Coordinator::new(CoordinatorConfig {
        repository_root,
        database_path,
        adapter: adapter.clone(),
        limits: limits(1 << 30),
        remote: None,
    })
    .expect("coordinator");
    let error = coordinator
        .flush()
        .await
        .expect_err("unknown target after drain must fail closed");
    assert!(error.to_string().contains("unknown"));
    assert_eq!(
        adapter
            .observations
            .load(std::sync::atomic::Ordering::SeqCst),
        2
    );
    assert!(coordinator.admission_gate().try_accept().is_err());
    assert_eq!(
        *adapter.admissions.lock().expect("admission lock"),
        vec![false]
    );
}

impl CoordinatorAdapter for UnknownBudgetAdapter {
    fn observe_txid(&self) -> io::Result<Option<u64>> {
        Ok(Some(1))
    }

    fn capture(&self) -> io::Result<CaptureResult> {
        Err(io::Error::other("unexpected capture"))
    }

    fn set_write_admission(&self, enabled: bool, _limits: Limits) -> io::Result<()> {
        if enabled && !self.known.load(std::sync::atomic::Ordering::Acquire) {
            return Err(io::Error::other("conservative budget unknown"));
        }
        Ok(())
    }
}

#[tokio::test]
async fn unknown_adapter_budget_fails_closed_until_verified_retry() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let adapter = Arc::new(UnknownBudgetAdapter {
        known: std::sync::atomic::AtomicBool::new(false),
    });
    let coordinator = Coordinator::new(CoordinatorConfig {
        repository_root,
        database_path,
        adapter: Arc::clone(&adapter) as Arc<dyn CoordinatorAdapter>,
        limits: limits(1 << 30),
        remote: None,
    })
    .expect("coordinator");
    let gate = coordinator.admission_gate();
    let error = coordinator
        .flush()
        .await
        .expect_err("unknown budget rejected");
    assert!(error.to_string().contains("conservative budget unknown"));
    assert!(gate.try_accept().is_err());
    adapter
        .known
        .store(true, std::sync::atomic::Ordering::Release);
    coordinator
        .flush()
        .await
        .expect("retry after budget becomes known");
    assert!(gate.try_accept().is_ok());
}

#[tokio::test]
async fn unknown_observed_target_reports_degraded_health() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let coordinator = Coordinator::new(CoordinatorConfig {
        repository_root,
        database_path,
        adapter: coordinator_adapter(None),
        limits: limits(1 << 30),
        remote: None,
    })
    .expect("coordinator");
    let health = coordinator.health().await;
    assert_eq!(health.local, ComponentState::Degraded);
    assert_eq!(health.local_error, ErrorCategory::Adapter);
    assert!(coordinator.flush().await.is_err());
    assert!(coordinator.admission_gate().try_accept().is_err());
}

impl CoordinatorAdapter for TestAdapter {
    fn observe_txid(&self) -> io::Result<Option<u64>> {
        Ok(self.observed_txid)
    }

    fn capture(&self) -> io::Result<CaptureResult> {
        Ok(CaptureResult {
            txid: self.observed_txid.unwrap_or(1),
            transactions: 0,
            wal_frames: 0,
        })
    }

    fn set_write_admission(&self, enabled: bool, limits: Limits) -> io::Result<()> {
        self.admission.lock().expect("admission lock").push(enabled);
        self.applied_limits
            .lock()
            .expect("limits lock")
            .push(limits);
        Ok(())
    }
}

#[tokio::test]
async fn stale_target_can_make_flush_skip_checkpointed_sqlite_changes() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let source = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    let connection = Connection::open(&source).expect("open source");
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE records(value TEXT);").expect("initialize");
    sqlite_replication_core::snapshot(&source, &repository).expect("snapshot");
    connection
        .execute("INSERT INTO records VALUES ('first')", [])
        .expect("commit");
    connection
        .execute("INSERT INTO records VALUES ('second')", [])
        .expect("commit");
    let busy: i64 = connection
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))
        .expect("checkpoint");
    assert_eq!(busy, 0);
    // Deliberately invalid test adapter reports the old durable TXID as observed.
    let coordinator = Coordinator::new(CoordinatorConfig {
        repository_root: repository.clone(),
        database_path: source,
        adapter: coordinator_adapter(Some(1)),
        limits: limits(1 << 30),
        remote: None,
    })
    .expect("coordinator");
    coordinator
        .flush()
        .await
        .expect("stale target makes flush return success");
    let restored = directory.path().join("restored.sqlite");
    sqlite_replication_core::restore_to_target(&repository, &restored, RestoreTarget::Txid(1))
        .expect("restore snapshot");
    let count: i64 = Connection::open(restored)
        .expect("open restore")
        .query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))
        .expect("count");
    assert_eq!(count, 0, "stale target skipped both committed rows");
}

#[test]
fn status_separates_observed_and_local_durable_positions() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root)
        .expect("create initial snapshot");
    let coordinator = coordinator(&repository_root, &database_path, Some(2), None);

    let status = coordinator.status().expect("read status");
    assert_eq!(status.observed_txid, Some(2));
    assert_eq!(status.local_durable.expect("local position").txid, 1);
    assert_eq!(status.remote_confirmed, None);
    assert!(!status.remote_ack_matches);
    assert!(status.unprotected_age <= Duration::from_secs(60));
}

#[tokio::test]
async fn flush_and_run_close_admission_on_limit_and_shutdown() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root)
        .expect("create initial snapshot");

    let adapter = coordinator_adapter(Some(1));
    let coordinator = Coordinator::new(CoordinatorConfig {
        repository_root: repository_root.clone(),
        database_path: database_path.clone(),
        adapter: Arc::clone(&adapter) as Arc<dyn CoordinatorAdapter>,
        limits: limits(1 << 30),
        remote: None,
    })
    .expect("construct coordinator");
    coordinator
        .flush()
        .await
        .expect("flush already durable target");
    assert_eq!(
        *adapter.admission.lock().expect("admission lock"),
        vec![false, true]
    );
    assert_eq!(
        *adapter.applied_limits.lock().expect("limits lock"),
        vec![limits(1 << 30), limits(1 << 30)]
    );

    let run_adapter = Arc::new(TestAdapter {
        observed_txid: Some(1),
        admission: Mutex::new(Vec::new()),
        applied_limits: Mutex::new(Vec::new()),
    });
    let run_coordinator = Coordinator::new(CoordinatorConfig {
        repository_root: repository_root.clone(),
        database_path: database_path.clone(),
        adapter: Arc::clone(&run_adapter) as Arc<dyn CoordinatorAdapter>,
        limits: limits(1 << 30),
        remote: None,
    })
    .expect("construct run coordinator");
    let (_event_sender, mut events) = tokio::sync::mpsc::channel(1);
    drop(_event_sender);
    run_coordinator
        .run(&mut events)
        .await
        .expect("run coordinator to closed event stream");
    assert_eq!(
        *run_adapter.admission.lock().expect("admission lock"),
        vec![false, false, true, false]
    );

    let limited_adapter = coordinator_adapter(Some(1));
    let limited = Coordinator::new(CoordinatorConfig {
        repository_root: repository_root.clone(),
        database_path: database_path.clone(),
        adapter: Arc::clone(&limited_adapter) as Arc<dyn CoordinatorAdapter>,
        limits: limits(1),
        remote: None,
    })
    .expect("construct limited coordinator");
    assert!(limited.flush().await.is_err());
    assert_eq!(
        *limited_adapter.admission.lock().expect("admission lock"),
        vec![false]
    );
    assert_eq!(limited.health().await.local, ComponentState::Degraded);
}

#[tokio::test]
async fn run_with_preexisting_cancellation_never_opens_admission() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root)
        .expect("create initial snapshot");
    let adapter = coordinator_adapter(Some(1));
    let coordinator = Coordinator::new(CoordinatorConfig {
        repository_root,
        database_path,
        adapter: Arc::clone(&adapter) as Arc<dyn CoordinatorAdapter>,
        limits: limits(1 << 30),
        remote: None,
    })
    .expect("construct coordinator");
    coordinator.admission_gate().cancel().expect("pre-cancel");
    let (_event_sender, mut events) = tokio::sync::mpsc::channel(1);

    tokio::time::timeout(Duration::from_millis(500), coordinator.run(&mut events))
        .await
        .expect("pre-cancelled worker exits promptly")
        .expect("worker closes admission");
    assert_eq!(*adapter.admission.lock().expect("admission lock"), [false]);
}

#[tokio::test]
async fn cancellation_during_capture_preserves_durable_data_and_closed_admission() {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return;
    }
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root)
        .expect("create initial snapshot");
    let connection = Connection::open(&database_path).expect("open source");
    connection
        .execute("INSERT INTO records (value) VALUES ('captured')", [])
        .expect("commit row");
    drop(connection);

    let (started_sender, started_receiver) = tokio::sync::oneshot::channel();
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let adapter = Arc::new(BlockingCaptureAdapter {
        database_path: database_path.clone(),
        repository_root: repository_root.clone(),
        admission: Mutex::new(Vec::new()),
        started: Mutex::new(Some(started_sender)),
        release: Arc::clone(&release),
    });
    let coordinator = Coordinator::new(CoordinatorConfig {
        repository_root: repository_root.clone(),
        database_path,
        adapter: Arc::clone(&adapter) as Arc<dyn CoordinatorAdapter>,
        limits: limits(1 << 30),
        remote: None,
    })
    .expect("construct coordinator");
    let gate = coordinator.admission_gate();
    let (_event_sender, mut events) = tokio::sync::mpsc::channel(1);
    let worker = tokio::spawn(async move { coordinator.run(&mut events).await });
    tokio::time::timeout(Duration::from_secs(5), started_receiver)
        .await
        .expect("capture starts")
        .expect("capture signal");
    gate.cancel().expect("cancel while capture is in flight");
    assert!(gate.try_accept().is_err());
    assert_eq!(
        adapter.admission.lock().expect("admission lock").last(),
        Some(&false)
    );
    let (lock, wake) = &*release;
    *lock.lock().expect("release lock") = true;
    wake.notify_all();
    tokio::time::timeout(Duration::from_secs(10), worker)
        .await
        .expect("worker drains capture")
        .expect("worker task")
        .expect("worker closes admission");
    assert!(
        adapter
            .admission
            .lock()
            .expect("admission lock")
            .iter()
            .all(|enabled| !enabled)
    );
    let restored = directory.path().join("restored.sqlite");
    sqlite_replication_core::restore_to_target(&repository_root, &restored, RestoreTarget::Txid(2))
        .expect("restore durable capture after cancellation");
    let count: i64 = Connection::open(restored)
        .expect("open restored database")
        .query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))
        .expect("count restored rows");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn legacy_independent_cancellation_can_reopen_adapter_admission() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root)
        .expect("create initial snapshot");
    let (cancel_sender, mut cancellation) = tokio::sync::watch::channel(false);
    let adapter = Arc::new(CancelOnOpenAdapter {
        cancellation: cancel_sender,
        admission: Mutex::new(Vec::new()),
    });
    let observation = Arc::new(AdmissionObservation {
        gate: Mutex::new(None),
        accepted: Mutex::new(None),
    });
    let coordinator = Coordinator::new(CoordinatorConfig {
        repository_root,
        database_path,
        adapter: Arc::clone(&adapter) as Arc<dyn CoordinatorAdapter>,
        limits: limits(1 << 30),
        remote: None,
    })
    .expect("construct coordinator")
    .with_metrics(observation.clone());
    *observation.gate.lock().expect("gate lock") = Some(coordinator.admission_gate());
    let (_event_sender, mut events) = tokio::sync::mpsc::channel(1);
    tokio::time::timeout(
        Duration::from_secs(5),
        coordinator.run_legacy(&mut events, &mut cancellation),
    )
    .await
    .expect("worker exits without deadlocking callback")
    .expect("worker closes admission");
    let transitions = adapter.admission.lock().expect("admission lock");
    assert!(
        transitions.contains(&true),
        "this legacy API deliberately characterizes the unsafe callback transition"
    );
    assert_eq!(
        *observation.accepted.lock().expect("observation lock"),
        Some(false)
    );
}

#[tokio::test]
async fn worker_handles_repeated_events_and_closes_on_cancellation() {
    const EVENTS: usize = 16;

    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root)
        .expect("create initial snapshot");
    let adapter = coordinator_adapter(Some(1));
    let coordinator = Arc::new(
        Coordinator::new(CoordinatorConfig {
            repository_root,
            database_path,
            adapter: Arc::clone(&adapter) as Arc<dyn CoordinatorAdapter>,
            limits: limits(1 << 30),
            remote: None,
        })
        .expect("construct coordinator"),
    );
    let gate = coordinator.admission_gate();
    let (event_sender, mut events) = tokio::sync::mpsc::channel(1);
    let worker = tokio::spawn(async move { coordinator.run(&mut events).await });

    for index in 0..EVENTS {
        event_sender.send(()).await.expect("send event");
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if adapter.admission.lock().expect("admission lock").len() >= 3 + (index + 1) * 2 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("worker processes event");
    }
    gate.cancel().expect("signal cancellation");
    assert!(gate.try_accept().is_err());
    tokio::time::timeout(Duration::from_secs(5), worker)
        .await
        .expect("worker stops after cancellation")
        .expect("worker task succeeds")
        .expect("worker closes admission");
    assert_eq!(
        adapter.admission.lock().expect("admission lock").last(),
        Some(&false)
    );
}

#[tokio::test]
async fn dropping_run_future_cancels_shared_admission() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let coordinator = coordinator(&repository_root, &database_path, Some(1), None);
    let gate = coordinator.admission_gate();
    let (_sender, mut events) = tokio::sync::mpsc::channel(1);
    let worker = tokio::spawn(async move { coordinator.run(&mut events).await });
    tokio::time::timeout(Duration::from_secs(5), async {
        while gate.try_accept().is_err() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("admission opens");
    worker.abort();
    assert!(worker.await.expect_err("task is aborted").is_cancelled());
    assert!(gate.try_accept().is_err());
}

#[tokio::test]
async fn flush_waits_for_previously_accepted_writer() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let adapter = coordinator_adapter(Some(1));
    let coordinator = Arc::new(
        Coordinator::new(CoordinatorConfig {
            repository_root,
            database_path,
            adapter: Arc::clone(&adapter) as Arc<dyn CoordinatorAdapter>,
            limits: limits(1 << 30),
            remote: None,
        })
        .expect("coordinator"),
    );
    coordinator.flush().await.expect("open admission");
    let gate = coordinator.admission_gate();
    let permit = gate.try_accept().expect("accept writer");
    let running = Arc::clone(&coordinator);
    let mut worker = tokio::spawn(async move { running.flush().await });
    tokio::time::timeout(Duration::from_secs(5), async {
        while adapter.admission.lock().expect("admission lock").len() < 3 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("flush closed admission");
    assert!(gate.try_accept().is_err());
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut worker)
            .await
            .is_err(),
        "flush must wait for the accepted writer"
    );
    drop(permit);
    tokio::time::timeout(Duration::from_secs(5), worker)
        .await
        .expect("flush completes after writer")
        .expect("worker task")
        .expect("flush succeeds");
}

#[tokio::test]
async fn timed_out_flush_during_drain_stays_closed_until_retry() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let coordinator = coordinator(&repository_root, &database_path, Some(1), None);
    coordinator.flush().await.expect("open admission");
    let gate = coordinator.admission_gate();
    let permit = gate.try_accept().expect("accept writer");

    assert!(
        tokio::time::timeout(Duration::from_millis(100), coordinator.flush())
            .await
            .is_err(),
        "flush should time out while the writer retains its permit"
    );
    assert!(
        gate.try_accept().is_err(),
        "timeout must not reopen admission"
    );
    drop(permit);
    tokio::time::timeout(Duration::from_secs(5), coordinator.flush())
        .await
        .expect("retry should finish")
        .expect("retry flush succeeds");
    assert!(
        gate.try_accept().is_ok(),
        "successful retry reopens admission"
    );
}

#[tokio::test]
async fn flush_drains_abandoned_budget_reservation_but_does_not_reconcile_it() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let adapter = coordinator_adapter(Some(1));
    let coordinator = Arc::new(
        Coordinator::new(CoordinatorConfig {
            repository_root,
            database_path,
            adapter: Arc::clone(&adapter) as Arc<dyn CoordinatorAdapter>,
            limits: limits(1 << 30),
            remote: None,
        })
        .expect("coordinator"),
    );
    coordinator.flush().await.expect("open admission");
    let budget = ConservativeBudget::new(coordinator.admission_gate(), 10).expect("budget");
    budget
        .reconcile_with(|| Ok(Some(0)))
        .expect("logical baseline");
    let reservation = budget.try_reserve(Some(2)).expect("accepted writer");
    let running = Arc::clone(&coordinator);
    let mut flush = tokio::spawn(async move { running.flush().await });
    tokio::time::timeout(Duration::from_secs(5), async {
        while adapter.admission.lock().expect("admission lock").len() < 3 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("flush closed admission");
    assert!(budget.try_reserve(Some(1)).is_err());
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut flush)
            .await
            .is_err()
    );
    drop(reservation);
    tokio::time::timeout(Duration::from_secs(5), flush)
        .await
        .expect("flush finishes")
        .expect("task")
        .expect("flush");
    assert!(
        coordinator.admission_gate().try_accept().is_ok(),
        "test adapter reopens gate"
    );
    assert!(
        budget.try_reserve(Some(1)).is_err(),
        "abandoned reservation remains unknown"
    );
    budget
        .reconcile_with(|| Ok(Some(2)))
        .expect("manual logical reconciliation");
    budget
        .try_reserve(Some(1))
        .expect("reconciled budget")
        .settle(Some(0))
        .expect("settle");
}

#[tokio::test]
async fn capacity_reference_blocks_overcommit_and_drains_with_coordinator() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let coordinator = Arc::new(coordinator(&repository_root, &database_path, Some(1), None));
    coordinator.flush().await.expect("open admission");
    let admission = coordinator.admission_gate();
    let capacity = CapacityGate::new(admission.clone(), 5).expect("capacity");
    let permit = capacity.try_reserve(4).expect("reserve");
    assert_eq!(
        capacity.try_reserve(2).err().expect("full").kind(),
        io::ErrorKind::WouldBlock
    );
    let running = Arc::clone(&coordinator);
    let mut flush = tokio::spawn(async move { running.flush().await });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut flush)
            .await
            .is_err()
    );
    assert!(capacity.try_reserve(1).is_err());
    drop(permit);
    flush.await.expect("flush task").expect("flush succeeds");
    let permit = capacity.try_reserve(5).expect("capacity recovered");
    admission.cancel().expect("cancel");
    drop(permit);
    assert!(capacity.try_reserve(1).is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn capacity_reference_controls_cooperative_sqlite_transactions() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let coordinator = coordinator(&repository_root, &database_path, Some(1), None);
    coordinator.flush().await.expect("open admission");
    let admission = coordinator.admission_gate();
    let capacity = CapacityGate::new(admission.clone(), 1).expect("capacity");
    let (started_tx, started_rx) = std::sync::mpsc::sync_channel(1);
    let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
    let writer_capacity = capacity.clone();
    let writer_path = database_path.clone();
    let writer = std::thread::spawn(move || {
        let _permit = writer_capacity.try_reserve(1).expect("reserve first write");
        let connection = Connection::open(writer_path).expect("open writer");
        connection
            .execute_batch("BEGIN IMMEDIATE; INSERT INTO records(value) VALUES ('first');")
            .expect("start transaction");
        started_tx.send(()).expect("signal started");
        release_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("release transaction");
        connection.execute_batch("COMMIT").expect("commit first");
    });
    started_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("transaction starts");
    assert_eq!(
        capacity.try_reserve(1).err().expect("saturated").kind(),
        io::ErrorKind::WouldBlock
    );
    release_tx.send(()).expect("release writer");
    writer.join().expect("writer exits");

    {
        let _permit = capacity.try_reserve(1).expect("capacity released");
        let connection = Connection::open(&database_path).expect("open second writer");
        connection
            .execute_batch("BEGIN IMMEDIATE; INSERT INTO records(value) VALUES ('second');")
            .expect("start second transaction");
        admission
            .cancel()
            .expect("cancel during accepted transaction");
        assert!(capacity.try_reserve(1).is_err());
        connection.execute_batch("COMMIT").expect("commit second");
    }
    assert!(capacity.try_reserve(1).is_err());
    let count: i64 = Connection::open(&database_path)
        .expect("open reader")
        .query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))
        .expect("count committed rows");
    assert_eq!(count, 2);
}

#[tokio::test]
async fn failed_sqlite_transaction_rolls_back_and_releases_capacity() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let coordinator = coordinator(&repository_root, &database_path, Some(1), None);
    coordinator.flush().await.expect("open admission");
    let gate = coordinator.admission_gate();
    let capacity = CapacityGate::new(gate, 1).expect("capacity");

    {
        let _permit = capacity.try_reserve(1).expect("reserve transaction");
        let mut connection = Connection::open(&database_path).expect("open writer");
        let transaction = connection.transaction().expect("begin transaction");
        transaction
            .execute("INSERT INTO records(id,value) VALUES (1,'temporary')", [])
            .expect("first insert");
        assert!(
            transaction
                .execute("INSERT INTO records(id,value) VALUES (1,'duplicate')", [])
                .is_err(),
            "duplicate key must fail"
        );
        assert_eq!(
            capacity
                .try_reserve(1)
                .err()
                .expect("still reserved")
                .kind(),
            io::ErrorKind::WouldBlock
        );
        transaction.rollback().expect("rollback failed transaction");
    }

    let _permit = capacity
        .try_reserve(1)
        .expect("capacity released after rollback");
    let connection = Connection::open(&database_path).expect("open reader");
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))
        .expect("count records");
    assert_eq!(count, 0, "rollback must not persist the first insert");
    connection
        .execute("INSERT INTO records(id,value) VALUES (1,'retry')", [])
        .expect("retry succeeds with recovered capacity");
}

#[tokio::test]
async fn characterize_operation_quota_does_not_enforce_wal_bytes() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let connection = Connection::open(&database_path).expect("open SQLite");
    connection
        .execute_batch("PRAGMA wal_autocheckpoint=0; PRAGMA wal_checkpoint(TRUNCATE)")
        .expect("truncate initial WAL");
    let adapter = coordinator_adapter(Some(1));
    let mut configured_limits = limits(1 << 30);
    configured_limits.max_wal_bytes = 9_000;
    let coordinator = Coordinator::new(CoordinatorConfig {
        repository_root,
        database_path,
        adapter,
        limits: configured_limits,
        remote: None,
    })
    .expect("coordinator");
    coordinator.flush().await.expect("initial flush");
    let capacity = CapacityGate::new(coordinator.admission_gate(), 1).expect("capacity");
    for _ in 0..3 {
        let _permit = capacity.try_reserve(1).expect("operation admitted");
        connection
            .execute("INSERT INTO records(value) VALUES ('test')", [])
            .expect("commit write");
    }
    let status = coordinator.status().expect("status after writes");
    println!(
        "wal_bytes={} configured_max={}",
        status.wal_bytes, configured_limits.max_wal_bytes
    );
    assert!(status.wal_bytes > configured_limits.max_wal_bytes);
    assert!(
        capacity.try_reserve(1).is_ok(),
        "characterization: operation budget alone does not enforce the WAL byte limit"
    );
}

#[tokio::test]
async fn cancellation_during_drain_leaves_admission_closed_without_waiting_for_writer() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let adapter = coordinator_adapter(Some(1));
    let coordinator = Coordinator::new(CoordinatorConfig {
        repository_root,
        database_path,
        adapter: Arc::clone(&adapter) as Arc<dyn CoordinatorAdapter>,
        limits: limits(1 << 30),
        remote: None,
    })
    .expect("coordinator");
    let gate = coordinator.admission_gate();
    let (sender, mut events) = tokio::sync::mpsc::channel(1);
    let worker = tokio::spawn(async move { coordinator.run(&mut events).await });
    tokio::time::timeout(Duration::from_secs(5), async {
        while gate.try_accept().is_err() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("admission opens");
    let permit = gate.try_accept().expect("accept writer");
    sender.send(()).await.expect("request flush");
    tokio::time::timeout(Duration::from_secs(5), async {
        while adapter.admission.lock().expect("admission lock").len() < 4 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("flush closes admission");
    gate.cancel().expect("cancel during drain");
    tokio::time::timeout(Duration::from_secs(1), worker)
        .await
        .expect("cancelled run exits without revoking the writer")
        .expect("worker task")
        .expect("clean cancellation");
    assert!(gate.try_accept().is_err());
    drop(permit);
    assert!(gate.try_accept().is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn second_run_is_rejected_without_cancelling_active_run() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let coordinator = Arc::new(coordinator(&repository_root, &database_path, Some(1), None));
    let gate = coordinator.admission_gate();
    let (_sender, mut events) = tokio::sync::mpsc::channel(1);
    let running = Arc::clone(&coordinator);
    let worker = tokio::spawn(async move { running.run(&mut events).await });
    tokio::time::timeout(Duration::from_secs(5), async {
        while gate.try_accept().is_err() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("first run opens admission");

    let (sender, mut second_events) = tokio::sync::mpsc::channel(1);
    drop(sender);
    let error = tokio::time::timeout(Duration::from_secs(5), coordinator.run(&mut second_events))
        .await
        .expect("second run is rejected promptly")
        .expect_err("second run must not run concurrently");
    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert!(gate.try_accept().is_ok(), "first run remains open");

    let (_cancel_sender, mut cancellation) = tokio::sync::watch::channel(false);
    let error = coordinator
        .run_legacy(&mut second_events, &mut cancellation)
        .await
        .expect_err("legacy run must not overlap the active run");
    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert!(gate.try_accept().is_ok(), "legacy call did not cancel run");

    gate.cancel().expect("stop first run");
    worker.await.expect("worker task").expect("worker exits");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cooperative_sqlite_writers_reject_after_cancellation() {
    const WRITERS: usize = 8;

    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root).expect("snapshot");
    let coordinator = coordinator(&repository_root, &database_path, Some(1), None);
    let gate = coordinator.admission_gate();
    let (_sender, mut events) = tokio::sync::mpsc::channel(1);
    let worker = tokio::spawn(async move { coordinator.run(&mut events).await });

    tokio::time::timeout(Duration::from_secs(5), async {
        while gate.try_accept().is_err() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("admission opens");

    let _permit = gate.try_accept().expect("accept control write");
    Connection::open(&database_path)
        .expect("open database")
        .execute("INSERT INTO records (value) VALUES ('before')", [])
        .expect("control write succeeds");

    let barrier = Arc::new(std::sync::Barrier::new(WRITERS + 1));
    let mut writers = Vec::new();
    for _ in 0..WRITERS {
        let gate = gate.clone();
        let path = database_path.clone();
        let barrier = Arc::clone(&barrier);
        writers.push(std::thread::spawn(move || {
            barrier.wait();
            let Ok(_permit) = gate.try_accept() else {
                return false;
            };
            Connection::open(path)
                .expect("open writer database")
                .execute("INSERT INTO records (value) VALUES ('after')", [])
                .expect("write admitted unexpectedly");
            true
        }));
    }
    gate.cancel().expect("cancel");
    barrier.wait();
    for writer in writers {
        assert!(!writer.join().expect("writer thread"));
    }
    worker.await.expect("worker task").expect("worker exits");
    let (before, after): (i64, i64) = Connection::open(&database_path)
        .expect("open database")
        .query_row(
            "SELECT COUNT(*) FILTER (WHERE value = 'before'), COUNT(*) FILTER (WHERE value = 'after') FROM records",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("count persisted rows");
    assert_eq!((before, after), (1, 0));
}

#[tokio::test]
async fn concurrent_flushes_keep_admission_transitions_serialized_under_stress() {
    const BATCHES: usize = 4;
    const CONCURRENT_FLUSHES: usize = 16;

    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let repository_root = directory.path().join("repository");
    sqlite_replication_core::snapshot(&database_path, &repository_root)
        .expect("create initial snapshot");

    let adapter = coordinator_adapter(Some(1));
    let coordinator = Arc::new(
        Coordinator::new(CoordinatorConfig {
            repository_root,
            database_path,
            adapter: Arc::clone(&adapter) as Arc<dyn CoordinatorAdapter>,
            limits: limits(1 << 30),
            remote: None,
        })
        .expect("construct coordinator"),
    );

    for _ in 0..BATCHES {
        let mut tasks = tokio::task::JoinSet::new();
        for _ in 0..CONCURRENT_FLUSHES {
            let coordinator = Arc::clone(&coordinator);
            tasks.spawn(async move { coordinator.flush().await });
        }
        while let Some(result) = tasks.join_next().await {
            result
                .expect("join concurrent Flush")
                .expect("concurrent Flush succeeds");
        }
    }

    let transitions = adapter.admission.lock().expect("admission lock");
    assert_eq!(transitions.len(), BATCHES * CONCURRENT_FLUSHES * 2);
    let (pairs, remainder) = transitions.as_slice().as_chunks::<2>();
    assert!(remainder.is_empty(), "every Flush has two transitions");
    for pair in pairs {
        assert_eq!(*pair, [false, true], "each Flush must serialize close/open");
    }
}

#[test]
fn constructor_requires_all_limits() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("source.sqlite");
    create_database(&database_path);
    let adapter = Arc::new(TestAdapter {
        observed_txid: Some(1),
        admission: Mutex::new(Vec::new()),
        applied_limits: Mutex::new(Vec::new()),
    });
    let mut config = CoordinatorConfig {
        repository_root: directory.path().join("repository"),
        database_path,
        adapter: adapter as Arc<dyn CoordinatorAdapter>,
        limits: limits(1 << 30),
        remote: None,
    };
    config.limits.max_wal_bytes = 0;
    assert!(Coordinator::new(config).is_err());
}

fn coordinator(
    repository_root: &Path,
    database_path: &Path,
    observed_txid: Option<u64>,
    max_spool_bytes: Option<u64>,
) -> Coordinator {
    let adapter = Arc::new(TestAdapter {
        observed_txid,
        admission: Mutex::new(Vec::new()),
        applied_limits: Mutex::new(Vec::new()),
    });
    Coordinator::new(CoordinatorConfig {
        repository_root: repository_root.to_path_buf(),
        database_path: database_path.to_path_buf(),
        adapter: adapter as Arc<dyn CoordinatorAdapter>,
        limits: limits(max_spool_bytes.unwrap_or(1 << 30)),
        remote: None,
    })
    .expect("construct coordinator")
}

fn coordinator_adapter(observed_txid: Option<u64>) -> Arc<TestAdapter> {
    Arc::new(TestAdapter {
        observed_txid,
        admission: Mutex::new(Vec::new()),
        applied_limits: Mutex::new(Vec::new()),
    })
}

fn limits(max_spool_bytes: u64) -> Limits {
    Limits {
        max_spool_bytes,
        max_wal_bytes: 64 << 20,
        max_unprotected_age: Duration::from_secs(60),
        max_uploads_in_flight: 1,
    }
}

fn create_database(path: &Path) {
    let connection = Connection::open(path).expect("open source database");
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .expect("enable WAL");
    connection
        .execute(
            "CREATE TABLE records (id INTEGER PRIMARY KEY, value TEXT NOT NULL)",
            [],
        )
        .expect("create records table");
}
