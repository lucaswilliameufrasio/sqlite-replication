use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use aws_credential_types::Credentials;
use aws_sdk_s3::config::Region;
use sqlite_replication_storage_s3::{ErrorKind, Metrics, S3Store};

#[derive(Default)]
struct Observations(Mutex<Vec<(&'static str, u64, Option<ErrorKind>)>>);

impl Metrics for Observations {
    fn observe_storage(
        &self,
        operation: &'static str,
        _duration: Duration,
        bytes: u64,
        error: Option<ErrorKind>,
    ) {
        self.0
            .lock()
            .expect("metrics lock")
            .push((operation, bytes, error));
    }
}

#[tokio::test]
async fn metrics_report_success_and_forbidden_across_local_http() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local server");
    let endpoint = format!("http://{}", listener.local_addr().expect("local address"));
    let server = thread::spawn(move || {
        for expected_method in ["GET ", "HEAD "] {
            let (mut stream, _) = listener.accept().expect("accept SDK request");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("read timeout");
            let mut request = [0_u8; 4096];
            let count = stream.read(&mut request).expect("read request");
            assert!(request[..count].starts_with(expected_method.as_bytes()));
            let response: &[u8] = if expected_method == "GET " {
                b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nvalue"
            } else {
                b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            };
            stream.write_all(response).expect("write response");
        }
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
                .operation_timeout(Duration::from_secs(5))
                .build(),
        )
        .build();
    let observations = Arc::new(Observations::default());
    let store = S3Store::new(aws_sdk_s3::Client::from_conf(config), "test-bucket")
        .expect("create store")
        .with_metrics(observations.clone());
    let output = store.get("sensitive-key-fixture").await.expect("local GET");
    let body = output.body.collect().await.expect("read body").into_bytes();
    assert_eq!(&body[..], b"value");
    let error = store
        .head("other-sensitive-key-fixture")
        .await
        .expect_err("forbidden HEAD");
    assert_eq!(error.kind, ErrorKind::Permanent);
    let message = error.to_string();
    assert!(!message.contains("test-bucket"));
    assert!(!message.contains("other-sensitive-key-fixture"));
    server.join().expect("server thread");
    assert_eq!(
        *observations.0.lock().expect("metrics lock"),
        [("get", 5, None), ("head", 0, Some(ErrorKind::Permanent))]
    );
}
