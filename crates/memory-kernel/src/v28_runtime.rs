use anyhow::{Result, anyhow, bail};
use memory_domain::{
    DistillationProfile, DistillationProfileId, DistillationProfileLevel,
    DistillationProfileStatus, Memory, MemoryId, MemoryKind, MemoryProposal, ProposalId,
    ProposalStatus, ProposalType, ScopeId,
};
use memory_observability::{
    record_v28_distillation_preview, record_v28_proposal_decision, record_v28_proposal_observation,
    record_v28_rollback,
};
use serde_json::json;

use crate::{
    ComposedDistillationProfile, DistillationPreview, DistillationPreviewError,
    DistillationPreviewService, DistillationProfileService, DistillationSessionOverride, Kernel,
    MemoryTimeline, ProposalExecutionError, ProposalExecutor, ReviewActorKind, ReviewPolicyAction,
    ReviewPolicyDecision, ReviewPolicyInput, ReviewPolicyService, RollbackError, RollbackPlan,
    RollbackService, TimelineAuditEvent, TimelineQueryService, TimelineVersion,
};

#[derive(Debug, Clone)]
pub struct ListMemoryProposalsRequest {
    pub scope_id: Option<ScopeId>,
    pub limit: usize,
}

impl ListMemoryProposalsRequest {
    pub fn new(scope_id: Option<ScopeId>) -> Self {
        Self {
            scope_id,
            limit: 50,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GetMemoryProposalRequest {
    pub proposal_id: ProposalId,
}

#[derive(Debug, Clone)]
pub struct ApproveMemoryProposalRequest {
    pub proposal_id: ProposalId,
    pub actor: String,
    pub actor_kind: ReviewActorKind,
    pub has_user_authorization: bool,
}

#[derive(Debug, Clone)]
pub struct RejectMemoryProposalRequest {
    pub proposal_id: ProposalId,
    pub actor: String,
}

#[derive(Debug, Clone)]
pub struct ApplyMemoryProposalRequest {
    pub proposal_id: ProposalId,
    pub actor: String,
    pub actor_kind: ReviewActorKind,
    pub has_user_authorization: bool,
}

#[derive(Debug, Clone)]
pub struct ListMemoryVersionsRequest {
    pub scope_id: ScopeId,
    pub memory_id: MemoryId,
    pub limit: usize,
}

impl ListMemoryVersionsRequest {
    pub fn new(scope_id: ScopeId, memory_id: MemoryId) -> Self {
        Self {
            scope_id,
            memory_id,
            limit: 50,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RollbackMemoryRequest {
    pub scope_id: ScopeId,
    pub memory_id: MemoryId,
    pub target_version: i32,
    pub actor: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct RollbackMemoryResult {
    pub plan: RollbackPlan,
    pub memory: Memory,
}

#[derive(Debug, Clone)]
pub struct GetMemoryTimelineRequest {
    pub scope_id: ScopeId,
    pub memory_id: MemoryId,
    pub limit: usize,
}

impl GetMemoryTimelineRequest {
    pub fn new(scope_id: ScopeId, memory_id: MemoryId) -> Self {
        Self {
            scope_id,
            memory_id,
            limit: 50,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ListDistillationProfilesRequest {
    pub scope_id: Option<ScopeId>,
    pub limit: usize,
}

impl ListDistillationProfilesRequest {
    pub fn new(scope_id: Option<ScopeId>) -> Self {
        Self {
            scope_id,
            limit: 50,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpsertDistillationProfileRequest {
    pub profile_id: DistillationProfileId,
    pub scope_id: Option<ScopeId>,
    pub profile_level: DistillationProfileLevel,
    pub status: DistillationProfileStatus,
    pub name: String,
    pub prompt_text: String,
    pub focus_topics: Vec<String>,
    pub prefer_memory_kinds: Vec<MemoryKind>,
    pub created_by: String,
}

#[derive(Debug, Clone)]
pub struct PreviewDistillationRequest {
    pub scope_id: ScopeId,
    pub input: String,
    pub evidence_refs: Vec<String>,
    pub session_override: Option<DistillationSessionOverride>,
}

#[derive(Debug, Clone)]
pub struct PreviewDistillationResult {
    pub preview: DistillationPreview,
    pub profile: ComposedDistillationProfile,
    pub stored_in_pg: bool,
}

impl Kernel {
    pub async fn list_memory_proposals(
        &self,
        request: ListMemoryProposalsRequest,
    ) -> Result<Vec<MemoryProposal>> {
        let pg_store = self
            .pg_store
            .as_ref()
            .ok_or_else(|| anyhow!("postgres store is required for memory proposals"))?;

        let proposals = pg_store
            .list_memory_proposals(request.scope_id.as_ref(), request.limit as i64)
            .await?;
        let open_count = proposals
            .iter()
            .filter(|proposal| proposal.status == ProposalStatus::Open)
            .count();
        let conflict_count = proposals
            .iter()
            .filter(|proposal| proposal.proposal_type == ProposalType::ConflictMark)
            .count();
        record_v28_proposal_observation(proposals.len(), open_count, conflict_count);

        Ok(proposals)
    }

    pub async fn get_memory_proposal(
        &self,
        request: GetMemoryProposalRequest,
    ) -> Result<Option<MemoryProposal>> {
        let pg_store = self
            .pg_store
            .as_ref()
            .ok_or_else(|| anyhow!("postgres store is required for memory proposals"))?;

        pg_store.get_memory_proposal(&request.proposal_id).await
    }

    pub async fn approve_memory_proposal(
        &self,
        request: ApproveMemoryProposalRequest,
    ) -> Result<Option<MemoryProposal>> {
        let Some(mut proposal) = self
            .get_memory_proposal(GetMemoryProposalRequest {
                proposal_id: request.proposal_id,
            })
            .await?
        else {
            record_v28_proposal_decision("approved", false);
            return Ok(None);
        };

        let evaluation = ReviewPolicyService::evaluate(ReviewPolicyInput {
            review_level: proposal.review_level,
            action: ReviewPolicyAction::Approve,
            actor_kind: request.actor_kind,
            has_user_authorization: request.has_user_authorization,
        });
        if evaluation.decision != ReviewPolicyDecision::Allow {
            record_v28_proposal_decision("approved", false);
            bail!("proposal approval denied by policy: {}", evaluation.reason);
        }

        if let Err(error) = proposal.approve(request.actor) {
            record_v28_proposal_decision("approved", false);
            return Err(error.into());
        }
        if let Err(error) = self
            .pg_store
            .as_ref()
            .expect("get_memory_proposal requires postgres store")
            .upsert_memory_proposal(&proposal)
            .await
        {
            record_v28_proposal_decision("approved", false);
            return Err(error);
        }
        record_v28_proposal_decision("approved", true);
        Ok(Some(proposal))
    }

    pub async fn reject_memory_proposal(
        &self,
        request: RejectMemoryProposalRequest,
    ) -> Result<Option<MemoryProposal>> {
        let Some(mut proposal) = self
            .get_memory_proposal(GetMemoryProposalRequest {
                proposal_id: request.proposal_id,
            })
            .await?
        else {
            record_v28_proposal_decision("rejected", false);
            return Ok(None);
        };

        if let Err(error) = proposal.reject(request.actor) {
            record_v28_proposal_decision("rejected", false);
            return Err(error.into());
        }
        if let Err(error) = self
            .pg_store
            .as_ref()
            .expect("get_memory_proposal requires postgres store")
            .upsert_memory_proposal(&proposal)
            .await
        {
            record_v28_proposal_decision("rejected", false);
            return Err(error);
        }
        record_v28_proposal_decision("rejected", true);
        Ok(Some(proposal))
    }

    pub async fn apply_memory_proposal(
        &self,
        request: ApplyMemoryProposalRequest,
    ) -> Result<Option<MemoryProposal>> {
        let Some(mut proposal) = self
            .get_memory_proposal(GetMemoryProposalRequest {
                proposal_id: request.proposal_id,
            })
            .await?
        else {
            record_v28_proposal_decision("applied", false);
            return Ok(None);
        };

        let evaluation = ReviewPolicyService::evaluate(ReviewPolicyInput {
            review_level: proposal.review_level,
            action: ReviewPolicyAction::Apply,
            actor_kind: request.actor_kind,
            has_user_authorization: request.has_user_authorization,
        });
        if evaluation.decision != ReviewPolicyDecision::Allow {
            record_v28_proposal_decision("applied", false);
            bail!("proposal apply denied by policy: {}", evaluation.reason);
        }

        let plan = match ProposalExecutor::plan(&proposal, request.actor.clone()) {
            Ok(plan) => plan,
            Err(error) => {
                record_v28_proposal_decision("applied", false);
                return Err(proposal_apply_error(error));
            }
        };
        let result = async {
            let pg_store = self
                .pg_store
                .as_ref()
                .expect("get_memory_proposal requires postgres store");

            for relation in plan.relation_actions {
                pg_store.insert_memory_relation(&relation).await?;
            }
            for version_action in plan.version_actions {
                let memory = self
                    .get_memory(proposal.scope_id.clone(), version_action.memory_id.clone())
                    .await?
                    .ok_or_else(|| anyhow!("proposal apply memory not found"))?;
                pg_store
                    .insert_memory_version_snapshot(
                        &version_action.memory_id,
                        &memory.title,
                        &memory.body,
                        version_action.change_kind,
                        &request.actor,
                        Some(&version_action.reason),
                        Some(&version_action.source_proposal_id),
                    )
                    .await?;
            }
            for audit_action in plan.audit_actions {
                let memory_id = audit_action
                    .memory_id
                    .ok_or_else(|| anyhow!("proposal audit memory is required"))?;
                pg_store
                    .insert_lifecycle_audit_event(
                        &proposal.scope_id,
                        &memory_id,
                        &audit_action.action,
                        &audit_action.actor,
                        None,
                        None,
                        Some(&audit_action.reason),
                        time::OffsetDateTime::now_utc(),
                    )
                    .await?;
            }

            if plan.mark_proposal_applied {
                proposal.mark_applied()?;
                pg_store.upsert_memory_proposal(&proposal).await?;
            }
            Ok(Some(proposal))
        }
        .await;
        record_v28_proposal_decision("applied", result.is_ok());
        result
    }

    pub async fn list_memory_versions(
        &self,
        request: ListMemoryVersionsRequest,
    ) -> Result<Vec<TimelineVersion>> {
        let pg_store = self
            .pg_store
            .as_ref()
            .ok_or_else(|| anyhow!("postgres store is required for memory timeline"))?;

        if self
            .get_memory(request.scope_id, request.memory_id.clone())
            .await?
            .is_none()
        {
            return Ok(Vec::new());
        }

        Ok(pg_store
            .list_memory_versions(&request.memory_id, request.limit as i64)
            .await?
            .into_iter()
            .map(timeline_version_from_record)
            .collect())
    }

    pub async fn rollback_memory(
        &self,
        request: RollbackMemoryRequest,
    ) -> Result<RollbackMemoryResult> {
        let Some(pg_store) = self.pg_store.as_ref() else {
            record_v28_rollback(false);
            return Err(anyhow!("postgres store is required for memory rollback"));
        };
        let result = async {
            let mut memory = self
                .get_memory(request.scope_id.clone(), request.memory_id.clone())
                .await?
                .ok_or_else(|| anyhow!("memory not found"))?;
            let versions = self
                .list_memory_versions(ListMemoryVersionsRequest::new(
                    request.scope_id.clone(),
                    request.memory_id.clone(),
                ))
                .await?;
            let plan = RollbackService::plan(
                request.memory_id,
                request.target_version,
                &versions,
                request.actor,
                request.reason,
            )
            .map_err(rollback_error)?;

            memory.title = plan.title.clone();
            memory.body = plan.body.clone();
            memory.updated_at = time::OffsetDateTime::now_utc();
            pg_store.upsert_memory(&memory).await?;
            pg_store
                .update_memory_version_metadata(
                    &plan.memory_id,
                    plan.new_version,
                    plan.change_kind,
                    &plan.actor,
                    Some(&plan.reason),
                    None,
                )
                .await?;
            pg_store
                .insert_lifecycle_audit_event(
                    &request.scope_id,
                    &plan.memory_id,
                    "memory.rollback",
                    &plan.actor,
                    None,
                    None,
                    Some(&plan.reason),
                    time::OffsetDateTime::now_utc(),
                )
                .await?;

            Ok(RollbackMemoryResult { plan, memory })
        }
        .await;
        record_v28_rollback(result.is_ok());
        result
    }

    pub async fn get_memory_timeline(
        &self,
        request: GetMemoryTimelineRequest,
    ) -> Result<Option<MemoryTimeline>> {
        let pg_store = self
            .pg_store
            .as_ref()
            .ok_or_else(|| anyhow!("postgres store is required for memory timeline"))?;

        if self
            .get_memory(request.scope_id.clone(), request.memory_id.clone())
            .await?
            .is_none()
        {
            return Ok(None);
        }

        let versions = pg_store
            .list_memory_versions(&request.memory_id, request.limit as i64)
            .await?
            .into_iter()
            .map(timeline_version_from_record)
            .collect::<Vec<_>>();
        let relations = pg_store
            .list_memory_relations(
                &request.scope_id,
                Some(&request.memory_id),
                request.limit as i64,
            )
            .await?;
        let proposals = pg_store
            .list_memory_proposals(Some(&request.scope_id), request.limit as i64)
            .await?;
        let audit_events = self
            .list_lifecycle_audit_events(
                Some(request.scope_id.clone()),
                Some(request.memory_id.clone()),
                request.limit,
            )
            .await?
            .into_iter()
            .map(timeline_audit_event_from_record)
            .collect::<Vec<_>>();

        Ok(Some(TimelineQueryService::build(
            request.memory_id,
            versions,
            relations,
            audit_events,
            proposals,
        )))
    }

    pub async fn list_distillation_profiles(
        &self,
        request: ListDistillationProfilesRequest,
    ) -> Result<Vec<DistillationProfile>> {
        let pg_store = self
            .pg_store
            .as_ref()
            .ok_or_else(|| anyhow!("postgres store is required for distillation profile"))?;

        pg_store
            .list_distillation_profiles(request.scope_id.as_ref(), request.limit as i64)
            .await
    }

    pub async fn upsert_distillation_profile(
        &self,
        request: UpsertDistillationProfileRequest,
    ) -> Result<DistillationProfile> {
        if request.profile_level == DistillationProfileLevel::Project && request.scope_id.is_none()
        {
            bail!("project profile requires scope_id");
        }

        let pg_store = self
            .pg_store
            .as_ref()
            .ok_or_else(|| anyhow!("postgres store is required for distillation profile"))?;

        let existing = pg_store
            .get_distillation_profile(&request.profile_id)
            .await?;
        let created_by = existing
            .as_ref()
            .map(|profile| profile.created_by.clone())
            .unwrap_or(request.created_by);

        let mut profile = match request.profile_level {
            DistillationProfileLevel::UserGlobal => {
                DistillationProfile::new_global(request.name, request.prompt_text, created_by)?
            }
            DistillationProfileLevel::Project => DistillationProfile::new_project(
                request
                    .scope_id
                    .clone()
                    .expect("validated project scope_id"),
                request.name,
                request.prompt_text,
                created_by,
            )?,
        };

        profile.id = request.profile_id;
        for topic in request.focus_topics {
            profile = profile.add_focus_topic(topic)?;
        }
        for memory_kind in request.prefer_memory_kinds {
            profile = profile.prefer_memory_kind(memory_kind);
        }
        match request.status {
            DistillationProfileStatus::Active => profile.activate(),
            DistillationProfileStatus::Archived => profile.archive(),
        }
        if let Some(existing) = existing {
            profile.created_at = existing.created_at;
            profile.created_by = existing.created_by;
        }
        profile.updated_at = time::OffsetDateTime::now_utc();

        pg_store.upsert_distillation_profile(&profile).await?;
        Ok(profile)
    }

    pub async fn preview_distillation(
        &self,
        request: PreviewDistillationRequest,
    ) -> Result<PreviewDistillationResult> {
        let result = async {
            let stored_profiles = match &self.pg_store {
                Some(pg_store) => pg_store.list_distillation_profiles(None, 500).await?,
                None => Vec::new(),
            };
            let profile = DistillationProfileService::compose(
                request.scope_id.clone(),
                &stored_profiles,
                request.session_override,
            );
            let mut preview = DistillationPreviewService::preview(
                request.scope_id,
                &request.input,
                &request.evidence_refs,
                &profile,
            )
            .map_err(distillation_preview_error)?;

            let mut stored_in_pg = false;
            if let Some(pg_store) = &self.pg_store {
                preview.run.profile_id = match profile.source_profile_ids.as_slice() {
                    [profile_id] => Some(profile_id.clone()),
                    _ => None,
                };
                pg_store
                    .insert_distillation_run(
                        &preview.run,
                        distillation_preview_json(&preview, &profile),
                    )
                    .await?;
                stored_in_pg = true;
            }

            Ok(PreviewDistillationResult {
                preview,
                profile,
                stored_in_pg,
            })
        }
        .await;
        match &result {
            Ok(result) => record_v28_distillation_preview(true, result.preview.candidates.len()),
            Err(_) => record_v28_distillation_preview(false, 0),
        }
        result
    }
}

fn distillation_preview_error(error: DistillationPreviewError) -> anyhow::Error {
    match error {
        DistillationPreviewError::EmptyInput => anyhow!("distillation input cannot be empty"),
        DistillationPreviewError::MissingEvidence => {
            anyhow!("distillation evidence_refs cannot be empty")
        }
    }
}

fn proposal_apply_error(error: ProposalExecutionError) -> anyhow::Error {
    match error {
        ProposalExecutionError::ProposalNotApproved => {
            anyhow!("proposal must be approved before apply")
        }
        ProposalExecutionError::MissingSubjectMemory => {
            anyhow!("proposal apply requires subject memory")
        }
        ProposalExecutionError::MissingTargetMemory => {
            anyhow!("proposal apply requires target memory")
        }
    }
}

fn rollback_error(error: RollbackError) -> anyhow::Error {
    match error {
        RollbackError::NoVersions => anyhow!("Invalid rollback: memory has no versions"),
        RollbackError::TargetVersionNotFound => {
            anyhow!("Invalid rollback target version: not found")
        }
        RollbackError::TargetVersionIsLatest => {
            anyhow!("Invalid rollback target version: already latest")
        }
        RollbackError::EmptyActor => anyhow!("rollback actor cannot be empty"),
        RollbackError::EmptyReason => anyhow!("rollback reason cannot be empty"),
    }
}

fn timeline_version_from_record(record: memory_store_pg::MemoryVersionRecord) -> TimelineVersion {
    TimelineVersion {
        memory_id: record.memory_id,
        version: record.version,
        title: record.title,
        body: record.body,
        change_kind: record.change_kind,
        actor: record.actor,
        reason: record.reason,
        source_proposal_id: record.source_proposal_id,
        created_at: record.created_at,
    }
}

fn timeline_audit_event_from_record(
    record: memory_store_pg::LifecycleAuditEventRecord,
) -> TimelineAuditEvent {
    TimelineAuditEvent {
        memory_id: Some(MemoryId::from_string(record.memory_id)),
        action: record.action,
        actor: record.actor,
        reason: record.reason,
        created_at: record.created_at,
    }
}

fn distillation_preview_json(
    preview: &DistillationPreview,
    profile: &ComposedDistillationProfile,
) -> serde_json::Value {
    json!({
        "run_id": preview.run.id.as_str(),
        "profile_source_ids": profile
            .source_profile_ids
            .iter()
            .map(|profile_id| profile_id.as_str())
            .collect::<Vec<_>>(),
        "focus_topics": profile.focus_topics,
        "prefer_memory_kinds": profile
            .prefer_memory_kinds
            .iter()
            .copied()
            .map(memory_kind_label)
            .collect::<Vec<_>>(),
        "candidates": preview
            .candidates
            .iter()
            .map(|candidate| {
                json!({
                    "title": candidate.title,
                    "memory_kind": memory_kind_label(candidate.memory_kind),
                    "body": candidate.body,
                    "confidence": candidate.confidence,
                    "why_keep": candidate.why_keep,
                    "evidence_refs": candidate.evidence_refs,
                })
            })
            .collect::<Vec<_>>(),
        "discarded": preview.discarded,
        "warnings": preview.warnings,
    })
}

fn memory_kind_label(kind: MemoryKind) -> &'static str {
    match kind {
        MemoryKind::Fact => "fact",
        MemoryKind::Preference => "preference",
        MemoryKind::Decision => "decision",
        MemoryKind::Procedure => "procedure",
        MemoryKind::Constraint => "constraint",
        MemoryKind::Risk => "risk",
        MemoryKind::Summary => "summary",
        MemoryKind::Insight => "insight",
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ApplyMemoryProposalRequest, ApproveMemoryProposalRequest, DistillationSessionOverride,
        GetMemoryProposalRequest, GetMemoryTimelineRequest, Kernel,
        ListDistillationProfilesRequest, ListMemoryProposalsRequest, ListMemoryVersionsRequest,
        MemoryKind, PreviewDistillationRequest, RejectMemoryProposalRequest, RollbackMemoryRequest,
        ScopeId, UpsertDistillationProfileRequest,
    };
    use memory_domain::{
        DistillationProfileLevel, DistillationProfileStatus, Memory, MemoryId, MemoryProposal,
        MemoryRelation, MemoryRelationSourceKind, MemoryRelationType, ProposalStatus, ProposalType,
        ReviewLevel,
    };
    use memory_observability::metrics_snapshot;
    use sqlx::Row;
    use std::env;
    use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
    use std::time::Duration;
    use tempfile::tempdir;

    fn test_database_url() -> String {
        env::var("MEAT_MEMORY_TEST_DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@127.0.0.1:5433/meat_memory_dev".into()
        })
    }

    fn local_pg_test_port_available() -> bool {
        let authority = test_database_url();
        let authority = authority
            .split('@')
            .nth(1)
            .map(|tail| tail.split('/').next().unwrap_or("").to_string())
            .filter(|authority| !authority.is_empty())
            .unwrap_or_else(|| "127.0.0.1:5433".to_string());
        let addr = authority
            .to_socket_addrs()
            .ok()
            .and_then(|mut addrs| addrs.next())
            .unwrap_or_else(|| "127.0.0.1:5433".parse::<SocketAddr>().unwrap());
        TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
    }

    async fn seed_active_memory(
        kernel: &Kernel,
        scope_id: &ScopeId,
        title: &str,
        body: &str,
        kind: MemoryKind,
    ) -> Memory {
        let store = kernel.pg_store.as_ref().unwrap();
        store
            .seed_scope(
                scope_id,
                scope_id.as_str(),
                &format!("default/scopes/{}", scope_id.as_str()),
            )
            .await
            .unwrap();

        let mut memory = Memory::new(scope_id.clone(), kind, title, body).unwrap();
        memory.activate().unwrap();
        store.insert_memory(&memory).await.unwrap();
        memory
    }

    #[tokio::test]
    async fn proposal_review_runtime_lists_inspects_approves_and_rejects_pg() {
        if !local_pg_test_port_available() {
            return;
        }
        let kernel = Kernel::builder()
            .with_postgres_url(&test_database_url())
            .await
            .unwrap()
            .build()
            .unwrap();
        let scope_id = ScopeId::new();
        kernel
            .pg_store
            .as_ref()
            .unwrap()
            .seed_scope(
                &scope_id,
                scope_id.as_str(),
                &format!("default/scopes/{}", scope_id.as_str()),
            )
            .await
            .unwrap();

        let mut approve_proposal = MemoryProposal::new(
            scope_id.clone(),
            ProposalType::Supersede,
            ReviewLevel::Required,
            "runtime proposal requires user approval",
        )
        .unwrap();
        approve_proposal
            .add_evidence("agent cannot approve without explicit authorization".to_string())
            .unwrap();
        kernel
            .pg_store
            .as_ref()
            .unwrap()
            .upsert_memory_proposal(&approve_proposal)
            .await
            .unwrap();
        let reject_proposal = MemoryProposal::new(
            scope_id.clone(),
            ProposalType::ConflictMark,
            ReviewLevel::Suggested,
            "runtime proposal should be rejected",
        )
        .unwrap();
        kernel
            .pg_store
            .as_ref()
            .unwrap()
            .upsert_memory_proposal(&reject_proposal)
            .await
            .unwrap();

        let metrics_before = metrics_snapshot();
        let listed = kernel
            .list_memory_proposals(ListMemoryProposalsRequest::new(Some(scope_id)))
            .await
            .unwrap();
        assert_eq!(listed.len(), 2);
        let inspected = kernel
            .get_memory_proposal(GetMemoryProposalRequest {
                proposal_id: approve_proposal.id.clone(),
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(inspected.status, ProposalStatus::Open);

        let denied = kernel
            .approve_memory_proposal(ApproveMemoryProposalRequest {
                proposal_id: approve_proposal.id.clone(),
                actor: "agent".to_string(),
                actor_kind: crate::ReviewActorKind::Agent,
                has_user_authorization: false,
            })
            .await
            .unwrap_err();
        assert!(denied.to_string().contains("denied by policy"));

        let approved = kernel
            .approve_memory_proposal(ApproveMemoryProposalRequest {
                proposal_id: approve_proposal.id.clone(),
                actor: "user".to_string(),
                actor_kind: crate::ReviewActorKind::User,
                has_user_authorization: false,
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(approved.status, ProposalStatus::Approved);
        assert_eq!(approved.decided_by.as_deref(), Some("user"));

        let rejected = kernel
            .reject_memory_proposal(RejectMemoryProposalRequest {
                proposal_id: reject_proposal.id.clone(),
                actor: "user".to_string(),
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(rejected.status, ProposalStatus::Rejected);

        let missing = kernel
            .get_memory_proposal(GetMemoryProposalRequest {
                proposal_id: memory_domain::ProposalId::from_string("prp_missing"),
            })
            .await
            .unwrap();
        assert!(missing.is_none());
        let missing_approval = kernel
            .approve_memory_proposal(ApproveMemoryProposalRequest {
                proposal_id: memory_domain::ProposalId::from_string("prp_missing"),
                actor: "user".to_string(),
                actor_kind: crate::ReviewActorKind::User,
                has_user_authorization: false,
            })
            .await
            .unwrap();
        assert!(missing_approval.is_none());
        let missing_rejection = kernel
            .reject_memory_proposal(RejectMemoryProposalRequest {
                proposal_id: memory_domain::ProposalId::from_string("prp_missing"),
                actor: "user".to_string(),
            })
            .await
            .unwrap();
        assert!(missing_rejection.is_none());

        let metrics_after = metrics_snapshot();
        assert!(
            metrics_after
                .v2_8
                .proposal_observations
                .saturating_sub(metrics_before.v2_8.proposal_observations)
                >= 2
        );
        assert!(
            metrics_after
                .v2_8
                .proposal_conflicts
                .saturating_sub(metrics_before.v2_8.proposal_conflicts)
                >= 1
        );
        assert!(
            metrics_after
                .v2_8
                .proposal_approved
                .saturating_sub(metrics_before.v2_8.proposal_approved)
                >= 1
        );
        assert!(
            metrics_after
                .v2_8
                .proposal_rejected
                .saturating_sub(metrics_before.v2_8.proposal_rejected)
                >= 1
        );
        assert!(
            metrics_after
                .v2_8
                .proposal_failures
                .saturating_sub(metrics_before.v2_8.proposal_failures)
                >= 1
        );
    }

    #[tokio::test]
    async fn proposal_review_runtime_requires_pg() {
        let tempdir = tempdir().unwrap();
        let kernel = Kernel::builder()
            .with_markdown_root(tempdir.path())
            .unwrap()
            .build()
            .unwrap();

        let list_error = kernel
            .list_memory_proposals(ListMemoryProposalsRequest::new(None))
            .await
            .unwrap_err();
        assert!(
            list_error
                .to_string()
                .contains("postgres store is required for memory proposals")
        );

        let get_error = kernel
            .get_memory_proposal(GetMemoryProposalRequest {
                proposal_id: memory_domain::ProposalId::from_string("prp_no_pg"),
            })
            .await
            .unwrap_err();
        assert!(
            get_error
                .to_string()
                .contains("postgres store is required for memory proposals")
        );
    }

    #[tokio::test]
    async fn proposal_apply_runtime_materializes_relation_version_audit_and_status() {
        if !local_pg_test_port_available() {
            return;
        }
        let kernel = Kernel::builder()
            .with_postgres_url(&test_database_url())
            .await
            .unwrap()
            .build()
            .unwrap();
        let scope_id = ScopeId::new();
        let subject = seed_active_memory(
            &kernel,
            &scope_id,
            "Apply proposal subject",
            "Newer apply proposal body.",
            MemoryKind::Decision,
        )
        .await;
        let target = seed_active_memory(
            &kernel,
            &scope_id,
            "Apply proposal target",
            "Older apply proposal body.",
            MemoryKind::Decision,
        )
        .await;

        let mut supersede = MemoryProposal::new(
            scope_id.clone(),
            ProposalType::Supersede,
            ReviewLevel::Required,
            "apply relation proposal",
        )
        .unwrap()
        .with_subject_memory(subject.id.clone());
        supersede.add_target_memory(target.id.clone());
        supersede.approve("user").unwrap();
        kernel
            .pg_store
            .as_ref()
            .unwrap()
            .upsert_memory_proposal(&supersede)
            .await
            .unwrap();

        let metrics_before = metrics_snapshot();
        let applied = kernel
            .apply_memory_proposal(ApplyMemoryProposalRequest {
                proposal_id: supersede.id.clone(),
                actor: "system".to_string(),
                actor_kind: crate::ReviewActorKind::System,
                has_user_authorization: false,
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(applied.status, ProposalStatus::Applied);
        let relations = kernel
            .pg_store
            .as_ref()
            .unwrap()
            .list_memory_relations(&scope_id, Some(&subject.id), 10)
            .await
            .unwrap();
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].source_proposal_id, Some(supersede.id.clone()));
        let audit_events = kernel
            .list_lifecycle_audit_events(Some(scope_id.clone()), Some(subject.id.clone()), 10)
            .await
            .unwrap();
        assert!(
            audit_events
                .iter()
                .any(|event| event.action == "memory.proposal.supersede")
        );

        let mut new_version = MemoryProposal::new(
            scope_id.clone(),
            ProposalType::NewVersion,
            ReviewLevel::Suggested,
            "apply version proposal",
        )
        .unwrap()
        .with_subject_memory(subject.id.clone());
        new_version.approve("user").unwrap();
        kernel
            .pg_store
            .as_ref()
            .unwrap()
            .upsert_memory_proposal(&new_version)
            .await
            .unwrap();
        kernel
            .apply_memory_proposal(ApplyMemoryProposalRequest {
                proposal_id: new_version.id.clone(),
                actor: "system".to_string(),
                actor_kind: crate::ReviewActorKind::System,
                has_user_authorization: false,
            })
            .await
            .unwrap()
            .unwrap();
        let versions = kernel
            .list_memory_versions(ListMemoryVersionsRequest::new(scope_id, subject.id.clone()))
            .await
            .unwrap();
        assert_eq!(versions[0].change_kind, "new_version");
        assert_eq!(versions[0].source_proposal_id, Some(new_version.id));
        let metrics_after = metrics_snapshot();
        assert!(
            metrics_after
                .v2_8
                .proposal_applied
                .saturating_sub(metrics_before.v2_8.proposal_applied)
                >= 2
        );
    }

    #[tokio::test]
    async fn distillation_profile_runtime_upserts_and_lists_profiles_with_pg() {
        if !local_pg_test_port_available() {
            return;
        }
        let kernel = Kernel::builder()
            .with_postgres_url(&test_database_url())
            .await
            .unwrap()
            .build()
            .unwrap();
        let scope_id = ScopeId::new();
        let global_profile_id = memory_domain::DistillationProfileId::new();
        let project_profile_id = memory_domain::DistillationProfileId::new();

        let global = kernel
            .upsert_distillation_profile(UpsertDistillationProfileRequest {
                profile_id: global_profile_id,
                scope_id: None,
                profile_level: DistillationProfileLevel::UserGlobal,
                status: DistillationProfileStatus::Active,
                name: "Kernel global".to_string(),
                prompt_text: "Keep governance decisions.".to_string(),
                focus_topics: vec!["governance".to_string()],
                prefer_memory_kinds: vec![MemoryKind::Decision],
                created_by: "user".to_string(),
            })
            .await
            .unwrap();
        let project = kernel
            .upsert_distillation_profile(UpsertDistillationProfileRequest {
                profile_id: project_profile_id,
                scope_id: Some(scope_id.clone()),
                profile_level: DistillationProfileLevel::Project,
                status: DistillationProfileStatus::Archived,
                name: "Kernel project".to_string(),
                prompt_text: "Prefer rollout constraints.".to_string(),
                focus_topics: vec!["rollout".to_string()],
                prefer_memory_kinds: vec![MemoryKind::Constraint],
                created_by: "user".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(global.profile_level, DistillationProfileLevel::UserGlobal);
        assert_eq!(project.status, DistillationProfileStatus::Archived);

        let scoped = kernel
            .list_distillation_profiles(ListDistillationProfilesRequest {
                scope_id: Some(scope_id.clone()),
                limit: 10,
            })
            .await
            .unwrap();
        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].id, project.id);

        let all = kernel
            .list_distillation_profiles(ListDistillationProfilesRequest::new(None))
            .await
            .unwrap();
        let all_ids = all
            .iter()
            .map(|profile| profile.id.as_str())
            .collect::<Vec<_>>();
        assert!(all_ids.contains(&global.id.as_str()));
        assert!(all_ids.contains(&project.id.as_str()));

        let stored_global = kernel
            .pg_store
            .as_ref()
            .unwrap()
            .get_distillation_profile(&global.id)
            .await
            .unwrap()
            .unwrap();
        let updated_global = kernel
            .upsert_distillation_profile(UpsertDistillationProfileRequest {
                profile_id: global.id.clone(),
                scope_id: None,
                profile_level: DistillationProfileLevel::UserGlobal,
                status: DistillationProfileStatus::Active,
                name: "Kernel global updated".to_string(),
                prompt_text: "Keep updated governance decisions.".to_string(),
                focus_topics: vec!["governance".to_string(), "updates".to_string()],
                prefer_memory_kinds: vec![MemoryKind::Decision],
                created_by: "other-user".to_string(),
            })
            .await
            .unwrap();
        assert_eq!(updated_global.created_at, stored_global.created_at);
        assert_eq!(updated_global.created_by, stored_global.created_by);
        assert_eq!(updated_global.name, "Kernel global updated");
    }

    #[tokio::test]
    async fn distillation_preview_runtime_uses_stored_profiles_and_persists_run() {
        if !local_pg_test_port_available() {
            return;
        }
        let kernel = Kernel::builder()
            .with_postgres_url(&test_database_url())
            .await
            .unwrap()
            .build()
            .unwrap();
        let scope_id = ScopeId::new();
        let global_profile_id = memory_domain::DistillationProfileId::new();
        let project_profile_id = memory_domain::DistillationProfileId::new();
        kernel
            .upsert_distillation_profile(UpsertDistillationProfileRequest {
                profile_id: global_profile_id,
                scope_id: None,
                profile_level: DistillationProfileLevel::UserGlobal,
                status: DistillationProfileStatus::Active,
                name: "Preview global".to_string(),
                prompt_text: "Keep decisions.".to_string(),
                focus_topics: vec!["decision".to_string()],
                prefer_memory_kinds: Vec::new(),
                created_by: "user".to_string(),
            })
            .await
            .unwrap();
        kernel
            .upsert_distillation_profile(UpsertDistillationProfileRequest {
                profile_id: project_profile_id,
                scope_id: Some(scope_id.clone()),
                profile_level: DistillationProfileLevel::Project,
                status: DistillationProfileStatus::Active,
                name: "Preview project".to_string(),
                prompt_text: "Prefer rollout constraints.".to_string(),
                focus_topics: vec!["rollout".to_string()],
                prefer_memory_kinds: vec![MemoryKind::Constraint],
                created_by: "user".to_string(),
            })
            .await
            .unwrap();

        let result = kernel
            .preview_distillation(PreviewDistillationRequest {
                scope_id: scope_id.clone(),
                input: "Keep rollback evidence visible.".to_string(),
                evidence_refs: vec!["agent-context://ctx_kernel".to_string()],
                session_override: Some(
                    DistillationSessionOverride::new("Prefer explicit rollback guidance.")
                        .add_focus_topic("rollback"),
                ),
            })
            .await
            .unwrap();

        assert!(result.stored_in_pg);
        assert_eq!(
            result.preview.candidates[0].memory_kind,
            MemoryKind::Constraint
        );
        assert_eq!(result.profile.prompt_segments[1].layer, "user_global");
        assert_eq!(result.profile.prompt_segments[2].layer, "project");
        assert_eq!(result.profile.prompt_segments[3].layer, "session_override");

        let row = sqlx::query("SELECT id FROM distillation_runs WHERE id = $1")
            .bind(result.preview.run.id.as_str())
            .fetch_one(kernel.pg_store.as_ref().unwrap().pool())
            .await
            .unwrap();
        assert_eq!(
            row.try_get::<String, _>("id").unwrap(),
            result.preview.run.id.as_str()
        );
    }

    #[tokio::test]
    async fn distillation_preview_runtime_persists_single_profile_source_and_kind_labels() {
        if !local_pg_test_port_available() {
            return;
        }
        let kernel = Kernel::builder()
            .with_postgres_url(&test_database_url())
            .await
            .unwrap()
            .build()
            .unwrap();
        let scope_id = ScopeId::new();
        let profile_id = memory_domain::DistillationProfileId::new();
        let profile = kernel
            .upsert_distillation_profile(UpsertDistillationProfileRequest {
                profile_id,
                scope_id: None,
                profile_level: DistillationProfileLevel::UserGlobal,
                status: DistillationProfileStatus::Active,
                name: "Single source profile".to_string(),
                prompt_text: "Keep every memory kind label visible.".to_string(),
                focus_topics: vec!["labels".to_string()],
                prefer_memory_kinds: vec![
                    MemoryKind::Fact,
                    MemoryKind::Preference,
                    MemoryKind::Decision,
                    MemoryKind::Procedure,
                    MemoryKind::Constraint,
                    MemoryKind::Risk,
                    MemoryKind::Summary,
                    MemoryKind::Insight,
                ],
                created_by: "user".to_string(),
            })
            .await
            .unwrap();

        let result = kernel
            .preview_distillation(PreviewDistillationRequest {
                scope_id,
                input: "Capture a fact with supporting evidence.".to_string(),
                evidence_refs: vec!["agent-context://ctx_kind_labels".to_string()],
                session_override: None,
            })
            .await
            .unwrap();

        assert_eq!(result.preview.run.profile_id, Some(profile.id.clone()));
        let row = sqlx::query("SELECT output_json FROM distillation_runs WHERE id = $1")
            .bind(result.preview.run.id.as_str())
            .fetch_one(kernel.pg_store.as_ref().unwrap().pool())
            .await
            .unwrap();
        let output = row
            .try_get::<sqlx::types::Json<serde_json::Value>, _>("output_json")
            .unwrap()
            .0;
        assert_eq!(
            output["profile_source_ids"],
            serde_json::json!([profile.id.as_str()])
        );
        assert_eq!(
            output["prefer_memory_kinds"],
            serde_json::json!([
                "fact",
                "preference",
                "decision",
                "procedure",
                "constraint",
                "risk",
                "summary",
                "insight"
            ])
        );
    }

    #[tokio::test]
    async fn distillation_preview_runtime_works_without_pg() {
        let tempdir = tempdir().unwrap();
        let kernel = Kernel::builder()
            .with_markdown_root(tempdir.path())
            .unwrap()
            .build()
            .unwrap();

        let metrics_before = metrics_snapshot();
        let result = kernel
            .preview_distillation(PreviewDistillationRequest {
                scope_id: ScopeId::from_string("scp_v28_no_pg"),
                input: "Keep rollback evidence visible.".to_string(),
                evidence_refs: vec!["agent-context://ctx_kernel".to_string()],
                session_override: None,
            })
            .await
            .unwrap();

        assert!(!result.stored_in_pg);
        assert_eq!(
            result.preview.candidates[0].memory_kind,
            MemoryKind::Summary
        );

        let empty_input = kernel
            .preview_distillation(PreviewDistillationRequest {
                scope_id: ScopeId::from_string("scp_v28_invalid_preview"),
                input: "   ".to_string(),
                evidence_refs: vec!["agent-context://ctx_kernel".to_string()],
                session_override: None,
            })
            .await
            .unwrap_err();
        assert!(
            empty_input
                .to_string()
                .contains("distillation input cannot be empty")
        );

        let missing_evidence = kernel
            .preview_distillation(PreviewDistillationRequest {
                scope_id: ScopeId::from_string("scp_v28_invalid_preview"),
                input: "Keep this preview candidate.".to_string(),
                evidence_refs: Vec::new(),
                session_override: None,
            })
            .await
            .unwrap_err();
        assert!(
            missing_evidence
                .to_string()
                .contains("distillation evidence_refs cannot be empty")
        );

        let metrics_after = metrics_snapshot();
        assert!(
            metrics_after
                .v2_8
                .distillation_previews
                .saturating_sub(metrics_before.v2_8.distillation_previews)
                >= 3
        );
        assert!(
            metrics_after
                .v2_8
                .distillation_preview_hits
                .saturating_sub(metrics_before.v2_8.distillation_preview_hits)
                >= 1
        );
        assert!(
            metrics_after
                .v2_8
                .distillation_preview_failures
                .saturating_sub(metrics_before.v2_8.distillation_preview_failures)
                >= 2
        );
    }

    #[tokio::test]
    async fn distillation_profile_runtime_rejects_project_profiles_without_scope() {
        let tempdir = tempdir().unwrap();
        let kernel = Kernel::builder()
            .with_markdown_root(tempdir.path())
            .unwrap()
            .build()
            .unwrap();

        let error = kernel
            .upsert_distillation_profile(UpsertDistillationProfileRequest {
                profile_id: memory_domain::DistillationProfileId::from_string("dpf_kernel_invalid"),
                scope_id: None,
                profile_level: DistillationProfileLevel::Project,
                status: DistillationProfileStatus::Active,
                name: "Invalid project".to_string(),
                prompt_text: "Missing scope.".to_string(),
                focus_topics: Vec::new(),
                prefer_memory_kinds: Vec::new(),
                created_by: "user".to_string(),
            })
            .await
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("project profile requires scope_id")
        );
    }

    #[tokio::test]
    async fn memory_versions_runtime_reads_pg_history_latest_first() {
        if !local_pg_test_port_available() {
            return;
        }
        let kernel = Kernel::builder()
            .with_postgres_url(&test_database_url())
            .await
            .unwrap()
            .build()
            .unwrap();
        let scope_id = ScopeId::new();
        let mut memory = seed_active_memory(
            &kernel,
            &scope_id,
            "Versioned rollback note",
            "First rollback body.",
            MemoryKind::Decision,
        )
        .await;

        memory.title = "Versioned rollback note v2".to_string();
        memory.body = "Second rollback body.".to_string();
        memory.updated_at = time::OffsetDateTime::now_utc();
        kernel
            .pg_store
            .as_ref()
            .unwrap()
            .upsert_memory(&memory)
            .await
            .unwrap();

        let versions = kernel
            .list_memory_versions(ListMemoryVersionsRequest::new(
                scope_id.clone(),
                memory.id.clone(),
            ))
            .await
            .unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version, 2);
        assert_eq!(versions[0].title, "Versioned rollback note v2");
        assert_eq!(versions[0].change_kind, "upsert");
        assert_eq!(versions[1].version, 1);
        assert_eq!(versions[1].title, "Versioned rollback note");

        let missing = kernel
            .list_memory_versions(ListMemoryVersionsRequest::new(
                scope_id,
                MemoryId::from_string("mem_missing"),
            ))
            .await
            .unwrap();
        assert!(missing.is_empty());
    }

    #[tokio::test]
    async fn memory_rollback_runtime_restores_old_snapshot_and_marks_version() {
        if !local_pg_test_port_available() {
            return;
        }
        let kernel = Kernel::builder()
            .with_postgres_url(&test_database_url())
            .await
            .unwrap()
            .build()
            .unwrap();
        let scope_id = ScopeId::new();
        let mut memory = seed_active_memory(
            &kernel,
            &scope_id,
            "Runtime rollback v1",
            "Runtime rollback original body.",
            MemoryKind::Decision,
        )
        .await;
        memory.title = "Runtime rollback v2".to_string();
        memory.body = "Runtime rollback updated body.".to_string();
        memory.updated_at = time::OffsetDateTime::now_utc();
        kernel
            .pg_store
            .as_ref()
            .unwrap()
            .upsert_memory(&memory)
            .await
            .unwrap();

        let metrics_before = metrics_snapshot();
        let result = kernel
            .rollback_memory(RollbackMemoryRequest {
                scope_id: scope_id.clone(),
                memory_id: memory.id.clone(),
                target_version: 1,
                actor: "user".to_string(),
                reason: "restore original wording".to_string(),
            })
            .await
            .unwrap();
        assert_eq!(result.plan.new_version, 3);
        assert_eq!(result.memory.title, "Runtime rollback v1");

        let restored = kernel
            .get_memory(scope_id.clone(), memory.id.clone())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(restored.title, "Runtime rollback v1");
        assert_eq!(restored.body, "Runtime rollback original body.");
        let versions = kernel
            .list_memory_versions(ListMemoryVersionsRequest::new(
                scope_id.clone(),
                memory.id.clone(),
            ))
            .await
            .unwrap();
        assert_eq!(versions[0].version, 3);
        assert_eq!(versions[0].change_kind, "rollback");
        assert_eq!(versions[0].actor, "user");
        assert_eq!(
            versions[0].reason.as_deref(),
            Some("restore original wording")
        );
        let audit_events = kernel
            .list_lifecycle_audit_events(Some(scope_id), Some(memory.id), 10)
            .await
            .unwrap();
        assert!(
            audit_events
                .iter()
                .any(|event| event.action == "memory.rollback")
        );
        let metrics_after = metrics_snapshot();
        assert!(
            metrics_after
                .v2_8
                .rollback_successes
                .saturating_sub(metrics_before.v2_8.rollback_successes)
                >= 1
        );
    }

    #[tokio::test]
    async fn memory_timeline_runtime_aggregates_versions_relations_proposals_and_audit() {
        if !local_pg_test_port_available() {
            return;
        }
        let kernel = Kernel::builder()
            .with_postgres_url(&test_database_url())
            .await
            .unwrap()
            .build()
            .unwrap();
        let scope_id = ScopeId::new();
        let mut subject = seed_active_memory(
            &kernel,
            &scope_id,
            "Timeline subject",
            "Initial subject body.",
            MemoryKind::Decision,
        )
        .await;
        let target = seed_active_memory(
            &kernel,
            &scope_id,
            "Timeline target",
            "Legacy target body.",
            MemoryKind::Decision,
        )
        .await;

        subject.title = "Timeline subject v2".to_string();
        subject.body = "Updated subject body.".to_string();
        subject.updated_at = time::OffsetDateTime::now_utc();
        kernel
            .pg_store
            .as_ref()
            .unwrap()
            .upsert_memory(&subject)
            .await
            .unwrap();

        let mut proposal = MemoryProposal::new(
            scope_id.clone(),
            ProposalType::Supersede,
            ReviewLevel::Required,
            "timeline supersede proposal",
        )
        .unwrap()
        .with_subject_memory(subject.id.clone());
        proposal.add_target_memory(target.id.clone());
        proposal
            .add_evidence("same project with newer guidance".to_string())
            .unwrap();
        proposal.approve("user").unwrap();
        kernel
            .pg_store
            .as_ref()
            .unwrap()
            .upsert_memory_proposal(&proposal)
            .await
            .unwrap();

        let relation = MemoryRelation::new(
            scope_id.clone(),
            subject.id.clone(),
            target.id.clone(),
            MemoryRelationType::Supersedes,
            MemoryRelationSourceKind::System,
        )
        .with_source_proposal_id(proposal.id.clone());
        kernel
            .pg_store
            .as_ref()
            .unwrap()
            .insert_memory_relation(&relation)
            .await
            .unwrap();
        kernel
            .pg_store
            .as_ref()
            .unwrap()
            .insert_lifecycle_audit_event(
                &scope_id,
                &subject.id,
                "memory.status.changed",
                "system",
                Some("active"),
                Some("deprecated"),
                Some("superseded by timeline test"),
                time::OffsetDateTime::now_utc(),
            )
            .await
            .unwrap();

        let timeline = kernel
            .get_memory_timeline(GetMemoryTimelineRequest::new(
                scope_id.clone(),
                subject.id.clone(),
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(timeline.memory_id, subject.id);
        assert_eq!(timeline.versions.len(), 2);
        assert_eq!(timeline.relations.len(), 1);
        assert_eq!(timeline.proposals.len(), 1);
        assert_eq!(timeline.proposals[0].status, ProposalStatus::Approved);
        assert_eq!(timeline.audit_events.len(), 1);
        assert!(
            timeline
                .events
                .iter()
                .any(|event| event.action == "proposal.approved")
        );
        assert!(
            timeline
                .events
                .iter()
                .any(|event| event.action == "relation.supersedes")
        );
        assert!(
            timeline
                .events
                .iter()
                .any(|event| event.action == "audit.memory.status.changed")
        );

        let missing = kernel
            .get_memory_timeline(GetMemoryTimelineRequest::new(
                scope_id,
                MemoryId::from_string("mem_missing"),
            ))
            .await
            .unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn memory_timeline_runtime_requires_pg() {
        let tempdir = tempdir().unwrap();
        let kernel = Kernel::builder()
            .with_markdown_root(tempdir.path())
            .unwrap()
            .build()
            .unwrap();

        let versions_error = kernel
            .list_memory_versions(ListMemoryVersionsRequest::new(
                ScopeId::from_string("scp_no_pg"),
                MemoryId::from_string("mem_no_pg"),
            ))
            .await
            .unwrap_err();
        assert!(
            versions_error
                .to_string()
                .contains("postgres store is required for memory timeline")
        );

        let timeline_error = kernel
            .get_memory_timeline(GetMemoryTimelineRequest::new(
                ScopeId::from_string("scp_no_pg"),
                MemoryId::from_string("mem_no_pg"),
            ))
            .await
            .unwrap_err();
        assert!(
            timeline_error
                .to_string()
                .contains("postgres store is required for memory timeline")
        );
    }
}
