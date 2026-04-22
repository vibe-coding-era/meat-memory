use memory_domain::{
    AgentContext, DocumentConflictState, Memory, MemoryLayer, MemoryRecord, MemoryRecordConfidence,
    MemoryRecordNativeKind, MemoryRecordSourceKind, MemoryRecordStatus, MemoryRecordType,
    ProjectDocument, RequestContext, Sensitivity,
};
use time::OffsetDateTime;

pub struct LifecycleNormalizer;

impl LifecycleNormalizer {
    pub fn normalize_memory(memory: &Memory) -> MemoryRecord {
        let record_type = RecordClassifier::classify_memory(memory);
        let source_kind = RecordClassifier::classify_memory_source_kind(memory);
        let mut record = MemoryRecord::new(
            MemoryRecordNativeKind::Memory,
            memory.id.as_str(),
            MemoryLayer::LongTerm,
            memory.scope_id.clone(),
            record_type,
            memory.title.clone(),
        );
        record.content = Some(memory.body.clone());
        record.summary = Some(compact_summary(&memory.body));
        record.owner_scope_id = Some(memory.owner_scope_id.clone());
        record.source_kind = source_kind;
        record.source_ref = memory.source_refs.first().cloned();
        record.confidence = MemoryRecordConfidence::Explicit;
        record.status = MemoryRecordStatus::from_memory_state(memory.state);
        record.visibility = memory.visibility;
        record.sensitivity = memory.sensitivity;
        record.importance = memory.scores.importance;
        record.freshness = memory.scores.freshness;
        record.stability = memory.scores.stability;
        record.created_at = memory.created_at;
        record.updated_at = memory.updated_at;
        record
    }

    pub fn normalize_agent_context(agent_context: &AgentContext) -> MemoryRecord {
        let record_type = RecordClassifier::classify_agent_context(agent_context);
        let mut record = MemoryRecord::new(
            MemoryRecordNativeKind::AgentContext,
            agent_context.id.as_str(),
            agent_context.layer,
            agent_context.scope_id.clone(),
            record_type,
            agent_context.title.clone(),
        );
        record.content = Some(agent_context.body.clone());
        record.summary = Some(compact_summary(&agent_context.body));
        record.owner_scope_id = Some(agent_context.scope_id.clone());
        record.source_kind = MemoryRecordSourceKind::Conversation;
        record.source_ref = Some(format!("session://{}", agent_context.session_id));
        record.visibility = memory_domain::Visibility::Private;
        record.sensitivity = Sensitivity::Internal;
        record.expires_at = agent_context.expires_at;
        record.created_at = agent_context.created_at;
        record.updated_at = agent_context.updated_at;
        record
    }

    pub fn normalize_project_document(document: &ProjectDocument) -> MemoryRecord {
        let record_type = RecordClassifier::classify_project_document(document);
        let mut record = MemoryRecord::new(
            MemoryRecordNativeKind::ProjectDocument,
            document.id.as_str(),
            MemoryLayer::MidTerm,
            document.scope_id.clone(),
            record_type,
            document.title.clone(),
        );
        record.content = document
            .local_path
            .clone()
            .or_else(|| Some(document.canonical_uri.clone()));
        record.summary = Some(compact_summary(&format!(
            "{} {}",
            document.title, document.canonical_uri
        )));
        record.owner_scope_id = Some(document.scope_id.clone());
        record.source_kind = MemoryRecordSourceKind::File;
        record.source_ref = Some(document.canonical_uri.clone());
        record.visibility = memory_domain::Visibility::Private;
        record.sensitivity = Sensitivity::Internal;
        record.status = if document.conflict_state == DocumentConflictState::None {
            MemoryRecordStatus::Active
        } else {
            MemoryRecordStatus::NeedsReview
        };
        record.created_at = document.created_at;
        record.updated_at = document.updated_at;
        record
    }
}

pub struct RecordClassifier;

impl RecordClassifier {
    pub fn classify_memory(memory: &Memory) -> MemoryRecordType {
        MemoryRecordType::from_memory_kind(memory.kind)
    }

    pub fn classify_memory_source_kind(memory: &Memory) -> MemoryRecordSourceKind {
        match memory.source_refs.first().map(String::as_str) {
            Some(source_ref) if source_ref.starts_with("agent-context://") => {
                MemoryRecordSourceKind::Conversation
            }
            Some(source_ref)
                if source_ref.starts_with("file://")
                    || source_ref.starts_with('/')
                    || source_ref.starts_with("./") =>
            {
                MemoryRecordSourceKind::File
            }
            Some(source_ref) if source_ref.starts_with("api://") => MemoryRecordSourceKind::Api,
            Some(source_ref) if source_ref.starts_with("meeting://") => {
                MemoryRecordSourceKind::Meeting
            }
            Some(source_ref) if source_ref.starts_with("system://") => {
                MemoryRecordSourceKind::System
            }
            Some(_) => MemoryRecordSourceKind::Legacy,
            None => MemoryRecordSourceKind::Manual,
        }
    }

    pub fn classify_agent_context(agent_context: &AgentContext) -> MemoryRecordType {
        classify_text(
            &agent_context.title,
            Some(&agent_context.body),
            Some(agent_context.task_id.is_some()),
            MemoryRecordType::TaskState,
        )
    }

    pub fn classify_project_document(document: &ProjectDocument) -> MemoryRecordType {
        let source = format!(
            "{} {} {}",
            document.title,
            document.canonical_uri,
            document.local_path.as_deref().unwrap_or_default()
        );
        classify_text(&source, None, None, MemoryRecordType::CodeContext)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSummary {
    pub goal: String,
    pub constraints: Vec<String>,
    pub decisions: Vec<String>,
    pub changes: Vec<String>,
    pub open_questions: Vec<String>,
    pub next_steps: Vec<String>,
    pub evidence: Vec<String>,
}

pub struct TaskSummaryService;

impl TaskSummaryService {
    pub fn summarize_records(goal: impl Into<String>, records: &[MemoryRecord]) -> TaskSummary {
        let goal = goal.into();
        let mut summary = TaskSummary {
            goal,
            constraints: Vec::new(),
            decisions: Vec::new(),
            changes: Vec::new(),
            open_questions: Vec::new(),
            next_steps: Vec::new(),
            evidence: Vec::new(),
        };

        for record in records {
            summary.evidence.push(record.record_id.clone());
            let mut sources = vec![record.title.clone()];
            if let Some(content) = &record.content {
                sources.extend(split_clauses(content));
            }
            for clause in sources {
                let lower = clause.to_lowercase();
                if matches_any(&lower, &["must", "constraint", "限制", "必须", "需要"]) {
                    push_unique(&mut summary.constraints, clause.clone());
                }
                if matches_any(&lower, &["decide", "decision", "决定", "采用", "改用"]) {
                    push_unique(&mut summary.decisions, clause.clone());
                }
                if matches_any(
                    &lower,
                    &[
                        "done",
                        "completed",
                        "implemented",
                        "fixed",
                        "完成",
                        "已完成",
                        "新增",
                        "修复",
                        "changed",
                    ],
                ) {
                    push_unique(&mut summary.changes, clause.clone());
                }
                if clause.contains('?')
                    || clause.contains('？')
                    || matches_any(&lower, &["question", "unclear", "待定", "待确认", "问题"])
                {
                    push_unique(&mut summary.open_questions, clause.clone());
                }
                if matches_any(
                    &lower,
                    &["next", "todo", "follow-up", "下一步", "待办", "后续"],
                ) {
                    push_unique(&mut summary.next_steps, clause);
                }
            }
        }

        summary
    }

    pub fn summarize_agent_contexts(
        goal: impl Into<String>,
        contexts: &[AgentContext],
    ) -> TaskSummary {
        let records = contexts
            .iter()
            .map(LifecycleNormalizer::normalize_agent_context)
            .collect::<Vec<_>>();
        Self::summarize_records(goal, &records)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecallExplanation {
    pub record_id: String,
    pub score: f32,
    pub matched_scope: String,
    pub matched_layer: MemoryLayer,
    pub matched_type: MemoryRecordType,
    pub status: MemoryRecordStatus,
    pub confidence: MemoryRecordConfidence,
    pub reason: String,
}

pub struct RecallExplainer;

impl RecallExplainer {
    pub fn explain(record: &MemoryRecord, query: &str, score: f32) -> RecallExplanation {
        let normalized_query = query.trim().to_lowercase();
        let matched_title =
            !normalized_query.is_empty() && record.title.to_lowercase().contains(&normalized_query);
        let matched_content = record.content.as_deref().is_some_and(|content| {
            !normalized_query.is_empty() && content.to_lowercase().contains(&normalized_query)
        });
        let matched_summary = record.summary.as_deref().is_some_and(|summary| {
            !normalized_query.is_empty() && summary.to_lowercase().contains(&normalized_query)
        });

        let reason = if matched_title {
            format!("query matched title: {}", record.title)
        } else if matched_summary {
            format!("query matched summary for {}", record.record_id)
        } else if matched_content {
            format!("query matched content for {}", record.record_id)
        } else if let Some(source_ref) = &record.source_ref {
            format!("included by ranked recall from source {source_ref}")
        } else {
            "included by ranked recall".to_string()
        };

        RecallExplanation {
            record_id: record.record_id.clone(),
            score,
            matched_scope: record.scope_id.as_str().to_string(),
            matched_layer: record.layer,
            matched_type: record.record_type,
            status: record.status,
            confidence: record.confidence,
            reason,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecallPackBudget {
    pub max_records: usize,
    pub max_chars: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackedRecallRecord {
    pub record_id: String,
    pub rendered: String,
    pub chars_used: usize,
}

pub struct BudgetPacker;

impl BudgetPacker {
    pub fn pack_records(
        records: &[MemoryRecord],
        budget: RecallPackBudget,
    ) -> Vec<PackedRecallRecord> {
        if budget.max_records == 0 || budget.max_chars == 0 {
            return Vec::new();
        }

        let mut packed = Vec::new();
        let mut remaining_chars = budget.max_chars;
        for record in records.iter().take(budget.max_records) {
            let rendered = render_recall_record(record);
            if rendered.chars().count() > remaining_chars {
                if packed.is_empty() {
                    let truncated = truncate_chars(&rendered, remaining_chars);
                    packed.push(PackedRecallRecord {
                        record_id: record.record_id.clone(),
                        chars_used: truncated.chars().count(),
                        rendered: truncated,
                    });
                }
                break;
            }

            remaining_chars -= rendered.chars().count();
            packed.push(PackedRecallRecord {
                record_id: record.record_id.clone(),
                chars_used: rendered.chars().count(),
                rendered,
            });
        }
        packed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvolutionEventType {
    Supersedes,
    ConflictsWith,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionEvent {
    pub event_type: EvolutionEventType,
    pub from_record_id: String,
    pub to_record_id: String,
    pub from_status: MemoryRecordStatus,
    pub to_status: MemoryRecordStatus,
    pub reason: String,
    pub created_at: OffsetDateTime,
}

pub struct EvolutionService;

impl EvolutionService {
    pub fn supersede(
        newer: &MemoryRecord,
        older: &MemoryRecord,
        reason: impl Into<String>,
    ) -> EvolutionEvent {
        EvolutionEvent {
            event_type: EvolutionEventType::Supersedes,
            from_record_id: newer.record_id.clone(),
            to_record_id: older.record_id.clone(),
            from_status: MemoryRecordStatus::Active,
            to_status: MemoryRecordStatus::Deprecated,
            reason: reason.into(),
            created_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn conflict(
        left: &MemoryRecord,
        right: &MemoryRecord,
        reason: impl Into<String>,
    ) -> EvolutionEvent {
        EvolutionEvent {
            event_type: EvolutionEventType::ConflictsWith,
            from_record_id: left.record_id.clone(),
            to_record_id: right.record_id.clone(),
            from_status: MemoryRecordStatus::NeedsReview,
            to_status: MemoryRecordStatus::NeedsReview,
            reason: reason.into(),
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernanceEventType {
    Forgotten,
    Restored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernanceEvent {
    pub event_type: GovernanceEventType,
    pub record_id: String,
    pub previous_status: MemoryRecordStatus,
    pub new_status: MemoryRecordStatus,
    pub reason: String,
    pub created_at: OffsetDateTime,
}

pub struct ForgetService;

impl ForgetService {
    pub fn soft_forget(
        mut record: MemoryRecord,
        reason: impl Into<String>,
    ) -> (MemoryRecord, GovernanceEvent) {
        let previous_status = record.status;
        record.status = MemoryRecordStatus::Forgotten;
        record.updated_at = OffsetDateTime::now_utc();
        let event = GovernanceEvent {
            event_type: GovernanceEventType::Forgotten,
            record_id: record.record_id.clone(),
            previous_status,
            new_status: record.status,
            reason: reason.into(),
            created_at: record.updated_at,
        };
        (record, event)
    }

    pub fn restore(
        mut record: MemoryRecord,
        reason: impl Into<String>,
    ) -> (MemoryRecord, GovernanceEvent) {
        let previous_status = record.status;
        record.status = MemoryRecordStatus::Active;
        record.updated_at = OffsetDateTime::now_utc();
        let event = GovernanceEvent {
            event_type: GovernanceEventType::Restored,
            record_id: record.record_id.clone(),
            previous_status,
            new_status: record.status,
            reason: reason.into(),
            created_at: record.updated_at,
        };
        (record, event)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    pub action: String,
    pub actor: String,
    pub record_id: String,
    pub before_status: Option<MemoryRecordStatus>,
    pub after_status: Option<MemoryRecordStatus>,
    pub reason: Option<String>,
    pub created_at: OffsetDateTime,
}

pub struct AuditLogService;

impl AuditLogService {
    pub fn record(
        action: impl Into<String>,
        actor: impl Into<String>,
        record: &MemoryRecord,
        before_status: Option<MemoryRecordStatus>,
        after_status: Option<MemoryRecordStatus>,
        reason: Option<String>,
    ) -> AuditEvent {
        AuditEvent {
            action: action.into(),
            actor: actor.into(),
            record_id: record.record_id.clone(),
            before_status,
            after_status,
            reason,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

pub struct RecallGuard;

impl RecallGuard {
    pub fn allows_record(
        record: &MemoryRecord,
        context: Option<&RequestContext>,
        now: OffsetDateTime,
    ) -> bool {
        if matches!(
            record.status,
            MemoryRecordStatus::Archived
                | MemoryRecordStatus::Deprecated
                | MemoryRecordStatus::Forgotten
                | MemoryRecordStatus::Deleted
                | MemoryRecordStatus::NeedsReview
        ) {
            return false;
        }
        if matches!(record.sensitivity, Sensitivity::Restricted)
            && !Self::context_can_access_restricted(record, context)
        {
            return false;
        }
        if record
            .expires_at
            .is_some_and(|expires_at| expires_at <= now)
        {
            return false;
        }
        true
    }

    fn context_can_access_restricted(
        record: &MemoryRecord,
        context: Option<&RequestContext>,
    ) -> bool {
        let Some(context) = context else {
            return false;
        };

        record
            .owner_scope_id
            .as_ref()
            .is_some_and(|owner_scope_id| owner_scope_id == &context.owner_scope_id)
            || record.scope_id == context.owner_scope_id
    }

    pub fn filter_memories(memories: Vec<Memory>, context: Option<&RequestContext>) -> Vec<Memory> {
        let now = OffsetDateTime::now_utc();
        memories
            .into_iter()
            .filter(|memory| {
                let record = LifecycleNormalizer::normalize_memory(memory);
                Self::allows_record(&record, context, now)
            })
            .collect()
    }
}

fn classify_text(
    title: &str,
    content: Option<&str>,
    has_task_id: Option<bool>,
    default_type: MemoryRecordType,
) -> MemoryRecordType {
    let combined = match content {
        Some(content) => format!("{title} {content}").to_lowercase(),
        None => title.to_lowercase(),
    };

    if matches_any(&combined, &["summary", "checkpoint", "总结", "摘要"]) {
        return MemoryRecordType::Summary;
    }
    if matches_any(&combined, &["constraint", "限制", "必须"]) {
        return MemoryRecordType::Constraint;
    }
    if matches_any(&combined, &["risk", "风险", "blocker", "阻塞"]) {
        return MemoryRecordType::Risk;
    }
    if matches_any(&combined, &["decision", "决定", "采用", "改用"]) {
        return MemoryRecordType::Decision;
    }
    if matches_any(&combined, &["issue", "bug", "error", "问题", "报错"]) {
        return MemoryRecordType::Issue;
    }
    if matches_any(
        &combined,
        &[
            "runbook",
            "playbook",
            "manual",
            "guide",
            "procedure",
            "操作",
            "手册",
        ],
    ) {
        return MemoryRecordType::Procedure;
    }
    if matches_any(
        &combined,
        &[
            "readme",
            "design",
            "architecture",
            "api",
            "rfc",
            "spec",
            "设计",
            "架构",
        ],
    ) {
        return MemoryRecordType::CodeContext;
    }
    if matches_any(&combined, &["meeting", "minutes", "纪要"]) {
        return MemoryRecordType::MeetingNote;
    }
    if matches_any(&combined, &["guess", "hypothesis", "猜测", "假设"]) {
        return MemoryRecordType::Hypothesis;
    }
    if has_task_id.unwrap_or(false) {
        return MemoryRecordType::TaskState;
    }
    default_type
}

fn matches_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| input.contains(pattern))
}

fn compact_summary(text: &str) -> String {
    const MAX_SUMMARY_CHARS: usize = 160;

    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= MAX_SUMMARY_CHARS {
        return normalized;
    }
    let mut summary = normalized
        .chars()
        .take(MAX_SUMMARY_CHARS.saturating_sub(3))
        .collect::<String>();
    summary.push_str("...");
    summary
}

fn split_clauses(text: &str) -> Vec<String> {
    text.replace(['。', '！', '？'], "\n")
        .replace(['.', '!', '?'], "\n")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn push_unique(target: &mut Vec<String>, value: String) {
    if !target.iter().any(|existing| existing == &value) {
        target.push(value);
    }
}

fn render_recall_record(record: &MemoryRecord) -> String {
    let body = record
        .summary
        .as_deref()
        .or(record.content.as_deref())
        .unwrap_or("");
    format!(
        "[{}][{}][{:?}] {}\n{}",
        record.record_id,
        record.scope_id.as_str(),
        record.record_type,
        record.title,
        body
    )
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }
    let mut truncated = input.chars().take(max_chars - 3).collect::<String>();
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod lifecycle_tests;
