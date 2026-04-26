use memory_domain::{
    DistillationProfile, DistillationProfileId, DistillationProfileLevel,
    DistillationProfileStatus, DistillationRun, MemoryId, MemoryKind, MemoryProposal, MemoryRecord,
    MemoryRecordType, MemoryRelation, MemoryRelationId, MemoryRelationSourceKind,
    MemoryRelationType, ProjectBindingKind, ProjectIdentityBinding, ProjectIdentityBindingId,
    ProposalId, ProposalStatus, ProposalType, ReviewLevel, ScopeId,
};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectIdentityInput {
    pub scope_id: ScopeId,
    pub repo_root: Option<String>,
    pub remote_url: Option<String>,
    pub source_refs: Vec<String>,
    pub alias: Option<String>,
}

impl ProjectIdentityInput {
    pub fn new(scope_id: ScopeId) -> Self {
        Self {
            scope_id,
            repo_root: None,
            remote_url: None,
            source_refs: Vec::new(),
            alias: None,
        }
    }

    pub fn with_repo_root(mut self, repo_root: impl Into<String>) -> Self {
        self.repo_root = Some(repo_root.into());
        self
    }

    pub fn with_remote_url(mut self, remote_url: impl Into<String>) -> Self {
        self.remote_url = Some(remote_url.into());
        self
    }

    pub fn with_source_ref(mut self, source_ref: impl Into<String>) -> Self {
        self.source_refs.push(source_ref.into());
        self
    }

    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectIdentityResolutionStatus {
    MatchedBinding,
    NeedsConfirmation,
    FallbackScope,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectIdentityResolution {
    pub status: ProjectIdentityResolutionStatus,
    pub scope_id: ScopeId,
    pub project_key: Option<String>,
    pub matched_binding_id: Option<ProjectIdentityBindingId>,
    pub confidence: f32,
}

pub struct ProjectIdentityResolver;

impl ProjectIdentityResolver {
    pub fn resolve(
        input: &ProjectIdentityInput,
        bindings: &[ProjectIdentityBinding],
    ) -> ProjectIdentityResolution {
        if let Some(binding) = bindings.iter().find(|binding| {
            binding.owner_scope_id == input.scope_id && binding_matches(input, binding)
        }) {
            return ProjectIdentityResolution {
                status: ProjectIdentityResolutionStatus::MatchedBinding,
                scope_id: input.scope_id.clone(),
                project_key: Some(binding.canonical_project_key.clone()),
                matched_binding_id: Some(binding.id.clone()),
                confidence: binding.confidence,
            };
        }

        if let Some(project_key) = first_project_signal(input) {
            return ProjectIdentityResolution {
                status: ProjectIdentityResolutionStatus::NeedsConfirmation,
                scope_id: input.scope_id.clone(),
                project_key: Some(project_key),
                matched_binding_id: None,
                confidence: 0.5,
            };
        }

        ProjectIdentityResolution {
            status: ProjectIdentityResolutionStatus::FallbackScope,
            scope_id: input.scope_id.clone(),
            project_key: None,
            matched_binding_id: None,
            confidence: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRelationship {
    ExactDuplicate,
    NearDuplicate,
    SupersedeCandidate,
    ConflictCandidate,
    Unrelated,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelationshipAssessment {
    pub relationship: MemoryRelationship,
    pub confidence: f32,
    pub review_level: ReviewLevel,
    pub matched_memory_ids: Vec<MemoryId>,
    pub evidence: Vec<String>,
}

pub struct RelationshipClassifier;

impl RelationshipClassifier {
    pub fn classify(
        incoming: &MemoryRecord,
        existing_records: &[MemoryRecord],
    ) -> RelationshipAssessment {
        for existing in existing_records
            .iter()
            .filter(|record| record.scope_id == incoming.scope_id)
        {
            if is_exact_duplicate(incoming, existing) {
                return assessment(
                    MemoryRelationship::ExactDuplicate,
                    1.0,
                    ReviewLevel::Auto,
                    existing,
                    "same normalized title, content, and source",
                );
            }
            if is_supersede_candidate(incoming, existing) {
                return assessment(
                    MemoryRelationship::SupersedeCandidate,
                    0.86,
                    ReviewLevel::Required,
                    existing,
                    "incoming decision contains replace / supersede wording",
                );
            }
            if is_conflict_candidate(incoming, existing) {
                return assessment(
                    MemoryRelationship::ConflictCandidate,
                    0.82,
                    ReviewLevel::Required,
                    existing,
                    "same fact title has opposite enabled / disabled polarity",
                );
            }
            if is_near_duplicate(incoming, existing) {
                return assessment(
                    MemoryRelationship::NearDuplicate,
                    0.72,
                    ReviewLevel::Suggested,
                    existing,
                    "same record type and normalized title",
                );
            }
        }

        RelationshipAssessment {
            relationship: MemoryRelationship::Unrelated,
            confidence: 0.0,
            review_level: ReviewLevel::Auto,
            matched_memory_ids: Vec::new(),
            evidence: Vec::new(),
        }
    }
}

pub struct ProposalOrchestrator;

impl ProposalOrchestrator {
    pub fn build_proposal(
        scope_id: ScopeId,
        subject_memory_id: MemoryId,
        assessment: &RelationshipAssessment,
    ) -> Option<MemoryProposal> {
        let (proposal_type, reason) = match assessment.relationship {
            MemoryRelationship::ExactDuplicate | MemoryRelationship::Unrelated => return None,
            MemoryRelationship::NearDuplicate => (
                ProposalType::Merge,
                "near duplicate memory should be reviewed for merge",
            ),
            MemoryRelationship::SupersedeCandidate => (
                ProposalType::Supersede,
                "new memory may supersede an existing memory",
            ),
            MemoryRelationship::ConflictCandidate => (
                ProposalType::ConflictMark,
                "new memory may conflict with an existing memory",
            ),
        };

        let mut proposal =
            MemoryProposal::new(scope_id, proposal_type, assessment.review_level, reason)
                .expect("proposal reason is static and non-empty")
                .with_subject_memory(subject_memory_id);
        for memory_id in &assessment.matched_memory_ids {
            proposal.add_target_memory(memory_id.clone());
        }
        for evidence in &assessment.evidence {
            proposal
                .add_evidence(evidence.clone())
                .expect("assessment evidence is static and non-empty");
        }
        Some(proposal)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalExecutionError {
    ProposalNotApproved,
    MissingSubjectMemory,
    MissingTargetMemory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalVersionAction {
    pub memory_id: MemoryId,
    pub change_kind: &'static str,
    pub reason: String,
    pub source_proposal_id: ProposalId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalAuditAction {
    pub memory_id: Option<MemoryId>,
    pub action: String,
    pub actor: String,
    pub reason: String,
    pub source_proposal_id: ProposalId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProposalExecutionPlan {
    pub proposal_id: ProposalId,
    pub version_actions: Vec<ProposalVersionAction>,
    pub relation_actions: Vec<MemoryRelation>,
    pub audit_actions: Vec<ProposalAuditAction>,
    pub mark_proposal_applied: bool,
}

pub struct ProposalExecutor;

impl ProposalExecutor {
    pub fn plan(
        proposal: &MemoryProposal,
        actor: impl Into<String>,
    ) -> Result<ProposalExecutionPlan, ProposalExecutionError> {
        if proposal.status != ProposalStatus::Approved {
            return Err(ProposalExecutionError::ProposalNotApproved);
        }

        let actor = actor.into();
        let mut plan = ProposalExecutionPlan {
            proposal_id: proposal.id.clone(),
            version_actions: Vec::new(),
            relation_actions: Vec::new(),
            audit_actions: Vec::new(),
            mark_proposal_applied: true,
        };

        match proposal.proposal_type {
            ProposalType::Merge | ProposalType::NewVersion | ProposalType::DistillUpsert => {
                let memory_id = required_subject_memory(proposal)?;
                plan.version_actions.push(ProposalVersionAction {
                    memory_id: memory_id.clone(),
                    change_kind: proposal.proposal_type.as_str(),
                    reason: proposal.reason.clone(),
                    source_proposal_id: proposal.id.clone(),
                });
                plan.audit_actions
                    .push(proposal_audit_action(proposal, Some(memory_id), &actor));
            }
            ProposalType::Supersede => {
                plan_relation_actions(proposal, &mut plan, MemoryRelationType::Supersedes)?;
                plan.audit_actions.push(proposal_audit_action(
                    proposal,
                    proposal.subject_memory_id.clone(),
                    &actor,
                ));
            }
            ProposalType::ConflictMark => {
                plan_relation_actions(proposal, &mut plan, MemoryRelationType::ConflictsWith)?;
                plan.audit_actions.push(proposal_audit_action(
                    proposal,
                    proposal.subject_memory_id.clone(),
                    &actor,
                ));
            }
            ProposalType::Archive
            | ProposalType::Forget
            | ProposalType::Restore
            | ProposalType::HardDelete => {
                let memory_id = required_subject_memory(proposal)?;
                plan.audit_actions
                    .push(proposal_audit_action(proposal, Some(memory_id), &actor));
            }
        }

        Ok(plan)
    }
}

fn required_subject_memory(proposal: &MemoryProposal) -> Result<MemoryId, ProposalExecutionError> {
    proposal
        .subject_memory_id
        .clone()
        .ok_or(ProposalExecutionError::MissingSubjectMemory)
}

fn required_target_memories(
    proposal: &MemoryProposal,
) -> Result<&[MemoryId], ProposalExecutionError> {
    if proposal.target_memory_ids.is_empty() {
        return Err(ProposalExecutionError::MissingTargetMemory);
    }
    Ok(&proposal.target_memory_ids)
}

fn plan_relation_actions(
    proposal: &MemoryProposal,
    plan: &mut ProposalExecutionPlan,
    relation_type: MemoryRelationType,
) -> Result<(), ProposalExecutionError> {
    let subject_memory_id = required_subject_memory(proposal)?;
    for target_memory_id in required_target_memories(proposal)? {
        plan.relation_actions.push(
            MemoryRelation::new(
                proposal.scope_id.clone(),
                subject_memory_id.clone(),
                target_memory_id.clone(),
                relation_type,
                MemoryRelationSourceKind::Agent,
            )
            .with_source_proposal_id(proposal.id.clone()),
        );
    }
    Ok(())
}

fn proposal_audit_action(
    proposal: &MemoryProposal,
    memory_id: Option<MemoryId>,
    actor: &str,
) -> ProposalAuditAction {
    ProposalAuditAction {
        memory_id,
        action: format!("memory.proposal.{}", proposal.proposal_type.as_str()),
        actor: actor.to_string(),
        reason: proposal.reason.clone(),
        source_proposal_id: proposal.id.clone(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineVersion {
    pub memory_id: MemoryId,
    pub version: i32,
    pub title: String,
    pub body: String,
    pub change_kind: String,
    pub actor: String,
    pub reason: Option<String>,
    pub source_proposal_id: Option<ProposalId>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineAuditEvent {
    pub memory_id: Option<MemoryId>,
    pub action: String,
    pub actor: String,
    pub reason: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineEventKind {
    Version,
    Proposal,
    Relation,
    Audit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEvent {
    pub kind: TimelineEventKind,
    pub action: String,
    pub occurred_at: OffsetDateTime,
    pub memory_id: Option<MemoryId>,
    pub proposal_id: Option<ProposalId>,
    pub relation_id: Option<MemoryRelationId>,
    pub version: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryTimeline {
    pub memory_id: MemoryId,
    pub versions: Vec<TimelineVersion>,
    pub relations: Vec<MemoryRelation>,
    pub audit_events: Vec<TimelineAuditEvent>,
    pub proposals: Vec<MemoryProposal>,
    pub events: Vec<TimelineEvent>,
}

pub struct TimelineQueryService;

impl TimelineQueryService {
    pub fn build(
        memory_id: MemoryId,
        versions: Vec<TimelineVersion>,
        relations: Vec<MemoryRelation>,
        audit_events: Vec<TimelineAuditEvent>,
        proposals: Vec<MemoryProposal>,
    ) -> MemoryTimeline {
        let versions = versions
            .into_iter()
            .filter(|version| version.memory_id == memory_id)
            .collect::<Vec<_>>();
        let relations = relations
            .into_iter()
            .filter(|relation| {
                relation.from_memory_id == memory_id || relation.to_memory_id == memory_id
            })
            .collect::<Vec<_>>();
        let audit_events = audit_events
            .into_iter()
            .filter(|event| event.memory_id.as_ref() == Some(&memory_id))
            .collect::<Vec<_>>();
        let proposals = proposals
            .into_iter()
            .filter(|proposal| proposal_mentions_memory(proposal, &memory_id))
            .collect::<Vec<_>>();

        let mut events = Vec::new();
        for version in &versions {
            events.push(TimelineEvent {
                kind: TimelineEventKind::Version,
                action: format!("version.{}", version.change_kind),
                occurred_at: version.created_at,
                memory_id: Some(version.memory_id.clone()),
                proposal_id: version.source_proposal_id.clone(),
                relation_id: None,
                version: Some(version.version),
            });
        }
        for relation in &relations {
            events.push(TimelineEvent {
                kind: TimelineEventKind::Relation,
                action: format!("relation.{}", relation.relation_type.as_str()),
                occurred_at: relation.created_at,
                memory_id: Some(relation.from_memory_id.clone()),
                proposal_id: relation.source_proposal_id.clone(),
                relation_id: Some(relation.id.clone()),
                version: None,
            });
        }
        for audit in &audit_events {
            events.push(TimelineEvent {
                kind: TimelineEventKind::Audit,
                action: format!("audit.{}", audit.action),
                occurred_at: audit.created_at,
                memory_id: audit.memory_id.clone(),
                proposal_id: None,
                relation_id: None,
                version: None,
            });
        }
        for proposal in &proposals {
            append_proposal_events(proposal, &mut events);
        }
        events.sort_by(|left, right| {
            right
                .occurred_at
                .cmp(&left.occurred_at)
                .then_with(|| left.action.cmp(&right.action))
        });

        MemoryTimeline {
            memory_id,
            versions,
            relations,
            audit_events,
            proposals,
            events,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackError {
    NoVersions,
    TargetVersionNotFound,
    TargetVersionIsLatest,
    EmptyActor,
    EmptyReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackPlan {
    pub memory_id: MemoryId,
    pub target_version: i32,
    pub new_version: i32,
    pub title: String,
    pub body: String,
    pub change_kind: &'static str,
    pub actor: String,
    pub reason: String,
}

pub struct RollbackService;

impl RollbackService {
    pub fn plan(
        memory_id: MemoryId,
        target_version: i32,
        versions: &[TimelineVersion],
        actor: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<RollbackPlan, RollbackError> {
        let actor = non_empty_rollback_input(actor.into(), RollbackError::EmptyActor)?;
        let reason = non_empty_rollback_input(reason.into(), RollbackError::EmptyReason)?;
        let memory_versions = versions
            .iter()
            .filter(|version| version.memory_id == memory_id)
            .collect::<Vec<_>>();
        if memory_versions.is_empty() {
            return Err(RollbackError::NoVersions);
        }

        let latest_version = memory_versions
            .iter()
            .map(|version| version.version)
            .max()
            .expect("memory_versions is not empty");
        let target = memory_versions
            .iter()
            .find(|version| version.version == target_version)
            .ok_or(RollbackError::TargetVersionNotFound)?;
        if target.version == latest_version {
            return Err(RollbackError::TargetVersionIsLatest);
        }

        Ok(RollbackPlan {
            memory_id,
            target_version: target.version,
            new_version: latest_version + 1,
            title: target.title.clone(),
            body: target.body.clone(),
            change_kind: "rollback",
            actor,
            reason,
        })
    }
}

fn non_empty_rollback_input(value: String, error: RollbackError) -> Result<String, RollbackError> {
    if value.trim().is_empty() {
        return Err(error);
    }
    Ok(value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistillationPromptSegment {
    pub layer: &'static str,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistillationSessionOverride {
    pub prompt_text: String,
    pub focus_topics: Vec<String>,
    pub prefer_memory_kinds: Vec<MemoryKind>,
}

impl DistillationSessionOverride {
    pub fn new(prompt_text: impl Into<String>) -> Self {
        Self {
            prompt_text: prompt_text.into(),
            focus_topics: Vec::new(),
            prefer_memory_kinds: Vec::new(),
        }
    }

    pub fn add_focus_topic(mut self, topic: impl Into<String>) -> Self {
        push_unique_string(&mut self.focus_topics, topic.into());
        self
    }

    pub fn prefer_memory_kind(mut self, memory_kind: MemoryKind) -> Self {
        push_unique_memory_kind(&mut self.prefer_memory_kinds, memory_kind);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposedDistillationProfile {
    pub scope_id: ScopeId,
    pub prompt_segments: Vec<DistillationPromptSegment>,
    pub focus_topics: Vec<String>,
    pub prefer_memory_kinds: Vec<MemoryKind>,
    pub source_profile_ids: Vec<DistillationProfileId>,
    pub safety_rules: Vec<&'static str>,
}

pub struct DistillationProfileService;

impl DistillationProfileService {
    pub fn compose(
        scope_id: ScopeId,
        profiles: &[DistillationProfile],
        session_override: Option<DistillationSessionOverride>,
    ) -> ComposedDistillationProfile {
        let global_profile = latest_active_profile(profiles.iter().filter(|profile| {
            profile.profile_level == DistillationProfileLevel::UserGlobal
                && profile.scope_id.is_none()
        }));
        let project_profile = latest_active_profile(profiles.iter().filter(|profile| {
            profile.profile_level == DistillationProfileLevel::Project
                && profile.scope_id.as_ref() == Some(&scope_id)
        }));

        let mut prompt_segments = vec![DistillationPromptSegment {
            layer: "system_base",
            text: system_base_prompt().to_string(),
        }];
        let mut focus_topics = Vec::new();
        let mut prefer_memory_kinds = Vec::new();
        let mut source_profile_ids = Vec::new();

        if let Some(profile) = global_profile {
            append_profile(
                profile,
                "user_global",
                &mut prompt_segments,
                &mut focus_topics,
                &mut prefer_memory_kinds,
                &mut source_profile_ids,
            );
        }
        if let Some(profile) = project_profile {
            append_profile(
                profile,
                "project",
                &mut prompt_segments,
                &mut focus_topics,
                &mut prefer_memory_kinds,
                &mut source_profile_ids,
            );
        }
        if let Some(session_override) = session_override {
            if !session_override.prompt_text.trim().is_empty() {
                prompt_segments.push(DistillationPromptSegment {
                    layer: "session_override",
                    text: session_override.prompt_text,
                });
            }
            for topic in session_override.focus_topics {
                push_unique_string(&mut focus_topics, topic);
            }
            for memory_kind in session_override.prefer_memory_kinds {
                push_unique_memory_kind(&mut prefer_memory_kinds, memory_kind);
            }
        }

        ComposedDistillationProfile {
            scope_id,
            prompt_segments,
            focus_topics,
            prefer_memory_kinds,
            source_profile_ids,
            safety_rules: system_safety_rules(),
        }
    }
}

fn latest_active_profile<'a>(
    profiles: impl Iterator<Item = &'a DistillationProfile>,
) -> Option<&'a DistillationProfile> {
    profiles
        .filter(|profile| profile.status == DistillationProfileStatus::Active)
        .max_by_key(|profile| profile.updated_at)
}

fn append_profile(
    profile: &DistillationProfile,
    layer: &'static str,
    prompt_segments: &mut Vec<DistillationPromptSegment>,
    focus_topics: &mut Vec<String>,
    prefer_memory_kinds: &mut Vec<MemoryKind>,
    source_profile_ids: &mut Vec<DistillationProfileId>,
) {
    prompt_segments.push(DistillationPromptSegment {
        layer,
        text: profile.prompt_text.clone(),
    });
    for topic in &profile.focus_topics {
        push_unique_string(focus_topics, topic.clone());
    }
    for memory_kind in &profile.prefer_memory_kinds {
        push_unique_memory_kind(prefer_memory_kinds, *memory_kind);
    }
    source_profile_ids.push(profile.id.clone());
}

fn push_unique_string(values: &mut Vec<String>, value: String) {
    if !value.trim().is_empty() && !values.contains(&value) {
        values.push(value);
    }
}

fn push_unique_memory_kind(values: &mut Vec<MemoryKind>, value: MemoryKind) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn system_base_prompt() -> &'static str {
    "Preserve evidence, confidence, scope boundaries, and proposal review for conflicts."
}

fn system_safety_rules() -> Vec<&'static str> {
    vec![
        "must preserve evidence references",
        "must label confidence",
        "must not present inference as fact",
        "must not bypass scope, sensitivity, or recall guard",
        "must route conflicts and supersedes through proposal governance",
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistillationPreviewError {
    EmptyInput,
    MissingEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistillationCandidate {
    pub title: String,
    pub memory_kind: MemoryKind,
    pub body: String,
    pub confidence: &'static str,
    pub why_keep: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DistillationPreview {
    pub run: DistillationRun,
    pub candidates: Vec<DistillationCandidate>,
    pub discarded: Vec<String>,
    pub warnings: Vec<String>,
}

pub struct DistillationPreviewService;

impl DistillationPreviewService {
    pub fn preview(
        scope_id: ScopeId,
        input: &str,
        evidence_refs: &[String],
        profile: &ComposedDistillationProfile,
    ) -> Result<DistillationPreview, DistillationPreviewError> {
        let body = input.trim();
        if body.is_empty() {
            return Err(DistillationPreviewError::EmptyInput);
        }
        if evidence_refs.is_empty() {
            return Err(DistillationPreviewError::MissingEvidence);
        }

        let run = DistillationRun::new(None, scope_id, stable_input_hash(body), true)
            .expect("trimmed input produces non-empty input hash");
        let memory_kind = profile
            .prefer_memory_kinds
            .first()
            .copied()
            .unwrap_or(MemoryKind::Summary);

        Ok(DistillationPreview {
            run,
            candidates: vec![DistillationCandidate {
                title: candidate_title(body),
                memory_kind,
                body: body.to_string(),
                confidence: "explicit",
                why_keep: "profile-guided preview candidate".to_string(),
                evidence_refs: evidence_refs.to_vec(),
            }],
            discarded: Vec::new(),
            warnings: vec![
                "preview only; candidates must be approved before writing memory".to_string(),
            ],
        })
    }
}

fn stable_input_hash(input: &str) -> String {
    format!(
        "len:{}:{}",
        input.len(),
        input.bytes().fold(0_u64, |sum, byte| sum + byte as u64)
    )
}

fn candidate_title(input: &str) -> String {
    input
        .split_whitespace()
        .take(6)
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewActorKind {
    User,
    Agent,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewPolicyAction {
    Approve,
    Apply,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewPolicyDecision {
    Allow,
    RequireUserApproval,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewPolicyInput {
    pub review_level: ReviewLevel,
    pub action: ReviewPolicyAction,
    pub actor_kind: ReviewActorKind,
    pub has_user_authorization: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewPolicyEvaluation {
    pub decision: ReviewPolicyDecision,
    pub reason: String,
}

pub struct ReviewPolicyService;

impl ReviewPolicyService {
    pub fn evaluate(input: ReviewPolicyInput) -> ReviewPolicyEvaluation {
        match input.action {
            ReviewPolicyAction::Approve => evaluate_review_approval(input),
            ReviewPolicyAction::Apply => evaluate_review_apply(input),
        }
    }
}

fn evaluate_review_approval(input: ReviewPolicyInput) -> ReviewPolicyEvaluation {
    match input.review_level {
        ReviewLevel::Auto => review_policy_evaluation(
            ReviewPolicyDecision::Allow,
            "auto proposal may be approved automatically",
        ),
        ReviewLevel::Suggested => match input.actor_kind {
            ReviewActorKind::User | ReviewActorKind::Agent => review_policy_evaluation(
                ReviewPolicyDecision::Allow,
                "suggested proposal may be approved by user or delegated agent",
            ),
            ReviewActorKind::System => review_policy_evaluation(
                ReviewPolicyDecision::RequireUserApproval,
                "suggested proposal needs user or delegated agent approval",
            ),
        },
        ReviewLevel::Required => match input.actor_kind {
            ReviewActorKind::User => review_policy_evaluation(
                ReviewPolicyDecision::Allow,
                "required proposal has explicit user approval",
            ),
            ReviewActorKind::Agent if input.has_user_authorization => review_policy_evaluation(
                ReviewPolicyDecision::Allow,
                "required proposal has explicit user authorization for agent approval",
            ),
            ReviewActorKind::Agent => review_policy_evaluation(
                ReviewPolicyDecision::RequireUserApproval,
                "required proposal needs explicit user approval",
            ),
            ReviewActorKind::System => review_policy_evaluation(
                ReviewPolicyDecision::Deny,
                "system cannot approve required proposal",
            ),
        },
        ReviewLevel::Blocked => review_policy_evaluation(
            ReviewPolicyDecision::Deny,
            "blocked proposal cannot be approved",
        ),
    }
}

fn evaluate_review_apply(input: ReviewPolicyInput) -> ReviewPolicyEvaluation {
    match input.review_level {
        ReviewLevel::Auto | ReviewLevel::Suggested => review_policy_evaluation(
            ReviewPolicyDecision::Allow,
            "approved low-risk proposal may be applied",
        ),
        ReviewLevel::Required => match input.actor_kind {
            ReviewActorKind::System | ReviewActorKind::User => review_policy_evaluation(
                ReviewPolicyDecision::Allow,
                "approved required proposal may be applied by system or user",
            ),
            ReviewActorKind::Agent if input.has_user_authorization => review_policy_evaluation(
                ReviewPolicyDecision::Allow,
                "approved required proposal has explicit user authorization for agent apply",
            ),
            ReviewActorKind::Agent => review_policy_evaluation(
                ReviewPolicyDecision::RequireUserApproval,
                "agent needs explicit user authorization to apply required proposal",
            ),
        },
        ReviewLevel::Blocked => review_policy_evaluation(
            ReviewPolicyDecision::Deny,
            "blocked proposal cannot be applied",
        ),
    }
}

fn review_policy_evaluation(
    decision: ReviewPolicyDecision,
    reason: &'static str,
) -> ReviewPolicyEvaluation {
    ReviewPolicyEvaluation {
        decision,
        reason: reason.to_string(),
    }
}

fn proposal_mentions_memory(proposal: &MemoryProposal, memory_id: &MemoryId) -> bool {
    proposal.subject_memory_id.as_ref() == Some(memory_id)
        || proposal
            .target_memory_ids
            .iter()
            .any(|target_memory_id| target_memory_id == memory_id)
}

fn append_proposal_events(proposal: &MemoryProposal, events: &mut Vec<TimelineEvent>) {
    events.push(proposal_event(
        proposal,
        "proposal.opened",
        proposal.created_at,
    ));

    if let Some(decided_at) = proposal.decided_at {
        let action = match proposal.status {
            ProposalStatus::Approved | ProposalStatus::Applied => "proposal.approved",
            ProposalStatus::Rejected => "proposal.rejected",
            ProposalStatus::Canceled => "proposal.canceled",
            ProposalStatus::Expired => "proposal.expired",
            ProposalStatus::Open => "proposal.decided",
        };
        events.push(proposal_event(proposal, action, decided_at));
    }

    if let Some(applied_at) = proposal.applied_at {
        events.push(proposal_event(proposal, "proposal.applied", applied_at));
    }
}

fn proposal_event(
    proposal: &MemoryProposal,
    action: &'static str,
    occurred_at: OffsetDateTime,
) -> TimelineEvent {
    TimelineEvent {
        kind: TimelineEventKind::Proposal,
        action: action.to_string(),
        occurred_at,
        memory_id: proposal.subject_memory_id.clone(),
        proposal_id: Some(proposal.id.clone()),
        relation_id: None,
        version: None,
    }
}

fn binding_matches(input: &ProjectIdentityInput, binding: &ProjectIdentityBinding) -> bool {
    match binding.binding_kind {
        ProjectBindingKind::RepoRoot => input.repo_root.as_ref() == Some(&binding.binding_value),
        ProjectBindingKind::RemoteUrl => input.remote_url.as_ref() == Some(&binding.binding_value),
        ProjectBindingKind::SourceRefPrefix => input
            .source_refs
            .iter()
            .any(|source_ref| source_ref.starts_with(&binding.binding_value)),
        ProjectBindingKind::Alias => input.alias.as_ref() == Some(&binding.binding_value),
    }
}

fn first_project_signal(input: &ProjectIdentityInput) -> Option<String> {
    input
        .alias
        .clone()
        .or_else(|| input.repo_root.clone().and_then(last_path_segment))
        .or_else(|| input.remote_url.clone().and_then(last_path_segment))
        .or_else(|| input.source_refs.first().cloned())
}

fn last_path_segment(value: String) -> Option<String> {
    value
        .trim_end_matches('/')
        .rsplit(['/', ':'])
        .next()
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.trim_end_matches(".git").to_string())
}

fn assessment(
    relationship: MemoryRelationship,
    confidence: f32,
    review_level: ReviewLevel,
    existing: &MemoryRecord,
    evidence: &str,
) -> RelationshipAssessment {
    RelationshipAssessment {
        relationship,
        confidence,
        review_level,
        matched_memory_ids: vec![MemoryId::from_string(existing.native_id.clone())],
        evidence: vec![evidence.to_string()],
    }
}

fn is_exact_duplicate(incoming: &MemoryRecord, existing: &MemoryRecord) -> bool {
    normalize(&incoming.title) == normalize(&existing.title)
        && normalized_content(incoming) == normalized_content(existing)
        && incoming.source_ref.is_some()
        && incoming.source_ref == existing.source_ref
}

fn is_supersede_candidate(incoming: &MemoryRecord, existing: &MemoryRecord) -> bool {
    incoming.record_type == MemoryRecordType::Decision
        && existing.record_type == MemoryRecordType::Decision
        && normalize(&incoming.title) == normalize(&existing.title)
        && contains_any(
            &normalized_content(incoming),
            &[
                "replace",
                "replaces",
                "supersede",
                "supersedes",
                "替代",
                "改用",
            ],
        )
}

fn is_conflict_candidate(incoming: &MemoryRecord, existing: &MemoryRecord) -> bool {
    incoming.record_type == MemoryRecordType::Fact
        && existing.record_type == MemoryRecordType::Fact
        && normalize(&incoming.title) == normalize(&existing.title)
        && has_opposite_polarity(&normalized_content(incoming), &normalized_content(existing))
}

fn is_near_duplicate(incoming: &MemoryRecord, existing: &MemoryRecord) -> bool {
    incoming.record_type == existing.record_type
        && normalize(&incoming.title) == normalize(&existing.title)
}

fn has_opposite_polarity(left: &str, right: &str) -> bool {
    (left.contains("enabled") && right.contains("disabled"))
        || (left.contains("disabled") && right.contains("enabled"))
        || (left.contains("true") && right.contains("false"))
        || (left.contains("false") && right.contains("true"))
        || (left.contains("必须") && right.contains("不再"))
        || (left.contains("不再") && right.contains("必须"))
}

fn normalized_content(record: &MemoryRecord) -> String {
    record.content.as_deref().map(normalize).unwrap_or_default()
}

fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

#[cfg(test)]
#[path = "v28_tests.rs"]
mod v28_tests;
