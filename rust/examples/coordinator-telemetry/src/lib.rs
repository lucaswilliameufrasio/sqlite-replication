//! In-process telemetry integration for the coordinator; no server or exporter.

use std::time::Duration;

use opentelemetry::{
    Context,
    trace::{Span, Status, Tracer},
};
use opentelemetry::{KeyValue, metrics::Meter};
use prometheus::{HistogramOpts, HistogramVec, Registry};
use sqlite_replication_core::{CoordinatorMetrics, FlushOutcome};

/// Demonstrates two backends; applications generally select one to avoid
/// counting the same operations twice in a shared downstream system.
pub struct FlushMetrics {
    prometheus: HistogramVec,
    otel: opentelemetry::metrics::Histogram<f64>,
}

impl FlushMetrics {
    pub fn new(registry: &Registry, meter: &Meter) -> Result<Self, prometheus::Error> {
        let prometheus = HistogramVec::new(
            HistogramOpts::new(
                "sqlite_replication_rust_flush_duration_seconds",
                "Duration of completed Rust flush futures, including lock wait.",
            ),
            &["outcome"],
        )?;
        registry.register(Box::new(prometheus.clone()))?;
        let otel = meter
            .f64_histogram("sqlite.replication.rust.flush.duration")
            .with_unit("s")
            .with_description("Duration of completed Rust flush futures, including lock wait.")
            .build();
        Ok(Self { prometheus, otel })
    }
}

impl CoordinatorMetrics for FlushMetrics {
    fn observe_flush(&self, duration: Duration, outcome: FlushOutcome) {
        let label = match outcome {
            FlushOutcome::ReturnedOk => "returned_ok",
            FlushOutcome::ReturnedError => "returned_error",
        };
        let seconds = duration.as_secs_f64();
        self.prometheus.with_label_values(&[label]).observe(seconds);
        self.otel
            .record(seconds, &[KeyValue::new("outcome", label)]);
    }
}

/// Traces an explicit flush as a child of `parent`, without recording error
/// details. The coordinator API does not propagate this context to its adapters.
pub async fn traced_flush<T: Tracer>(
    coordinator: &sqlite_replication_core::Coordinator,
    tracer: &T,
    parent: &Context,
) -> std::io::Result<()> {
    let mut span = tracer.start_with_context("sqlite.replication.rust.flush", parent);
    let result = coordinator.flush().await;
    if result.is_ok() {
        span.set_attribute(KeyValue::new("outcome", "returned_ok"));
    } else {
        span.set_attribute(KeyValue::new("outcome", "returned_error"));
        span.set_status(Status::error("flush failed"));
    }
    span.end();
    result
}
