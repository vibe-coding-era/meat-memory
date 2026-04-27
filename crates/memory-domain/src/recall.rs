use crate::{MemoryId, RecallBudgetPackId, RecallTraceId, ScopeId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecallTrace {
    pub id: RecallTraceId,
    pub scope_id: ScopeId,
    pub query: String,
    pub retrieval_mode: String,
    pub candidate_count: usize,
    pub selected_count: usize,
    pub filtered_count: usize,
    pub budget: RecallBudgetSummary,
    pub retention: RecallTraceRetention,
    pub candidates: Vec<RecallTraceCandidate>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecallTraceRetention {
    SummaryOnly,
    DebugCandidates,
}

impl RecallTraceRetention {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SummaryOnly => "summary_only",
            Self::DebugCandidates => "debug_candidates",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecallTraceCandidate {
    pub memory_id: MemoryId,
    pub title: String,
    pub rank: usize,
    pub score: i64,
    pub selected: bool,
    pub filtered_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecallBudgetSummary {
    pub max_records: usize,
    pub max_chars: usize,
    pub used_chars: usize,
    pub trimmed_items: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecallTraceExplanation {
    pub trace_id: RecallTraceId,
    pub summary: String,
    pub matched_terms: Vec<String>,
    pub filtered_reasons: Vec<String>,
    pub budget_reasons: Vec<String>,
    pub failure: Option<RecallFailureClassification>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecallFailureClassification {
    pub kind: RecallFailureKind,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecallFailureKind {
    NoCandidates,
    AllCandidatesFiltered,
    BudgetTrimmed,
    ExpectedNotSelected,
}

impl RecallFailureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoCandidates => "no_candidates",
            Self::AllCandidatesFiltered => "all_candidates_filtered",
            Self::BudgetTrimmed => "budget_trimmed",
            Self::ExpectedNotSelected => "expected_not_selected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecallBudgetPack {
    pub id: RecallBudgetPackId,
    pub trace_id: RecallTraceId,
    pub max_records: usize,
    pub max_chars: usize,
    pub used_chars: usize,
    pub items: Vec<RecallBudgetItem>,
    pub trimmed_items: Vec<RecallBudgetTrimmedItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecallBudgetItem {
    pub memory_id: MemoryId,
    pub title: String,
    pub chars_used: usize,
    pub render_mode: RecallBudgetRenderMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecallBudgetRenderMode {
    Full,
    Summary,
    Truncated,
}

impl RecallBudgetRenderMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Summary => "summary",
            Self::Truncated => "truncated",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecallBudgetTrimmedItem {
    pub memory_id: MemoryId,
    pub title: String,
    pub reason: String,
}

impl RecallBudgetSummary {
    pub fn from_pack(pack: &RecallBudgetPack) -> Self {
        Self {
            max_records: pack.max_records,
            max_chars: pack.max_chars,
            used_chars: pack.used_chars,
            trimmed_items: pack.trimmed_items.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RecallBudgetItem, RecallBudgetPack, RecallBudgetRenderMode, RecallBudgetSummary,
        RecallBudgetTrimmedItem, RecallFailureKind, RecallTraceRetention,
    };
    use crate::{MemoryId, RecallBudgetPackId, RecallTraceId};

    #[test]
    fn recall_enums_expose_stable_labels() {
        assert_eq!(RecallTraceRetention::SummaryOnly.as_str(), "summary_only");
        assert_eq!(
            RecallTraceRetention::DebugCandidates.as_str(),
            "debug_candidates"
        );
        assert_eq!(RecallFailureKind::NoCandidates.as_str(), "no_candidates");
        assert_eq!(
            RecallFailureKind::AllCandidatesFiltered.as_str(),
            "all_candidates_filtered"
        );
        assert_eq!(RecallFailureKind::BudgetTrimmed.as_str(), "budget_trimmed");
        assert_eq!(
            RecallFailureKind::ExpectedNotSelected.as_str(),
            "expected_not_selected"
        );
        assert_eq!(RecallBudgetRenderMode::Full.as_str(), "full");
        assert_eq!(RecallBudgetRenderMode::Summary.as_str(), "summary");
        assert_eq!(RecallBudgetRenderMode::Truncated.as_str(), "truncated");
    }

    #[test]
    fn recall_budget_summary_is_derived_from_pack() {
        let pack = RecallBudgetPack {
            id: RecallBudgetPackId::from_string("rbp_test"),
            trace_id: RecallTraceId::from_string("rtr_test"),
            max_records: 2,
            max_chars: 100,
            used_chars: 64,
            items: vec![RecallBudgetItem {
                memory_id: MemoryId::from_string("mem_keep"),
                title: "keep".to_string(),
                chars_used: 64,
                render_mode: RecallBudgetRenderMode::Full,
            }],
            trimmed_items: vec![RecallBudgetTrimmedItem {
                memory_id: MemoryId::from_string("mem_trim"),
                title: "trim".to_string(),
                reason: "budget exhausted".to_string(),
            }],
        };

        assert_eq!(
            RecallBudgetSummary::from_pack(&pack),
            RecallBudgetSummary {
                max_records: 2,
                max_chars: 100,
                used_chars: 64,
                trimmed_items: 1,
            }
        );
    }
}
