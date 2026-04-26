use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use memory_domain::{
    DistillationProfile, DistillationProfileId, DistillationProfileLevel,
    DistillationProfileStatus, MemoryId, MemoryProposal, MemoryRelation, ProposalId, ReviewLevel,
    ScopeId,
};
use memory_kernel::{
    ApplyMemoryProposalRequest, ApproveMemoryProposalRequest, ComposedDistillationProfile,
    DistillationCandidate, DistillationPromptSegment, DistillationSessionOverride,
    GetMemoryProposalRequest, GetMemoryTimelineRequest, ListDistillationProfilesRequest,
    ListMemoryProposalsRequest, ListMemoryVersionsRequest, MemoryTimeline,
    PreviewDistillationRequest, RejectMemoryProposalRequest, ReviewActorKind, ReviewPolicyAction,
    ReviewPolicyDecision, ReviewPolicyInput, ReviewPolicyService, RollbackMemoryRequest,
    RollbackPlan, TimelineAuditEvent, TimelineEvent, TimelineEventKind, TimelineVersion,
    UpsertDistillationProfileRequest,
};
use serde::{Deserialize, Serialize};

use super::{
    ApiError, HttpAppState, api_error_from_anyhow, format_timestamp, memory_kind_label,
    parse_memory_kind,
};

#[derive(Debug, Deserialize)]
pub(crate) struct ReviewPolicyEvaluateHttpRequest {
    pub review_level: String,
    pub action: String,
    pub actor_kind: String,
    #[serde(default)]
    pub has_user_authorization: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReviewPolicyEvaluateHttpResponse {
    pub decision: &'static str,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MemoryProposalsHttpQuery {
    pub scope_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DecideMemoryProposalHttpRequest {
    pub actor: String,
    pub actor_kind: Option<String>,
    #[serde(default)]
    pub has_user_authorization: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MemoryTimelineHttpQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RollbackMemoryHttpRequest {
    pub target_version: i32,
    pub actor: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct RollbackMemoryHttpResponse {
    pub memory_id: String,
    pub target_version: i32,
    pub new_version: i32,
    pub title: String,
    pub body: String,
    pub change_kind: &'static str,
    pub actor: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct MemoryVersionHttpResponse {
    pub memory_id: String,
    pub version: i32,
    pub title: String,
    pub body: String,
    pub change_kind: String,
    pub actor: String,
    pub reason: Option<String>,
    pub source_proposal_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct MemoryRelationHttpResponse {
    pub relation_id: String,
    pub from_memory_id: String,
    pub to_memory_id: String,
    pub relation_type: &'static str,
    pub confidence: f32,
    pub source_kind: &'static str,
    pub source_proposal_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct MemoryProposalHttpResponse {
    pub proposal_id: String,
    pub scope_id: String,
    pub proposal_type: &'static str,
    pub status: &'static str,
    pub review_level: &'static str,
    pub subject_memory_id: Option<String>,
    pub target_memory_ids: Vec<String>,
    pub reason: String,
    pub evidence: Vec<String>,
    pub decided_by: Option<String>,
    pub decided_at: Option<String>,
    pub applied_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct TimelineAuditEventHttpResponse {
    pub memory_id: Option<String>,
    pub action: String,
    pub actor: String,
    pub reason: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct TimelineEventHttpResponse {
    pub kind: &'static str,
    pub action: String,
    pub occurred_at: String,
    pub memory_id: Option<String>,
    pub proposal_id: Option<String>,
    pub relation_id: Option<String>,
    pub version: Option<i32>,
}

#[derive(Debug, Serialize)]
pub(crate) struct MemoryTimelineHttpResponse {
    pub memory_id: String,
    pub versions: Vec<MemoryVersionHttpResponse>,
    pub relations: Vec<MemoryRelationHttpResponse>,
    pub audit_events: Vec<TimelineAuditEventHttpResponse>,
    pub proposals: Vec<MemoryProposalHttpResponse>,
    pub events: Vec<TimelineEventHttpResponse>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DistillationProfilesHttpQuery {
    pub scope_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpsertDistillationProfileHttpRequest {
    pub scope_id: Option<String>,
    pub profile_level: String,
    pub status: Option<String>,
    pub name: String,
    pub prompt_text: String,
    #[serde(default)]
    pub focus_topics: Vec<String>,
    #[serde(default)]
    pub prefer_memory_kinds: Vec<String>,
    pub created_by: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct StoredDistillationProfileHttpResponse {
    pub profile_id: String,
    pub scope_id: Option<String>,
    pub profile_level: &'static str,
    pub status: &'static str,
    pub name: String,
    pub prompt_text: String,
    pub focus_topics: Vec<String>,
    pub prefer_memory_kinds: Vec<&'static str>,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DistillationPreviewHttpRequest {
    pub scope_id: Option<String>,
    pub input: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    pub prompt_text: Option<String>,
    #[serde(default)]
    pub focus_topics: Vec<String>,
    #[serde(default)]
    pub prefer_memory_kinds: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DistillationPreviewHttpResponse {
    pub run: DistillationRunHttpResponse,
    pub profile: ComposedDistillationProfileHttpResponse,
    pub candidates: Vec<DistillationCandidateHttpResponse>,
    pub discarded: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DistillationRunHttpResponse {
    pub run_id: String,
    pub scope_id: String,
    pub input_hash: String,
    pub preview: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct ComposedDistillationProfileHttpResponse {
    pub prompt_segments: Vec<DistillationPromptSegmentHttpResponse>,
    pub focus_topics: Vec<String>,
    pub prefer_memory_kinds: Vec<&'static str>,
    pub safety_rules: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DistillationPromptSegmentHttpResponse {
    pub layer: &'static str,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct DistillationCandidateHttpResponse {
    pub title: String,
    pub memory_kind: &'static str,
    pub body: String,
    pub confidence: &'static str,
    pub why_keep: String,
    pub evidence_refs: Vec<String>,
}

pub(crate) async fn evaluate_review_policy(
    Json(payload): Json<ReviewPolicyEvaluateHttpRequest>,
) -> Result<Json<ReviewPolicyEvaluateHttpResponse>, ApiError> {
    let evaluation = ReviewPolicyService::evaluate(ReviewPolicyInput {
        review_level: parse_review_level(&payload.review_level)?,
        action: parse_review_policy_action(&payload.action)?,
        actor_kind: parse_review_actor_kind(&payload.actor_kind)?,
        has_user_authorization: payload.has_user_authorization,
    });

    Ok(Json(ReviewPolicyEvaluateHttpResponse {
        decision: review_policy_decision_label(evaluation.decision),
        reason: evaluation.reason,
    }))
}

pub(crate) async fn list_memory_proposals(
    State(state): State<HttpAppState>,
    Query(query): Query<MemoryProposalsHttpQuery>,
) -> Result<Json<Vec<MemoryProposalHttpResponse>>, ApiError> {
    let proposals = state
        .kernel
        .list_memory_proposals(ListMemoryProposalsRequest {
            scope_id: query.scope_id.as_deref().map(ScopeId::from_string),
            limit: query.limit.unwrap_or(50),
        })
        .await
        .map_err(api_error_from_anyhow)?;

    Ok(Json(
        proposals
            .into_iter()
            .map(memory_proposal_response)
            .collect(),
    ))
}

pub(crate) async fn get_memory_proposal(
    State(state): State<HttpAppState>,
    Path(proposal_id): Path<String>,
) -> Result<Json<MemoryProposalHttpResponse>, ApiError> {
    let proposal = state
        .kernel
        .get_memory_proposal(GetMemoryProposalRequest {
            proposal_id: ProposalId::from_string(proposal_id),
        })
        .await
        .map_err(api_error_from_anyhow)?
        .ok_or_else(proposal_not_found)?;

    Ok(Json(memory_proposal_response(proposal)))
}

pub(crate) async fn approve_memory_proposal(
    State(state): State<HttpAppState>,
    Path(proposal_id): Path<String>,
    Json(payload): Json<DecideMemoryProposalHttpRequest>,
) -> Result<Json<MemoryProposalHttpResponse>, ApiError> {
    let actor_kind = payload
        .actor_kind
        .as_deref()
        .map(parse_review_actor_kind)
        .transpose()?
        .unwrap_or(ReviewActorKind::User);
    let proposal = state
        .kernel
        .approve_memory_proposal(ApproveMemoryProposalRequest {
            proposal_id: ProposalId::from_string(proposal_id),
            actor: payload.actor,
            actor_kind,
            has_user_authorization: payload.has_user_authorization,
        })
        .await
        .map_err(api_error_from_anyhow)?
        .ok_or_else(proposal_not_found)?;

    Ok(Json(memory_proposal_response(proposal)))
}

pub(crate) async fn reject_memory_proposal(
    State(state): State<HttpAppState>,
    Path(proposal_id): Path<String>,
    Json(payload): Json<DecideMemoryProposalHttpRequest>,
) -> Result<Json<MemoryProposalHttpResponse>, ApiError> {
    let proposal = state
        .kernel
        .reject_memory_proposal(RejectMemoryProposalRequest {
            proposal_id: ProposalId::from_string(proposal_id),
            actor: payload.actor,
        })
        .await
        .map_err(api_error_from_anyhow)?
        .ok_or_else(proposal_not_found)?;

    Ok(Json(memory_proposal_response(proposal)))
}

pub(crate) async fn apply_memory_proposal(
    State(state): State<HttpAppState>,
    Path(proposal_id): Path<String>,
    Json(payload): Json<DecideMemoryProposalHttpRequest>,
) -> Result<Json<MemoryProposalHttpResponse>, ApiError> {
    let actor_kind = payload
        .actor_kind
        .as_deref()
        .map(parse_review_actor_kind)
        .transpose()?
        .unwrap_or(ReviewActorKind::System);
    let proposal = state
        .kernel
        .apply_memory_proposal(ApplyMemoryProposalRequest {
            proposal_id: ProposalId::from_string(proposal_id),
            actor: payload.actor,
            actor_kind,
            has_user_authorization: payload.has_user_authorization,
        })
        .await
        .map_err(api_error_from_anyhow)?
        .ok_or_else(proposal_not_found)?;

    Ok(Json(memory_proposal_response(proposal)))
}

pub(crate) async fn list_memory_versions(
    State(state): State<HttpAppState>,
    Path((scope_id, memory_id)): Path<(String, String)>,
    Query(query): Query<MemoryTimelineHttpQuery>,
) -> Result<Json<Vec<MemoryVersionHttpResponse>>, ApiError> {
    let scope_id = ScopeId::from_string(scope_id);
    let memory_id = MemoryId::from_string(memory_id);
    ensure_memory_exists(&state, &scope_id, &memory_id).await?;

    let versions = state
        .kernel
        .list_memory_versions(ListMemoryVersionsRequest {
            scope_id,
            memory_id,
            limit: query.limit.unwrap_or(50),
        })
        .await
        .map_err(api_error_from_anyhow)?;

    Ok(Json(
        versions
            .into_iter()
            .map(memory_version_response)
            .collect::<Vec<_>>(),
    ))
}

pub(crate) async fn get_memory_timeline(
    State(state): State<HttpAppState>,
    Path((scope_id, memory_id)): Path<(String, String)>,
    Query(query): Query<MemoryTimelineHttpQuery>,
) -> Result<Json<MemoryTimelineHttpResponse>, ApiError> {
    let scope_id = ScopeId::from_string(scope_id);
    let memory_id = MemoryId::from_string(memory_id);
    ensure_memory_exists(&state, &scope_id, &memory_id).await?;

    let timeline = state
        .kernel
        .get_memory_timeline(GetMemoryTimelineRequest {
            scope_id,
            memory_id,
            limit: query.limit.unwrap_or(50),
        })
        .await
        .map_err(api_error_from_anyhow)?
        .ok_or_else(memory_not_found)?;

    Ok(Json(memory_timeline_response(timeline)))
}

pub(crate) async fn rollback_memory(
    State(state): State<HttpAppState>,
    Path((scope_id, memory_id)): Path<(String, String)>,
    Json(payload): Json<RollbackMemoryHttpRequest>,
) -> Result<Json<RollbackMemoryHttpResponse>, ApiError> {
    let scope_id = ScopeId::from_string(scope_id);
    let memory_id = MemoryId::from_string(memory_id);
    ensure_memory_exists(&state, &scope_id, &memory_id).await?;

    let result = state
        .kernel
        .rollback_memory(RollbackMemoryRequest {
            scope_id,
            memory_id,
            target_version: payload.target_version,
            actor: payload.actor,
            reason: payload.reason,
        })
        .await
        .map_err(api_error_from_anyhow)?;

    Ok(Json(rollback_memory_response(result.plan)))
}

pub(crate) async fn list_distillation_profiles(
    State(state): State<HttpAppState>,
    Query(query): Query<DistillationProfilesHttpQuery>,
) -> Result<Json<Vec<StoredDistillationProfileHttpResponse>>, ApiError> {
    let profiles = state
        .kernel
        .list_distillation_profiles(ListDistillationProfilesRequest {
            scope_id: query.scope_id.as_deref().map(ScopeId::from_string),
            limit: query.limit.unwrap_or(50),
        })
        .await
        .map_err(api_error_from_anyhow)?;

    Ok(Json(
        profiles
            .into_iter()
            .map(stored_distillation_profile_response)
            .collect(),
    ))
}

pub(crate) async fn upsert_distillation_profile(
    State(state): State<HttpAppState>,
    Path(profile_id): Path<String>,
    Json(payload): Json<UpsertDistillationProfileHttpRequest>,
) -> Result<Json<StoredDistillationProfileHttpResponse>, ApiError> {
    let profile = state
        .kernel
        .upsert_distillation_profile(UpsertDistillationProfileRequest {
            profile_id: DistillationProfileId::from_string(profile_id),
            scope_id: payload.scope_id.as_deref().map(ScopeId::from_string),
            profile_level: parse_distillation_profile_level(&payload.profile_level)?,
            status: parse_distillation_profile_status(
                payload.status.as_deref().unwrap_or("active"),
            )?,
            name: payload.name,
            prompt_text: payload.prompt_text,
            focus_topics: payload.focus_topics,
            prefer_memory_kinds: payload
                .prefer_memory_kinds
                .iter()
                .map(|memory_kind| parse_memory_kind(memory_kind))
                .collect::<Result<Vec<_>, _>>()?,
            created_by: payload.created_by,
        })
        .await
        .map_err(api_error_from_anyhow)?;

    Ok(Json(stored_distillation_profile_response(profile)))
}

pub(crate) async fn evaluate_distillation_preview(
    State(state): State<HttpAppState>,
    Json(payload): Json<DistillationPreviewHttpRequest>,
) -> Result<Json<DistillationPreviewHttpResponse>, ApiError> {
    let session_override = build_distillation_session_override(&payload)?;
    let result = state
        .kernel
        .preview_distillation(PreviewDistillationRequest {
            scope_id: ScopeId::from_string(
                payload
                    .scope_id
                    .as_deref()
                    .unwrap_or(state.default_scope_id.as_str()),
            ),
            input: payload.input,
            evidence_refs: payload.evidence_refs,
            session_override,
        })
        .await
        .map_err(api_error_from_anyhow)?;

    Ok(Json(distillation_preview_response(
        result.preview,
        &result.profile,
    )))
}

fn build_distillation_session_override(
    payload: &DistillationPreviewHttpRequest,
) -> Result<Option<DistillationSessionOverride>, ApiError> {
    if payload
        .prompt_text
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
        && payload.focus_topics.is_empty()
        && payload.prefer_memory_kinds.is_empty()
    {
        return Ok(None);
    }

    let mut session =
        DistillationSessionOverride::new(payload.prompt_text.clone().unwrap_or_default());
    for topic in &payload.focus_topics {
        session = session.add_focus_topic(topic.clone());
    }
    for memory_kind in &payload.prefer_memory_kinds {
        session = session.prefer_memory_kind(parse_memory_kind(memory_kind)?);
    }
    Ok(Some(session))
}

async fn ensure_memory_exists(
    state: &HttpAppState,
    scope_id: &ScopeId,
    memory_id: &MemoryId,
) -> Result<(), ApiError> {
    if state
        .kernel
        .get_memory(scope_id.clone(), memory_id.clone())
        .await
        .map_err(api_error_from_anyhow)?
        .is_some()
    {
        return Ok(());
    }
    Err(memory_not_found())
}

fn memory_not_found() -> ApiError {
    ApiError {
        status: StatusCode::NOT_FOUND,
        message: "memory not found".to_string(),
    }
}

fn proposal_not_found() -> ApiError {
    ApiError {
        status: StatusCode::NOT_FOUND,
        message: "proposal not found".to_string(),
    }
}

fn memory_timeline_response(timeline: MemoryTimeline) -> MemoryTimelineHttpResponse {
    MemoryTimelineHttpResponse {
        memory_id: timeline.memory_id.as_str().to_string(),
        versions: timeline
            .versions
            .into_iter()
            .map(memory_version_response)
            .collect(),
        relations: timeline
            .relations
            .into_iter()
            .map(memory_relation_response)
            .collect(),
        audit_events: timeline
            .audit_events
            .into_iter()
            .map(timeline_audit_event_response)
            .collect(),
        proposals: timeline
            .proposals
            .into_iter()
            .map(memory_proposal_response)
            .collect(),
        events: timeline
            .events
            .into_iter()
            .map(timeline_event_response)
            .collect(),
    }
}

fn memory_version_response(version: TimelineVersion) -> MemoryVersionHttpResponse {
    MemoryVersionHttpResponse {
        memory_id: version.memory_id.as_str().to_string(),
        version: version.version,
        title: version.title,
        body: version.body,
        change_kind: version.change_kind,
        actor: version.actor,
        reason: version.reason,
        source_proposal_id: version
            .source_proposal_id
            .map(|proposal_id| proposal_id.as_str().to_string()),
        created_at: format_timestamp(version.created_at),
    }
}

fn rollback_memory_response(plan: RollbackPlan) -> RollbackMemoryHttpResponse {
    RollbackMemoryHttpResponse {
        memory_id: plan.memory_id.as_str().to_string(),
        target_version: plan.target_version,
        new_version: plan.new_version,
        title: plan.title,
        body: plan.body,
        change_kind: plan.change_kind,
        actor: plan.actor,
        reason: plan.reason,
    }
}

fn memory_relation_response(relation: MemoryRelation) -> MemoryRelationHttpResponse {
    MemoryRelationHttpResponse {
        relation_id: relation.id.as_str().to_string(),
        from_memory_id: relation.from_memory_id.as_str().to_string(),
        to_memory_id: relation.to_memory_id.as_str().to_string(),
        relation_type: relation.relation_type.as_str(),
        confidence: relation.confidence,
        source_kind: relation.source_kind.as_str(),
        source_proposal_id: relation
            .source_proposal_id
            .map(|proposal_id| proposal_id.as_str().to_string()),
        created_at: format_timestamp(relation.created_at),
    }
}

fn memory_proposal_response(proposal: MemoryProposal) -> MemoryProposalHttpResponse {
    MemoryProposalHttpResponse {
        proposal_id: proposal.id.as_str().to_string(),
        scope_id: proposal.scope_id.as_str().to_string(),
        proposal_type: proposal.proposal_type.as_str(),
        status: proposal.status.as_str(),
        review_level: proposal.review_level.as_str(),
        subject_memory_id: proposal
            .subject_memory_id
            .map(|memory_id| memory_id.as_str().to_string()),
        target_memory_ids: proposal
            .target_memory_ids
            .into_iter()
            .map(|memory_id| memory_id.as_str().to_string())
            .collect(),
        reason: proposal.reason,
        evidence: proposal.evidence,
        decided_by: proposal.decided_by,
        decided_at: proposal.decided_at.map(format_timestamp),
        applied_at: proposal.applied_at.map(format_timestamp),
        created_at: format_timestamp(proposal.created_at),
        updated_at: format_timestamp(proposal.updated_at),
    }
}

fn timeline_audit_event_response(
    audit_event: TimelineAuditEvent,
) -> TimelineAuditEventHttpResponse {
    TimelineAuditEventHttpResponse {
        memory_id: audit_event
            .memory_id
            .map(|memory_id| memory_id.as_str().to_string()),
        action: audit_event.action,
        actor: audit_event.actor,
        reason: audit_event.reason,
        created_at: format_timestamp(audit_event.created_at),
    }
}

fn timeline_event_response(event: TimelineEvent) -> TimelineEventHttpResponse {
    TimelineEventHttpResponse {
        kind: timeline_event_kind_label(event.kind),
        action: event.action,
        occurred_at: format_timestamp(event.occurred_at),
        memory_id: event
            .memory_id
            .map(|memory_id| memory_id.as_str().to_string()),
        proposal_id: event
            .proposal_id
            .map(|proposal_id| proposal_id.as_str().to_string()),
        relation_id: event
            .relation_id
            .map(|relation_id| relation_id.as_str().to_string()),
        version: event.version,
    }
}

fn stored_distillation_profile_response(
    profile: DistillationProfile,
) -> StoredDistillationProfileHttpResponse {
    StoredDistillationProfileHttpResponse {
        profile_id: profile.id.as_str().to_string(),
        scope_id: profile
            .scope_id
            .as_ref()
            .map(|scope_id| scope_id.as_str().to_string()),
        profile_level: distillation_profile_level_label(profile.profile_level),
        status: distillation_profile_status_label(profile.status),
        name: profile.name,
        prompt_text: profile.prompt_text,
        focus_topics: profile.focus_topics,
        prefer_memory_kinds: profile
            .prefer_memory_kinds
            .into_iter()
            .map(memory_kind_label)
            .collect(),
        created_by: profile.created_by,
        created_at: format_timestamp(profile.created_at),
        updated_at: format_timestamp(profile.updated_at),
    }
}

fn distillation_preview_response(
    preview: memory_kernel::DistillationPreview,
    profile: &ComposedDistillationProfile,
) -> DistillationPreviewHttpResponse {
    DistillationPreviewHttpResponse {
        run: DistillationRunHttpResponse {
            run_id: preview.run.id.as_str().to_string(),
            scope_id: preview.run.scope_id.as_str().to_string(),
            input_hash: preview.run.input_hash,
            preview: preview.run.preview,
        },
        profile: composed_distillation_profile_response(profile),
        candidates: preview
            .candidates
            .into_iter()
            .map(distillation_candidate_response)
            .collect(),
        discarded: preview.discarded,
        warnings: preview.warnings,
    }
}

fn composed_distillation_profile_response(
    profile: &ComposedDistillationProfile,
) -> ComposedDistillationProfileHttpResponse {
    ComposedDistillationProfileHttpResponse {
        prompt_segments: profile
            .prompt_segments
            .iter()
            .map(distillation_prompt_segment_response)
            .collect(),
        focus_topics: profile.focus_topics.clone(),
        prefer_memory_kinds: profile
            .prefer_memory_kinds
            .iter()
            .copied()
            .map(memory_kind_label)
            .collect(),
        safety_rules: profile.safety_rules.clone(),
    }
}

fn distillation_prompt_segment_response(
    segment: &DistillationPromptSegment,
) -> DistillationPromptSegmentHttpResponse {
    DistillationPromptSegmentHttpResponse {
        layer: segment.layer,
        text: segment.text.clone(),
    }
}

fn distillation_candidate_response(
    candidate: DistillationCandidate,
) -> DistillationCandidateHttpResponse {
    DistillationCandidateHttpResponse {
        title: candidate.title,
        memory_kind: memory_kind_label(candidate.memory_kind),
        body: candidate.body,
        confidence: candidate.confidence,
        why_keep: candidate.why_keep,
        evidence_refs: candidate.evidence_refs,
    }
}

pub(crate) fn parse_distillation_profile_level(
    raw: &str,
) -> Result<DistillationProfileLevel, ApiError> {
    match raw {
        "user_global" => Ok(DistillationProfileLevel::UserGlobal),
        "project" => Ok(DistillationProfileLevel::Project),
        other => Err(ApiError::bad_request(format!(
            "unsupported distillation_profile_level: {other}"
        ))),
    }
}

pub(crate) fn parse_distillation_profile_status(
    raw: &str,
) -> Result<DistillationProfileStatus, ApiError> {
    match raw {
        "active" => Ok(DistillationProfileStatus::Active),
        "archived" => Ok(DistillationProfileStatus::Archived),
        other => Err(ApiError::bad_request(format!(
            "unsupported distillation_profile_status: {other}"
        ))),
    }
}

pub(crate) fn distillation_profile_level_label(level: DistillationProfileLevel) -> &'static str {
    match level {
        DistillationProfileLevel::UserGlobal => "user_global",
        DistillationProfileLevel::Project => "project",
    }
}

pub(crate) fn distillation_profile_status_label(status: DistillationProfileStatus) -> &'static str {
    match status {
        DistillationProfileStatus::Active => "active",
        DistillationProfileStatus::Archived => "archived",
    }
}

pub(crate) fn parse_review_level(raw: &str) -> Result<ReviewLevel, ApiError> {
    match raw {
        "auto" => Ok(ReviewLevel::Auto),
        "suggested" => Ok(ReviewLevel::Suggested),
        "required" => Ok(ReviewLevel::Required),
        "blocked" => Ok(ReviewLevel::Blocked),
        other => Err(ApiError::bad_request(format!(
            "unsupported review_level: {other}"
        ))),
    }
}

pub(crate) fn parse_review_policy_action(raw: &str) -> Result<ReviewPolicyAction, ApiError> {
    match raw {
        "approve" => Ok(ReviewPolicyAction::Approve),
        "apply" => Ok(ReviewPolicyAction::Apply),
        other => Err(ApiError::bad_request(format!(
            "unsupported review_policy_action: {other}"
        ))),
    }
}

pub(crate) fn parse_review_actor_kind(raw: &str) -> Result<ReviewActorKind, ApiError> {
    match raw {
        "user" => Ok(ReviewActorKind::User),
        "agent" => Ok(ReviewActorKind::Agent),
        "system" => Ok(ReviewActorKind::System),
        other => Err(ApiError::bad_request(format!(
            "unsupported review_actor_kind: {other}"
        ))),
    }
}

pub(crate) fn review_policy_decision_label(decision: ReviewPolicyDecision) -> &'static str {
    match decision {
        ReviewPolicyDecision::Allow => "allow",
        ReviewPolicyDecision::RequireUserApproval => "require_user_approval",
        ReviewPolicyDecision::Deny => "deny",
    }
}

fn timeline_event_kind_label(kind: TimelineEventKind) -> &'static str {
    match kind {
        TimelineEventKind::Version => "version",
        TimelineEventKind::Proposal => "proposal",
        TimelineEventKind::Relation => "relation",
        TimelineEventKind::Audit => "audit",
    }
}
