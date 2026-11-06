use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak, mpsc};
use std::time::Duration;

use rusqlite::Connection;
use sqlite_replication_core::{
    CaptureResult, Coordinator, CoordinatorAdapter, CoordinatorConfig, CoordinatorMetrics,
    FlushOutcome, Limits,
};

struct Adapter(AtomicBool);

impl CoordinatorAdapter for Adapter {
    fn observe_txid(&self) -> io::Result<Option<u64>> {
        Ok(self.0.load(Ordering::Acquire).then_some(1))
    }

    fn capture(&self) -> io::Result<CaptureResult> {
        Err(io::Error::other("capture is not expected"))
    }

    fn set_write_admission(&self, _enabled: bool, _limits: Limits) -> io::Result<()> {
        Ok(())
    }
}

struct Observations {
    values: Mutex<Vec<FlushOutcome>>,
    coordinator: Mutex<Weak<Coordinator>>,
    reentered: AtomicBool,
    nested_ok: AtomicBool,
}

impl CoordinatorMetrics for Observations {
    fn observe_flush(&self, _duration: Duration, outcome: FlushOutcome) {
        self.values.lock().expect("metrics lock").push(outcome);
        if !self.reentered.swap(true, Ordering::AcqRel) {
            let coordinator = self
                .coordinator
                .lock()
                .expect("coordinator lock")
                .upgrade()
                .expect("coordinator exists");
            let (sender, receiver) = mpsc::sync_channel(1);
            tokio::spawn(async move {
                let _ = sender.send(coordinator.flush().await.is_ok());
            });
            self.nested_ok.store(
                receiver
                    .recv_timeout(Duration::from_secs(2))
                    .unwrap_or(false),
                Ordering::Release,
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn flush_metrics_report_outcomes_after_unlock() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    let connection = Connection::open(&database).expect("open SQLite");
    connection.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE data (value TEXT); INSERT INTO data VALUES ('test');").expect("create source");
    drop(connection);
    sqlite_replication_core::snapshot(&database, &repository).expect("snapshot");
    let adapter = Arc::new(Adapter(AtomicBool::new(true)));
    let metrics = Arc::new(Observations {
        values: Mutex::new(Vec::new()),
        coordinator: Mutex::new(Weak::new()),
        reentered: AtomicBool::new(false),
        nested_ok: AtomicBool::new(false),
    });
    let coordinator = Arc::new(
        Coordinator::new(CoordinatorConfig {
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
        .expect("coordinator")
        .with_metrics(metrics.clone()),
    );
    *metrics.coordinator.lock().expect("coordinator lock") = Arc::downgrade(&coordinator);
    coordinator.flush().await.expect("flush succeeds");
    assert!(
        metrics.nested_ok.load(Ordering::Acquire),
        "callback's nested flush must complete after lock release"
    );
    adapter.0.store(false, Ordering::Release);
    assert!(coordinator.flush().await.is_err());
    assert_eq!(
        *metrics.values.lock().expect("metrics lock"),
        [
            FlushOutcome::ReturnedOk,
            FlushOutcome::ReturnedOk,
            FlushOutcome::ReturnedError
        ]
    );
}
