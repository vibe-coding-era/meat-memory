use crate::{BenchmarkRunId, BenchmarkSuiteId, MemoryId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkSuite {
    pub id: BenchmarkSuiteId,
    pub name: String,
    pub version: String,
    pub description: String,
    pub metric_profile: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkRunStatus {
    Running,
    Passed,
    Failed,
}

impl BenchmarkRunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkRun {
    pub id: BenchmarkRunId,
    pub suite_id: BenchmarkSuiteId,
    pub suite_name: String,
    pub status: BenchmarkRunStatus,
    pub started_at: OffsetDateTime,
    pub finished_at: Option<OffsetDateTime>,
    pub case_count: usize,
    pub metrics: BenchmarkMetrics,
    pub report_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkCaseResult {
    pub case_id: String,
    pub query: String,
    pub expected_memory_titles: Vec<String>,
    pub actual_memory_ids: Vec<MemoryId>,
    pub actual_memory_titles: Vec<String>,
    pub recall_at_1: bool,
    pub recall_at_5: bool,
    pub latency_ms: u64,
    pub leakage: bool,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    pub case_count: usize,
    pub recall_at_1: f64,
    pub recall_at_5: f64,
    pub leakage_count: usize,
    pub p50_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub failure_count: usize,
}

impl Default for BenchmarkMetrics {
    fn default() -> Self {
        Self {
            case_count: 0,
            recall_at_1: 0.0,
            recall_at_5: 0.0,
            leakage_count: 0,
            p50_latency_ms: 0,
            p95_latency_ms: 0,
            failure_count: 0,
        }
    }
}

impl BenchmarkMetrics {
    pub fn from_cases(cases: &[BenchmarkCaseResult]) -> Self {
        if cases.is_empty() {
            return Self::default();
        }

        let case_count = cases.len();
        let recall_at_1_hits = cases.iter().filter(|case| case.recall_at_1).count();
        let recall_at_5_hits = cases.iter().filter(|case| case.recall_at_5).count();
        let leakage_count = cases.iter().filter(|case| case.leakage).count();
        let failure_count = cases
            .iter()
            .filter(|case| case.failure_reason.is_some())
            .count();
        let mut latencies = cases.iter().map(|case| case.latency_ms).collect::<Vec<_>>();
        latencies.sort_unstable();

        Self {
            case_count,
            recall_at_1: recall_at_1_hits as f64 / case_count as f64,
            recall_at_5: recall_at_5_hits as f64 / case_count as f64,
            leakage_count,
            p50_latency_ms: percentile(&latencies, 0.50),
            p95_latency_ms: percentile(&latencies, 0.95),
            failure_count,
        }
    }
}

fn percentile(sorted: &[u64], pct: f64) -> u64 {
    let index = ((sorted.len().saturating_sub(1)) as f64 * pct).round() as usize;
    sorted[index]
}

#[cfg(test)]
mod tests {
    use super::{
        BenchmarkCaseResult, BenchmarkMetrics, BenchmarkRunStatus, BenchmarkSuite, percentile,
    };
    use crate::{BenchmarkSuiteId, MemoryId};

    fn case(
        case_id: &str,
        recall_at_1: bool,
        recall_at_5: bool,
        latency_ms: u64,
    ) -> BenchmarkCaseResult {
        BenchmarkCaseResult {
            case_id: case_id.to_string(),
            query: format!("query {case_id}"),
            expected_memory_titles: vec![format!("expected {case_id}")],
            actual_memory_ids: vec![MemoryId::from_string(format!("mem_{case_id}"))],
            actual_memory_titles: vec![format!("expected {case_id}")],
            recall_at_1,
            recall_at_5,
            latency_ms,
            leakage: false,
            failure_reason: (!recall_at_5).then(|| "expected memory not found".to_string()),
        }
    }

    #[test]
    fn benchmark_run_status_serializes_to_stable_labels() {
        assert_eq!(BenchmarkRunStatus::Running.as_str(), "running");
        assert_eq!(BenchmarkRunStatus::Passed.as_str(), "passed");
        assert_eq!(BenchmarkRunStatus::Failed.as_str(), "failed");
    }

    #[test]
    fn benchmark_suite_keeps_metric_profile_metadata() {
        let suite = BenchmarkSuite {
            id: BenchmarkSuiteId::from_string("bms_meat_code_zh"),
            name: "meat-code-zh".to_string(),
            version: "2.91.0".to_string(),
            description: "中文代码项目记忆基线".to_string(),
            metric_profile: "recall-latency-leakage".to_string(),
        };

        assert_eq!(suite.id.as_str(), "bms_meat_code_zh");
        assert_eq!(suite.metric_profile, "recall-latency-leakage");
    }

    #[test]
    fn metrics_from_cases_reports_recall_latency_and_failures() {
        let metrics = BenchmarkMetrics::from_cases(&[
            case("one", true, true, 10),
            case("two", false, true, 20),
            case("three", false, false, 30),
        ]);

        assert_eq!(metrics.case_count, 3);
        assert_eq!(metrics.recall_at_1, 1.0 / 3.0);
        assert_eq!(metrics.recall_at_5, 2.0 / 3.0);
        assert_eq!(metrics.failure_count, 1);
        assert_eq!(metrics.p50_latency_ms, 20);
        assert_eq!(metrics.p95_latency_ms, 30);
    }

    #[test]
    fn metrics_from_empty_cases_uses_zero_baseline() {
        assert_eq!(
            BenchmarkMetrics::from_cases(&[]),
            BenchmarkMetrics::default()
        );
    }

    #[test]
    fn percentile_handles_single_and_multiple_values() {
        assert_eq!(percentile(&[7], 0.95), 7);
        assert_eq!(percentile(&[1, 2, 3, 4, 5], 0.50), 3);
        assert_eq!(percentile(&[1, 2, 3, 4, 5], 0.95), 5);
    }
}
