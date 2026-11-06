use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use opentelemetry::trace::{TraceContextExt, Tracer, TracerProvider};
use rusqlite::Connection;
use sqlite_replication_coordinator_telemetry_example::traced_flush;
use sqlite_replication_core::{
    CaptureResult, Coordinator, CoordinatorAdapter, CoordinatorConfig, Limits,
};
use tokio::sync::oneshot;

struct BlockingAdapter {
    started: Mutex<Option<oneshot::Sender<()>>>,
    finished: Mutex<Option<oneshot::Sender<()>>>,
    release: Arc<(Mutex<bool>, Condvar)>,
    completed: AtomicBool,
    admission: Mutex<Vec<bool>>,
}

impl CoordinatorAdapter for BlockingAdapter {
    fn observe_txid(&self) -> io::Result<Option<u64>> {
        Ok(Some(2))
    }

    fn capture(&self) -> io::Result<CaptureResult> {
        self.started
            .lock()
            .expect("started lock")
            .take()
            .expect("start sender")
            .send(())
            .expect("signal start");
        let (lock, wake) = &*self.release;
        let released = lock.lock().expect("release lock");
        let (released, _) = wake
            .wait_timeout_while(released, Duration::from_secs(5), |released| !*released)
            .expect("wait for release");
        if !*released {
            return Err(io::Error::other("test capture timed out"));
        }
        self.completed.store(true, Ordering::Release);
        self.finished
            .lock()
            .expect("finished lock")
            .take()
            .expect("finish sender")
            .send(())
            .expect("signal finish");
        Err(io::Error::other("synthetic capture failure after release"))
    }

    fn set_write_admission(&self, enabled: bool, _limits: Limits) -> io::Result<()> {
        self.admission.lock().expect("admission lock").push(enabled);
        Ok(())
    }
}

async fn check_dropped_flush(abort_task: bool) {
    let exporter = opentelemetry_sdk::trace::InMemorySpanExporter::default();
    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_simple_exporter(exporter.clone())
        .build();
    let tracer = provider.tracer("sqlite-replication-drop-test");
    let parent = opentelemetry::Context::new().with_span(tracer.start("synthetic-parent"));
    let parent_id = parent.span().span_context().clone();
    let directory = tempfile::tempdir().expect("temporary directory");
    let database = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    let connection = Connection::open(&database).expect("open database");
    connection.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE data(value TEXT); INSERT INTO data VALUES ('fixture');").expect("create fixture");
    drop(connection);
    sqlite_replication_core::snapshot(&database, &repository).expect("snapshot");
    let (started_tx, started_rx) = oneshot::channel();
    let (finished_tx, finished_rx) = oneshot::channel();
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let adapter = Arc::new(BlockingAdapter {
        started: Mutex::new(Some(started_tx)),
        finished: Mutex::new(Some(finished_tx)),
        release: release.clone(),
        completed: AtomicBool::new(false),
        admission: Mutex::new(Vec::new()),
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
        .expect("coordinator"),
    );
    let (discard_tx, discard_rx) = oneshot::channel();
    let child_parent = parent.clone();
    let task = tokio::spawn(async move {
        tokio::select! {
            result = traced_flush(&coordinator, &tracer, &child_parent) => Some(result),
            _ = discard_rx => None,
        }
    });
    tokio::time::timeout(Duration::from_secs(3), started_rx)
        .await
        .expect("capture started")
        .expect("start signal");
    if abort_task {
        task.abort();
        assert!(task.await.expect_err("task aborted").is_cancelled());
    } else {
        discard_tx.send(()).expect("discard future");
        assert!(
            tokio::time::timeout(Duration::from_secs(3), task)
                .await
                .expect("task ends")
                .expect("task succeeds")
                .is_none()
        );
    }
    assert!(
        !adapter.completed.load(Ordering::Acquire),
        "blocking capture is still running"
    );
    let spans = exporter.get_finished_spans().expect("finished spans");
    assert_eq!(spans.len(), 1);
    let span = &spans[0];
    assert_eq!(span.name, "sqlite.replication.rust.flush");
    assert_eq!(span.parent_span_id, parent_id.span_id());
    assert_eq!(span.span_context.trace_id(), parent_id.trace_id());
    assert_eq!(span.status, opentelemetry::trace::Status::Unset);
    assert!(
        span.attributes.is_empty(),
        "no outcome is recorded when future is dropped"
    );
    assert!(span.events.is_empty());
    assert_eq!(*adapter.admission.lock().expect("admission lock"), [false]);
    let (lock, wake) = &*release;
    *lock.lock().expect("release lock") = true;
    wake.notify_all();
    tokio::time::timeout(Duration::from_secs(3), finished_rx)
        .await
        .expect("capture finishes after release")
        .expect("finish signal");
    assert!(adapter.completed.load(Ordering::Acquire));
    parent.span().end();
    provider.shutdown().expect("shut down provider");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropping_flush_future_ends_span_before_blocking_capture_finishes() {
    check_dropped_flush(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn aborting_flush_task_ends_span_before_blocking_capture_finishes() {
    check_dropped_flush(true).await;
}
