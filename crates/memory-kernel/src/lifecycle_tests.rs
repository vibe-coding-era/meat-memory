use super::{
    AuditLogService, BudgetPacker, EvolutionEventType, EvolutionService, ForgetService,
    GovernanceEventType, LifecycleNormalizer, RecallExplainer, RecallGuard, RecallPackBudget,
    RecordClassifier, TaskSummaryService, classify_text, compact_summary, push_unique,
    render_recall_record, split_clauses, truncate_chars,
};
use memory_domain::{
    AccessKeyId, AgentContext, DocumentConflictState, KeyScopeKind, KeySourceKind, Memory,
    MemoryKind, MemoryLayer, MemoryRecordSourceKind, MemoryRecordStatus, MemoryRecordType,
    ProjectDocument, RequestContext, ScopeId, Sensitivity, SourceId, StorageMode,
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
fn normalizer_preserves_memory_source_metadata() {
    let mut memory = Memory::new(
        ScopeId::from_string("scp_v27"),
        MemoryKind::Decision,
        "Lifecycle direction",
        "V2.7 should happen before multimodal.",
    )
    .unwrap();
    memory.source_refs = vec!["agent-context://ctx_123".to_string()];

    let record = LifecycleNormalizer::normalize_memory(&memory);

    assert_eq!(
        record.source_ref.as_deref(),
        Some("agent-context://ctx_123")
    );
    assert_eq!(record.source_kind, MemoryRecordSourceKind::Conversation);
}

#[test]
fn compact_summary_handles_long_chinese_text_without_panic() {
    let memory = Memory::new(
        ScopeId::from_string("scp_v27"),
        MemoryKind::Fact,
        "中文摘要",
        "这是一个很长的中文记忆内容，用来验证摘要截断不会切到 UTF-8 字符边界导致 panic。"
            .repeat(20),
    )
    .unwrap();

    let record = LifecycleNormalizer::normalize_memory(&memory);

    let summary = record.summary.expect("summary");
    assert!(summary.ends_with("..."));
    assert_eq!(summary.chars().count(), 160);
    assert!(summary.is_char_boundary(summary.len()));
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
fn normalizer_maps_clean_document_to_active_record_with_uri_content() {
    let document = ProjectDocument::new(
        SourceId::from_string("src_docs"),
        ScopeId::from_string("scp_v27"),
        "file:///tmp/README.md",
        "README design",
        "hash_clean",
    )
    .unwrap();

    let record = LifecycleNormalizer::normalize_project_document(&document);

    assert_eq!(record.status, MemoryRecordStatus::Active);
    assert_eq!(record.content.as_deref(), Some("file:///tmp/README.md"));
    assert_eq!(record.source_kind, MemoryRecordSourceKind::File);
    assert_eq!(record.record_type, MemoryRecordType::CodeContext);
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
fn classifier_covers_all_text_driven_record_types() {
    let cases = [
        (
            "Constraint",
            "must keep data private",
            MemoryRecordType::Constraint,
        ),
        ("Risk", "blocker may break recall", MemoryRecordType::Risk),
        (
            "Decision",
            "we decided to use lifecycle",
            MemoryRecordType::Decision,
        ),
        ("Issue", "bug in projection", MemoryRecordType::Issue),
        (
            "Procedure",
            "runbook for migration",
            MemoryRecordType::Procedure,
        ),
        (
            "Meeting",
            "minutes from sync",
            MemoryRecordType::MeetingNote,
        ),
        (
            "Hypothesis",
            "hypothesis about ranking",
            MemoryRecordType::Hypothesis,
        ),
    ];

    for (title, body, expected) in cases {
        assert_eq!(
            classify_text(title, Some(body), None, MemoryRecordType::Fact),
            expected
        );
    }
    assert_eq!(
        classify_text("Plain", None, Some(true), MemoryRecordType::Fact),
        MemoryRecordType::TaskState
    );
    assert_eq!(
        classify_text("Plain", None, Some(false), MemoryRecordType::Fact),
        MemoryRecordType::Fact
    );
}

#[test]
fn classifier_covers_memory_source_ref_kinds() {
    let refs = [
        ("file:///tmp/a.md", MemoryRecordSourceKind::File),
        ("/tmp/a.md", MemoryRecordSourceKind::File),
        ("./a.md", MemoryRecordSourceKind::File),
        ("api://memory", MemoryRecordSourceKind::Api),
        ("meeting://daily", MemoryRecordSourceKind::Meeting),
        ("system://seed", MemoryRecordSourceKind::System),
        ("legacy-ref", MemoryRecordSourceKind::Legacy),
    ];

    for (source_ref, expected) in refs {
        let mut memory = Memory::new(
            ScopeId::from_string("scp_v27"),
            MemoryKind::Fact,
            "ref",
            "body",
        )
        .unwrap();
        memory.source_refs = vec![source_ref.to_string()];
        assert_eq!(
            RecordClassifier::classify_memory_source_kind(&memory),
            expected
        );
    }
}

#[test]
fn normalizer_preserves_source_ref_and_kind_for_long_term_records() {
    let cases = [
        (
            "agent-context://ctx_123",
            MemoryRecordSourceKind::Conversation,
        ),
        ("file:///tmp/source.md", MemoryRecordSourceKind::File),
        ("api://import", MemoryRecordSourceKind::Api),
        ("meeting://daily", MemoryRecordSourceKind::Meeting),
        ("system://seed", MemoryRecordSourceKind::System),
        ("legacy-import", MemoryRecordSourceKind::Legacy),
    ];

    for (source_ref, expected_kind) in cases {
        let mut memory = Memory::new(
            ScopeId::from_string("scp_v27"),
            MemoryKind::Fact,
            "source backed",
            "source backed body",
        )
        .unwrap();
        memory.source_refs = vec![source_ref.to_string(), "file:///second.md".to_string()];

        let record = LifecycleNormalizer::normalize_memory(&memory);

        assert_eq!(record.source_ref.as_deref(), Some(source_ref));
        assert_eq!(record.source_kind, expected_kind);
    }

    let memory = Memory::new(
        ScopeId::from_string("scp_v27"),
        MemoryKind::Fact,
        "manual",
        "manual body",
    )
    .unwrap();
    let record = LifecycleNormalizer::normalize_memory(&memory);
    assert_eq!(record.source_ref, None);
    assert_eq!(record.source_kind, MemoryRecordSourceKind::Manual);
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
fn task_summary_helpers_split_and_deduplicate_clauses() {
    let clauses = split_clauses("Must do this. Must do this! 下一步继续？");
    assert_eq!(clauses.len(), 3);

    let mut values = Vec::new();
    push_unique(&mut values, "same".to_string());
    push_unique(&mut values, "same".to_string());
    push_unique(&mut values, "other".to_string());
    assert_eq!(values, vec!["same".to_string(), "other".to_string()]);
}

#[test]
fn recall_explainer_returns_traceable_reason() {
    let memory = Memory::new(
        ScopeId::from_string("scp_v27"),
        MemoryKind::Decision,
        "Lifecycle direction",
        "V2.7 should explain recall.",
    )
    .unwrap();
    let record = LifecycleNormalizer::normalize_memory(&memory);

    let explanation = RecallExplainer::explain(&record, "lifecycle", 0.91);

    assert_eq!(explanation.record_id, record.record_id);
    assert_eq!(explanation.matched_scope, "scp_v27");
    assert_eq!(explanation.score, 0.91);
    assert!(explanation.reason.contains("query matched title"));
}

#[test]
fn recall_explainer_covers_summary_content_source_and_default_reasons() {
    let mut record = LifecycleNormalizer::normalize_memory(
        &Memory::new(
            ScopeId::from_string("scp_v27"),
            MemoryKind::Fact,
            "Unmatched title",
            "content has needle",
        )
        .unwrap(),
    );
    record.summary = Some("summary has marker".to_string());
    assert!(
        RecallExplainer::explain(&record, "marker", 0.5)
            .reason
            .contains("summary")
    );

    record.summary = None;
    assert!(
        RecallExplainer::explain(&record, "needle", 0.5)
            .reason
            .contains("content")
    );

    record.source_ref = Some("file:///tmp/source.md".to_string());
    assert!(
        RecallExplainer::explain(&record, "missing", 0.5)
            .reason
            .contains("source")
    );

    record.source_ref = None;
    assert_eq!(
        RecallExplainer::explain(&record, "missing", 0.5).reason,
        "included by ranked recall"
    );
}

#[test]
fn budget_packer_respects_record_and_character_limits() {
    let first = LifecycleNormalizer::normalize_memory(
        &Memory::new(
            ScopeId::from_string("scp_v27"),
            MemoryKind::Summary,
            "第一条",
            "这是第一条较长的中文摘要内容，应该可以被安全打包。",
        )
        .unwrap(),
    );
    let second = LifecycleNormalizer::normalize_memory(
        &Memory::new(
            ScopeId::from_string("scp_v27"),
            MemoryKind::Summary,
            "第二条",
            "第二条不应该进入结果，因为 max_records 只有一条。",
        )
        .unwrap(),
    );

    let packed = BudgetPacker::pack_records(
        &[first, second],
        RecallPackBudget {
            max_records: 1,
            max_chars: 80,
        },
    );

    assert_eq!(packed.len(), 1);
    assert!(packed[0].chars_used <= 80);
    assert!(packed[0].rendered.contains("..."));
}

#[test]
fn budget_packer_covers_zero_and_fit_paths() {
    let record = LifecycleNormalizer::normalize_memory(
        &Memory::new(
            ScopeId::from_string("scp_v27"),
            MemoryKind::Summary,
            "Short",
            "small body",
        )
        .unwrap(),
    );

    assert!(
        BudgetPacker::pack_records(
            std::slice::from_ref(&record),
            RecallPackBudget {
                max_records: 0,
                max_chars: 100
            }
        )
        .is_empty()
    );
    assert!(
        BudgetPacker::pack_records(
            std::slice::from_ref(&record),
            RecallPackBudget {
                max_records: 1,
                max_chars: 0
            }
        )
        .is_empty()
    );

    let packed = BudgetPacker::pack_records(
        std::slice::from_ref(&record),
        RecallPackBudget {
            max_records: 1,
            max_chars: 1_000,
        },
    );
    assert_eq!(packed.len(), 1);
    assert_eq!(packed[0].rendered, render_recall_record(&record));
}

#[test]
fn summary_and_truncation_helpers_cover_short_and_tiny_limits() {
    assert_eq!(compact_summary(" short \n text "), "short text");
    assert_eq!(truncate_chars("abc", 10), "abc");
    assert_eq!(truncate_chars("abcdef", 3), "...");
    assert_eq!(truncate_chars("abcdef", 2), "..");
}

#[test]
fn evolution_service_describes_supersede_and_conflict_events() {
    let old_record = LifecycleNormalizer::normalize_memory(
        &Memory::new(
            ScopeId::from_string("scp_v27"),
            MemoryKind::Decision,
            "Old decision",
            "Use the old policy.",
        )
        .unwrap(),
    );
    let new_record = LifecycleNormalizer::normalize_memory(
        &Memory::new(
            ScopeId::from_string("scp_v27"),
            MemoryKind::Decision,
            "New decision",
            "Use the new lifecycle policy.",
        )
        .unwrap(),
    );

    let supersede = EvolutionService::supersede(&new_record, &old_record, "newer evidence");
    assert_eq!(supersede.event_type, EvolutionEventType::Supersedes);
    assert_eq!(supersede.from_record_id, new_record.record_id);
    assert_eq!(supersede.to_status, MemoryRecordStatus::Deprecated);

    let conflict = EvolutionService::conflict(&new_record, &old_record, "claims disagree");
    assert_eq!(conflict.event_type, EvolutionEventType::ConflictsWith);
    assert_eq!(conflict.from_status, MemoryRecordStatus::NeedsReview);
    assert_eq!(conflict.to_status, MemoryRecordStatus::NeedsReview);
}

#[test]
fn forget_service_soft_forgets_and_restores_records() {
    let record = LifecycleNormalizer::normalize_memory(
        &Memory::new(
            ScopeId::from_string("scp_v27"),
            MemoryKind::Fact,
            "Temporary fact",
            "This can be forgotten.",
        )
        .unwrap(),
    );

    let (forgotten, forget_event) = ForgetService::soft_forget(record, "user requested");
    assert_eq!(forgotten.status, MemoryRecordStatus::Forgotten);
    assert_eq!(forget_event.event_type, GovernanceEventType::Forgotten);
    assert_eq!(forget_event.previous_status, MemoryRecordStatus::Candidate);

    let (restored, restore_event) = ForgetService::restore(forgotten, "needed again");
    assert_eq!(restored.status, MemoryRecordStatus::Active);
    assert_eq!(restore_event.event_type, GovernanceEventType::Restored);
    assert_eq!(restore_event.previous_status, MemoryRecordStatus::Forgotten);
}

#[test]
fn audit_log_service_records_status_transition() {
    let record = LifecycleNormalizer::normalize_memory(
        &Memory::new(
            ScopeId::from_string("scp_v27"),
            MemoryKind::Risk,
            "Recall risk",
            "Restricted memories need scoped access.",
        )
        .unwrap(),
    );

    let event = AuditLogService::record(
        "recall.filtered",
        "codex",
        &record,
        Some(MemoryRecordStatus::Candidate),
        Some(MemoryRecordStatus::Active),
        Some("policy check".to_string()),
    );

    assert_eq!(event.action, "recall.filtered");
    assert_eq!(event.actor, "codex");
    assert_eq!(event.record_id, record.record_id);
    assert_eq!(event.reason.as_deref(), Some("policy check"));
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

    let owner_context = request_context("scp_v27");
    assert!(RecallGuard::allows_record(
        &restricted,
        Some(&owner_context),
        OffsetDateTime::now_utc()
    ));

    let other_context = request_context("scp_other");
    assert!(!RecallGuard::allows_record(
        &restricted,
        Some(&other_context),
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

#[test]
fn recall_guard_filters_restricted_memories_by_request_context() {
    let mut memory = Memory::new(
        ScopeId::from_string("scp_v27"),
        MemoryKind::Fact,
        "restricted owner note",
        "owner can recall this",
    )
    .unwrap();
    memory.activate().unwrap();
    memory.sensitivity = Sensitivity::Restricted;

    assert!(RecallGuard::filter_memories(vec![memory.clone()], None).is_empty());

    let owner_context = request_context("scp_v27");
    assert_eq!(
        RecallGuard::filter_memories(vec![memory.clone()], Some(&owner_context)).len(),
        1
    );

    let other_context = request_context("scp_other");
    assert!(RecallGuard::filter_memories(vec![memory], Some(&other_context)).is_empty());
}

fn request_context(scope_id: &str) -> RequestContext {
    RequestContext {
        key_id: AccessKeyId::from_string("ak_test"),
        source_id: None,
        source_kind: KeySourceKind::Cli,
        principal_id: "tester".to_string(),
        owner_scope_id: ScopeId::from_string(scope_id),
        scope_kind: KeyScopeKind::Personal,
        storage_mode: StorageMode::All,
        is_fully_isolated: false,
        isolation_group_id: "default".to_string(),
    }
}
