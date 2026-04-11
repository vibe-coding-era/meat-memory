use memory_sync::{
    FileReplicationEngine, OplogOperation, ReplicationEngine, SyncBatch, SyncCursor,
    SyncObjectKind, append_oplog_entry,
};
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
#[ignore = "run with ./scripts/v2-perf.sh"]
async fn file_replication_engine_smoke_benchmark_reports_latency_percentiles() {
    let tempdir = tempdir().unwrap();
    let path = tempdir.path().join("sync-state.json");
    let engine = FileReplicationEngine::open(&path).unwrap();

    let mut append_durations = Vec::new();
    let mut pull_durations = Vec::new();
    let mut apply_durations = Vec::new();

    for index in 0..250 {
        let started_at = Instant::now();
        append_oplog_entry(
            &engine,
            OplogOperation::CreateObject,
            SyncObjectKind::Memory,
            format!("mem_perf_{index}"),
            "node-local",
            "actor-local",
            0,
            1,
            serde_json::json!({"title": format!("memory-{index}")}),
        )
        .await
        .unwrap();
        append_durations.push(started_at.elapsed());
    }

    for _ in 0..40 {
        let started_at = Instant::now();
        let batch = engine
            .pull(SyncCursor {
                after_op_id: None,
                limit: 50,
            })
            .await
            .unwrap();
        pull_durations.push(started_at.elapsed());
        assert_eq!(batch.entries.len(), 50);
    }

    for index in 0..40 {
        let started_at = Instant::now();
        let result = engine
            .apply(SyncBatch {
                entries: vec![memory_sync::OplogEntry::new(
                    OplogOperation::UpdateObject,
                    SyncObjectKind::Memory,
                    format!("mem_perf_{index}"),
                    "node-remote",
                    "actor-remote",
                    1,
                    2,
                    serde_json::json!({"title": format!("memory-{index}-updated")}),
                )],
                next_cursor: None,
            })
            .await
            .unwrap();
        apply_durations.push(started_at.elapsed());
        assert_eq!(result.applied, 1);
    }

    let summaries = [
        PerfSummary::from_durations("append", &append_durations),
        PerfSummary::from_durations("pull", &pull_durations),
        PerfSummary::from_durations("apply", &apply_durations),
    ];

    for summary in summaries {
        println!(
            "{{\"suite\":\"memory-sync\",\"op\":\"{}\",\"count\":{},\"min_ms\":{},\"p50_ms\":{},\"p95_ms\":{},\"max_ms\":{},\"throughput_per_sec\":{:.2}}}",
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
