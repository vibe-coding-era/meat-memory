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
        record.source_kind = source_kind;
        record.source_ref = memory.source_ref();
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
        match memory.source_ref().as_deref() {
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

pub struct RecallGuard;

impl RecallGuard {
    pub fn allows_record(
        record: &MemoryRecord,
        _context: Option<&RequestContext>,
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
        if matches!(record.sensitivity, Sensitivity::Restricted) {
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
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.len() <= 160 {
        return normalized;
    }
    format!("{}...", &normalized[..157])
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

trait MemorySourceRef {
    fn source_ref(&self) -> Option<String>;
}

impl MemorySourceRef for Memory {
    fn source_ref(&self) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{LifecycleNormalizer, RecallGuard, RecordClassifier, TaskSummaryService};
    use memory_domain::{
        AgentContext, DocumentConflictState, Memory, MemoryKind, MemoryLayer, MemoryRecordStatus,
        MemoryRecordType, ProjectDocument, ScopeId, Sensitivity, SourceId,
    };
    use time::{Duration, OffsetDateTime};

    #[test]
    fn normalizer_maps_memory_to_long_term_record() {
        let mut memory = Memory::new(
            ScopeId::from_string("scp_v27"),
            MemoryKind::Decision,
            "Lifecycle direction",
            "V2.7 should happen before multimodal.",
        )
        .unwrap();
        memory.activate().unwrap();

        let record = LifecycleNormalizer::normalize_memory(&memory);

        assert_eq!(record.layer, MemoryLayer::LongTerm);
        assert_eq!(record.record_type, MemoryRecordType::Decision);
        assert_eq!(record.status, MemoryRecordStatus::Active);
        assert_eq!(
            record.content.as_deref(),
            Some("V2.7 should happen before multimodal.")
        );
    }

    #[test]
    fn normalizer_maps_agent_context_to_short_term_record() {
        let mut context = AgentContext::new(
            ScopeId::from_string("scp_v27"),
            "session-1",
            "Kernel task",
            "Next step is wiring RecallGuard.",
        )
        .unwrap();
        context.task_id = Some("V2.7-KER-004".to_string());

        let record = LifecycleNormalizer::normalize_agent_context(&context);

        assert_eq!(record.layer, MemoryLayer::ShortTerm);
        assert_eq!(record.record_type, MemoryRecordType::TaskState);
        assert_eq!(record.source_ref.as_deref(), Some("session://session-1"));
    }

    #[test]
    fn normalizer_maps_conflicted_document_to_needs_review_record() {
        let mut document = ProjectDocument::new(
            SourceId::from_string("src_docs"),
            ScopeId::from_string("scp_v27"),
            "file:///tmp/runbook.md",
            "runbook",
            "hash_123",
        )
        .unwrap();
        document.conflict_state = DocumentConflictState::BothChanged;

        let record = LifecycleNormalizer::normalize_project_document(&document);

        assert_eq!(record.layer, MemoryLayer::MidTerm);
        assert_eq!(record.status, MemoryRecordStatus::NeedsReview);
        assert_eq!(record.record_type, MemoryRecordType::Procedure);
    }

    #[test]
    fn classifier_detects_summary_and_code_context_keywords() {
        let context = AgentContext::new(
            ScopeId::from_string("scp_v27"),
            "session-2",
            "Checkpoint summary",
            "Summary for the current task.",
        )
        .unwrap();
        let document = ProjectDocument::new(
            SourceId::from_string("src_docs"),
            ScopeId::from_string("scp_v27"),
            "file:///tmp/architecture.md",
            "Architecture design",
            "hash_456",
        )
        .unwrap();

        assert_eq!(
            RecordClassifier::classify_agent_context(&context),
            MemoryRecordType::Summary
        );
        assert_eq!(
            RecordClassifier::classify_project_document(&document),
            MemoryRecordType::CodeContext
        );
    }

    #[test]
    fn task_summary_service_extracts_structured_fields() {
        let context = AgentContext::new(
            ScopeId::from_string("scp_v27"),
            "session-3",
            "Finish V2.7 P0",
            "Must keep the diff small. We decided to add RecallGuard first. Completed the normalizer. Next step is add tests. Open question: should private memories stay searchable?",
        )
        .unwrap();

        let summary = TaskSummaryService::summarize_agent_contexts("Finish V2.7 P0", &[context]);

        assert_eq!(summary.goal, "Finish V2.7 P0");
        assert!(
            summary
                .constraints
                .iter()
                .any(|item| item.contains("Must keep"))
        );
        assert!(
            summary
                .decisions
                .iter()
                .any(|item| item.contains("decided"))
        );
        assert!(
            summary
                .changes
                .iter()
                .any(|item| item.contains("Completed"))
        );
        assert!(
            summary
                .next_steps
                .iter()
                .any(|item| item.contains("Next step"))
        );
        assert!(
            summary
                .open_questions
                .iter()
                .any(|item| item.contains("Open question"))
        );
    }

    #[test]
    fn recall_guard_excludes_restricted_archived_and_expired_records() {
        let mut memory = Memory::new(
            ScopeId::from_string("scp_v27"),
            MemoryKind::Fact,
            "restricted note",
            "do not surface this",
        )
        .unwrap();
        memory.activate().unwrap();
        memory.sensitivity = Sensitivity::Restricted;

        let restricted = LifecycleNormalizer::normalize_memory(&memory);
        assert!(!RecallGuard::allows_record(
            &restricted,
            None,
            OffsetDateTime::now_utc()
        ));

        let mut context = LifecycleNormalizer::normalize_agent_context(
            &AgentContext::new(
                ScopeId::from_string("scp_v27"),
                "session-4",
                "expired",
                "old context",
            )
            .unwrap(),
        );
        context.expires_at = Some(OffsetDateTime::now_utc() - Duration::hours(1));
        assert!(!RecallGuard::allows_record(
            &context,
            None,
            OffsetDateTime::now_utc()
        ));

        let mut archived = restricted.clone();
        archived.sensitivity = Sensitivity::Internal;
        archived.status = MemoryRecordStatus::Archived;
        assert!(!RecallGuard::allows_record(
            &archived,
            None,
            OffsetDateTime::now_utc()
        ));
    }
}
