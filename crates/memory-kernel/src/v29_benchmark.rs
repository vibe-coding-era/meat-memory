use crate::{Kernel, RememberTextRequest, SearchContextRequest};
use anyhow::{Result, bail};
use async_trait::async_trait;
use memory_domain::{
    BenchmarkCaseResult, BenchmarkMetrics, BenchmarkRun, BenchmarkRunId, BenchmarkRunStatus,
    BenchmarkSuite, BenchmarkSuiteId, Memory, MemoryKind, ScopeId, Sensitivity,
};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchmarkSuiteKind {
    MeatCodeZh,
}

impl BenchmarkSuiteKind {
    pub fn from_name(raw: &str) -> Result<Self> {
        match raw {
            "meat-code-zh" => Ok(Self::MeatCodeZh),
            _ => bail!("unsupported benchmark suite: {raw}"),
        }
    }

    pub fn as_name(self) -> &'static str {
        match self {
            Self::MeatCodeZh => "meat-code-zh",
        }
    }

    fn suite(self) -> BenchmarkSuite {
        match self {
            Self::MeatCodeZh => BenchmarkSuite {
                id: BenchmarkSuiteId::from_string("bms_meat_code_zh"),
                name: self.as_name().to_string(),
                version: "2.91.0".to_string(),
                description: "Meat Memory 自有中文代码项目记忆基线".to_string(),
                metric_profile: "recall@1,recall@5,latency,leakage,failure_reason".to_string(),
            },
        }
    }

    fn cases(self) -> &'static [BenchmarkFixtureCase] {
        match self {
            Self::MeatCodeZh => &MEAT_CODE_ZH_CASES,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BenchmarkRunRequest {
    pub suite: BenchmarkSuiteKind,
    pub scope_id: ScopeId,
    pub report_dir: PathBuf,
}

impl BenchmarkRunRequest {
    pub fn new(
        suite: BenchmarkSuiteKind,
        scope_id: ScopeId,
        report_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            suite,
            scope_id,
            report_dir: report_dir.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BenchmarkReportPaths {
    pub summary: PathBuf,
    pub metrics: PathBuf,
    pub failures: PathBuf,
    pub latency: PathBuf,
    pub leakage: PathBuf,
}

#[derive(Debug, Clone)]
pub struct BenchmarkRunOutput {
    pub suite: BenchmarkSuite,
    pub run: BenchmarkRun,
    pub cases: Vec<BenchmarkCaseResult>,
    pub report_paths: BenchmarkReportPaths,
}

#[async_trait]
pub trait BenchmarkRunner {
    async fn run_benchmark_suite(&self, request: BenchmarkRunRequest)
    -> Result<BenchmarkRunOutput>;
}

#[async_trait]
impl BenchmarkRunner for Kernel {
    async fn run_benchmark_suite(
        &self,
        request: BenchmarkRunRequest,
    ) -> Result<BenchmarkRunOutput> {
        self.run_benchmark(request).await
    }
}

impl Kernel {
    pub async fn run_benchmark(&self, request: BenchmarkRunRequest) -> Result<BenchmarkRunOutput> {
        let suite = request.suite.suite();
        let started_at = OffsetDateTime::now_utc();
        let mut case_results = Vec::new();

        for fixture in request.suite.cases() {
            self.ingest_benchmark_case(&request.scope_id, fixture)
                .await?;
            case_results.push(
                self.evaluate_benchmark_case(&request.scope_id, fixture)
                    .await?,
            );
        }

        let metrics = BenchmarkMetrics::from_cases(&case_results);
        let status = if metrics.failure_count == 0 && metrics.leakage_count == 0 {
            BenchmarkRunStatus::Passed
        } else {
            BenchmarkRunStatus::Failed
        };
        let summary_path = request.report_dir.join("summary.md");
        let run = BenchmarkRun {
            id: BenchmarkRunId::new(),
            suite_id: suite.id.clone(),
            suite_name: suite.name.clone(),
            status,
            started_at,
            finished_at: Some(OffsetDateTime::now_utc()),
            case_count: case_results.len(),
            metrics,
            report_path: Some(summary_path.display().to_string()),
        };
        let report_paths =
            write_benchmark_report(&request.report_dir, &suite, &run, &case_results)?;

        Ok(BenchmarkRunOutput {
            suite,
            run,
            cases: case_results,
            report_paths,
        })
    }

    async fn ingest_benchmark_case(
        &self,
        scope_id: &ScopeId,
        fixture: &BenchmarkFixtureCase,
    ) -> Result<()> {
        let mut request = RememberTextRequest::new(scope_id.clone(), fixture.body);
        request.title = Some(fixture.title.to_string());
        request.memory_kind = Some(fixture.memory_kind);
        request.source_refs = vec![format!("benchmark://meat-code-zh/{}", fixture.case_id)];
        request.sensitivity = Sensitivity::Internal;
        self.remember_text(request).await?;
        Ok(())
    }

    async fn evaluate_benchmark_case(
        &self,
        scope_id: &ScopeId,
        fixture: &BenchmarkFixtureCase,
    ) -> Result<BenchmarkCaseResult> {
        let started_at = Instant::now();
        let mut request = SearchContextRequest::new(scope_id.clone(), fixture.query);
        request.limit = 5;
        let bundle = self.search_context(request).await?;
        let latency_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        let actual_memory_ids = bundle
            .memories
            .iter()
            .map(|memory| memory.id.clone())
            .collect::<Vec<_>>();
        let actual_memory_titles = bundle
            .memories
            .iter()
            .map(|memory| memory.title.clone())
            .collect::<Vec<_>>();
        let recall_at_1 = actual_memory_titles
            .first()
            .is_some_and(|title| title == fixture.title);
        let recall_at_5 = actual_memory_titles
            .iter()
            .any(|title| title == fixture.title);
        let leakage = contains_restricted_memory(&bundle.memories);
        let failure_reason = benchmark_failure_reason(fixture.title, recall_at_5, leakage);

        Ok(BenchmarkCaseResult {
            case_id: fixture.case_id.to_string(),
            query: fixture.query.to_string(),
            expected_memory_titles: vec![fixture.title.to_string()],
            actual_memory_ids,
            actual_memory_titles,
            recall_at_1,
            recall_at_5,
            latency_ms,
            leakage,
            failure_reason,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct BenchmarkFixtureCase {
    case_id: &'static str,
    title: &'static str,
    body: &'static str,
    query: &'static str,
    memory_kind: MemoryKind,
}

static MEAT_CODE_ZH_CASES: [BenchmarkFixtureCase; 4] = [
    BenchmarkFixtureCase {
        case_id: "compat-legacy-entrypoints",
        title: "V2.91 兼容契约 remember search fetch_context",
        body: "V2.91 兼容契约要求 remember search fetch_context 旧入口保持参数语义、返回字段和默认行为稳定。",
        query: "V2.91 兼容契约 remember search fetch_context",
        memory_kind: MemoryKind::Constraint,
    },
    BenchmarkFixtureCase {
        case_id: "review-risk-first",
        title: "评审输出 风险优先 摘要靠后",
        body: "代码评审输出必须风险优先，先列阻塞问题、行为回归和缺失测试，再给摘要靠后。",
        query: "评审输出 风险优先 摘要靠后",
        memory_kind: MemoryKind::Preference,
    },
    BenchmarkFixtureCase {
        case_id: "secret-guard-baseline",
        title: "Secret Guard token API key 默认脱敏",
        body: "Secret Guard 基线要求 token API key 默认脱敏，进入拒绝、降敏或人工确认路径，不能作为普通长期记忆扩散。",
        query: "Secret Guard token API key 默认脱敏",
        memory_kind: MemoryKind::Risk,
    },
    BenchmarkFixtureCase {
        case_id: "benchmark-report-contract",
        title: "Benchmark 报告 recall@1 recall@5 latency leakage failure reason",
        body: "Benchmark 报告契约必须包含 recall@1 recall@5 latency leakage failure reason，便于比较 Supermemory mem0 MemoryLake。",
        query: "Benchmark 报告 recall@1 recall@5 latency leakage failure reason",
        memory_kind: MemoryKind::Procedure,
    },
];

fn contains_restricted_memory(memories: &[Memory]) -> bool {
    memories
        .iter()
        .any(|memory| memory.sensitivity == Sensitivity::Restricted)
}

fn benchmark_failure_reason(
    expected_title: &str,
    recall_at_5: bool,
    leakage: bool,
) -> Option<String> {
    if leakage {
        Some("restricted memory leaked into benchmark result".to_string())
    } else if !recall_at_5 {
        Some(format!(
            "expected memory title `{expected_title}` was not found in top 5"
        ))
    } else {
        None
    }
}

fn write_benchmark_report(
    report_dir: &Path,
    suite: &BenchmarkSuite,
    run: &BenchmarkRun,
    cases: &[BenchmarkCaseResult],
) -> Result<BenchmarkReportPaths> {
    fs::create_dir_all(report_dir)?;
    let paths = BenchmarkReportPaths {
        summary: report_dir.join("summary.md"),
        metrics: report_dir.join("metrics.json"),
        failures: report_dir.join("failures.jsonl"),
        latency: report_dir.join("latency.jsonl"),
        leakage: report_dir.join("leakage.jsonl"),
    };

    fs::write(&paths.summary, render_summary(suite, run, cases))?;
    fs::write(
        &paths.metrics,
        serde_json::to_string_pretty(&json!({
            "suite": suite,
            "run": run,
            "metrics": run.metrics,
        }))?,
    )?;
    fs::write(
        &paths.failures,
        render_jsonl(
            cases
                .iter()
                .filter(|case| case.failure_reason.is_some())
                .map(|case| json!(case)),
        )?,
    )?;
    fs::write(
        &paths.latency,
        render_jsonl(cases.iter().map(|case| {
            json!({
                "case_id": case.case_id,
                "latency_ms": case.latency_ms,
            })
        }))?,
    )?;
    fs::write(
        &paths.leakage,
        render_jsonl(cases.iter().filter(|case| case.leakage).map(|case| {
            json!({
                "case_id": case.case_id,
                "actual_memory_titles": case.actual_memory_titles,
            })
        }))?,
    )?;

    Ok(paths)
}

fn render_summary(
    suite: &BenchmarkSuite,
    run: &BenchmarkRun,
    cases: &[BenchmarkCaseResult],
) -> String {
    let mut summary = format!(
        "# Benchmark Summary\n\nsuite: {}\nversion: {}\nrun_id: {}\nstatus: {}\ncase_count: {}\nrecall@1: {:.3}\nrecall@5: {:.3}\np50_latency_ms: {}\np95_latency_ms: {}\nleakage_count: {}\nfailure_count: {}\n\n## Cases\n",
        suite.name,
        suite.version,
        run.id.as_str(),
        run.status.as_str(),
        run.case_count,
        run.metrics.recall_at_1,
        run.metrics.recall_at_5,
        run.metrics.p50_latency_ms,
        run.metrics.p95_latency_ms,
        run.metrics.leakage_count,
        run.metrics.failure_count
    );

    for case in cases {
        summary.push_str(&format!(
            "- {}: recall@1={} recall@5={} latency_ms={} leakage={} failure_reason={}\n",
            case.case_id,
            case.recall_at_1,
            case.recall_at_5,
            case.latency_ms,
            case.leakage,
            case.failure_reason.as_deref().unwrap_or("none")
        ));
    }

    summary
}

fn render_jsonl<I>(items: I) -> Result<String>
where
    I: IntoIterator<Item = serde_json::Value>,
{
    let mut output = String::new();
    for item in items {
        output.push_str(&serde_json::to_string(&item)?);
        output.push('\n');
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{
        BenchmarkReportPaths, BenchmarkRunRequest, BenchmarkRunner, BenchmarkSuiteKind,
        benchmark_failure_reason, contains_restricted_memory, render_summary,
    };
    use crate::{Kernel, RememberTextRequest};
    use memory_domain::{
        BenchmarkMetrics, BenchmarkRun, BenchmarkRunId, BenchmarkRunStatus, BenchmarkSuite,
        BenchmarkSuiteId, MemoryKind, ScopeId, Sensitivity,
    };
    use tempfile::tempdir;
    use time::macros::datetime;

    #[test]
    fn benchmark_suite_kind_accepts_meat_code_zh_only() {
        assert_eq!(
            BenchmarkSuiteKind::from_name("meat-code-zh").unwrap(),
            BenchmarkSuiteKind::MeatCodeZh
        );
        assert!(BenchmarkSuiteKind::from_name("locomo").is_err());
        assert_eq!(BenchmarkSuiteKind::MeatCodeZh.as_name(), "meat-code-zh");
    }

    #[test]
    fn benchmark_failure_reason_prefers_leakage_over_recall_miss() {
        assert_eq!(
            benchmark_failure_reason("expected", false, true).unwrap(),
            "restricted memory leaked into benchmark result"
        );
        assert!(
            benchmark_failure_reason("expected", true, false)
                .as_deref()
                .is_none()
        );
    }

    #[tokio::test]
    async fn benchmark_runner_writes_summary_and_json_reports() {
        let tempdir = tempdir().unwrap();
        let report_dir = tempdir.path().join("reports");
        let kernel = Kernel::builder()
            .with_markdown_root(tempdir.path().join("memory"))
            .unwrap()
            .build()
            .unwrap();
        let output = kernel
            .run_benchmark_suite(BenchmarkRunRequest::new(
                BenchmarkSuiteKind::MeatCodeZh,
                ScopeId::from_string("scp_benchmark_test"),
                &report_dir,
            ))
            .await
            .unwrap();

        assert_eq!(output.suite.name, "meat-code-zh");
        assert_eq!(output.run.status, BenchmarkRunStatus::Passed);
        assert_eq!(output.run.metrics.case_count, 4);
        assert_eq!(output.run.metrics.failure_count, 0);
        assert_eq!(output.run.metrics.leakage_count, 0);
        assert!(output.report_paths.summary.exists());
        assert!(output.report_paths.metrics.exists());
        assert!(output.report_paths.failures.exists());
        assert!(output.report_paths.latency.exists());
        assert!(output.report_paths.leakage.exists());

        let summary = std::fs::read_to_string(output.report_paths.summary).unwrap();
        assert!(summary.contains("recall@1"));
        assert!(summary.contains("recall@5"));
        assert!(summary.contains("failure_reason=none"));
    }

    #[tokio::test]
    async fn contains_restricted_memory_flags_leakage_candidates() {
        let tempdir = tempdir().unwrap();
        let kernel = Kernel::builder()
            .with_markdown_root(tempdir.path())
            .unwrap()
            .build()
            .unwrap();
        let mut request = RememberTextRequest::new(
            ScopeId::from_string("scp_leakage_test"),
            "restricted token must not leak",
        );
        request.title = Some("restricted sample".to_string());
        request.memory_kind = Some(MemoryKind::Risk);
        request.sensitivity = Sensitivity::Restricted;
        let remembered = kernel.remember_text(request).await.unwrap();

        assert!(contains_restricted_memory(&[remembered.memory]));
    }

    #[test]
    fn render_summary_includes_required_metric_names() {
        let suite = BenchmarkSuite {
            id: BenchmarkSuiteId::from_string("bms_test"),
            name: "meat-code-zh".to_string(),
            version: "2.91.0".to_string(),
            description: "test".to_string(),
            metric_profile: "recall".to_string(),
        };
        let run = BenchmarkRun {
            id: BenchmarkRunId::from_string("bmr_test"),
            suite_id: suite.id.clone(),
            suite_name: suite.name.clone(),
            status: BenchmarkRunStatus::Passed,
            started_at: datetime!(2026-04-27 00:00:00 UTC),
            finished_at: Some(datetime!(2026-04-27 00:00:01 UTC)),
            case_count: 0,
            metrics: BenchmarkMetrics::default(),
            report_path: None,
        };
        let summary = render_summary(&suite, &run, &[]);

        assert!(summary.contains("latency"));
        assert!(summary.contains("leakage_count"));
        assert!(summary.contains("failure_count"));
    }

    #[test]
    fn report_paths_are_stable() {
        let paths = BenchmarkReportPaths {
            summary: "summary.md".into(),
            metrics: "metrics.json".into(),
            failures: "failures.jsonl".into(),
            latency: "latency.jsonl".into(),
            leakage: "leakage.jsonl".into(),
        };

        assert_eq!(paths.summary, std::path::PathBuf::from("summary.md"));
        assert_eq!(paths.metrics, std::path::PathBuf::from("metrics.json"));
        assert_eq!(paths.failures, std::path::PathBuf::from("failures.jsonl"));
        assert_eq!(paths.latency, std::path::PathBuf::from("latency.jsonl"));
        assert_eq!(paths.leakage, std::path::PathBuf::from("leakage.jsonl"));
    }
}
