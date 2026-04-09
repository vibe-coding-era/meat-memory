use anyhow::Result;
use serde::Serialize;
use std::{
    collections::VecDeque,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tracing::{Span, info_span};
use tracing_subscriber::{EnvFilter, fmt};
use ulid::Ulid;

const LATENCY_WINDOW: usize = 2048;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LatencySnapshot {
    pub sample_count: usize,
    pub avg_ms: f64,
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub max_ms: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SearchMetricsSnapshot {
    pub total_queries: u64,
    pub successful_queries: u64,
    pub failed_queries: u64,
    pub hit_queries: u64,
    pub empty_queries: u64,
    pub hit_rate: f64,
    pub latency: LatencySnapshot,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WriteMetricsSnapshot {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub pg_writes: u64,
    pub markdown_writes: u64,
    pub latency: LatencySnapshot,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MetricsSnapshot {
    pub search: SearchMetricsSnapshot,
    pub write: WriteMetricsSnapshot,
}

#[derive(Debug)]
struct ObservabilityRegistry {
    search_total: AtomicU64,
    search_success: AtomicU64,
    search_failed: AtomicU64,
    search_hits: AtomicU64,
    search_empty: AtomicU64,
    write_total: AtomicU64,
    write_success: AtomicU64,
    write_failed: AtomicU64,
    write_pg: AtomicU64,
    write_markdown: AtomicU64,
    search_latency: Mutex<LatencyReservoir>,
    write_latency: Mutex<LatencyReservoir>,
}

impl Default for ObservabilityRegistry {
    fn default() -> Self {
        Self {
            search_total: AtomicU64::new(0),
            search_success: AtomicU64::new(0),
            search_failed: AtomicU64::new(0),
            search_hits: AtomicU64::new(0),
            search_empty: AtomicU64::new(0),
            write_total: AtomicU64::new(0),
            write_success: AtomicU64::new(0),
            write_failed: AtomicU64::new(0),
            write_pg: AtomicU64::new(0),
            write_markdown: AtomicU64::new(0),
            search_latency: Mutex::new(LatencyReservoir::new(LATENCY_WINDOW)),
            write_latency: Mutex::new(LatencyReservoir::new(LATENCY_WINDOW)),
        }
    }
}

#[derive(Debug)]
struct LatencyReservoir {
    capacity: usize,
    samples: VecDeque<u64>,
}

impl LatencyReservoir {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            samples: VecDeque::with_capacity(capacity),
        }
    }

    fn record(&mut self, duration: Duration) {
        let millis = duration.as_millis().min(u128::from(u64::MAX)) as u64;
        if self.samples.len() == self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back(millis);
    }

    fn snapshot(&self) -> LatencySnapshot {
        if self.samples.is_empty() {
            return LatencySnapshot {
                sample_count: 0,
                avg_ms: 0.0,
                p50_ms: 0,
                p95_ms: 0,
                max_ms: 0,
            };
        }

        let mut ordered = self.samples.iter().copied().collect::<Vec<_>>();
        ordered.sort_unstable();
        let total = ordered.iter().sum::<u64>();
        let sample_count = ordered.len();

        LatencySnapshot {
            sample_count,
            avg_ms: total as f64 / sample_count as f64,
            p50_ms: percentile(&ordered, 0.50),
            p95_ms: percentile(&ordered, 0.95),
            max_ms: *ordered.last().unwrap_or(&0),
        }
    }
}

pub fn init(level: &str, format: &str) -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    match format {
        "json" => fmt()
            .json()
            .with_env_filter(filter)
            .with_current_span(true)
            .without_time()
            .try_init()
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        _ => fmt()
            .with_env_filter(filter)
            .without_time()
            .try_init()
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
    }

    Ok(())
}

pub fn operation_span(
    component: &str,
    operation: &str,
    scope_id: Option<&str>,
    task_id: Option<&str>,
) -> Span {
    let trace_id = Ulid::new().to_string();
    let span_id = Ulid::new().to_string();

    info_span!(
        "memory_operation",
        trace_id = %trace_id,
        span_id = %span_id,
        scope_id = %scope_id.unwrap_or("n/a"),
        task_id = %task_id.unwrap_or(operation),
        component = %component,
        operation = %operation
    )
}

pub fn record_search_success(result_count: usize, latency: Duration) {
    let registry = registry();
    registry.search_total.fetch_add(1, Ordering::Relaxed);
    registry.search_success.fetch_add(1, Ordering::Relaxed);
    if result_count > 0 {
        registry.search_hits.fetch_add(1, Ordering::Relaxed);
    } else {
        registry.search_empty.fetch_add(1, Ordering::Relaxed);
    }
    record_latency(&registry.search_latency, latency);
}

pub fn record_search_failure(latency: Duration) {
    let registry = registry();
    registry.search_total.fetch_add(1, Ordering::Relaxed);
    registry.search_failed.fetch_add(1, Ordering::Relaxed);
    record_latency(&registry.search_latency, latency);
}

pub fn record_write_success(wrote_pg: bool, wrote_markdown: bool, latency: Duration) {
    let registry = registry();
    registry.write_total.fetch_add(1, Ordering::Relaxed);
    registry.write_success.fetch_add(1, Ordering::Relaxed);
    if wrote_pg {
        registry.write_pg.fetch_add(1, Ordering::Relaxed);
    }
    if wrote_markdown {
        registry.write_markdown.fetch_add(1, Ordering::Relaxed);
    }
    record_latency(&registry.write_latency, latency);
}

pub fn record_write_failure(latency: Duration) {
    let registry = registry();
    registry.write_total.fetch_add(1, Ordering::Relaxed);
    registry.write_failed.fetch_add(1, Ordering::Relaxed);
    record_latency(&registry.write_latency, latency);
}

pub fn metrics_snapshot() -> MetricsSnapshot {
    let registry = registry();
    let hit_queries = registry.search_hits.load(Ordering::Relaxed);
    let successful_queries = registry.search_success.load(Ordering::Relaxed);

    MetricsSnapshot {
        search: SearchMetricsSnapshot {
            total_queries: registry.search_total.load(Ordering::Relaxed),
            successful_queries,
            failed_queries: registry.search_failed.load(Ordering::Relaxed),
            hit_queries,
            empty_queries: registry.search_empty.load(Ordering::Relaxed),
            hit_rate: rate(hit_queries, successful_queries),
            latency: registry.search_latency.lock().unwrap().snapshot(),
        },
        write: WriteMetricsSnapshot {
            total_requests: registry.write_total.load(Ordering::Relaxed),
            successful_requests: registry.write_success.load(Ordering::Relaxed),
            failed_requests: registry.write_failed.load(Ordering::Relaxed),
            pg_writes: registry.write_pg.load(Ordering::Relaxed),
            markdown_writes: registry.write_markdown.load(Ordering::Relaxed),
            latency: registry.write_latency.lock().unwrap().snapshot(),
        },
    }
}

fn registry() -> &'static ObservabilityRegistry {
    static REGISTRY: OnceLock<ObservabilityRegistry> = OnceLock::new();
    REGISTRY.get_or_init(ObservabilityRegistry::default)
}

fn record_latency(store: &Mutex<LatencyReservoir>, latency: Duration) {
    store.lock().unwrap().record(latency);
}

fn percentile(sorted: &[u64], quantile: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let index = ((sorted.len() - 1) as f64 * quantile).round() as usize;
    sorted[index]
}

fn rate(part: u64, total: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }
    part as f64 / total as f64
}

#[cfg(test)]
mod tests {
    use super::{
        LatencyReservoir, init, metrics_snapshot, operation_span, percentile, rate,
        record_search_failure, record_search_success, record_write_failure, record_write_success,
    };
    use std::time::Duration;

    #[test]
    fn latency_reservoir_computes_percentiles() {
        let mut reservoir = LatencyReservoir::new(8);
        for millis in [10_u64, 20, 30, 40, 50] {
            reservoir.record(Duration::from_millis(millis));
        }

        let snapshot = reservoir.snapshot();
        assert_eq!(snapshot.sample_count, 5);
        assert_eq!(snapshot.p50_ms, 30);
        assert_eq!(snapshot.p95_ms, 50);
        assert_eq!(snapshot.max_ms, 50);
    }

    #[test]
    fn latency_reservoir_discards_oldest_samples_when_full() {
        let mut reservoir = LatencyReservoir::new(2);
        reservoir.record(Duration::from_millis(10));
        reservoir.record(Duration::from_millis(20));
        reservoir.record(Duration::from_millis(30));

        let snapshot = reservoir.snapshot();
        assert_eq!(snapshot.sample_count, 2);
        assert_eq!(snapshot.p50_ms, 30);
        assert_eq!(snapshot.p95_ms, 30);
        assert_eq!(snapshot.max_ms, 30);
    }

    #[test]
    fn percentile_returns_zero_for_empty_input() {
        assert_eq!(percentile(&[], 0.95), 0);
    }

    #[test]
    fn rate_handles_zero_totals() {
        assert_eq!(rate(1, 0), 0.0);
    }

    #[test]
    fn rate_computes_fraction_for_non_zero_totals() {
        assert_eq!(rate(2, 4), 0.5);
    }

    #[test]
    fn operation_span_uses_expected_metadata() {
        let span = operation_span("kernel", "search", None, None);
        let _guard = span.enter();
    }

    #[test]
    fn init_supports_json_then_reports_duplicate_pretty_setup() {
        init("info", "json").expect("first tracing init should succeed");
        assert!(init("info", "pretty").is_err());
    }

    #[test]
    fn record_functions_update_metrics_counters() {
        let before = metrics_snapshot();

        record_search_success(2, Duration::from_millis(8));
        record_search_success(0, Duration::from_millis(16));
        record_search_failure(Duration::from_millis(32));
        record_write_success(true, false, Duration::from_millis(4));
        record_write_success(false, true, Duration::from_millis(12));
        record_write_failure(Duration::from_millis(20));

        let after = metrics_snapshot();

        assert!(
            after
                .search
                .total_queries
                .saturating_sub(before.search.total_queries)
                >= 3
        );
        assert!(
            after
                .search
                .successful_queries
                .saturating_sub(before.search.successful_queries)
                >= 2
        );
        assert!(
            after
                .search
                .failed_queries
                .saturating_sub(before.search.failed_queries)
                >= 1
        );
        assert!(
            after
                .search
                .hit_queries
                .saturating_sub(before.search.hit_queries)
                >= 1
        );
        assert!(
            after
                .search
                .empty_queries
                .saturating_sub(before.search.empty_queries)
                >= 1
        );
        assert!(
            after
                .write
                .total_requests
                .saturating_sub(before.write.total_requests)
                >= 3
        );
        assert!(
            after
                .write
                .successful_requests
                .saturating_sub(before.write.successful_requests)
                >= 2
        );
        assert!(
            after
                .write
                .failed_requests
                .saturating_sub(before.write.failed_requests)
                >= 1
        );
        assert!(after.write.pg_writes.saturating_sub(before.write.pg_writes) >= 1);
        assert!(
            after
                .write
                .markdown_writes
                .saturating_sub(before.write.markdown_writes)
                >= 1
        );
        assert!(after.search.latency.sample_count >= before.search.latency.sample_count + 3);
        assert!(after.write.latency.sample_count >= before.write.latency.sample_count + 3);
    }

    #[test]
    fn metrics_snapshot_is_serializable_shape() {
        let snapshot = metrics_snapshot();
        assert!(snapshot.search.total_queries >= snapshot.search.successful_queries);
        assert!(snapshot.write.total_requests >= snapshot.write.successful_requests);
    }
}
