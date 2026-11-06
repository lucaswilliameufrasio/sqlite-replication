use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use aws_credential_types::Credentials;
use aws_sdk_s3::config::Region;
use rusqlite::Connection;
use sqlite_replication_core::{
    CaptureResult, Coordinator, CoordinatorAdapter, CoordinatorConfig, Limits, RemoteConfig,
};
use sqlite_replication_storage_s3::S3Store;

struct Adapter(Mutex<Vec<bool>>);

impl CoordinatorAdapter for Adapter {
    fn observe_txid(&self) -> io::Result<Option<u64>> {
        Ok(Some(1))
    }

    fn capture(&self) -> io::Result<CaptureResult> {
        Err(io::Error::other("capture is not expected"))
    }

    fn set_write_admission(&self, enabled: bool, _limits: Limits) -> io::Result<()> {
        self.0.lock().expect("admission lock").push(enabled);
        Ok(())
    }
}

#[tokio::test]
async fn cancellation_during_failed_remote_sync_keeps_local_snapshot_and_no_ack() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    let connection = Connection::open(&database).expect("open database");
    connection.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE data (value TEXT); INSERT INTO data VALUES ('durable');").expect("create data");
    drop(connection);
    sqlite_replication_core::snapshot(&database, &repository).expect("create snapshot");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
    let endpoint = format!("http://{}", listener.local_addr().expect("local address"));
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept S3 request");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read timeout");
        let mut request = [0_u8; 4096];
        let read = stream.read(&mut request).expect("read S3 request");
        assert!(read > 0, "S3 request must reach server");
        started_tx.send(()).expect("signal request started");
        release_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("release server response");
        stream
            .write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .expect("write S3 response");
    });
    let config = aws_sdk_s3::Config::builder()
        .behavior_version_latest()
        .region(Region::new("us-east-1"))
        .endpoint_url(endpoint)
        .force_path_style(true)
        .credentials_provider(Credentials::new("test", "test", None, None, "local-test"))
        .retry_config(aws_sdk_s3::config::retry::RetryConfig::disabled())
        .timeout_config(
            aws_sdk_s3::config::timeout::TimeoutConfig::builder()
                .operation_timeout(Duration::from_secs(10))
                .build(),
        )
        .build();
    let store =
        S3Store::new(aws_sdk_s3::Client::from_conf(config), "test-bucket").expect("create store");
    let adapter = Arc::new(Adapter(Mutex::new(Vec::new())));
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
        remote: Some(RemoteConfig {
            store,
            prefix: "test".into(),
            remote_id: "local-test".into(),
        }),
    })
    .expect("create coordinator");
    let (cancel_tx, mut cancellation) = tokio::sync::watch::channel(false);
    let (_events_tx, mut events) = tokio::sync::mpsc::channel(1);
    let mut worker =
        tokio::spawn(async move { coordinator.run_legacy(&mut events, &mut cancellation).await });
    tokio::time::timeout(Duration::from_secs(5), started_rx)
        .await
        .expect("S3 request starts")
        .expect("server signal");
    cancel_tx.send(true).expect("signal cancellation");
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut worker)
            .await
            .is_err(),
        "worker waits for in-flight remote request"
    );
    assert!(
        adapter
            .0
            .lock()
            .expect("admission lock")
            .iter()
            .all(|enabled| !enabled)
    );
    assert_eq!(
        sqlite_replication_spool::read_remote_ack(&repository)
            .expect_err("no acknowledgement before remote response")
            .kind(),
        io::ErrorKind::NotFound
    );
    release_tx.send(()).expect("release response");
    let result = tokio::time::timeout(Duration::from_secs(5), worker)
        .await
        .expect("worker finishes")
        .expect("worker task");
    assert!(result.is_err(), "remote failure must be surfaced");
    server.join().expect("server thread");
    assert!(
        adapter
            .0
            .lock()
            .expect("admission lock")
            .iter()
            .all(|enabled| !enabled)
    );
    assert_eq!(
        sqlite_replication_spool::read_remote_ack(&repository)
            .expect_err("failed sync must not record acknowledgement")
            .kind(),
        io::ErrorKind::NotFound
    );
    let restored = directory.path().join("restored.sqlite");
    sqlite_replication_core::restore_if_missing(&repository, &restored)
        .expect("restore local snapshot");
    let value: String = Connection::open(restored)
        .expect("open restore")
        .query_row("SELECT value FROM data", [], |row| row.get(0))
        .expect("read restored data");
    assert_eq!(value, "durable");
}

fn isolated_floci_address(endpoint: &str) -> io::Result<SocketAddr> {
    let address: SocketAddr = endpoint
        .strip_prefix("http://")
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "expected HTTP loopback endpoint",
            )
        })?
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid endpoint address"))?;
    if address.ip().to_string() != "127.0.0.1" || address.port() == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected 127.0.0.1 and a nonzero port",
        ));
    }
    Ok(address)
}

#[test]
fn floci_test_endpoint_rejects_non_loopback_and_invalid_addresses() {
    assert!(isolated_floci_address("http://127.0.0.1:12345").is_ok());
    for endpoint in [
        "http://127.0.0.1:0",
        "http://localhost:4566",
        "http://192.0.2.1:4566",
        "https://127.0.0.1:4566",
        "http://127.0.0.1:4566/path",
    ] {
        assert!(isolated_floci_address(endpoint).is_err(), "{endpoint}");
    }
}

// Run only against a disposable isolated Floci; failure can leave a bucket behind.
#[tokio::test]
#[ignore = "requires disposable isolated Floci on 127.0.0.1"]
async fn cancellation_during_successful_remote_sync_drains_and_confirms() {
    let endpoint = std::env::var("SQLITE_REPLICATION_FLOCI_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:4566".to_owned());
    let floci_address = isolated_floci_address(&endpoint).expect("isolated Floci endpoint");
    let directory = tempfile::tempdir().expect("temporary directory");
    let database = directory.path().join("source.sqlite");
    let repository = directory.path().join("repository");
    let connection = Connection::open(&database).expect("open database");
    connection.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE data (value TEXT); INSERT INTO data VALUES ('remote durable');").expect("create data");
    drop(connection);
    sqlite_replication_core::snapshot(&database, &repository).expect("create snapshot");

    let direct_config = aws_sdk_s3::Config::builder()
        .behavior_version_latest()
        .region(Region::new("us-east-1"))
        .endpoint_url(endpoint)
        .force_path_style(true)
        .credentials_provider(Credentials::new("test", "test", None, None, "local-test"))
        .retry_config(aws_sdk_s3::config::retry::RetryConfig::disabled())
        .timeout_config(
            aws_sdk_s3::config::timeout::TimeoutConfig::builder()
                .operation_timeout(Duration::from_secs(10))
                .build(),
        )
        .build();
    let direct = aws_sdk_s3::Client::from_conf(direct_config);
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let bucket = format!("sqlite-replication-cancel-{suffix}");
    direct
        .create_bucket()
        .bucket(&bucket)
        .send()
        .await
        .expect("create test bucket");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind proxy");
    listener.set_nonblocking(true).expect("configure proxy");
    let proxy_endpoint = format!("http://{}", listener.local_addr().expect("proxy address"));
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_proxy = Arc::clone(&stop);
    let proxy = thread::spawn(move || {
        let mut first = Some((started_tx, release_rx));
        while !stop_proxy.load(Ordering::Acquire) {
            match listener.accept() {
                Ok((client, _)) => {
                    if let Some((started, release)) = first.take() {
                        started.send(()).expect("signal in-flight connection");
                        release
                            .recv_timeout(Duration::from_secs(5))
                            .expect("release proxy");
                    }
                    thread::spawn(move || {
                        let upstream =
                            TcpStream::connect(floci_address).expect("connect isolated Floci");
                        client
                            .set_read_timeout(Some(Duration::from_secs(10)))
                            .expect("client timeout");
                        upstream
                            .set_read_timeout(Some(Duration::from_secs(10)))
                            .expect("upstream timeout");
                        let mut client_reader = client.try_clone().expect("clone client");
                        let mut upstream_writer = upstream.try_clone().expect("clone upstream");
                        let forward = thread::spawn(move || {
                            let _ = io::copy(&mut client_reader, &mut upstream_writer);
                            let _ = upstream_writer.shutdown(Shutdown::Write);
                        });
                        let mut client_writer = client;
                        let mut upstream_reader = upstream;
                        let _ = io::copy(&mut upstream_reader, &mut client_writer);
                        let _ = client_writer.shutdown(Shutdown::Both);
                        let _ = forward.join();
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5))
                }
                Err(error) => panic!("proxy accept failed: {error}"),
            }
        }
    });
    let proxy_config = aws_sdk_s3::Config::builder()
        .behavior_version_latest()
        .region(Region::new("us-east-1"))
        .endpoint_url(proxy_endpoint)
        .force_path_style(true)
        .credentials_provider(Credentials::new("test", "test", None, None, "local-test"))
        .retry_config(aws_sdk_s3::config::retry::RetryConfig::disabled())
        .timeout_config(
            aws_sdk_s3::config::timeout::TimeoutConfig::builder()
                .operation_timeout(Duration::from_secs(10))
                .build(),
        )
        .build();
    let store =
        S3Store::new(aws_sdk_s3::Client::from_conf(proxy_config), &bucket).expect("create store");
    let adapter = Arc::new(Adapter(Mutex::new(Vec::new())));
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
        remote: Some(RemoteConfig {
            store,
            prefix: "test".into(),
            remote_id: "local-test".into(),
        }),
    })
    .expect("create coordinator");
    let (cancel_tx, mut cancellation) = tokio::sync::watch::channel(false);
    let (_events_tx, mut events) = tokio::sync::mpsc::channel(1);
    let mut worker =
        tokio::spawn(async move { coordinator.run_legacy(&mut events, &mut cancellation).await });
    tokio::time::timeout(Duration::from_secs(5), started_rx)
        .await
        .expect("request starts")
        .expect("proxy signal");
    cancel_tx.send(true).expect("signal cancellation");
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut worker)
            .await
            .is_err()
    );
    assert_eq!(
        sqlite_replication_spool::read_remote_ack(&repository)
            .expect_err("no premature acknowledgement")
            .kind(),
        io::ErrorKind::NotFound
    );
    release_tx.send(()).expect("release proxy");
    tokio::time::timeout(Duration::from_secs(20), worker)
        .await
        .expect("worker finishes")
        .expect("worker task")
        .expect("sync completes");
    stop.store(true, Ordering::Release);
    proxy.join().expect("proxy thread");
    assert!(
        adapter
            .0
            .lock()
            .expect("admission lock")
            .iter()
            .all(|enabled| !enabled)
    );
    let ack =
        sqlite_replication_spool::read_remote_ack(&repository).expect("confirmed acknowledgement");
    assert_eq!(
        (ack.txid, ack.remote_id.as_str(), ack.prefix.as_str()),
        (1, "local-test", "test")
    );
    let direct_store = S3Store::new(direct.clone(), &bucket).expect("direct store");
    let restored = directory.path().join("remote-restored.sqlite");
    sqlite_replication_core::restore_from_s3(
        &direct_store,
        "test",
        &restored,
        sqlite_replication_core::RestoreTarget::Txid(1),
    )
    .await
    .expect("restore from remote");
    let value: String = Connection::open(restored)
        .expect("open restored database")
        .query_row("SELECT value FROM data", [], |row| row.get(0))
        .expect("read restored data");
    assert_eq!(value, "remote durable");
    let objects = direct
        .list_objects_v2()
        .bucket(&bucket)
        .send()
        .await
        .expect("list test objects");
    for object in objects.contents() {
        direct
            .delete_object()
            .bucket(&bucket)
            .key(object.key().expect("object key"))
            .send()
            .await
            .expect("delete test object");
    }
    direct
        .delete_bucket()
        .bucket(&bucket)
        .send()
        .await
        .expect("delete test bucket");
}
