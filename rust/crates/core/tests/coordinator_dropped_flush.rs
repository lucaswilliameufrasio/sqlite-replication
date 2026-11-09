use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use rusqlite::Connection;
use sqlite_replication_core::{
    CaptureResult, Coordinator, CoordinatorAdapter, CoordinatorConfig, Limits,
};
use tokio::sync::mpsc;

struct BlockedCaptures {
    entered: mpsc::UnboundedSender<usize>,
    finished: mpsc::UnboundedSender<usize>,
    sequence: AtomicUsize,
    release: (Mutex<bool>, Condvar),
    admission: Mutex<Vec<bool>>,
}

impl CoordinatorAdapter for BlockedCaptures {
    fn observe_txid(&self) -> io::Result<Option<u64>> {
        Ok(Some(2))
    }

    fn capture(&self) -> io::Result<CaptureResult> {
        let number = self.sequence.fetch_add(1, Ordering::SeqCst) + 1;
        self.entered.send(number).expect("report capture entry");
        let (lock, wake) = &self.release;
        let released = lock.lock().expect("release lock");
        let (released, _) = wake
            .wait_timeout_while(released, Duration::from_secs(10), |released| !*released)
            .expect("wait for release");
        if !*released {
            return Err(io::Error::other("capture release timed out"));
        }
        self.finished
            .send(number)
            .expect("report capture completion");
        Err(io::Error::other("synthetic capture failure"))
    }

    fn set_write_admission(&self, enabled: bool, _limits: Limits) -> io::Result<()> {
        self.admission.lock().expect("admission lock").push(enabled);
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn second_flush_waits_for_dropped_flush_capture_to_finish() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    let connection = Connection::open(&database).expect("open SQLite");
    connection.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE data(value TEXT); INSERT INTO data VALUES ('durable');").expect("create source");
    drop(connection);
    sqlite_replication_core::snapshot(&database, &repository).expect("snapshot");
    let (entered_tx, mut entered_rx) = mpsc::unbounded_channel();
    let (finished_tx, mut finished_rx) = mpsc::unbounded_channel();
    let adapter = Arc::new(BlockedCaptures {
        entered: entered_tx,
        finished: finished_tx,
        sequence: AtomicUsize::new(0),
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
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), entered_rx.recv())
            .await
            .expect("first entry timeout"),
        Some(1)
    );
    first.abort();
    assert!(
        first
            .await
            .expect_err("first future aborted")
            .is_cancelled()
    );
    let second = tokio::spawn(async move { coordinator.flush().await });
    let second_entry = tokio::time::timeout(Duration::from_millis(150), entered_rx.recv()).await;
    let finished_before_release = finished_rx.try_recv();
    let (lock, wake) = &adapter.release;
    *lock.lock().expect("release lock") = true;
    wake.notify_all();
    let second_result = tokio::time::timeout(Duration::from_secs(5), second)
        .await
        .expect("second task exits")
        .expect("second task succeeds");
    assert!(second_result.is_err(), "synthetic capture fails");
    let mut finished = vec![
        tokio::time::timeout(Duration::from_secs(5), finished_rx.recv())
            .await
            .expect("first completion"),
        tokio::time::timeout(Duration::from_secs(5), finished_rx.recv())
            .await
            .expect("second completion"),
    ];
    finished.sort();
    assert!(second_entry.is_err(), "second capture must wait for first");
    assert_eq!(entered_rx.try_recv().expect("second capture entry"), 2);
    assert!(
        matches!(
            finished_before_release,
            Err(mpsc::error::TryRecvError::Empty)
        ),
        "neither blocked capture completed before release"
    );
    assert_eq!(finished, [Some(1), Some(2)]);
    assert_eq!(
        *adapter.admission.lock().expect("admission lock"),
        [false, false]
    );
    let restored = directory.path().join("restored.sqlite");
    sqlite_replication_core::restore_if_missing(&repository, &restored)
        .expect("restore original snapshot");
    let value: String = Connection::open(restored)
        .expect("open restored database")
        .query_row("SELECT value FROM data", [], |row| row.get(0))
        .expect("read restored row");
    assert_eq!(value, "durable");
}
