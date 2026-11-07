use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use opentelemetry::metrics::MeterProvider;
use opentelemetry::trace::{TraceContextExt, Tracer, TracerProvider};
use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData};
use opentelemetry_sdk::metrics::{InMemoryMetricExporter, PeriodicReader, SdkMeterProvider};
use prometheus::Registry;
use rusqlite::Connection;
use sqlite_replication_coordinator_telemetry_example::{FlushMetrics, traced_flush};
use sqlite_replication_core::{
    CaptureResult, Coordinator, CoordinatorAdapter, CoordinatorConfig, Limits,
};

struct Adapter(AtomicBool);

impl CoordinatorAdapter for Adapter {
    fn observe_txid(&self) -> io::Result<Option<u64>> {
        Ok(self.0.load(Ordering::Acquire).then_some(1))
    }

    fn capture(&self) -> io::Result<CaptureResult> {
        if !self.0.load(Ordering::Acquire) {
            return Err(io::Error::other("injected capture failure"));
        }
        Ok(CaptureResult {
            txid: 1,
            transactions: 0,
            wal_frames: 0,
        })
    }

    fn set_write_admission(&self, _enabled: bool, _limits: Limits) -> io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn coordinator_records_both_backends_without_http() {
    let exporter = InMemoryMetricExporter::default();
    let provider = SdkMeterProvider::builder()
        .with_reader(PeriodicReader::builder(exporter.clone()).build())
        .build();
    let registry = Registry::new();
    let observer = Arc::new(
        FlushMetrics::new(&registry, &provider.meter("sqlite-replication-example"))
            .expect("metrics"),
    );
    let dir = tempfile::tempdir().expect("temporary directory");
    let database = dir.path().join("source.sqlite");
    let repository = dir.path().join("repository");
    let connection = Connection::open(&database).expect("open database");
    connection.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE data(value TEXT); INSERT INTO data VALUES ('example');").expect("create database");
    drop(connection);
    sqlite_replication_core::snapshot(&database, &repository).expect("snapshot");
    let adapter = Arc::new(Adapter(AtomicBool::new(true)));
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
    .expect("coordinator")
    .with_metrics(observer);
    coordinator.flush().await.expect("successful flush");
    adapter.0.store(false, Ordering::Release);
    assert!(coordinator.flush().await.is_err());

    let families = registry.gather();
    assert_eq!(families.len(), 1);
    let mut prom_counts = std::collections::BTreeMap::new();
    for metric in families[0].get_metric() {
        assert_eq!(metric.get_label().len(), 1);
        let label = &metric.get_label()[0];
        assert_eq!(label.name(), "outcome");
        prom_counts.insert(
            label.value().to_owned(),
            metric.get_histogram().get_sample_count(),
        );
    }
    provider.force_flush().expect("collect OTel");
    let records = exporter.get_finished_metrics().expect("read OTel");
    let mut otel_counts = std::collections::BTreeMap::new();
    for resource in &records {
        for scope in resource.scope_metrics() {
            for metric in scope.metrics() {
                assert_eq!(metric.name(), "sqlite.replication.rust.flush.duration");
                if let AggregatedMetrics::F64(MetricData::Histogram(histogram)) = metric.data() {
                    for point in histogram.data_points() {
                        let attributes: Vec<_> = point.attributes().collect();
                        assert_eq!(attributes.len(), 1);
                        assert_eq!(attributes[0].key.as_str(), "outcome");
                        otel_counts.insert(attributes[0].value.to_string(), point.count());
                    }
                } else {
                    panic!("unexpected OpenTelemetry aggregation");
                }
            }
        }
    }
    let expected = std::collections::BTreeMap::from([
        ("returned_ok".to_owned(), 1),
        ("returned_error".to_owned(), 1),
    ]);
    assert_eq!(prom_counts, expected);
    assert_eq!(otel_counts, expected);
    provider.shutdown().expect("shut down provider");
}

#[tokio::test]
async fn traced_flush_records_parent_and_bounded_results() {
    let exporter = opentelemetry_sdk::trace::InMemorySpanExporter::default();
    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_simple_exporter(exporter.clone())
        .build();
    let tracer = provider.tracer("sqlite-replication-example");
    let parent = opentelemetry::Context::new().with_span(tracer.start("synthetic-parent"));
    let parent_id = parent.span().span_context().clone();
    let dir = tempfile::tempdir().expect("temporary directory");
    let database = dir.path().join("source.sqlite");
    let repository = dir.path().join("repository");
    let connection = Connection::open(&database).expect("open database");
    connection.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE data(value TEXT); INSERT INTO data VALUES ('example');").expect("create database");
    drop(connection);
    sqlite_replication_core::snapshot(&database, &repository).expect("snapshot");
    let adapter = Arc::new(Adapter(AtomicBool::new(true)));
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
    traced_flush(&coordinator, &tracer, &parent)
        .await
        .expect("successful flush");
    adapter.0.store(false, Ordering::Release);
    assert!(traced_flush(&coordinator, &tracer, &parent).await.is_err());
    let spans = exporter.get_finished_spans().expect("read finished spans");
    assert_eq!(spans.len(), 2);
    for (span, (outcome, status)) in spans.iter().zip([
        ("returned_ok", opentelemetry::trace::Status::Unset),
        (
            "returned_error",
            opentelemetry::trace::Status::error("flush failed"),
        ),
    ]) {
        assert_eq!(span.name, "sqlite.replication.rust.flush");
        assert_eq!(span.parent_span_id, parent_id.span_id());
        assert_eq!(span.span_context.trace_id(), parent_id.trace_id());
        assert_eq!(span.status, status);
        assert!(span.events.is_empty());
        assert_eq!(
            span.attributes,
            [opentelemetry::KeyValue::new("outcome", outcome)]
        );
    }
    parent.span().end();
    provider.shutdown().expect("shutdown provider");
}
