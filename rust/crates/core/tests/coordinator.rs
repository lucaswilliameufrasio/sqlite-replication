use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::Connection;
use sqlite_replication_core::{
    CaptureResult, ComponentState, Coordinator, CoordinatorAdapter, CoordinatorConfig, Limits,
};

struct TestAdapter {
    observed_txid: Option<u64>,
    admission: Mutex<Vec<bool>>,
    applied_limits: Mutex<Vec<Limits>>,
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
    let (_cancel_sender, mut cancellation) = tokio::sync::watch::channel(false);
    let (_event_sender, mut events) = tokio::sync::mpsc::channel(1);
    drop(_event_sender);
    run_coordinator
        .run(&mut events, &mut cancellation)
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
    let (_cancel_sender, mut cancellation) = tokio::sync::watch::channel(true);
    let (_event_sender, mut events) = tokio::sync::mpsc::channel(1);

    tokio::time::timeout(
        Duration::from_millis(500),
        coordinator.run(&mut events, &mut cancellation),
    )
    .await
    .expect("pre-cancelled worker exits promptly")
    .expect("worker closes admission");
    assert_eq!(*adapter.admission.lock().expect("admission lock"), [false]);
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
    let (cancel_sender, mut cancellation) = tokio::sync::watch::channel(false);
    let (event_sender, mut events) = tokio::sync::mpsc::channel(1);
    let worker = tokio::spawn(async move { coordinator.run(&mut events, &mut cancellation).await });

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
    cancel_sender.send(true).expect("signal cancellation");
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
