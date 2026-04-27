use crate::{
    Kernel, LifecycleNormalizer, RecallGuard, SearchContextRequest, build_context_graph,
    memory_rank_score, merge_memories, rerank_memories,
};
use anyhow::Result;
use memory_domain::{
    ContextBundle, Memory, MemoryKind, MemoryRecordStatus, RecallBudgetItem, RecallBudgetPack,
    RecallBudgetPackId, RecallBudgetRenderMode, RecallBudgetSummary, RecallBudgetTrimmedItem,
    RecallFailureClassification, RecallFailureKind, RecallTrace, RecallTraceCandidate,
    RecallTraceExplanation, RecallTraceId, RecallTraceRetention, RequestContext, ScopeId,
    Sensitivity,
};
use memory_index::{SearchQuery, normalize_query};
use serde_json::json;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecallTraceBudget {
    pub max_records: usize,
    pub max_chars: usize,
}

impl Default for RecallTraceBudget {
    fn default() -> Self {
        Self {
            max_records: 5,
            max_chars: 2_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TraceSearchContextRequest {
    pub search: SearchContextRequest,
    pub budget: RecallTraceBudget,
    pub include_debug_candidates: bool,
    pub expected_titles: Vec<String>,
}

impl TraceSearchContextRequest {
    pub fn new(search: SearchContextRequest) -> Self {
        Self {
            search,
            budget: RecallTraceBudget::default(),
            include_debug_candidates: false,
            expected_titles: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TraceSearchContextResult {
    pub bundle: ContextBundle,
    pub trace: RecallTrace,
    pub explanation: RecallTraceExplanation,
    pub budget_pack: RecallBudgetPack,
}

#[derive(Debug, Clone)]
pub struct RecallTraceReportPaths {
    pub trace: PathBuf,
    pub explanation: PathBuf,
    pub budget: PathBuf,
}

struct RetrievedMemories {
    normalized_query: String,
    retrieval_mode: String,
    ranked_candidates: Vec<Memory>,
}

impl Kernel {
    pub async fn search_context_with_trace(
        &self,
        request: TraceSearchContextRequest,
    ) -> Result<TraceSearchContextResult> {
        let limit = request.search.limit.clamp(1, 50);
        self.ensure_key_can_access_scope(
            request.search.context.as_ref(),
            &request.search.scope_id,
        )?;
        let retrieved = self
            .retrieve_ranked_candidates(
                &request.search.scope_id,
                &request.search.query,
                limit,
                request.search.context.as_ref(),
            )
            .await?;
        let selected = RecallGuard::filter_memories(
            retrieved.ranked_candidates.clone(),
            request.search.context.as_ref(),
        );
        let (entities, relations) = build_context_graph(&request.search.scope_id, &selected);
        let bundle = ContextBundle {
            query: retrieved.normalized_query.clone(),
            scope_id: request.search.scope_id.clone(),
            memories: selected.clone(),
            entities,
            relations,
            generated_at: OffsetDateTime::now_utc(),
        };
        let trace_id = RecallTraceId::new();
        let budget_pack = pack_recall_budget(trace_id.clone(), &selected, request.budget);
        let trace = build_recall_trace(RecallTraceBuildInput {
            trace_id,
            scope_id: &request.search.scope_id,
            normalized_query: &retrieved.normalized_query,
            retrieval_mode: retrieved.retrieval_mode,
            ranked_candidates: &retrieved.ranked_candidates,
            selected: &selected,
            context: request.search.context.as_ref(),
            budget_pack: &budget_pack,
            include_debug_candidates: request.include_debug_candidates,
        });
        let explanation = explain_recall_trace(&trace, &budget_pack, &request.expected_titles);
        memory_observability::record_recall_trace(
            trace.selected_count,
            trace.filtered_count,
            trace.budget.trimmed_items,
        );

        Ok(TraceSearchContextResult {
            bundle,
            trace,
            explanation,
            budget_pack,
        })
    }

    async fn retrieve_ranked_candidates(
        &self,
        scope_id: &ScopeId,
        query: &str,
        limit: usize,
        context: Option<&RequestContext>,
    ) -> Result<RetrievedMemories> {
        let normalized_query = normalize_query(&SearchQuery::new(query, limit));
        let (mut memories, retrieval_mode) = match &self.pg_store {
            Some(pg_store) if self.should_query_pg(context) => {
                let mut keyword_memories =
                    if let Some(context) = context.filter(|context| context.is_fully_isolated) {
                        pg_store
                            .search_by_keyword_for_isolation_group(
                                scope_id,
                                &normalized_query,
                                &context.isolation_group_id,
                                limit as i64,
                            )
                            .await?
                    } else {
                        pg_store
                            .search_by_keyword(scope_id, &normalized_query, limit as i64)
                            .await?
                    };
                if self.should_embed(context) {
                    let vector_memories = self
                        .search_embedding_memories(
                            pg_store,
                            scope_id,
                            &normalized_query,
                            context,
                            limit,
                        )
                        .await?;
                    merge_memories(&mut keyword_memories, vector_memories, limit);
                }
                self.expand_graph_memories(
                    pg_store,
                    scope_id,
                    &normalized_query,
                    context,
                    &mut keyword_memories,
                    limit,
                )
                .await?;
                (keyword_memories, "postgres_keyword_graph".to_string())
            }
            _ => {
                let mut markdown_memories =
                    self.search_markdown_memories(scope_id, &normalized_query, limit)?;
                self.expand_markdown_graph_memories(
                    scope_id,
                    &normalized_query,
                    &mut markdown_memories,
                    limit,
                )?;
                (markdown_memories, "markdown_keyword_graph".to_string())
            }
        };

        rerank_memories(&mut memories, &normalized_query);
        memories.truncate(limit);

        Ok(RetrievedMemories {
            normalized_query,
            retrieval_mode,
            ranked_candidates: memories,
        })
    }
}

struct RecallTraceBuildInput<'a> {
    trace_id: RecallTraceId,
    scope_id: &'a ScopeId,
    normalized_query: &'a str,
    retrieval_mode: String,
    ranked_candidates: &'a [Memory],
    selected: &'a [Memory],
    context: Option<&'a RequestContext>,
    budget_pack: &'a RecallBudgetPack,
    include_debug_candidates: bool,
}

fn build_recall_trace(input: RecallTraceBuildInput<'_>) -> RecallTrace {
    let selected_ids = input
        .selected
        .iter()
        .map(|memory| memory.id.as_str().to_string())
        .collect::<HashSet<_>>();
    let query_terms = query_terms(input.normalized_query);
    let candidates = input
        .ranked_candidates
        .iter()
        .enumerate()
        .map(|(index, memory)| RecallTraceCandidate {
            memory_id: memory.id.clone(),
            title: memory.title.clone(),
            rank: index + 1,
            score: memory_rank_score(memory, &query_terms),
            selected: selected_ids.contains(memory.id.as_str()),
            filtered_reason: filtered_reason(memory, input.context),
        })
        .collect::<Vec<_>>();
    let selected_count = input.selected.len();
    let filtered_count = candidates
        .iter()
        .filter(|candidate| candidate.filtered_reason.is_some())
        .count();

    RecallTrace {
        id: input.trace_id,
        scope_id: input.scope_id.clone(),
        query: input.normalized_query.to_string(),
        retrieval_mode: input.retrieval_mode,
        candidate_count: input.ranked_candidates.len(),
        selected_count,
        filtered_count,
        budget: RecallBudgetSummary::from_pack(input.budget_pack),
        retention: if input.include_debug_candidates {
            RecallTraceRetention::DebugCandidates
        } else {
            RecallTraceRetention::SummaryOnly
        },
        candidates,
        created_at: OffsetDateTime::now_utc(),
    }
}

fn filtered_reason(memory: &Memory, context: Option<&RequestContext>) -> Option<String> {
    let record = LifecycleNormalizer::normalize_memory(memory);
    if matches!(
        record.status,
        MemoryRecordStatus::Archived
            | MemoryRecordStatus::Deprecated
            | MemoryRecordStatus::Forgotten
            | MemoryRecordStatus::Deleted
            | MemoryRecordStatus::NeedsReview
    ) {
        return Some(format!("status filtered: {}", record.status.as_str()));
    }
    if matches!(record.sensitivity, Sensitivity::Restricted)
        && !context_can_access_restricted(memory, context)
    {
        return Some("restricted sensitivity requires owner scope access".to_string());
    }
    None
}

fn context_can_access_restricted(memory: &Memory, context: Option<&RequestContext>) -> bool {
    let Some(context) = context else {
        return false;
    };
    memory.owner_scope_id == context.owner_scope_id || memory.scope_id == context.owner_scope_id
}

fn pack_recall_budget(
    trace_id: RecallTraceId,
    memories: &[Memory],
    budget: RecallTraceBudget,
) -> RecallBudgetPack {
    let mut items = Vec::new();
    let mut trimmed_items = Vec::new();
    let mut used_chars = 0;

    if budget.max_records == 0 || budget.max_chars == 0 {
        for memory in memories {
            trimmed_items.push(trimmed_item(memory, "budget is zero"));
        }
        return RecallBudgetPack {
            id: RecallBudgetPackId::new(),
            trace_id,
            max_records: budget.max_records,
            max_chars: budget.max_chars,
            used_chars,
            items,
            trimmed_items,
        };
    }

    for memory in memories {
        if items.len() >= budget.max_records {
            trimmed_items.push(trimmed_item(memory, "record limit exceeded"));
            continue;
        }

        let rendered_chars = rendered_memory_chars(memory);
        let remaining_chars = budget.max_chars.saturating_sub(used_chars);
        if rendered_chars > remaining_chars {
            if items.is_empty() && remaining_chars > 0 {
                items.push(RecallBudgetItem {
                    memory_id: memory.id.clone(),
                    title: memory.title.clone(),
                    chars_used: remaining_chars,
                    render_mode: RecallBudgetRenderMode::Truncated,
                });
                used_chars += remaining_chars;
            } else {
                trimmed_items.push(trimmed_item(memory, "character budget exhausted"));
            }
            continue;
        }

        used_chars += rendered_chars;
        items.push(RecallBudgetItem {
            memory_id: memory.id.clone(),
            title: memory.title.clone(),
            chars_used: rendered_chars,
            render_mode: if matches!(memory.kind, MemoryKind::Summary) {
                RecallBudgetRenderMode::Summary
            } else {
                RecallBudgetRenderMode::Full
            },
        });
    }

    RecallBudgetPack {
        id: RecallBudgetPackId::new(),
        trace_id,
        max_records: budget.max_records,
        max_chars: budget.max_chars,
        used_chars,
        items,
        trimmed_items,
    }
}

fn rendered_memory_chars(memory: &Memory) -> usize {
    format!("{}\n{}", memory.title, memory.body).chars().count()
}

fn trimmed_item(memory: &Memory, reason: &str) -> RecallBudgetTrimmedItem {
    RecallBudgetTrimmedItem {
        memory_id: memory.id.clone(),
        title: memory.title.clone(),
        reason: reason.to_string(),
    }
}

pub fn explain_recall_trace(
    trace: &RecallTrace,
    budget_pack: &RecallBudgetPack,
    expected_titles: &[String],
) -> RecallTraceExplanation {
    let filtered_reasons = unique_strings(
        trace
            .candidates
            .iter()
            .filter_map(|candidate| candidate.filtered_reason.clone()),
    );
    let budget_reasons = budget_reasons(budget_pack);
    let failure = classify_recall_failure(trace, budget_pack, expected_titles);
    let summary = format!(
        "selected {} of {} candidates; filtered {}; used {}/{} chars",
        trace.selected_count,
        trace.candidate_count,
        trace.filtered_count,
        trace.budget.used_chars,
        trace.budget.max_chars
    );

    RecallTraceExplanation {
        trace_id: trace.id.clone(),
        summary,
        matched_terms: query_terms(&trace.query),
        filtered_reasons,
        budget_reasons,
        failure,
    }
}

pub fn classify_recall_failure(
    trace: &RecallTrace,
    budget_pack: &RecallBudgetPack,
    expected_titles: &[String],
) -> Option<RecallFailureClassification> {
    if trace.candidate_count == 0 {
        return Some(RecallFailureClassification {
            kind: RecallFailureKind::NoCandidates,
            reason: "no candidates matched the query".to_string(),
        });
    }
    if trace.selected_count == 0 && trace.filtered_count > 0 {
        return Some(RecallFailureClassification {
            kind: RecallFailureKind::AllCandidatesFiltered,
            reason: "all matched candidates were filtered by recall guard".to_string(),
        });
    }
    if !expected_titles.is_empty()
        && !trace
            .candidates
            .iter()
            .any(|candidate| candidate.selected && expected_titles.contains(&candidate.title))
    {
        return Some(RecallFailureClassification {
            kind: RecallFailureKind::ExpectedNotSelected,
            reason: "expected title was not selected in recall results".to_string(),
        });
    }
    if !budget_pack.trimmed_items.is_empty() {
        return Some(RecallFailureClassification {
            kind: RecallFailureKind::BudgetTrimmed,
            reason: "one or more selected memories were trimmed by budget".to_string(),
        });
    }
    None
}

pub fn write_recall_trace_report(
    report_dir: &Path,
    result: &TraceSearchContextResult,
) -> Result<RecallTraceReportPaths> {
    fs::create_dir_all(report_dir)?;
    let paths = RecallTraceReportPaths {
        trace: report_dir.join("trace.json"),
        explanation: report_dir.join("explanation.md"),
        budget: report_dir.join("budget.json"),
    };
    fs::write(
        &paths.trace,
        serde_json::to_string_pretty(&json!({
            "trace": result.trace,
            "explanation": result.explanation,
            "budget_pack": result.budget_pack,
            "bundle": result.bundle,
        }))?,
    )?;
    fs::write(&paths.explanation, render_trace_explanation(result))?;
    fs::write(
        &paths.budget,
        serde_json::to_string_pretty(&result.budget_pack)?,
    )?;
    Ok(paths)
}

fn render_trace_explanation(result: &TraceSearchContextResult) -> String {
    let mut output = format!(
        "# Recall Trace Explanation\n\ntrace_id: {}\nquery: {}\nsummary: {}\nretrieval_mode: {}\nretention: {}\n\n## Budget\n\nused_chars: {}\nmax_chars: {}\ntrimmed_items: {}\n\n",
        result.trace.id.as_str(),
        result.trace.query,
        result.explanation.summary,
        result.trace.retrieval_mode,
        result.trace.retention.as_str(),
        result.budget_pack.used_chars,
        result.budget_pack.max_chars,
        result.budget_pack.trimmed_items.len()
    );

    output.push_str("## Candidates\n\n");
    for candidate in &result.trace.candidates {
        output.push_str(&format!(
            "- rank={} selected={} score={} title={} filtered_reason={}\n",
            candidate.rank,
            candidate.selected,
            candidate.score,
            candidate.title,
            candidate.filtered_reason.as_deref().unwrap_or("none")
        ));
    }
    if let Some(failure) = &result.explanation.failure {
        output.push_str(&format!(
            "\n## Failure\n\nkind: {}\nreason: {}\n",
            failure.kind.as_str(),
            failure.reason
        ));
    }
    output
}

fn query_terms(query: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    query
        .split_whitespace()
        .map(|term| term.to_ascii_lowercase())
        .filter(|term| !term.is_empty())
        .filter(|term| seen.insert(term.clone()))
        .collect()
}

fn unique_strings<I>(items: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut seen = HashSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(item.clone()))
        .collect()
}

fn budget_reasons(pack: &RecallBudgetPack) -> Vec<String> {
    let mut reasons = vec![format!(
        "packed {} items within {} chars",
        pack.items.len(),
        pack.max_chars
    )];
    if !pack.trimmed_items.is_empty() {
        reasons.push(format!(
            "trimmed {} items due to budget",
            pack.trimmed_items.len()
        ));
    }
    reasons
}

#[cfg(test)]
mod tests {
    use super::{
        RecallTraceBudget, TraceSearchContextRequest, classify_recall_failure,
        write_recall_trace_report,
    };
    use crate::{Kernel, RememberTextRequest, SearchContextRequest};
    use memory_domain::{MemoryKind, RecallFailureKind, ScopeId, Sensitivity};
    use tempfile::tempdir;

    #[tokio::test]
    async fn traced_search_reports_candidates_filtering_and_budget() {
        let tempdir = tempdir().unwrap();
        let kernel = Kernel::builder()
            .with_markdown_root(tempdir.path())
            .unwrap()
            .build()
            .unwrap();
        let scope_id = ScopeId::from_string("scp_trace_test");

        let mut public = RememberTextRequest::new(
            scope_id.clone(),
            "trace budget public memory should be selected",
        );
        public.title = Some("trace budget public".to_string());
        public.memory_kind = Some(MemoryKind::Fact);
        kernel.remember_text(public).await.unwrap();

        let mut restricted = RememberTextRequest::new(
            scope_id.clone(),
            "trace budget restricted memory should be filtered",
        );
        restricted.title = Some("trace budget restricted".to_string());
        restricted.memory_kind = Some(MemoryKind::Risk);
        restricted.sensitivity = Sensitivity::Restricted;
        kernel.remember_text(restricted).await.unwrap();

        let mut request =
            TraceSearchContextRequest::new(SearchContextRequest::new(scope_id, "trace budget"));
        request.budget = RecallTraceBudget {
            max_records: 1,
            max_chars: 48,
        };
        request.include_debug_candidates = true;
        let result = kernel.search_context_with_trace(request).await.unwrap();

        assert_eq!(result.bundle.memories.len(), 1);
        assert_eq!(result.trace.candidate_count, 2);
        assert_eq!(result.trace.selected_count, 1);
        assert_eq!(result.trace.filtered_count, 1);
        assert!(!result.explanation.filtered_reasons.is_empty());
        assert_eq!(result.budget_pack.items.len(), 1);
    }

    #[tokio::test]
    async fn trace_report_writer_outputs_json_and_markdown() {
        let tempdir = tempdir().unwrap();
        let kernel = Kernel::builder()
            .with_markdown_root(tempdir.path().join("memory"))
            .unwrap()
            .build()
            .unwrap();
        let scope_id = ScopeId::from_string("scp_trace_report");
        let mut remember = RememberTextRequest::new(scope_id.clone(), "trace report body");
        remember.title = Some("trace report".to_string());
        kernel.remember_text(remember).await.unwrap();

        let result = kernel
            .search_context_with_trace(TraceSearchContextRequest::new(SearchContextRequest::new(
                scope_id,
                "trace report",
            )))
            .await
            .unwrap();
        let paths = write_recall_trace_report(&tempdir.path().join("report"), &result).unwrap();

        assert!(paths.trace.exists());
        assert!(paths.explanation.exists());
        assert!(paths.budget.exists());
        let explanation = std::fs::read_to_string(paths.explanation).unwrap();
        assert!(explanation.contains("Recall Trace Explanation"));
        assert!(explanation.contains(result.trace.id.as_str()));
    }

    #[test]
    fn failure_classifier_distinguishes_empty_and_expected_miss() {
        let trace = memory_domain::RecallTrace {
            id: memory_domain::RecallTraceId::from_string("rtr_empty"),
            scope_id: ScopeId::from_string("scp_empty"),
            query: "missing".to_string(),
            retrieval_mode: "markdown".to_string(),
            candidate_count: 0,
            selected_count: 0,
            filtered_count: 0,
            budget: memory_domain::RecallBudgetSummary {
                max_records: 5,
                max_chars: 100,
                used_chars: 0,
                trimmed_items: 0,
            },
            retention: memory_domain::RecallTraceRetention::SummaryOnly,
            candidates: Vec::new(),
            created_at: time::OffsetDateTime::now_utc(),
        };
        let pack = memory_domain::RecallBudgetPack {
            id: memory_domain::RecallBudgetPackId::from_string("rbp_empty"),
            trace_id: trace.id.clone(),
            max_records: 5,
            max_chars: 100,
            used_chars: 0,
            items: Vec::new(),
            trimmed_items: Vec::new(),
        };

        let failure = classify_recall_failure(&trace, &pack, &[]).unwrap();
        assert_eq!(failure.kind, RecallFailureKind::NoCandidates);
    }
}
