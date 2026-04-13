use memory_domain::{ScopeId, ScopeType, Visibility};
use memory_kernel::{Kernel, PromoteMemoryRequest, RememberTextRequest};
use std::time::{Duration, Instant};
use tempfile::tempdir;

#[derive(Debug)]
struct PerfSummary {
    label: &'static str,
    count: usize,
    min_ms: u128,
    p50_ms: u128,
    p95_ms: u128,
    max_ms: u128,
    total_ms: u128,
}

impl PerfSummary {
    fn from_durations(label: &'static str, durations: &[Duration]) -> Self {
        let mut millis = durations
            .iter()
            .map(|duration| duration.as_micros() / 1_000)
            .collect::<Vec<_>>();
        millis.sort_unstable();
        let count = millis.len();
        let percentile = |pct: f64| -> u128 {
            let index = ((count.saturating_sub(1)) as f64 * pct).round() as usize;
            millis[index]
        };

        Self {
            label,
            count,
            min_ms: *millis.first().unwrap_or(&0),
            p50_ms: percentile(0.50),
            p95_ms: percentile(0.95),
            max_ms: *millis.last().unwrap_or(&0),
            total_ms: durations.iter().map(|duration| duration.as_millis()).sum(),
        }
    }

    fn throughput_per_sec(&self) -> f64 {
        if self.total_ms == 0 {
            self.count as f64
        } else {
            self.count as f64 / (self.total_ms as f64 / 1_000.0)
        }
    }
}

#[tokio::test]
#[ignore = "run with ./docs/scripts/v2-perf.sh"]
async fn markdown_kernel_smoke_benchmark_reports_latency_percentiles() {
    let tempdir = tempdir().unwrap();
    let kernel = Kernel::builder()
        .with_markdown_root(tempdir.path())
        .unwrap()
        .build()
        .unwrap();

    let source_scope_id = ScopeId::from_string("scp_user_perf");
    let mut remember_durations = Vec::new();
    let mut browse_durations = Vec::new();
    let mut promote_durations = Vec::new();
    let mut memory_ids = Vec::new();

    for index in 0..200 {
        let mut request = RememberTextRequest::new(
            source_scope_id.clone(),
            format!(
                "benchmark memory {index}: service gateway depends on postgres and rollout checklist"
            ),
        );
        request.title = Some(format!("benchmark-memory-{index}"));
        let started_at = Instant::now();
        let remembered = kernel.remember_text(request).await.unwrap();
        remember_durations.push(started_at.elapsed());
        memory_ids.push(remembered.memory.id);
    }

    for _ in 0..40 {
        let started_at = Instant::now();
        let result = kernel
            .browse_memories(Some(source_scope_id.clone()), 50)
            .await
            .unwrap();
        browse_durations.push(started_at.elapsed());
        assert!(result.len() >= 50);
    }

    for memory_id in memory_ids.iter().take(40) {
        let started_at = Instant::now();
        let promoted = kernel
            .promote_memory_by_id(
                source_scope_id.clone(),
                memory_id.clone(),
                PromoteMemoryRequest {
                    source_scope_type: ScopeType::User,
                    target_scope_id: ScopeId::from_string("scp_project_perf"),
                    target_scope_type: ScopeType::Project,
                    target_visibility: Visibility::Project,
                },
            )
            .await
            .unwrap();
        promote_durations.push(started_at.elapsed());
        assert_eq!(promoted.memory.scope_id.as_str(), "scp_project_perf");
    }

    let summaries = [
        PerfSummary::from_durations("remember_text", &remember_durations),
        PerfSummary::from_durations("browse_memories", &browse_durations),
        PerfSummary::from_durations("promote_memory", &promote_durations),
    ];

    for summary in summaries {
        println!(
            "{{\"suite\":\"memory-kernel\",\"op\":\"{}\",\"count\":{},\"min_ms\":{},\"p50_ms\":{},\"p95_ms\":{},\"max_ms\":{},\"throughput_per_sec\":{:.2}}}",
            summary.label,
            summary.count,
            summary.min_ms,
            summary.p50_ms,
            summary.p95_ms,
            summary.max_ms,
            summary.throughput_per_sec()
        );
    }
}
