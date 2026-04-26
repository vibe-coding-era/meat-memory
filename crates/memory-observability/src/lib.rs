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
    pub key: KeyMetricsSnapshot,
    pub v2_4: V24MetricsSnapshot,
    pub v2_7: V27MetricsSnapshot,
    pub v2_8: V28MetricsSnapshot,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct KeyMetricsSnapshot {
    pub keyed_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub file_mode_operations: u64,
    pub vector_mode_operations: u64,
    pub all_mode_operations: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct V24MetricsSnapshot {
    pub source_operations: u64,
    pub source_failures: u64,
    pub context_operations: u64,
    pub context_failures: u64,
    pub docs_operations: u64,
    pub docs_failures: u64,
    pub docs_imported_documents: u64,
    pub docs_missing_documents: u64,
    pub docs_conflicts: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct V27MetricsSnapshot {
    pub lifecycle_operations: u64,
    pub lifecycle_failures: u64,
    pub forget_operations: u64,
    pub restore_operations: u64,
    pub archive_operations: u64,
    pub supersede_operations: u64,
    pub conflict_operations: u64,
    pub report_operations: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct V28MetricsSnapshot {
    pub proposal_operations: u64,
    pub proposal_failures: u64,
    pub proposal_observations: u64,
    pub proposal_backlog: u64,
    pub proposal_approved: u64,
    pub proposal_rejected: u64,
    pub proposal_applied: u64,
    pub proposal_conflicts: u64,
    pub proposal_approval_rate: f64,
    pub proposal_conflict_rate: f64,
    pub rollback_operations: u64,
    pub rollback_successes: u64,
    pub rollback_failures: u64,
    pub distillation_previews: u64,
    pub distillation_preview_failures: u64,
    pub distillation_preview_candidates: u64,
    pub distillation_preview_hits: u64,
    pub distillation_preview_hit_rate: f64,
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
    key_operations: AtomicU64,
    key_success: AtomicU64,
    key_failed: AtomicU64,
    key_file_mode: AtomicU64,
    key_vector_mode: AtomicU64,
    key_all_mode: AtomicU64,
    source_operations: AtomicU64,
    source_failures: AtomicU64,
    context_operations: AtomicU64,
    context_failures: AtomicU64,
    docs_operations: AtomicU64,
    docs_failures: AtomicU64,
    docs_imported_documents: AtomicU64,
    docs_missing_documents: AtomicU64,
    docs_conflicts: AtomicU64,
    lifecycle_operations: AtomicU64,
    lifecycle_failures: AtomicU64,
    lifecycle_forgets: AtomicU64,
    lifecycle_restores: AtomicU64,
    lifecycle_archives: AtomicU64,
    lifecycle_supersedes: AtomicU64,
    lifecycle_conflicts: AtomicU64,
    lifecycle_reports: AtomicU64,
    v28_proposal_operations: AtomicU64,
    v28_proposal_failures: AtomicU64,
    v28_proposal_observations: AtomicU64,
    v28_proposal_backlog: AtomicU64,
    v28_proposal_approved: AtomicU64,
    v28_proposal_rejected: AtomicU64,
    v28_proposal_applied: AtomicU64,
    v28_proposal_conflicts: AtomicU64,
    v28_rollback_operations: AtomicU64,
    v28_rollback_successes: AtomicU64,
    v28_rollback_failures: AtomicU64,
    v28_distillation_previews: AtomicU64,
    v28_distillation_preview_failures: AtomicU64,
    v28_distillation_preview_candidates: AtomicU64,
    v28_distillation_preview_hits: AtomicU64,
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
            key_operations: AtomicU64::new(0),
            key_success: AtomicU64::new(0),
            key_failed: AtomicU64::new(0),
            key_file_mode: AtomicU64::new(0),
            key_vector_mode: AtomicU64::new(0),
            key_all_mode: AtomicU64::new(0),
            source_operations: AtomicU64::new(0),
            source_failures: AtomicU64::new(0),
            context_operations: AtomicU64::new(0),
            context_failures: AtomicU64::new(0),
            docs_operations: AtomicU64::new(0),
            docs_failures: AtomicU64::new(0),
            docs_imported_documents: AtomicU64::new(0),
            docs_missing_documents: AtomicU64::new(0),
            docs_conflicts: AtomicU64::new(0),
            lifecycle_operations: AtomicU64::new(0),
            lifecycle_failures: AtomicU64::new(0),
            lifecycle_forgets: AtomicU64::new(0),
            lifecycle_restores: AtomicU64::new(0),
            lifecycle_archives: AtomicU64::new(0),
            lifecycle_supersedes: AtomicU64::new(0),
            lifecycle_conflicts: AtomicU64::new(0),
            lifecycle_reports: AtomicU64::new(0),
            v28_proposal_operations: AtomicU64::new(0),
            v28_proposal_failures: AtomicU64::new(0),
            v28_proposal_observations: AtomicU64::new(0),
            v28_proposal_backlog: AtomicU64::new(0),
            v28_proposal_approved: AtomicU64::new(0),
            v28_proposal_rejected: AtomicU64::new(0),
            v28_proposal_applied: AtomicU64::new(0),
            v28_proposal_conflicts: AtomicU64::new(0),
            v28_rollback_operations: AtomicU64::new(0),
            v28_rollback_successes: AtomicU64::new(0),
            v28_rollback_failures: AtomicU64::new(0),
            v28_distillation_previews: AtomicU64::new(0),
            v28_distillation_preview_failures: AtomicU64::new(0),
            v28_distillation_preview_candidates: AtomicU64::new(0),
            v28_distillation_preview_hits: AtomicU64::new(0),
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

pub fn record_key_operation(storage_mode: &str, success: bool) {
    let registry = registry();
    registry.key_operations.fetch_add(1, Ordering::Relaxed);
    if success {
        registry.key_success.fetch_add(1, Ordering::Relaxed);
    } else {
        registry.key_failed.fetch_add(1, Ordering::Relaxed);
    }
    match storage_mode {
        "file" => {
            registry.key_file_mode.fetch_add(1, Ordering::Relaxed);
        }
        "vector" => {
            registry.key_vector_mode.fetch_add(1, Ordering::Relaxed);
        }
        _ => {
            registry.key_all_mode.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub fn record_source_operation(success: bool) {
    let registry = registry();
    registry.source_operations.fetch_add(1, Ordering::Relaxed);
    if !success {
        registry.source_failures.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn record_context_operation(success: bool) {
    let registry = registry();
    registry.context_operations.fetch_add(1, Ordering::Relaxed);
    if !success {
        registry.context_failures.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn record_docs_operation(
    success: bool,
    imported_documents: usize,
    missing_documents: usize,
    conflicts: usize,
) {
    let registry = registry();
    registry.docs_operations.fetch_add(1, Ordering::Relaxed);
    if success {
        registry.docs_imported_documents.fetch_add(
            imported_documents.min(u64::MAX as usize) as u64,
            Ordering::Relaxed,
        );
        registry.docs_missing_documents.fetch_add(
            missing_documents.min(u64::MAX as usize) as u64,
            Ordering::Relaxed,
        );
        registry
            .docs_conflicts
            .fetch_add(conflicts.min(u64::MAX as usize) as u64, Ordering::Relaxed);
    } else {
        registry.docs_failures.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn record_lifecycle_operation(action: &str, success: bool) {
    let registry = registry();
    registry
        .lifecycle_operations
        .fetch_add(1, Ordering::Relaxed);
    if !success {
        registry.lifecycle_failures.fetch_add(1, Ordering::Relaxed);
        return;
    }

    match action {
        "forget" | "forgotten" => {
            registry.lifecycle_forgets.fetch_add(1, Ordering::Relaxed);
        }
        "restore" | "active" => {
            registry.lifecycle_restores.fetch_add(1, Ordering::Relaxed);
        }
        "archive" | "archived" => {
            registry.lifecycle_archives.fetch_add(1, Ordering::Relaxed);
        }
        "supersede" | "deprecated" => {
            registry
                .lifecycle_supersedes
                .fetch_add(1, Ordering::Relaxed);
        }
        "conflict" | "needs_review" => {
            registry.lifecycle_conflicts.fetch_add(1, Ordering::Relaxed);
        }
        "report" => {
            registry.lifecycle_reports.fetch_add(1, Ordering::Relaxed);
        }
        _ => {}
    }
}

pub fn record_v28_proposal_observation(
    proposal_count: usize,
    open_count: usize,
    conflict_count: usize,
) {
    let registry = registry();
    registry
        .v28_proposal_observations
        .fetch_add(to_u64(proposal_count), Ordering::Relaxed);
    registry
        .v28_proposal_backlog
        .store(to_u64(open_count), Ordering::Relaxed);
    registry
        .v28_proposal_conflicts
        .fetch_add(to_u64(conflict_count), Ordering::Relaxed);
}

pub fn record_v28_proposal_decision(action: &str, success: bool) {
    let registry = registry();
    registry
        .v28_proposal_operations
        .fetch_add(1, Ordering::Relaxed);
    if !success {
        registry
            .v28_proposal_failures
            .fetch_add(1, Ordering::Relaxed);
        return;
    }

    match action {
        "approved" | "approve" => {
            registry
                .v28_proposal_approved
                .fetch_add(1, Ordering::Relaxed);
            decrement_atomic_saturating(&registry.v28_proposal_backlog);
        }
        "rejected" | "reject" => {
            registry
                .v28_proposal_rejected
                .fetch_add(1, Ordering::Relaxed);
            decrement_atomic_saturating(&registry.v28_proposal_backlog);
        }
        "applied" | "apply" => {
            registry
                .v28_proposal_applied
                .fetch_add(1, Ordering::Relaxed);
        }
        _ => {}
    }
}

pub fn record_v28_rollback(success: bool) {
    let registry = registry();
    registry
        .v28_rollback_operations
        .fetch_add(1, Ordering::Relaxed);
    if success {
        registry
            .v28_rollback_successes
            .fetch_add(1, Ordering::Relaxed);
    } else {
        registry
            .v28_rollback_failures
            .fetch_add(1, Ordering::Relaxed);
    }
}

pub fn record_v28_distillation_preview(success: bool, candidate_count: usize) {
    let registry = registry();
    registry
        .v28_distillation_previews
        .fetch_add(1, Ordering::Relaxed);
    if !success {
        registry
            .v28_distillation_preview_failures
            .fetch_add(1, Ordering::Relaxed);
        return;
    }

    let candidate_count = to_u64(candidate_count);
    registry
        .v28_distillation_preview_candidates
        .fetch_add(candidate_count, Ordering::Relaxed);
    if candidate_count > 0 {
        registry
            .v28_distillation_preview_hits
            .fetch_add(1, Ordering::Relaxed);
    }
}

pub fn metrics_snapshot() -> MetricsSnapshot {
    let registry = registry();
    let hit_queries = registry.search_hits.load(Ordering::Relaxed);
    let successful_queries = registry.search_success.load(Ordering::Relaxed);
    let proposal_approved = registry.v28_proposal_approved.load(Ordering::Relaxed);
    let proposal_rejected = registry.v28_proposal_rejected.load(Ordering::Relaxed);
    let proposal_observations = registry.v28_proposal_observations.load(Ordering::Relaxed);
    let proposal_conflicts = registry.v28_proposal_conflicts.load(Ordering::Relaxed);
    let distillation_previews = registry.v28_distillation_previews.load(Ordering::Relaxed);
    let distillation_preview_hits = registry
        .v28_distillation_preview_hits
        .load(Ordering::Relaxed);

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
        key: KeyMetricsSnapshot {
            keyed_operations: registry.key_operations.load(Ordering::Relaxed),
            successful_operations: registry.key_success.load(Ordering::Relaxed),
            failed_operations: registry.key_failed.load(Ordering::Relaxed),
            file_mode_operations: registry.key_file_mode.load(Ordering::Relaxed),
            vector_mode_operations: registry.key_vector_mode.load(Ordering::Relaxed),
            all_mode_operations: registry.key_all_mode.load(Ordering::Relaxed),
        },
        v2_4: V24MetricsSnapshot {
            source_operations: registry.source_operations.load(Ordering::Relaxed),
            source_failures: registry.source_failures.load(Ordering::Relaxed),
            context_operations: registry.context_operations.load(Ordering::Relaxed),
            context_failures: registry.context_failures.load(Ordering::Relaxed),
            docs_operations: registry.docs_operations.load(Ordering::Relaxed),
            docs_failures: registry.docs_failures.load(Ordering::Relaxed),
            docs_imported_documents: registry.docs_imported_documents.load(Ordering::Relaxed),
            docs_missing_documents: registry.docs_missing_documents.load(Ordering::Relaxed),
            docs_conflicts: registry.docs_conflicts.load(Ordering::Relaxed),
        },
        v2_7: V27MetricsSnapshot {
            lifecycle_operations: registry.lifecycle_operations.load(Ordering::Relaxed),
            lifecycle_failures: registry.lifecycle_failures.load(Ordering::Relaxed),
            forget_operations: registry.lifecycle_forgets.load(Ordering::Relaxed),
            restore_operations: registry.lifecycle_restores.load(Ordering::Relaxed),
            archive_operations: registry.lifecycle_archives.load(Ordering::Relaxed),
            supersede_operations: registry.lifecycle_supersedes.load(Ordering::Relaxed),
            conflict_operations: registry.lifecycle_conflicts.load(Ordering::Relaxed),
            report_operations: registry.lifecycle_reports.load(Ordering::Relaxed),
        },
        v2_8: V28MetricsSnapshot {
            proposal_operations: registry.v28_proposal_operations.load(Ordering::Relaxed),
            proposal_failures: registry.v28_proposal_failures.load(Ordering::Relaxed),
            proposal_observations,
            proposal_backlog: registry.v28_proposal_backlog.load(Ordering::Relaxed),
            proposal_approved,
            proposal_rejected,
            proposal_applied: registry.v28_proposal_applied.load(Ordering::Relaxed),
            proposal_conflicts,
            proposal_approval_rate: rate(proposal_approved, proposal_approved + proposal_rejected),
            proposal_conflict_rate: rate(proposal_conflicts, proposal_observations),
            rollback_operations: registry.v28_rollback_operations.load(Ordering::Relaxed),
            rollback_successes: registry.v28_rollback_successes.load(Ordering::Relaxed),
            rollback_failures: registry.v28_rollback_failures.load(Ordering::Relaxed),
            distillation_previews,
            distillation_preview_failures: registry
                .v28_distillation_preview_failures
                .load(Ordering::Relaxed),
            distillation_preview_candidates: registry
                .v28_distillation_preview_candidates
                .load(Ordering::Relaxed),
            distillation_preview_hits,
            distillation_preview_hit_rate: rate(distillation_preview_hits, distillation_previews),
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

fn to_u64(value: usize) -> u64 {
    value.min(u64::MAX as usize) as u64
}

fn decrement_atomic_saturating(counter: &AtomicU64) {
    let _ = counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        current.checked_sub(1)
    });
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
        record_context_operation, record_docs_operation, record_key_operation,
        record_lifecycle_operation, record_search_failure, record_search_success,
        record_source_operation, record_v28_distillation_preview, record_v28_proposal_decision,
        record_v28_proposal_observation, record_v28_rollback, record_write_failure,
        record_write_success,
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
        record_key_operation("file", true);
        record_key_operation("vector", false);
        record_key_operation("all", true);
        record_source_operation(true);
        record_context_operation(true);
        record_docs_operation(true, 2, 1, 1);
        record_docs_operation(false, 0, 0, 0);
        record_lifecycle_operation("forgotten", true);
        record_lifecycle_operation("needs_review", true);
        record_lifecycle_operation("report", true);
        record_lifecycle_operation("archive", false);
        record_v28_proposal_observation(5, 3, 2);
        record_v28_proposal_decision("approved", true);
        record_v28_proposal_decision("rejected", true);
        record_v28_proposal_decision("applied", true);
        record_v28_proposal_decision("noop", true);
        record_v28_proposal_decision("approved", false);
        record_v28_rollback(true);
        record_v28_rollback(false);
        record_v28_distillation_preview(true, 2);
        record_v28_distillation_preview(true, 0);
        record_v28_distillation_preview(false, 0);

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
        assert!(
            after
                .key
                .keyed_operations
                .saturating_sub(before.key.keyed_operations)
                >= 3
        );
        assert!(
            after
                .v2_4
                .source_operations
                .saturating_sub(before.v2_4.source_operations)
                >= 1
        );
        assert!(
            after
                .v2_4
                .context_operations
                .saturating_sub(before.v2_4.context_operations)
                >= 1
        );
        assert!(
            after
                .v2_4
                .docs_operations
                .saturating_sub(before.v2_4.docs_operations)
                >= 2
        );
        assert!(
            after
                .v2_4
                .docs_imported_documents
                .saturating_sub(before.v2_4.docs_imported_documents)
                >= 2
        );
        assert!(
            after
                .v2_4
                .docs_missing_documents
                .saturating_sub(before.v2_4.docs_missing_documents)
                >= 1
        );
        assert!(
            after
                .v2_4
                .docs_conflicts
                .saturating_sub(before.v2_4.docs_conflicts)
                >= 1
        );
        assert!(
            after
                .v2_7
                .lifecycle_operations
                .saturating_sub(before.v2_7.lifecycle_operations)
                >= 4
        );
        assert!(
            after
                .v2_7
                .forget_operations
                .saturating_sub(before.v2_7.forget_operations)
                >= 1
        );
        assert!(
            after
                .v2_7
                .conflict_operations
                .saturating_sub(before.v2_7.conflict_operations)
                >= 1
        );
        assert!(
            after
                .v2_7
                .report_operations
                .saturating_sub(before.v2_7.report_operations)
                >= 1
        );
        assert!(
            after
                .v2_7
                .lifecycle_failures
                .saturating_sub(before.v2_7.lifecycle_failures)
                >= 1
        );
        assert!(
            after
                .v2_8
                .proposal_observations
                .saturating_sub(before.v2_8.proposal_observations)
                >= 5
        );
        assert!(after.v2_8.proposal_backlog <= 3);
        assert!(
            after
                .v2_8
                .proposal_conflicts
                .saturating_sub(before.v2_8.proposal_conflicts)
                >= 2
        );
        assert!(
            after
                .v2_8
                .proposal_approved
                .saturating_sub(before.v2_8.proposal_approved)
                >= 1
        );
        assert!(
            after
                .v2_8
                .proposal_rejected
                .saturating_sub(before.v2_8.proposal_rejected)
                >= 1
        );
        assert!(
            after
                .v2_8
                .proposal_applied
                .saturating_sub(before.v2_8.proposal_applied)
                >= 1
        );
        assert!(
            after
                .v2_8
                .proposal_failures
                .saturating_sub(before.v2_8.proposal_failures)
                >= 1
        );
        assert!(after.v2_8.proposal_approval_rate > 0.0);
        assert!(after.v2_8.proposal_conflict_rate > 0.0);
        assert!(
            after
                .v2_8
                .rollback_successes
                .saturating_sub(before.v2_8.rollback_successes)
                >= 1
        );
        assert!(
            after
                .v2_8
                .rollback_failures
                .saturating_sub(before.v2_8.rollback_failures)
                >= 1
        );
        assert!(
            after
                .v2_8
                .distillation_previews
                .saturating_sub(before.v2_8.distillation_previews)
                >= 3
        );
        assert!(
            after
                .v2_8
                .distillation_preview_hits
                .saturating_sub(before.v2_8.distillation_preview_hits)
                >= 1
        );
        assert!(
            after
                .v2_8
                .distillation_preview_candidates
                .saturating_sub(before.v2_8.distillation_preview_candidates)
                >= 2
        );
        assert!(after.v2_8.distillation_preview_hit_rate > 0.0);
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
        assert!(snapshot.v2_8.proposal_observations >= snapshot.v2_8.proposal_conflicts);
        assert!(snapshot.v2_8.distillation_previews >= snapshot.v2_8.distillation_preview_hits);
    }
}
