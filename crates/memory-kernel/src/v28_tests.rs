use super::{
    DistillationPreviewService, DistillationProfileService, DistillationSessionOverride,
    MemoryRelationship, ProjectIdentityInput, ProjectIdentityResolutionStatus,
    ProjectIdentityResolver, ProposalExecutionError, ProposalExecutor, ProposalOrchestrator,
    RelationshipClassifier, ReviewActorKind, ReviewPolicyAction, ReviewPolicyDecision,
    ReviewPolicyInput, ReviewPolicyService, RollbackError, RollbackService, TimelineAuditEvent,
    TimelineEventKind, TimelineQueryService, TimelineVersion,
};
use memory_domain::{
    BindingConfirmedBy, DistillationProfile, MemoryId, MemoryKind, MemoryLayer, MemoryProposal,
    MemoryRecord, MemoryRecordNativeKind, MemoryRecordType, MemoryRelationType, ProjectBindingKind,
    ProjectIdentityBinding, ProposalStatus, ProposalType, ReviewLevel, ScopeId,
};
use time::macros::datetime;

fn record(
    scope_id: ScopeId,
    native_id: &str,
    record_type: MemoryRecordType,
    title: &str,
    content: &str,
    source_ref: Option<&str>,
) -> MemoryRecord {
    let mut record = MemoryRecord::new(
        MemoryRecordNativeKind::Memory,
        native_id,
        MemoryLayer::LongTerm,
        scope_id,
        record_type,
        title,
    );
    record.content = Some(content.to_string());
    record.source_ref = source_ref.map(str::to_string);
    record
}

fn approved_proposal(
    proposal_type: ProposalType,
    subject_memory_id: Option<&str>,
    target_memory_ids: &[&str],
) -> MemoryProposal {
    let mut proposal = MemoryProposal::new(
        ScopeId::from_string("scp_v28"),
        proposal_type,
        ReviewLevel::Required,
        "approved governance action",
    )
    .unwrap();
    if let Some(subject_memory_id) = subject_memory_id {
        proposal = proposal.with_subject_memory(MemoryId::from_string(subject_memory_id));
    }
    for target_memory_id in target_memory_ids {
        proposal.add_target_memory(MemoryId::from_string(*target_memory_id));
    }
    proposal.approve("reviewer").unwrap();
    proposal
}

fn timeline_version(memory_id: &str, version: i32, change_kind: &str) -> TimelineVersion {
    TimelineVersion {
        memory_id: MemoryId::from_string(memory_id),
        version,
        title: format!("version {version}"),
        body: format!("body {version}"),
        change_kind: change_kind.to_string(),
        actor: "agent".to_string(),
        reason: Some(format!("{change_kind} reason")),
        source_proposal_id: None,
        created_at: datetime!(2026-04-22 10:00 UTC) + time::Duration::minutes(version.into()),
    }
}

fn global_profile(name: &str, prompt: &str) -> DistillationProfile {
    DistillationProfile::new_global(name, prompt, "user")
        .unwrap()
        .add_focus_topic("decision")
        .unwrap()
        .prefer_memory_kind(MemoryKind::Decision)
}

fn project_profile(scope_id: ScopeId, name: &str, prompt: &str) -> DistillationProfile {
    DistillationProfile::new_project(scope_id, name, prompt, "user")
        .unwrap()
        .add_focus_topic("proposal")
        .unwrap()
        .prefer_memory_kind(MemoryKind::Constraint)
}

#[test]
fn resolver_matches_confirmed_repo_root_binding() {
    let scope_id = ScopeId::from_string("scp_v28");
    let binding = ProjectIdentityBinding::new(
        scope_id.clone(),
        ProjectBindingKind::RepoRoot,
        "/Users/Rou/dev_projects/meat-memory",
        "proj_meat_memory",
        BindingConfirmedBy::User,
    )
    .unwrap()
    .with_confidence(0.96)
    .unwrap();
    let input = ProjectIdentityInput::new(scope_id.clone())
        .with_repo_root("/Users/Rou/dev_projects/meat-memory");

    let resolution = ProjectIdentityResolver::resolve(&input, &[binding.clone()]);

    assert_eq!(
        resolution.status,
        ProjectIdentityResolutionStatus::MatchedBinding
    );
    assert_eq!(resolution.scope_id, scope_id);
    assert_eq!(resolution.project_key.as_deref(), Some("proj_meat_memory"));
    assert_eq!(
        resolution.matched_binding_id.as_ref().map(|id| id.as_str()),
        Some(binding.id.as_str())
    );
    assert_eq!(resolution.confidence, 0.96);
}

#[test]
fn resolver_asks_confirmation_when_signal_has_no_binding() {
    let scope_id = ScopeId::from_string("scp_v28");
    let input = ProjectIdentityInput::new(scope_id.clone()).with_alias("meat-memory");

    let resolution = ProjectIdentityResolver::resolve(&input, &[]);

    assert_eq!(
        resolution.status,
        ProjectIdentityResolutionStatus::NeedsConfirmation
    );
    assert_eq!(resolution.project_key.as_deref(), Some("meat-memory"));
    assert_eq!(resolution.confidence, 0.5);
}

#[test]
fn resolver_matches_remote_source_prefix_and_alias_bindings() {
    let scope_id = ScopeId::from_string("scp_v28");
    let remote_binding = ProjectIdentityBinding::new(
        scope_id.clone(),
        ProjectBindingKind::RemoteUrl,
        "git@example.com:team/meat-memory.git",
        "proj_remote",
        BindingConfirmedBy::User,
    )
    .unwrap();
    let source_binding = ProjectIdentityBinding::new(
        scope_id.clone(),
        ProjectBindingKind::SourceRefPrefix,
        "file:///Users/Rou/dev_projects/meat-memory",
        "proj_source",
        BindingConfirmedBy::User,
    )
    .unwrap();
    let alias_binding = ProjectIdentityBinding::new(
        scope_id.clone(),
        ProjectBindingKind::Alias,
        "meat-memory",
        "proj_alias",
        BindingConfirmedBy::User,
    )
    .unwrap();

    let remote = ProjectIdentityResolver::resolve(
        &ProjectIdentityInput::new(scope_id.clone())
            .with_remote_url("git@example.com:team/meat-memory.git"),
        std::slice::from_ref(&remote_binding),
    );
    let source = ProjectIdentityResolver::resolve(
        &ProjectIdentityInput::new(scope_id.clone())
            .with_source_ref("file:///Users/Rou/dev_projects/meat-memory/docs/README.md"),
        std::slice::from_ref(&source_binding),
    );
    let alias = ProjectIdentityResolver::resolve(
        &ProjectIdentityInput::new(scope_id).with_alias("meat-memory"),
        std::slice::from_ref(&alias_binding),
    );

    assert_eq!(remote.project_key.as_deref(), Some("proj_remote"));
    assert_eq!(source.project_key.as_deref(), Some("proj_source"));
    assert_eq!(alias.project_key.as_deref(), Some("proj_alias"));
}

#[test]
fn resolver_ignores_bindings_from_other_scopes() {
    let binding = ProjectIdentityBinding::new(
        ScopeId::from_string("scp_other"),
        ProjectBindingKind::Alias,
        "meat-memory",
        "proj_other",
        BindingConfirmedBy::User,
    )
    .unwrap();
    let resolution = ProjectIdentityResolver::resolve(
        &ProjectIdentityInput::new(ScopeId::from_string("scp_v28")).with_alias("meat-memory"),
        &[binding],
    );

    assert_eq!(
        resolution.status,
        ProjectIdentityResolutionStatus::NeedsConfirmation
    );
    assert!(resolution.matched_binding_id.is_none());
}

#[test]
fn resolver_derives_project_key_from_repo_remote_or_source_signal() {
    let scope_id = ScopeId::from_string("scp_v28");
    let repo = ProjectIdentityResolver::resolve(
        &ProjectIdentityInput::new(scope_id.clone()).with_repo_root("/tmp/meat-memory/"),
        &[],
    );
    let remote = ProjectIdentityResolver::resolve(
        &ProjectIdentityInput::new(scope_id.clone())
            .with_remote_url("git@example.com:team/meat-memory.git"),
        &[],
    );
    let source = ProjectIdentityResolver::resolve(
        &ProjectIdentityInput::new(scope_id).with_source_ref("agent-context://ctx_1"),
        &[],
    );

    assert_eq!(repo.project_key.as_deref(), Some("meat-memory"));
    assert_eq!(remote.project_key.as_deref(), Some("meat-memory"));
    assert_eq!(source.project_key.as_deref(), Some("agent-context://ctx_1"));
}

#[test]
fn resolver_falls_back_to_scope_without_project_signal() {
    let scope_id = ScopeId::from_string("scp_v28");
    let input = ProjectIdentityInput::new(scope_id);

    let resolution = ProjectIdentityResolver::resolve(&input, &[]);

    assert_eq!(
        resolution.status,
        ProjectIdentityResolutionStatus::FallbackScope
    );
    assert!(resolution.project_key.is_none());
    assert_eq!(resolution.confidence, 0.0);
}

#[test]
fn classifier_detects_exact_duplicate_with_same_source() {
    let scope_id = ScopeId::from_string("scp_v28");
    let incoming = record(
        scope_id.clone(),
        "mem_new",
        MemoryRecordType::Decision,
        "V2.8 review policy",
        "Required proposals need user approval.",
        Some("agent-context://ctx_1"),
    );
    let existing = record(
        scope_id,
        "mem_old",
        MemoryRecordType::Decision,
        " V2.8 review policy ",
        "Required proposals need user approval.",
        Some("agent-context://ctx_1"),
    );

    let assessment = RelationshipClassifier::classify(&incoming, &[existing]);

    assert_eq!(assessment.relationship, MemoryRelationship::ExactDuplicate);
    assert_eq!(assessment.review_level, ReviewLevel::Auto);
    assert_eq!(assessment.matched_memory_ids[0].as_str(), "mem_old");
}

#[test]
fn classifier_detects_supersede_candidate_for_new_decision() {
    let scope_id = ScopeId::from_string("scp_v28");
    let incoming = record(
        scope_id.clone(),
        "mem_new",
        MemoryRecordType::Decision,
        "Storage decision",
        "We should replace direct merge with proposal-first review.",
        None,
    );
    let existing = record(
        scope_id,
        "mem_old",
        MemoryRecordType::Decision,
        "Storage decision",
        "Direct merge is allowed for memory updates.",
        None,
    );

    let assessment = RelationshipClassifier::classify(&incoming, &[existing]);

    assert_eq!(
        assessment.relationship,
        MemoryRelationship::SupersedeCandidate
    );
    assert_eq!(assessment.review_level, ReviewLevel::Required);
    assert!(
        assessment
            .evidence
            .iter()
            .any(|item| item.contains("replace"))
    );
}

#[test]
fn classifier_detects_conflict_candidate_for_opposite_fact() {
    let scope_id = ScopeId::from_string("scp_v28");
    let incoming = record(
        scope_id.clone(),
        "mem_new",
        MemoryRecordType::Fact,
        "Proposal apply mode",
        "Automatic apply is disabled.",
        None,
    );
    let existing = record(
        scope_id,
        "mem_old",
        MemoryRecordType::Fact,
        "Proposal apply mode",
        "Automatic apply is enabled.",
        None,
    );

    let assessment = RelationshipClassifier::classify(&incoming, &[existing]);

    assert_eq!(
        assessment.relationship,
        MemoryRelationship::ConflictCandidate
    );
    assert_eq!(assessment.review_level, ReviewLevel::Required);
}

#[test]
fn classifier_detects_true_false_and_chinese_conflicts() {
    let scope_id = ScopeId::from_string("scp_v28");
    let true_fact = record(
        scope_id.clone(),
        "mem_true",
        MemoryRecordType::Fact,
        "Feature flag",
        "The feature flag is true.",
        None,
    );
    let false_fact = record(
        scope_id.clone(),
        "mem_false",
        MemoryRecordType::Fact,
        "Feature flag",
        "The feature flag is false.",
        None,
    );
    let must_fact = record(
        scope_id.clone(),
        "mem_must",
        MemoryRecordType::Fact,
        "发布规则",
        "发布前必须跑验收。",
        None,
    );
    let no_longer_fact = record(
        scope_id,
        "mem_no_longer",
        MemoryRecordType::Fact,
        "发布规则",
        "发布前不再需要跑验收。",
        None,
    );

    assert_eq!(
        RelationshipClassifier::classify(&true_fact, &[false_fact]).relationship,
        MemoryRelationship::ConflictCandidate
    );
    assert_eq!(
        RelationshipClassifier::classify(&must_fact, &[no_longer_fact]).relationship,
        MemoryRelationship::ConflictCandidate
    );
}

#[test]
fn classifier_detects_reverse_polarity_conflicts() {
    let scope_id = ScopeId::from_string("scp_v28");
    let enabled_fact = record(
        scope_id.clone(),
        "mem_enabled",
        MemoryRecordType::Fact,
        "Proposal apply mode",
        "Automatic apply is enabled.",
        None,
    );
    let disabled_fact = record(
        scope_id.clone(),
        "mem_disabled",
        MemoryRecordType::Fact,
        "Proposal apply mode",
        "Automatic apply is disabled.",
        None,
    );
    let false_fact = record(
        scope_id.clone(),
        "mem_false",
        MemoryRecordType::Fact,
        "Feature flag",
        "The feature flag is false.",
        None,
    );
    let true_fact = record(
        scope_id.clone(),
        "mem_true",
        MemoryRecordType::Fact,
        "Feature flag",
        "The feature flag is true.",
        None,
    );
    let no_longer_fact = record(
        scope_id.clone(),
        "mem_no_longer",
        MemoryRecordType::Fact,
        "发布规则",
        "发布前不再需要跑验收。",
        None,
    );
    let must_fact = record(
        scope_id,
        "mem_must",
        MemoryRecordType::Fact,
        "发布规则",
        "发布前必须跑验收。",
        None,
    );

    assert_eq!(
        RelationshipClassifier::classify(&enabled_fact, &[disabled_fact]).relationship,
        MemoryRelationship::ConflictCandidate
    );
    assert_eq!(
        RelationshipClassifier::classify(&false_fact, &[true_fact]).relationship,
        MemoryRelationship::ConflictCandidate
    );
    assert_eq!(
        RelationshipClassifier::classify(&no_longer_fact, &[must_fact]).relationship,
        MemoryRelationship::ConflictCandidate
    );
}

#[test]
fn classifier_returns_unrelated_for_same_scope_different_title() {
    let scope_id = ScopeId::from_string("scp_v28");
    let incoming = record(
        scope_id.clone(),
        "mem_new",
        MemoryRecordType::Summary,
        "V2.8 proposal work",
        "Proposal review and rollback were discussed.",
        None,
    );
    let existing = record(
        scope_id,
        "mem_old",
        MemoryRecordType::Summary,
        "V2.7 lifecycle work",
        "Recall guard was discussed.",
        None,
    );

    let assessment = RelationshipClassifier::classify(&incoming, &[existing]);

    assert_eq!(assessment.relationship, MemoryRelationship::Unrelated);
}

#[test]
fn classifier_detects_near_duplicate_by_type_and_title() {
    let scope_id = ScopeId::from_string("scp_v28");
    let incoming = record(
        scope_id.clone(),
        "mem_new",
        MemoryRecordType::Summary,
        "V2.8 task summary",
        "Proposal review and rollback were discussed.",
        None,
    );
    let existing = record(
        scope_id,
        "mem_old",
        MemoryRecordType::Summary,
        "V2.8 task summary",
        "Proposal review was discussed.",
        None,
    );

    let assessment = RelationshipClassifier::classify(&incoming, &[existing]);

    assert_eq!(assessment.relationship, MemoryRelationship::NearDuplicate);
    assert_eq!(assessment.review_level, ReviewLevel::Suggested);
}

#[test]
fn classifier_returns_unrelated_when_scope_differs() {
    let incoming = record(
        ScopeId::from_string("scp_left"),
        "mem_new",
        MemoryRecordType::Summary,
        "V2.8 task summary",
        "Proposal review and rollback were discussed.",
        None,
    );
    let existing = record(
        ScopeId::from_string("scp_right"),
        "mem_old",
        MemoryRecordType::Summary,
        "V2.8 task summary",
        "Proposal review was discussed.",
        None,
    );

    let assessment = RelationshipClassifier::classify(&incoming, &[existing]);

    assert_eq!(assessment.relationship, MemoryRelationship::Unrelated);
    assert_eq!(assessment.review_level, ReviewLevel::Auto);
}

#[test]
fn orchestrator_creates_required_supersede_proposal() {
    let scope_id = ScopeId::from_string("scp_v28");
    let incoming = record(
        scope_id.clone(),
        "mem_new",
        MemoryRecordType::Decision,
        "Storage decision",
        "We should replace direct merge with proposal-first review.",
        None,
    );
    let existing = record(
        scope_id.clone(),
        "mem_old",
        MemoryRecordType::Decision,
        "Storage decision",
        "Direct merge is allowed for memory updates.",
        None,
    );
    let assessment = RelationshipClassifier::classify(&incoming, &[existing]);

    let proposal = ProposalOrchestrator::build_proposal(
        scope_id,
        MemoryId::from_string("mem_new"),
        &assessment,
    )
    .expect("proposal");

    assert_eq!(proposal.proposal_type, ProposalType::Supersede);
    assert_eq!(proposal.review_level, ReviewLevel::Required);
    assert_eq!(proposal.status, ProposalStatus::Open);
    assert_eq!(proposal.target_memory_ids[0].as_str(), "mem_old");
    assert!(!proposal.evidence.is_empty());
}

#[test]
fn orchestrator_maps_near_duplicate_and_conflict_to_proposals() {
    let scope_id = ScopeId::from_string("scp_v28");
    let summary_new = record(
        scope_id.clone(),
        "mem_summary_new",
        MemoryRecordType::Summary,
        "V2.8 task summary",
        "Proposal review and rollback were discussed.",
        None,
    );
    let summary_old = record(
        scope_id.clone(),
        "mem_summary_old",
        MemoryRecordType::Summary,
        "V2.8 task summary",
        "Proposal review was discussed.",
        None,
    );
    let merge = RelationshipClassifier::classify(&summary_new, &[summary_old]);
    let merge_proposal = ProposalOrchestrator::build_proposal(
        scope_id.clone(),
        MemoryId::from_string("mem_summary_new"),
        &merge,
    )
    .expect("merge proposal");

    let fact_new = record(
        scope_id.clone(),
        "mem_fact_new",
        MemoryRecordType::Fact,
        "Proposal apply mode",
        "Automatic apply is disabled.",
        None,
    );
    let fact_old = record(
        scope_id.clone(),
        "mem_fact_old",
        MemoryRecordType::Fact,
        "Proposal apply mode",
        "Automatic apply is enabled.",
        None,
    );
    let conflict = RelationshipClassifier::classify(&fact_new, &[fact_old]);
    let conflict_proposal = ProposalOrchestrator::build_proposal(
        scope_id,
        MemoryId::from_string("mem_fact_new"),
        &conflict,
    )
    .expect("conflict proposal");

    assert_eq!(merge_proposal.proposal_type, ProposalType::Merge);
    assert_eq!(merge_proposal.review_level, ReviewLevel::Suggested);
    assert_eq!(conflict_proposal.proposal_type, ProposalType::ConflictMark);
    assert_eq!(conflict_proposal.review_level, ReviewLevel::Required);
}

#[test]
fn orchestrator_skips_exact_duplicate_and_unrelated_assessments() {
    let scope_id = ScopeId::from_string("scp_v28");
    let incoming = record(
        scope_id.clone(),
        "mem_new",
        MemoryRecordType::Decision,
        "V2.8 review policy",
        "Required proposals need user approval.",
        Some("agent-context://ctx_1"),
    );
    let duplicate = record(
        scope_id.clone(),
        "mem_old",
        MemoryRecordType::Decision,
        "V2.8 review policy",
        "Required proposals need user approval.",
        Some("agent-context://ctx_1"),
    );
    let exact = RelationshipClassifier::classify(&incoming, &[duplicate]);
    assert!(
        ProposalOrchestrator::build_proposal(
            scope_id.clone(),
            MemoryId::from_string("mem_new"),
            &exact,
        )
        .is_none()
    );

    let unrelated = RelationshipClassifier::classify(&incoming, &[]);
    assert!(
        ProposalOrchestrator::build_proposal(
            scope_id,
            MemoryId::from_string("mem_new"),
            &unrelated,
        )
        .is_none()
    );
}

#[test]
fn executor_rejects_non_approved_proposals() {
    let proposal = MemoryProposal::new(
        ScopeId::from_string("scp_v28"),
        ProposalType::Supersede,
        ReviewLevel::Required,
        "needs approval before execution",
    )
    .unwrap()
    .with_subject_memory(MemoryId::from_string("mem_new"));

    let error = ProposalExecutor::plan(&proposal, "agent").unwrap_err();

    assert_eq!(error, ProposalExecutionError::ProposalNotApproved);
}

#[test]
fn executor_requires_subject_and_target_when_needed() {
    let missing_subject = approved_proposal(ProposalType::Archive, None, &[]);
    let merge_missing_subject = approved_proposal(ProposalType::Merge, None, &["mem_target"]);
    let supersede_missing_subject = approved_proposal(ProposalType::Supersede, None, &["mem_old"]);
    let missing_target = approved_proposal(ProposalType::Supersede, Some("mem_new"), &[]);
    let conflict_missing_target =
        approved_proposal(ProposalType::ConflictMark, Some("mem_left"), &[]);

    assert_eq!(
        ProposalExecutor::plan(&missing_subject, "agent").unwrap_err(),
        ProposalExecutionError::MissingSubjectMemory
    );
    assert_eq!(
        ProposalExecutor::plan(&merge_missing_subject, "agent").unwrap_err(),
        ProposalExecutionError::MissingSubjectMemory
    );
    assert_eq!(
        ProposalExecutor::plan(&supersede_missing_subject, "agent").unwrap_err(),
        ProposalExecutionError::MissingSubjectMemory
    );
    assert_eq!(
        ProposalExecutor::plan(&missing_target, "agent").unwrap_err(),
        ProposalExecutionError::MissingTargetMemory
    );
    assert_eq!(
        ProposalExecutor::plan(&conflict_missing_target, "agent").unwrap_err(),
        ProposalExecutionError::MissingTargetMemory
    );
}

#[test]
fn executor_plans_supersede_and_conflict_relations() {
    let supersede = approved_proposal(ProposalType::Supersede, Some("mem_new"), &["mem_old"]);
    let conflict = approved_proposal(ProposalType::ConflictMark, Some("mem_left"), &["mem_right"]);

    let supersede_plan = ProposalExecutor::plan(&supersede, "agent").unwrap();
    let conflict_plan = ProposalExecutor::plan(&conflict, "agent").unwrap();

    assert_eq!(supersede_plan.relation_actions.len(), 1);
    assert_eq!(
        supersede_plan.relation_actions[0].relation_type,
        MemoryRelationType::Supersedes
    );
    assert_eq!(
        supersede_plan.relation_actions[0].source_proposal_id,
        Some(supersede.id.clone())
    );
    assert_eq!(
        conflict_plan.relation_actions[0].relation_type,
        MemoryRelationType::ConflictsWith
    );
    assert!(supersede_plan.version_actions.is_empty());
    assert!(supersede_plan.mark_proposal_applied);
}

#[test]
fn executor_plans_version_actions_for_content_changing_proposals() {
    for proposal_type in [
        ProposalType::Merge,
        ProposalType::NewVersion,
        ProposalType::DistillUpsert,
    ] {
        let proposal = approved_proposal(proposal_type, Some("mem_subject"), &["mem_target"]);

        let plan = ProposalExecutor::plan(&proposal, "agent").unwrap();

        assert_eq!(plan.version_actions.len(), 1);
        assert_eq!(plan.version_actions[0].memory_id.as_str(), "mem_subject");
        assert_eq!(
            plan.version_actions[0].source_proposal_id,
            proposal.id.clone()
        );
        assert_eq!(plan.version_actions[0].change_kind, proposal_type.as_str());
        assert_eq!(plan.audit_actions[0].actor, "agent");
    }
}

#[test]
fn executor_plans_audit_only_for_lifecycle_proposals() {
    for proposal_type in [
        ProposalType::Archive,
        ProposalType::Forget,
        ProposalType::Restore,
        ProposalType::HardDelete,
    ] {
        let proposal = approved_proposal(proposal_type, Some("mem_subject"), &[]);

        let plan = ProposalExecutor::plan(&proposal, "agent").unwrap();

        assert!(plan.version_actions.is_empty());
        assert!(plan.relation_actions.is_empty());
        assert_eq!(plan.audit_actions.len(), 1);
        assert_eq!(
            plan.audit_actions[0]
                .memory_id
                .as_ref()
                .map(|memory_id| memory_id.as_str()),
            Some("mem_subject")
        );
        assert!(
            plan.audit_actions[0]
                .action
                .contains(proposal_type.as_str())
        );
    }
}

#[test]
fn timeline_filters_related_inputs_and_sorts_events_newest_first() {
    let memory_id = MemoryId::from_string("mem_subject");
    let scope_id = ScopeId::from_string("scp_v28");
    let mut relation = memory_domain::MemoryRelation::new(
        scope_id.clone(),
        memory_id.clone(),
        MemoryId::from_string("mem_old"),
        MemoryRelationType::Supersedes,
        memory_domain::MemoryRelationSourceKind::Agent,
    );
    relation.created_at = datetime!(2026-04-22 10:05 UTC);
    let unrelated_relation = memory_domain::MemoryRelation::new(
        scope_id.clone(),
        MemoryId::from_string("mem_left"),
        MemoryId::from_string("mem_right"),
        MemoryRelationType::RelatedTo,
        memory_domain::MemoryRelationSourceKind::Agent,
    );
    let mut proposal =
        approved_proposal(ProposalType::Supersede, Some("mem_subject"), &["mem_old"]);
    proposal.created_at = datetime!(2026-04-22 10:03 UTC);
    proposal.decided_at = Some(datetime!(2026-04-22 10:04 UTC));
    let audit = TimelineAuditEvent {
        memory_id: Some(memory_id.clone()),
        action: "memory.lifecycle.restore".to_string(),
        actor: "user".to_string(),
        reason: Some("restore after review".to_string()),
        created_at: datetime!(2026-04-22 10:06 UTC),
    };
    let unrelated_audit = TimelineAuditEvent {
        memory_id: Some(MemoryId::from_string("mem_other")),
        action: "memory.lifecycle.forget".to_string(),
        actor: "user".to_string(),
        reason: None,
        created_at: datetime!(2026-04-22 10:07 UTC),
    };

    let timeline = TimelineQueryService::build(
        memory_id.clone(),
        vec![
            timeline_version("mem_subject", 1, "create"),
            timeline_version("mem_subject", 2, "edit"),
            timeline_version("mem_other", 3, "edit"),
        ],
        vec![relation.clone(), unrelated_relation],
        vec![audit, unrelated_audit],
        vec![proposal],
    );

    assert_eq!(timeline.memory_id, memory_id);
    assert_eq!(timeline.versions.len(), 2);
    assert_eq!(timeline.relations, vec![relation]);
    assert_eq!(timeline.audit_events.len(), 1);
    assert_eq!(timeline.proposals.len(), 1);
    assert_eq!(
        timeline
            .events
            .iter()
            .map(|event| event.action.as_str())
            .collect::<Vec<_>>(),
        vec![
            "audit.memory.lifecycle.restore",
            "relation.supersedes",
            "proposal.approved",
            "proposal.opened",
            "version.edit",
            "version.create",
        ]
    );
    assert_eq!(timeline.events[0].kind, TimelineEventKind::Audit);
}

#[test]
fn timeline_includes_target_side_relations_and_proposals() {
    let memory_id = MemoryId::from_string("mem_old");
    let scope_id = ScopeId::from_string("scp_v28");
    let mut relation = memory_domain::MemoryRelation::new(
        scope_id.clone(),
        MemoryId::from_string("mem_new"),
        memory_id.clone(),
        MemoryRelationType::Supersedes,
        memory_domain::MemoryRelationSourceKind::Agent,
    );
    relation.created_at = datetime!(2026-04-22 10:05 UTC);
    let mut proposal = approved_proposal(ProposalType::Supersede, Some("mem_new"), &["mem_old"]);
    proposal.created_at = datetime!(2026-04-22 10:03 UTC);

    let timeline = TimelineQueryService::build(
        memory_id.clone(),
        Vec::new(),
        vec![relation],
        Vec::new(),
        vec![proposal],
    );

    assert_eq!(timeline.relations.len(), 1);
    assert_eq!(timeline.proposals.len(), 1);
    assert!(
        timeline
            .events
            .iter()
            .any(|event| event.kind == TimelineEventKind::Relation)
    );
    assert!(
        timeline
            .events
            .iter()
            .any(|event| event.action == "proposal.opened")
    );
}

#[test]
fn timeline_emits_rejected_and_applied_proposal_events() {
    let memory_id = MemoryId::from_string("mem_subject");
    let mut applied = approved_proposal(ProposalType::Merge, Some("mem_subject"), &["mem_old"]);
    applied.created_at = datetime!(2026-04-22 10:00 UTC);
    applied.decided_at = Some(datetime!(2026-04-22 10:01 UTC));
    applied.applied_at = Some(datetime!(2026-04-22 10:02 UTC));
    applied.status = ProposalStatus::Applied;
    let mut rejected = MemoryProposal::new(
        ScopeId::from_string("scp_v28"),
        ProposalType::ConflictMark,
        ReviewLevel::Required,
        "conflict rejected",
    )
    .unwrap()
    .with_subject_memory(memory_id.clone());
    rejected.created_at = datetime!(2026-04-22 10:03 UTC);
    rejected.reject("user").unwrap();
    rejected.decided_at = Some(datetime!(2026-04-22 10:04 UTC));

    let timeline = TimelineQueryService::build(
        memory_id,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![applied, rejected],
    );

    assert_eq!(
        timeline
            .events
            .iter()
            .map(|event| event.action.as_str())
            .collect::<Vec<_>>(),
        vec![
            "proposal.rejected",
            "proposal.opened",
            "proposal.applied",
            "proposal.approved",
            "proposal.opened",
        ]
    );
}

#[test]
fn timeline_emits_remaining_proposal_decision_statuses_and_tie_breaker() {
    let memory_id = MemoryId::from_string("mem_subject");
    let mut canceled = MemoryProposal::new(
        ScopeId::from_string("scp_v28"),
        ProposalType::Archive,
        ReviewLevel::Required,
        "canceled archive",
    )
    .unwrap()
    .with_subject_memory(memory_id.clone());
    canceled.status = ProposalStatus::Canceled;
    canceled.created_at = datetime!(2026-04-22 10:00 UTC);
    canceled.decided_at = Some(datetime!(2026-04-22 10:01 UTC));
    let mut expired = MemoryProposal::new(
        ScopeId::from_string("scp_v28"),
        ProposalType::Forget,
        ReviewLevel::Required,
        "expired forget",
    )
    .unwrap()
    .with_subject_memory(memory_id.clone());
    expired.status = ProposalStatus::Expired;
    expired.created_at = datetime!(2026-04-22 10:00 UTC);
    expired.decided_at = Some(datetime!(2026-04-22 10:01 UTC));
    let mut decided_open = MemoryProposal::new(
        ScopeId::from_string("scp_v28"),
        ProposalType::Restore,
        ReviewLevel::Required,
        "legacy open decision",
    )
    .unwrap()
    .with_subject_memory(memory_id.clone());
    decided_open.created_at = datetime!(2026-04-22 10:00 UTC);
    decided_open.decided_at = Some(datetime!(2026-04-22 10:01 UTC));
    let mut open = MemoryProposal::new(
        ScopeId::from_string("scp_v28"),
        ProposalType::NewVersion,
        ReviewLevel::Suggested,
        "still open",
    )
    .unwrap()
    .with_subject_memory(memory_id.clone());
    open.created_at = datetime!(2026-04-22 10:02 UTC);

    let timeline = TimelineQueryService::build(
        memory_id,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![canceled, expired, decided_open, open],
    );

    assert_eq!(
        timeline
            .events
            .iter()
            .map(|event| event.action.as_str())
            .collect::<Vec<_>>(),
        vec![
            "proposal.opened",
            "proposal.canceled",
            "proposal.decided",
            "proposal.expired",
            "proposal.opened",
            "proposal.opened",
            "proposal.opened",
        ]
    );
}

#[test]
fn timeline_handles_empty_inputs() {
    let timeline = TimelineQueryService::build(
        MemoryId::from_string("mem_empty"),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );

    assert!(timeline.versions.is_empty());
    assert!(timeline.relations.is_empty());
    assert!(timeline.audit_events.is_empty());
    assert!(timeline.proposals.is_empty());
    assert!(timeline.events.is_empty());
}

#[test]
fn rollback_plans_new_version_from_old_snapshot() {
    let memory_id = MemoryId::from_string("mem_subject");
    let versions = vec![
        timeline_version("mem_subject", 1, "create"),
        timeline_version("mem_subject", 2, "edit"),
        timeline_version("mem_other", 9, "edit"),
    ];

    let plan = RollbackService::plan(
        memory_id.clone(),
        1,
        &versions,
        "user",
        "restore stable version",
    )
    .unwrap();

    assert_eq!(plan.memory_id, memory_id);
    assert_eq!(plan.target_version, 1);
    assert_eq!(plan.new_version, 3);
    assert_eq!(plan.title, "version 1");
    assert_eq!(plan.body, "body 1");
    assert_eq!(plan.change_kind, "rollback");
    assert_eq!(plan.actor, "user");
    assert_eq!(plan.reason, "restore stable version");
}

#[test]
fn rollback_rejects_missing_or_latest_targets() {
    let memory_id = MemoryId::from_string("mem_subject");
    let versions = vec![
        timeline_version("mem_subject", 1, "create"),
        timeline_version("mem_subject", 2, "edit"),
        timeline_version("mem_other", 3, "edit"),
    ];

    assert_eq!(
        RollbackService::plan(memory_id.clone(), 9, &versions, "user", "restore").unwrap_err(),
        RollbackError::TargetVersionNotFound
    );
    assert_eq!(
        RollbackService::plan(memory_id, 2, &versions, "user", "restore").unwrap_err(),
        RollbackError::TargetVersionIsLatest
    );
}

#[test]
fn rollback_rejects_empty_inputs() {
    let memory_id = MemoryId::from_string("mem_subject");

    assert_eq!(
        RollbackService::plan(memory_id.clone(), 1, &[], "user", "restore").unwrap_err(),
        RollbackError::NoVersions
    );
    assert_eq!(
        RollbackService::plan(
            memory_id.clone(),
            1,
            &[timeline_version("mem_subject", 1, "create")],
            " ",
            "restore",
        )
        .unwrap_err(),
        RollbackError::EmptyActor
    );
    assert_eq!(
        RollbackService::plan(
            memory_id,
            1,
            &[timeline_version("mem_subject", 1, "create")],
            "user",
            " ",
        )
        .unwrap_err(),
        RollbackError::EmptyReason
    );
}

#[test]
fn profile_service_composes_system_user_project_and_session_layers() {
    let scope_id = ScopeId::from_string("scp_v28");
    let global = global_profile("Global", "Prefer durable decisions.");
    let project = project_profile(scope_id.clone(), "Project", "Focus on V2.8 governance.");
    let override_profile = DistillationSessionOverride::new("Mention rollback risks.")
        .add_focus_topic("rollback")
        .prefer_memory_kind(MemoryKind::Risk);

    let composed = DistillationProfileService::compose(
        scope_id,
        &[global.clone(), project.clone()],
        Some(override_profile),
    );

    assert_eq!(composed.prompt_segments.len(), 4);
    assert_eq!(composed.prompt_segments[0].layer, "system_base");
    assert_eq!(composed.prompt_segments[1].layer, "user_global");
    assert_eq!(composed.prompt_segments[2].layer, "project");
    assert_eq!(composed.prompt_segments[3].layer, "session_override");
    assert_eq!(
        composed.source_profile_ids,
        vec![global.id.clone(), project.id.clone()]
    );
    assert_eq!(
        composed.focus_topics,
        vec![
            "decision".to_string(),
            "proposal".to_string(),
            "rollback".to_string()
        ]
    );
    assert_eq!(
        composed.prefer_memory_kinds,
        vec![
            MemoryKind::Decision,
            MemoryKind::Constraint,
            MemoryKind::Risk
        ]
    );
    assert!(
        composed
            .safety_rules
            .iter()
            .any(|rule| rule.contains("evidence"))
    );
}

#[test]
fn profile_service_ignores_archived_and_other_project_profiles() {
    let scope_id = ScopeId::from_string("scp_v28");
    let mut archived_global = global_profile("Archived", "Ignore archived profile.");
    archived_global.archive();
    let other_project = project_profile(
        ScopeId::from_string("scp_other"),
        "Other project",
        "Ignore other project.",
    );

    let composed =
        DistillationProfileService::compose(scope_id, &[archived_global, other_project], None);

    assert_eq!(composed.prompt_segments.len(), 1);
    assert_eq!(composed.prompt_segments[0].layer, "system_base");
    assert!(composed.source_profile_ids.is_empty());
    assert!(composed.focus_topics.is_empty());
    assert!(composed.prefer_memory_kinds.is_empty());
}

#[test]
fn profile_service_uses_latest_active_profile_per_persistent_layer() {
    let scope_id = ScopeId::from_string("scp_v28");
    let old_global = global_profile("Old global", "Old global prompt.");
    let mut new_global = global_profile("New global", "New global prompt.");
    new_global.updated_at = old_global.updated_at + time::Duration::minutes(1);
    let old_project = project_profile(scope_id.clone(), "Old project", "Old project prompt.");
    let mut new_project = project_profile(scope_id.clone(), "New project", "New project prompt.");
    new_project.updated_at = old_project.updated_at + time::Duration::minutes(1);

    let composed = DistillationProfileService::compose(
        scope_id,
        &[
            old_global.clone(),
            new_global.clone(),
            old_project.clone(),
            new_project.clone(),
        ],
        None,
    );

    assert_eq!(
        composed.source_profile_ids,
        vec![new_global.id.clone(), new_project.id.clone()]
    );
    assert!(
        composed
            .prompt_segments
            .iter()
            .any(|segment| segment.text == "New global prompt.")
    );
    assert!(
        composed
            .prompt_segments
            .iter()
            .all(|segment| segment.text != "Old global prompt.")
    );
}

#[test]
fn profile_service_deduplicates_session_preferences() {
    let scope_id = ScopeId::from_string("scp_v28");
    let global = global_profile("Global", "Prefer durable decisions.");
    let session = DistillationSessionOverride::new("Prefer explicit constraints.")
        .add_focus_topic("decision")
        .add_focus_topic("constraint")
        .add_focus_topic("constraint")
        .prefer_memory_kind(MemoryKind::Decision)
        .prefer_memory_kind(MemoryKind::Constraint)
        .prefer_memory_kind(MemoryKind::Constraint);

    let composed = DistillationProfileService::compose(scope_id, &[global], Some(session));

    assert_eq!(
        composed.focus_topics,
        vec!["decision".to_string(), "constraint".to_string()]
    );
    assert_eq!(
        composed.prefer_memory_kinds,
        vec![MemoryKind::Decision, MemoryKind::Constraint]
    );
}

#[test]
fn profile_service_allows_session_preferences_without_prompt_text() {
    let scope_id = ScopeId::from_string("scp_v28");
    let session = DistillationSessionOverride::new(" ")
        .add_focus_topic("risk")
        .prefer_memory_kind(MemoryKind::Risk);

    let composed = DistillationProfileService::compose(scope_id, &[], Some(session));

    assert_eq!(composed.prompt_segments.len(), 1);
    assert_eq!(composed.prompt_segments[0].layer, "system_base");
    assert_eq!(composed.focus_topics, vec!["risk".to_string()]);
    assert_eq!(composed.prefer_memory_kinds, vec![MemoryKind::Risk]);
}

#[test]
fn distillation_preview_builds_candidate_from_input_and_profile() {
    let scope_id = ScopeId::from_string("scp_v28");
    let profile = DistillationProfileService::compose(
        scope_id.clone(),
        &[project_profile(
            scope_id.clone(),
            "Project",
            "Focus on governance decisions.",
        )],
        None,
    );

    let preview = DistillationPreviewService::preview(
        scope_id.clone(),
        "V2.8 should keep rollback decisions with evidence.",
        &["agent-context://ctx_1".to_string()],
        &profile,
    )
    .unwrap();

    assert_eq!(preview.run.scope_id, scope_id);
    assert!(preview.run.preview);
    assert_eq!(preview.candidates.len(), 1);
    assert_eq!(preview.candidates[0].memory_kind, MemoryKind::Constraint);
    assert!(
        preview.candidates[0]
            .title
            .contains("V2.8 should keep rollback")
    );
    assert_eq!(
        preview.candidates[0].evidence_refs,
        vec!["agent-context://ctx_1".to_string()]
    );
    assert!(
        preview
            .warnings
            .iter()
            .any(|warning| warning.contains("preview"))
    );
}

#[test]
fn distillation_preview_rejects_empty_input_and_evidence() {
    let scope_id = ScopeId::from_string("scp_v28");
    let profile = DistillationProfileService::compose(scope_id.clone(), &[], None);

    assert!(DistillationPreviewService::preview(scope_id.clone(), " ", &[], &profile).is_err());
    assert!(
        DistillationPreviewService::preview(scope_id, "Keep this decision.", &[], &profile)
            .is_err()
    );
}

#[test]
fn distillation_preview_uses_default_memory_kind_when_profile_has_no_preference() {
    let scope_id = ScopeId::from_string("scp_v28");
    let profile = DistillationProfileService::compose(scope_id.clone(), &[], None);

    let preview = DistillationPreviewService::preview(
        scope_id,
        "Remember this stable fact.",
        &["agent-context://ctx_1".to_string()],
        &profile,
    )
    .unwrap();

    assert_eq!(preview.candidates[0].memory_kind, MemoryKind::Summary);
    assert_eq!(
        preview.candidates[0].why_keep,
        "profile-guided preview candidate"
    );
    assert!(preview.discarded.is_empty());
}

#[test]
fn review_policy_allows_auto_and_suggested_approval_boundaries() {
    assert_eq!(
        ReviewPolicyService::evaluate(ReviewPolicyInput {
            review_level: ReviewLevel::Auto,
            action: ReviewPolicyAction::Approve,
            actor_kind: ReviewActorKind::System,
            has_user_authorization: false,
        })
        .decision,
        ReviewPolicyDecision::Allow
    );
    assert_eq!(
        ReviewPolicyService::evaluate(ReviewPolicyInput {
            review_level: ReviewLevel::Suggested,
            action: ReviewPolicyAction::Approve,
            actor_kind: ReviewActorKind::Agent,
            has_user_authorization: false,
        })
        .decision,
        ReviewPolicyDecision::Allow
    );
    assert_eq!(
        ReviewPolicyService::evaluate(ReviewPolicyInput {
            review_level: ReviewLevel::Suggested,
            action: ReviewPolicyAction::Approve,
            actor_kind: ReviewActorKind::System,
            has_user_authorization: false,
        })
        .decision,
        ReviewPolicyDecision::RequireUserApproval
    );
}

#[test]
fn review_policy_requires_user_authorization_for_required_approval() {
    assert_eq!(
        ReviewPolicyService::evaluate(ReviewPolicyInput {
            review_level: ReviewLevel::Required,
            action: ReviewPolicyAction::Approve,
            actor_kind: ReviewActorKind::User,
            has_user_authorization: false,
        })
        .decision,
        ReviewPolicyDecision::Allow
    );
    assert_eq!(
        ReviewPolicyService::evaluate(ReviewPolicyInput {
            review_level: ReviewLevel::Required,
            action: ReviewPolicyAction::Approve,
            actor_kind: ReviewActorKind::Agent,
            has_user_authorization: false,
        })
        .decision,
        ReviewPolicyDecision::RequireUserApproval
    );
    assert_eq!(
        ReviewPolicyService::evaluate(ReviewPolicyInput {
            review_level: ReviewLevel::Required,
            action: ReviewPolicyAction::Approve,
            actor_kind: ReviewActorKind::Agent,
            has_user_authorization: true,
        })
        .decision,
        ReviewPolicyDecision::Allow
    );
    assert_eq!(
        ReviewPolicyService::evaluate(ReviewPolicyInput {
            review_level: ReviewLevel::Required,
            action: ReviewPolicyAction::Approve,
            actor_kind: ReviewActorKind::System,
            has_user_authorization: true,
        })
        .decision,
        ReviewPolicyDecision::Deny
    );
}

#[test]
fn review_policy_denies_blocked_approval_and_apply() {
    for action in [ReviewPolicyAction::Approve, ReviewPolicyAction::Apply] {
        for actor_kind in [
            ReviewActorKind::User,
            ReviewActorKind::Agent,
            ReviewActorKind::System,
        ] {
            let evaluation = ReviewPolicyService::evaluate(ReviewPolicyInput {
                review_level: ReviewLevel::Blocked,
                action,
                actor_kind,
                has_user_authorization: true,
            });
            assert_eq!(evaluation.decision, ReviewPolicyDecision::Deny);
            assert!(evaluation.reason.contains("blocked"));
        }
    }
}

#[test]
fn review_policy_allows_system_apply_but_guards_agent_required_apply() {
    for review_level in [ReviewLevel::Auto, ReviewLevel::Suggested] {
        assert_eq!(
            ReviewPolicyService::evaluate(ReviewPolicyInput {
                review_level,
                action: ReviewPolicyAction::Apply,
                actor_kind: ReviewActorKind::Agent,
                has_user_authorization: false,
            })
            .decision,
            ReviewPolicyDecision::Allow
        );
    }
    assert_eq!(
        ReviewPolicyService::evaluate(ReviewPolicyInput {
            review_level: ReviewLevel::Required,
            action: ReviewPolicyAction::Apply,
            actor_kind: ReviewActorKind::System,
            has_user_authorization: false,
        })
        .decision,
        ReviewPolicyDecision::Allow
    );
    assert_eq!(
        ReviewPolicyService::evaluate(ReviewPolicyInput {
            review_level: ReviewLevel::Required,
            action: ReviewPolicyAction::Apply,
            actor_kind: ReviewActorKind::Agent,
            has_user_authorization: false,
        })
        .decision,
        ReviewPolicyDecision::RequireUserApproval
    );
    assert_eq!(
        ReviewPolicyService::evaluate(ReviewPolicyInput {
            review_level: ReviewLevel::Required,
            action: ReviewPolicyAction::Apply,
            actor_kind: ReviewActorKind::Agent,
            has_user_authorization: true,
        })
        .decision,
        ReviewPolicyDecision::Allow
    );
}
