use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, patch, post, put},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use memory_domain::{
    AccessKeyId, AccessKeyStatus, AgentContext, AgentContextId, ArtifactKind,
    DocumentConflictState, DocumentSyncState, KeyScopeKind, KeySourceKind, Memory, MemoryId,
    MemoryKind, MemoryRecord, MemoryRecordStatus, MemorySource, ProjectDocument, RequestContext,
    ScopeId, ScopeType, Sensitivity, SourceId, SourceSyncMode, StorageMode, Visibility,
};
use memory_kernel::{
    ApplyProjectDocumentSyncPlanRequest, BenchmarkRunOutput, BenchmarkRunRequest,
    BenchmarkSuiteKind, ChangeMemoryLifecycleStatusRequest, ConnectorDryRunRequest,
    ConnectorImportDraftRequest, ConnectorProposalApplyExecutorRequest,
    ConnectorProposalApplyPlanRequest, ConnectorProposalQueueReport, ConnectorProposalQueueRequest,
    ConnectorSyncPlanRequest, CreateAccessKeyRequest, ImportProjectDocumentRequest,
    InspectMemoryLifecycleRequest, Kernel, ListAgentContextsRequest, ListProjectDocumentsRequest,
    MemoryPassportBundle, MemoryPassportExportRequest, MemoryPassportImportRequest,
    MemoryPassportImportResult, MemoryPassportPaths, PromoteAgentContextRequest,
    PromoteMemoryRequest, RecallTraceBudget, RecallTraceReportPaths, RememberImageRequest,
    RememberTextRequest, RememberTextResult, SearchContextRequest, TraceSearchContextRequest,
    TraceSearchContextResult, UpdateAccessKeyRequest, UpsertAgentContextRequest,
    apply_connector_proposal_apply_plan, build_competitor_compatibility_report,
    build_connector_import_draft_report, build_connector_proposal_apply_plan_report,
    build_connector_proposal_apply_plan_report_from_queue, build_connector_proposal_queue_report,
    build_connector_sync_plan, bundle_json, compatibility_report_json, connector_dry_run_json,
    connector_import_draft_json, connector_proposal_apply_plan_execution_json,
    connector_proposal_apply_plan_json, connector_proposal_queue_json, connector_sync_plan_json,
    health_json, run_connector_dry_run, verification_json, verify_memory_passport_bundle,
    write_memory_passport_bundle, write_recall_trace_report,
};
use memory_sync::{
    LocalProjectDocumentDraft, LocalProjectDocumentSyncEngine, MissingProjectDocument,
    ProjectDocumentConflictReport, ProjectDocumentSnapshot,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::ErrorKind,
    path::{Path as FsPath, PathBuf},
    sync::Arc,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tracing::error;

#[path = "v28_http.rs"]
mod v28_http;

use v28_http::{
    apply_memory_proposal, approve_memory_proposal, evaluate_distillation_preview,
    evaluate_review_policy, get_memory_proposal, get_memory_timeline, list_distillation_profiles,
    list_memory_proposals, list_memory_versions, reject_memory_proposal, rollback_memory,
    upsert_distillation_profile,
};
#[cfg(test)]
pub(crate) use v28_http::{
    distillation_profile_level_label, distillation_profile_status_label,
    parse_distillation_profile_level, parse_distillation_profile_status, parse_review_actor_kind,
    parse_review_level, parse_review_policy_action, review_policy_decision_label,
};

const MAX_MEMORY_BODY_CHARS: usize = 16_000;
const MAX_QUERY_CHARS: usize = 1_024;
const MAX_PROMPT_CHARS: usize = 4_000;
const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;

pub const HTTP_ROUTES: &[&str] = &[
    "/",
    "/healthz",
    "/readyz",
    "/livez",
    "/metrics",
    "/api/v1/meta",
    "/api/v1/memories",
    "/api/v1/memories/promote",
    "/api/v1/lifecycle/memories/{scope_id}/{memory_id}",
    "/api/v1/lifecycle/memories/{scope_id}/{memory_id}/status",
    "/api/v1/lifecycle/memories/{scope_id}/{memory_id}/forget",
    "/api/v1/lifecycle/memories/{scope_id}/{memory_id}/restore",
    "/api/v1/lifecycle/audit",
    "/api/v1/lifecycle/report",
    "/api/v1/benchmark/run",
    "/api/v1/benchmark/runs",
    "/api/v1/benchmark/report",
    "/api/v1/benchmark/failures",
    "/api/v1/recall/traces/latest",
    "/api/v1/recall/traces/inspect",
    "/api/v1/health/report",
    "/api/v1/passports/export",
    "/api/v1/passports/import",
    "/api/v1/passports/manifest",
    "/api/v1/compat/report",
    "/api/v1/compat/connectors/dry-run",
    "/api/v1/compat/connectors/sync-plan",
    "/api/v1/compat/connectors/import-draft",
    "/api/v1/compat/connectors/proposal-queue",
    "/api/v1/compat/connectors/proposal-queue/{queue_id}",
    "/api/v1/compat/connectors/proposal-apply-plan",
    "/api/v1/compat/connectors/proposal-apply-plan/apply",
    "/api/v1/images",
    "/api/v1/context",
    "/api/v1/context/search",
    "/api/v1/agent-contexts",
    "/api/v1/agent-contexts/{context_id}",
    "/api/v1/agent-contexts/{context_id}/promote",
    "/api/v1/proposals",
    "/api/v1/proposals/{proposal_id}",
    "/api/v1/proposals/{proposal_id}/approve",
    "/api/v1/proposals/{proposal_id}/reject",
    "/api/v1/proposals/{proposal_id}/apply",
    "/api/v1/proposals/review-policy/evaluate",
    "/api/v1/memories/{scope_id}/{memory_id}/versions",
    "/api/v1/memories/{scope_id}/{memory_id}/timeline",
    "/api/v1/memories/{scope_id}/{memory_id}/rollback",
    "/api/v1/distillation/profiles",
    "/api/v1/distillation/profiles/{profile_id}",
    "/api/v1/distillation/preview",
    "/api/v1/explorer/memories",
    "/api/v1/assistant/chat",
    "/api/v1/keys",
    "/api/v1/keys/{key_id}",
    "/api/v1/keys/{key_id}/rotate",
    "/api/v1/keys/{key_id}/stats",
    "/api/v1/sources",
    "/api/v1/sources/{source_id}",
    "/api/v1/sources/{source_id}/keys",
    "/api/v1/sources/{source_id}/documents",
    "/api/v1/sources/{source_id}/documents/{document_id}/projection",
    "/api/v1/sources/{source_id}/documents/import",
    "/api/v1/sources/{source_id}/documents/conflicts",
    "/api/v1/sources/{source_id}/documents/sync",
    "/api/v1/metrics/keys",
];

#[derive(Debug, Clone, Serialize)]
pub struct ApiFeatureFlags {
    pub pg: bool,
    pub markdown: bool,
    pub http: bool,
    pub mcp: bool,
    pub require_key: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiMetadata {
    pub service: String,
    pub version: String,
    pub default_scope: String,
    pub features: ApiFeatureFlags,
}

#[derive(Clone)]
pub struct HttpAppState {
    default_scope_id: ScopeId,
    metadata: ApiMetadata,
    kernel: Arc<Kernel>,
    require_key: bool,
    connector_proposal_store: Option<ConnectorProposalStore>,
}

impl HttpAppState {
    pub fn new(default_scope_id: ScopeId, metadata: ApiMetadata, kernel: Arc<Kernel>) -> Self {
        Self {
            default_scope_id,
            require_key: metadata.features.require_key,
            metadata,
            kernel,
            connector_proposal_store: None,
        }
    }

    pub fn with_connector_proposal_store(mut self, root: impl Into<PathBuf>) -> Self {
        self.connector_proposal_store = Some(ConnectorProposalStore { root: root.into() });
        self
    }
}

#[derive(Clone)]
struct ConnectorProposalStore {
    root: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct CreateAccessKeyHttpRequest {
    pub raw_key: Option<String>,
    pub name: String,
    pub source: Option<String>,
    pub owner_principal_id: Option<String>,
    pub owner_scope_id: Option<String>,
    pub scope_kind: Option<String>,
    pub storage_mode: Option<String>,
    pub isolated: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct AccessKeyResponse {
    pub key_id: String,
    pub raw_key: Option<String>,
    pub name: String,
    pub source_id: Option<String>,
    pub source: String,
    pub owner_principal_id: String,
    pub owner_scope_id: String,
    pub scope_kind: String,
    pub storage_mode: String,
    pub isolated: bool,
    pub isolation_group_id: String,
    pub status: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMemorySourceHttpRequest {
    pub name: String,
    pub source_kind: Option<String>,
    pub source: Option<String>,
    pub source_uri: Option<String>,
    pub sync_mode: Option<String>,
    pub local_root: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSourceAccessKeyHttpRequest {
    pub raw_key: Option<String>,
    pub name: String,
    pub source: Option<String>,
    pub owner_principal_id: Option<String>,
    pub scope_kind: Option<String>,
    pub storage_mode: Option<String>,
    pub isolated: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct MemorySourceResponse {
    pub source_id: String,
    pub source_kind: String,
    pub name: String,
    pub owner_principal_id: String,
    pub owner_scope_id: String,
    pub source_uri: Option<String>,
    pub sync_mode: String,
    pub local_root: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListMemorySourcesQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ImportProjectDocumentHttpRequest {
    pub scope_id: Option<String>,
    pub canonical_uri: String,
    pub title: String,
    pub content_text: String,
    pub local_path: Option<String>,
    pub sync_state: Option<String>,
    pub conflict_state: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListProjectDocumentsQuery {
    pub limit: Option<usize>,
    pub query: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SyncProjectDocumentsHttpRequest {
    pub scope_id: Option<String>,
    pub local_root: Option<String>,
    pub dry_run: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ProjectDocumentResponse {
    pub document_id: String,
    pub source_id: String,
    pub scope_id: String,
    pub local_path: Option<String>,
    pub canonical_uri: String,
    pub title: String,
    pub content_hash: String,
    pub last_seen_mtime: Option<String>,
    pub sync_state: String,
    pub conflict_state: String,
    pub artifact_id: Option<String>,
    pub memory_id: Option<String>,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectDocumentDraftResponse {
    pub canonical_uri: String,
    pub local_path: String,
    pub title: String,
    pub content_hash: String,
    pub sync_state: String,
    pub metadata: Value,
}

#[derive(Debug, Serialize)]
pub struct MissingProjectDocumentResponse {
    pub canonical_uri: String,
    pub sync_state: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectDocumentConflictResponse {
    pub canonical_uri: String,
    pub sync_state: String,
    pub conflict_state: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProjectDocumentSyncResponse {
    pub dry_run: bool,
    pub planned_documents: Vec<ProjectDocumentDraftResponse>,
    pub imported: Vec<ProjectDocumentResponse>,
    pub missing: Vec<MissingProjectDocumentResponse>,
    pub conflicts: Vec<ProjectDocumentConflictResponse>,
}

#[derive(Debug, Serialize)]
pub struct ProjectDocumentProjectionResponse {
    pub document: ProjectDocumentResponse,
    pub projection_path: String,
    pub markdown: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAccessKeyHttpRequest {
    pub name: Option<String>,
    pub storage_mode: Option<String>,
    pub isolated: Option<bool>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMemoryRequest {
    pub scope_id: Option<String>,
    pub title: Option<String>,
    pub body: String,
    pub artifact_kind: Option<String>,
    pub memory_kind: Option<String>,
    #[serde(default)]
    pub source_refs: Vec<String>,
    pub visibility: Option<String>,
    pub sensitivity: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateImageRequest {
    pub scope_id: Option<String>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub image_base64: String,
    pub media_type: String,
    pub file_extension: Option<String>,
    pub memory_kind: Option<String>,
    #[serde(default)]
    pub source_refs: Vec<String>,
    pub visibility: Option<String>,
    pub sensitivity: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateMemoryResponse {
    pub artifact_id: String,
    pub memory_id: String,
    pub scope_id: String,
    pub owner_scope_id: String,
    pub title: String,
    pub body: String,
    pub language_code: Option<String>,
    pub memory_kind: String,
    pub memory_state: String,
    pub evidence_count: usize,
    pub wrote_pg: bool,
    pub wrote_markdown: bool,
}

#[derive(Debug, Serialize)]
pub struct CreateImageResponse {
    pub asset_id: String,
    pub asset_uri: String,
    pub artifact_id: String,
    pub memory_id: String,
    pub scope_id: String,
    pub owner_scope_id: String,
    pub title: String,
    pub body: String,
    pub language_code: Option<String>,
    pub memory_kind: String,
    pub memory_state: String,
    pub evidence_count: usize,
    pub vision_caption: Option<String>,
    pub vision_model_alias: Option<String>,
    pub llm_notice: Option<String>,
    pub wrote_pg: bool,
    pub wrote_markdown: bool,
}

#[derive(Debug, Deserialize)]
pub struct PromoteMemoryHttpRequest {
    pub source_scope_id: String,
    pub memory_id: String,
    pub source_scope_type: String,
    pub target_scope_id: String,
    pub target_scope_type: String,
    pub target_visibility: String,
}

#[derive(Debug, Serialize)]
pub struct PromoteMemoryHttpResponse {
    pub memory_id: String,
    pub scope_id: String,
    pub owner_scope_id: String,
    pub published_from_scope_id: Option<String>,
    pub title: String,
    pub body: String,
    pub memory_kind: String,
    pub memory_state: String,
    pub visibility: String,
    pub language_code: Option<String>,
    pub wrote_pg: bool,
    pub wrote_markdown: bool,
}

#[derive(Debug, Deserialize, Default)]
pub struct InspectLifecycleQuery {
    pub query: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct LifecycleAuditQuery {
    pub scope_id: Option<String>,
    pub memory_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
pub struct LifecycleReportQuery {
    pub scope_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ReportInputQuery {
    pub input_dir: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct BenchmarkRunHttpRequest {
    pub suite: Option<String>,
    pub scope_id: Option<String>,
    pub output_dir: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TraceInspectQuery {
    pub input_dir: Option<String>,
    pub trace_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TraceLatestHttpRequest {
    pub scope_id: Option<String>,
    pub query: String,
    pub limit: Option<usize>,
    pub max_records: Option<usize>,
    pub max_chars: Option<usize>,
    pub debug_candidates: Option<bool>,
    pub output_dir: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PassportExportHttpRequest {
    pub scope_id: Option<String>,
    pub output_dir: Option<String>,
    pub limit: Option<usize>,
    pub redact_sensitive: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PassportImportHttpRequest {
    pub input_dir: String,
    pub target_scope_id: Option<String>,
    pub dry_run: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CompatReportQuery {
    pub scope_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ConnectorDryRunQuery {
    pub connector: Option<String>,
    pub root_path: Option<String>,
    pub max_items: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ConnectorSyncPlanQuery {
    pub connector: Option<String>,
    pub root_path: Option<String>,
    pub scope_id: Option<String>,
    pub max_items: Option<usize>,
    pub allow_remote_fetch: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ConnectorImportDraftQuery {
    pub connector: Option<String>,
    pub root_path: Option<String>,
    pub scope_id: Option<String>,
    pub max_items: Option<usize>,
    pub proposal: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ConnectorProposalQueueQuery {
    pub connector: Option<String>,
    pub root_path: Option<String>,
    pub scope_id: Option<String>,
    pub max_items: Option<usize>,
    pub persist: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ConnectorProposalApplyPlanQuery {
    pub queue_id: Option<String>,
    pub connector: Option<String>,
    pub root_path: Option<String>,
    pub scope_id: Option<String>,
    pub approved_queue_item_ids: Option<String>,
    pub confirmation_token: Option<String>,
    pub max_items: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ConnectorProposalApplyPlanApplyRequest {
    pub queue_id: Option<String>,
    pub connector: Option<String>,
    pub root_path: Option<String>,
    pub scope_id: Option<String>,
    pub source_id: Option<String>,
    pub approved_queue_item_ids: Vec<String>,
    pub confirmation_token: String,
    pub max_items: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct LifecycleStatusHttpRequest {
    pub status: Option<String>,
    pub reason: Option<String>,
    pub actor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchContextHttpRequest {
    pub scope_id: Option<String>,
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SearchContextHttpResponse {
    pub query: String,
    pub scope_id: String,
    pub memory_count: usize,
    pub memories: Vec<MemorySummary>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertAgentContextHttpRequest {
    pub scope_id: Option<String>,
    pub session_id: String,
    pub task_id: Option<String>,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub labels: Vec<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListAgentContextsQuery {
    pub scope_id: Option<String>,
    pub session_id: String,
    pub task_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct PromoteAgentContextHttpRequest {
    pub memory_kind: Option<String>,
    pub visibility: Option<String>,
    pub sensitivity: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AgentContextResponse {
    pub context_id: String,
    pub source_id: Option<String>,
    pub key_id: Option<String>,
    pub scope_id: String,
    pub session_id: String,
    pub task_id: Option<String>,
    pub layer: String,
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct MemorySummary {
    pub memory_id: String,
    pub title: String,
    pub body: String,
    pub memory_kind: String,
    pub memory_state: String,
    pub evidence_count: usize,
}

#[derive(Debug, Deserialize, Default)]
pub struct BrowseMemoriesQuery {
    pub scope_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ExplorerFacet {
    pub key: String,
    pub label: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct ExplorerMemoryItem {
    pub memory_id: String,
    pub scope_id: String,
    pub title: String,
    pub body: String,
    pub preview: String,
    pub memory_kind: String,
    pub memory_state: String,
    pub visibility: String,
    pub sensitivity: String,
    pub evidence_count: usize,
    pub created_at: String,
    pub updated_at: String,
    pub ownership_key: String,
    pub ownership_label: String,
    pub agent_key: String,
    pub agent_label: String,
    pub source_label: String,
}

#[derive(Debug, Serialize)]
pub struct ExplorerMemoriesResponse {
    pub total_count: usize,
    pub filtered_scope: Option<String>,
    pub ownership_groups: Vec<ExplorerFacet>,
    pub agent_groups: Vec<ExplorerFacet>,
    pub scope_groups: Vec<ExplorerFacet>,
    pub memories: Vec<ExplorerMemoryItem>,
}

#[derive(Debug, Deserialize)]
pub struct AssistantChatRequest {
    pub prompt: String,
    pub scope_id: Option<String>,
    pub ownership: Option<String>,
    pub agent: Option<String>,
    pub memory_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct AssistantCitation {
    pub memory_id: String,
    pub title: String,
    pub scope_id: String,
    pub ownership_label: String,
    pub agent_label: String,
}

#[derive(Debug, Serialize)]
pub struct AssistantChatResponse {
    pub answer: String,
    pub matched_count: usize,
    pub citations: Vec<AssistantCitation>,
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({
                "error": self.message,
            })),
        )
            .into_response()
    }
}

impl ConnectorProposalStore {
    fn queue_path(&self, queue_id: &str) -> Result<PathBuf, ApiError> {
        let queue_id = queue_id.trim();
        if queue_id.is_empty() {
            return Err(ApiError::bad_request("queue_id is required"));
        }
        if !queue_id
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || value == '_' || value == '-')
        {
            return Err(ApiError::bad_request(
                "queue_id contains unsupported characters",
            ));
        }
        Ok(self.root.join(format!("{queue_id}.json")))
    }

    fn save(&self, report: &ConnectorProposalQueueReport) -> Result<PathBuf, ApiError> {
        fs::create_dir_all(&self.root).map_err(|error| {
            ApiError::internal(format!(
                "failed to create connector proposal store {}: {error}",
                self.root.display()
            ))
        })?;
        let path = self.queue_path(&report.queue_id)?;
        fs::write(
            &path,
            serde_json::to_string_pretty(&connector_proposal_queue_json(report)).map_err(
                |error| {
                    ApiError::internal(format!(
                        "failed to serialize connector proposal queue: {error}"
                    ))
                },
            )?,
        )
        .map_err(|error| {
            ApiError::internal(format!(
                "failed to write connector proposal queue {}: {error}",
                path.display()
            ))
        })?;
        Ok(path)
    }

    fn load(&self, queue_id: &str) -> Result<ConnectorProposalQueueReport, ApiError> {
        let path = self.queue_path(queue_id)?;
        let raw = fs::read_to_string(&path).map_err(|error| {
            if error.kind() == ErrorKind::NotFound {
                return ApiError::not_found(format!(
                    "connector proposal queue not found: {queue_id}"
                ));
            }
            ApiError::internal(format!(
                "failed to read connector proposal queue {}: {error}",
                path.display()
            ))
        })?;
        serde_json::from_str(&raw).map_err(|error| {
            ApiError::internal(format!(
                "failed to parse connector proposal queue {}: {error}",
                path.display()
            ))
        })
    }
}

fn connector_proposal_store_required(
    state: &HttpAppState,
) -> Result<&ConnectorProposalStore, ApiError> {
    state
        .connector_proposal_store
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("connector proposal store is not configured"))
}

fn connector_proposal_queue_http_json(
    report: &ConnectorProposalQueueReport,
    store_path: Option<PathBuf>,
    store_enabled: bool,
) -> serde_json::Value {
    let mut payload = connector_proposal_queue_json(report);
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "proposal_store".to_string(),
            json!({
                "enabled": store_enabled,
                "persisted": store_path.is_some(),
                "queue_id": report.queue_id,
                "path": store_path.map(|path| path.display().to_string()),
                "load_url": format!("/api/v1/compat/connectors/proposal-queue/{}", report.queue_id),
                "apply_plan_param": format!("queue_id={}", report.queue_id),
            }),
        );
        if let Some(regions) = object
            .get_mut("coverage_gate")
            .and_then(|value| value.get_mut("covered_regions"))
            .and_then(|value| value.as_array_mut())
        {
            regions.push(json!("service_side_proposal_store"));
        }
    }
    payload
}

fn attach_connector_proposal_store_metadata(
    mut payload: serde_json::Value,
    queue_id: &str,
) -> serde_json::Value {
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "proposal_store".to_string(),
            json!({
                "enabled": true,
                "persisted": true,
                "queue_id": queue_id,
                "load_url": format!("/api/v1/compat/connectors/proposal-queue/{queue_id}"),
                "apply_plan_param": format!("queue_id={queue_id}"),
            }),
        );
        if let Some(regions) = object
            .get_mut("coverage_gate")
            .and_then(|value| value.get_mut("covered_regions"))
            .and_then(|value| value.as_array_mut())
        {
            regions.push(json!("service_side_proposal_store"));
        }
    }
    payload
}

pub fn has_route(path: &str) -> bool {
    HTTP_ROUTES.contains(&path)
}

pub fn build_router(state: HttpAppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/livez", get(livez))
        .route("/metrics", get(metrics))
        .route("/api/v1/meta", get(meta))
        .route("/api/v1/memories", post(create_memory))
        .route("/api/v1/memories/promote", post(promote_memory))
        .route(
            "/api/v1/lifecycle/memories/{scope_id}/{memory_id}",
            get(inspect_memory_lifecycle),
        )
        .route(
            "/api/v1/lifecycle/memories/{scope_id}/{memory_id}/status",
            post(change_memory_lifecycle_status),
        )
        .route(
            "/api/v1/lifecycle/memories/{scope_id}/{memory_id}/forget",
            post(forget_memory_lifecycle),
        )
        .route(
            "/api/v1/lifecycle/memories/{scope_id}/{memory_id}/restore",
            post(restore_memory_lifecycle),
        )
        .route("/api/v1/lifecycle/audit", get(list_lifecycle_audit))
        .route("/api/v1/lifecycle/report", get(lifecycle_report))
        .route("/api/v1/benchmark/run", post(benchmark_run))
        .route("/api/v1/benchmark/runs", get(benchmark_runs))
        .route("/api/v1/benchmark/report", get(benchmark_report))
        .route("/api/v1/benchmark/failures", get(benchmark_failures))
        .route("/api/v1/recall/traces/latest", post(trace_latest))
        .route("/api/v1/recall/traces/inspect", get(trace_inspect))
        .route("/api/v1/health/report", get(health_report))
        .route("/api/v1/passports/export", post(passport_export))
        .route("/api/v1/passports/import", post(passport_import))
        .route("/api/v1/passports/manifest", get(passport_manifest))
        .route("/api/v1/compat/report", get(compat_report))
        .route(
            "/api/v1/compat/connectors/dry-run",
            get(compat_connector_dry_run),
        )
        .route(
            "/api/v1/compat/connectors/sync-plan",
            get(compat_connector_sync_plan),
        )
        .route(
            "/api/v1/compat/connectors/import-draft",
            get(compat_connector_import_draft),
        )
        .route(
            "/api/v1/compat/connectors/proposal-queue",
            get(compat_connector_proposal_queue),
        )
        .route(
            "/api/v1/compat/connectors/proposal-queue/{queue_id}",
            get(compat_connector_proposal_queue_get),
        )
        .route(
            "/api/v1/compat/connectors/proposal-apply-plan",
            get(compat_connector_proposal_apply_plan),
        )
        .route(
            "/api/v1/compat/connectors/proposal-apply-plan/apply",
            post(compat_connector_proposal_apply_plan_apply),
        )
        .route("/api/v1/images", post(create_image))
        .route("/api/v1/context", post(search_context))
        .route("/api/v1/context/search", post(search_context))
        .route(
            "/api/v1/agent-contexts",
            post(upsert_agent_context).get(list_agent_contexts),
        )
        .route(
            "/api/v1/agent-contexts/{context_id}",
            delete(delete_agent_context),
        )
        .route(
            "/api/v1/agent-contexts/{context_id}/promote",
            post(promote_agent_context),
        )
        .route("/api/v1/proposals", get(list_memory_proposals))
        .route("/api/v1/proposals/{proposal_id}", get(get_memory_proposal))
        .route(
            "/api/v1/proposals/{proposal_id}/approve",
            post(approve_memory_proposal),
        )
        .route(
            "/api/v1/proposals/{proposal_id}/reject",
            post(reject_memory_proposal),
        )
        .route(
            "/api/v1/proposals/{proposal_id}/apply",
            post(apply_memory_proposal),
        )
        .route(
            "/api/v1/proposals/review-policy/evaluate",
            post(evaluate_review_policy),
        )
        .route(
            "/api/v1/memories/{scope_id}/{memory_id}/versions",
            get(list_memory_versions),
        )
        .route(
            "/api/v1/memories/{scope_id}/{memory_id}/timeline",
            get(get_memory_timeline),
        )
        .route(
            "/api/v1/memories/{scope_id}/{memory_id}/rollback",
            post(rollback_memory),
        )
        .route(
            "/api/v1/distillation/profiles",
            get(list_distillation_profiles),
        )
        .route(
            "/api/v1/distillation/profiles/{profile_id}",
            put(upsert_distillation_profile),
        )
        .route(
            "/api/v1/distillation/preview",
            post(evaluate_distillation_preview),
        )
        .route("/api/v1/explorer/memories", get(browse_memories))
        .route("/api/v1/assistant/chat", post(chat_with_memory_assistant))
        .route(
            "/api/v1/keys",
            post(create_access_key).get(list_access_keys),
        )
        .route("/api/v1/keys/{key_id}", patch(update_access_key))
        .route("/api/v1/keys/{key_id}/rotate", post(rotate_access_key))
        .route("/api/v1/keys/{key_id}/stats", get(access_key_stats))
        .route(
            "/api/v1/sources",
            post(create_memory_source).get(list_memory_sources),
        )
        .route("/api/v1/sources/{source_id}", get(get_memory_source))
        .route(
            "/api/v1/sources/{source_id}/keys",
            post(create_source_access_key).get(list_source_access_keys),
        )
        .route(
            "/api/v1/sources/{source_id}/documents",
            get(list_project_documents),
        )
        .route(
            "/api/v1/sources/{source_id}/documents/{document_id}/projection",
            get(get_project_document_projection),
        )
        .route(
            "/api/v1/sources/{source_id}/documents/import",
            post(import_project_document),
        )
        .route(
            "/api/v1/sources/{source_id}/documents/conflicts",
            get(list_project_document_conflicts),
        )
        .route(
            "/api/v1/sources/{source_id}/documents/sync",
            post(sync_project_documents),
        )
        .route("/api/v1/metrics/keys", get(key_metrics))
        .with_state(state)
}

async fn index(State(state): State<HttpAppState>) -> Html<String> {
    Html(build_console_page(&state.metadata))
}

async fn healthz() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

async fn readyz(State(state): State<HttpAppState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "ready",
        "stores": {
            "pg": state.metadata.features.pg,
            "markdown": state.metadata.features.markdown,
        }
    }))
}

async fn livez() -> Json<serde_json::Value> {
    Json(json!({ "status": "alive" }))
}

async fn metrics() -> Json<memory_observability::MetricsSnapshot> {
    Json(memory_observability::metrics_snapshot())
}

async fn key_metrics() -> Json<memory_observability::MetricsSnapshot> {
    Json(memory_observability::metrics_snapshot())
}

async fn create_access_key(
    State(state): State<HttpAppState>,
    Json(payload): Json<CreateAccessKeyHttpRequest>,
) -> Result<(StatusCode, Json<AccessKeyResponse>), ApiError> {
    let source_kind = parse_key_source(payload.source.as_deref().unwrap_or("http"))?;
    let scope_kind = parse_key_scope(payload.scope_kind.as_deref().unwrap_or("personal"))?;
    let storage_mode = parse_storage_mode(payload.storage_mode.as_deref().unwrap_or("all"))?;
    let owner_principal_id = payload
        .owner_principal_id
        .unwrap_or_else(|| "local-user".to_string());
    let owner_scope_id = payload
        .owner_scope_id
        .map(ScopeId::from_string)
        .unwrap_or_else(|| state.default_scope_id.clone());
    let result = state
        .kernel
        .create_access_key(CreateAccessKeyRequest {
            raw_key: payload.raw_key,
            display_name: payload.name,
            source_id: None,
            source_kind,
            owner_principal_id,
            owner_scope_id,
            scope_kind,
            storage_mode,
            is_fully_isolated: payload.isolated.unwrap_or(false),
        })
        .await
        .map_err(api_error_from_anyhow)?;

    Ok((
        StatusCode::CREATED,
        Json(access_key_response(result.access_key, Some(result.raw_key))),
    ))
}

async fn list_access_keys(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AccessKeyResponse>>, ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let keys = state
        .kernel
        .list_access_keys_for_context(&context, 200)
        .await
        .map_err(api_error_from_anyhow)?;
    Ok(Json(
        keys.into_iter()
            .map(|key| access_key_response(key, None))
            .collect(),
    ))
}

async fn update_access_key(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Path(key_id): Path<String>,
    Json(payload): Json<UpdateAccessKeyHttpRequest>,
) -> Result<Json<AccessKeyResponse>, ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let updated = state
        .kernel
        .update_access_key_for_context(
            &context,
            UpdateAccessKeyRequest {
                key_id: AccessKeyId::from_string(key_id),
                display_name: payload.name,
                storage_mode: payload
                    .storage_mode
                    .as_deref()
                    .map(parse_storage_mode)
                    .transpose()?,
                status: payload
                    .status
                    .as_deref()
                    .map(parse_key_status)
                    .transpose()?,
                is_fully_isolated: payload.isolated,
            },
        )
        .await
        .map_err(api_error_from_anyhow)?;
    Ok(Json(access_key_response(updated, None)))
}

async fn rotate_access_key(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Path(key_id): Path<String>,
) -> Result<(StatusCode, Json<AccessKeyResponse>), ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let result = state
        .kernel
        .rotate_access_key_for_context(&context, &AccessKeyId::from_string(key_id), None)
        .await
        .map_err(api_error_from_anyhow)?;
    Ok((
        StatusCode::CREATED,
        Json(access_key_response(result.access_key, Some(result.raw_key))),
    ))
}

async fn access_key_stats(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Path(key_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let stats = state
        .kernel
        .access_key_usage_stats_for_context(&context, &AccessKeyId::from_string(key_id))
        .await
        .map_err(api_error_from_anyhow)?
        .ok_or_else(|| ApiError::not_found("access key not found"))?;
    Ok(Json(json!({ "stats": stats })))
}

async fn create_memory_source(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateMemorySourceHttpRequest>,
) -> Result<(StatusCode, Json<MemorySourceResponse>), ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let source_kind = payload
        .source_kind
        .or(payload.source)
        .unwrap_or_else(|| "custom".to_string());
    let mut source = MemorySource::new(
        source_kind,
        payload.name,
        context.principal_id.clone(),
        context.owner_scope_id.clone(),
    )
    .map_err(api_error_from_domain)?;
    if let Some(source_uri) = payload.source_uri {
        source = source
            .with_source_uri(source_uri)
            .map_err(api_error_from_domain)?;
    }
    if let Some(local_root) = payload.local_root {
        source = source
            .with_local_root(local_root)
            .map_err(api_error_from_domain)?;
    }
    if let Some(sync_mode) = payload.sync_mode {
        source = source.with_sync_mode(parse_source_sync_mode(&sync_mode)?);
    }

    let source = state
        .kernel
        .upsert_memory_source(source, Some(&context))
        .await
        .map_err(api_error_from_anyhow)?;
    Ok((StatusCode::CREATED, Json(memory_source_response(source))))
}

async fn list_memory_sources(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Query(query): Query<ListMemorySourcesQuery>,
) -> Result<Json<Vec<MemorySourceResponse>>, ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let sources = state
        .kernel
        .list_memory_sources(
            context.owner_scope_id.clone(),
            query.limit.unwrap_or(100).clamp(1, 500),
            Some(&context),
        )
        .await
        .map_err(api_error_from_anyhow)?;
    Ok(Json(
        sources
            .into_iter()
            .map(memory_source_response)
            .collect::<Vec<_>>(),
    ))
}

async fn get_memory_source(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Path(source_id): Path<String>,
) -> Result<Json<MemorySourceResponse>, ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let source =
        get_memory_source_for_context(&state, &context, SourceId::from_string(source_id)).await?;
    Ok(Json(memory_source_response(source)))
}

async fn list_source_access_keys(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Path(source_id): Path<String>,
    Query(query): Query<ListMemorySourcesQuery>,
) -> Result<Json<Vec<AccessKeyResponse>>, ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let source_id = SourceId::from_string(source_id);
    let _source = get_memory_source_for_context(&state, &context, source_id.clone()).await?;
    let keys = state
        .kernel
        .list_access_keys_for_source(source_id, query.limit.unwrap_or(100).clamp(1, 500))
        .await
        .map_err(api_error_from_anyhow)?;
    Ok(Json(
        keys.into_iter()
            .map(|key| access_key_response(key, None))
            .collect(),
    ))
}

async fn create_source_access_key(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Path(source_id): Path<String>,
    Json(payload): Json<CreateSourceAccessKeyHttpRequest>,
) -> Result<(StatusCode, Json<AccessKeyResponse>), ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let source =
        get_memory_source_for_context(&state, &context, SourceId::from_string(source_id)).await?;
    let source_kind = payload
        .source
        .as_deref()
        .map(parse_key_source)
        .transpose()?
        .unwrap_or(KeySourceKind::Custom);
    let scope_kind = parse_key_scope(payload.scope_kind.as_deref().unwrap_or("personal"))?;
    let storage_mode = parse_storage_mode(payload.storage_mode.as_deref().unwrap_or("all"))?;
    let owner_principal_id = payload
        .owner_principal_id
        .unwrap_or_else(|| source.owner_principal_id.clone());
    let result = state
        .kernel
        .create_access_key(CreateAccessKeyRequest {
            raw_key: payload.raw_key,
            display_name: payload.name,
            source_id: Some(source.id),
            source_kind,
            owner_principal_id,
            owner_scope_id: source.owner_scope_id,
            scope_kind,
            storage_mode,
            is_fully_isolated: payload.isolated.unwrap_or(false),
        })
        .await
        .map_err(api_error_from_anyhow)?;

    Ok((
        StatusCode::CREATED,
        Json(access_key_response(result.access_key, Some(result.raw_key))),
    ))
}

async fn import_project_document(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Path(source_id): Path<String>,
    Json(payload): Json<ImportProjectDocumentHttpRequest>,
) -> Result<(StatusCode, Json<ProjectDocumentResponse>), ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let source =
        get_memory_source_for_context(&state, &context, SourceId::from_string(source_id)).await?;
    let scope_id = payload
        .scope_id
        .map(ScopeId::from_string)
        .unwrap_or_else(|| source.owner_scope_id.clone());
    ensure_context_scope_access(&context, &scope_id)?;
    let mut request = ImportProjectDocumentRequest::new(
        source.id,
        scope_id,
        payload.canonical_uri,
        payload.title,
        payload.content_text,
    );
    request.local_path = payload.local_path;
    request.sync_state = payload
        .sync_state
        .as_deref()
        .map(parse_document_sync_state)
        .transpose()?
        .unwrap_or(DocumentSyncState::Clean);
    request.conflict_state = payload
        .conflict_state
        .as_deref()
        .map(parse_document_conflict_state)
        .transpose()?
        .unwrap_or(DocumentConflictState::None);
    request.metadata = payload.metadata.unwrap_or_else(|| json!({}));
    request.context = Some(context);

    let document = state
        .kernel
        .import_project_document(request)
        .await
        .map_err(api_error_from_anyhow)?;
    Ok((
        StatusCode::CREATED,
        Json(project_document_response(document)),
    ))
}

async fn list_project_documents(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Path(source_id): Path<String>,
    Query(query): Query<ListProjectDocumentsQuery>,
) -> Result<Json<Vec<ProjectDocumentResponse>>, ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let source =
        get_memory_source_for_context(&state, &context, SourceId::from_string(source_id)).await?;
    let mut request = ListProjectDocumentsRequest::new(source.id);
    request.limit = query.limit.unwrap_or(100).clamp(1, 500);
    request.query = query.query;
    request.context = Some(context);

    let documents = state
        .kernel
        .list_project_documents(request)
        .await
        .map_err(api_error_from_anyhow)?;
    Ok(Json(
        documents
            .into_iter()
            .map(project_document_response)
            .collect(),
    ))
}

async fn get_project_document_projection(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Path((source_id, document_id)): Path<(String, String)>,
) -> Result<Json<ProjectDocumentProjectionResponse>, ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let projection = state
        .kernel
        .get_project_document_projection(
            SourceId::from_string(source_id),
            memory_domain::ProjectDocumentId::from_string(document_id),
            Some(&context),
        )
        .await
        .map_err(api_error_from_anyhow)?;
    Ok(Json(ProjectDocumentProjectionResponse {
        document: project_document_response(projection.document),
        projection_path: projection.projection_path.display().to_string(),
        markdown: projection.markdown,
    }))
}

async fn list_project_document_conflicts(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Path(source_id): Path<String>,
    Query(query): Query<ListMemorySourcesQuery>,
) -> Result<Json<Vec<ProjectDocumentResponse>>, ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let source =
        get_memory_source_for_context(&state, &context, SourceId::from_string(source_id)).await?;
    let documents = state
        .kernel
        .list_project_document_conflicts(
            source.id,
            query.limit.unwrap_or(100).clamp(1, 500),
            Some(&context),
        )
        .await
        .map_err(api_error_from_anyhow)?;
    Ok(Json(
        documents
            .into_iter()
            .map(project_document_response)
            .collect(),
    ))
}

async fn sync_project_documents(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Path(source_id): Path<String>,
    Json(payload): Json<SyncProjectDocumentsHttpRequest>,
) -> Result<Json<ProjectDocumentSyncResponse>, ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let source =
        get_memory_source_for_context(&state, &context, SourceId::from_string(source_id)).await?;
    let scope_id = payload
        .scope_id
        .map(ScopeId::from_string)
        .unwrap_or_else(|| source.owner_scope_id.clone());
    ensure_context_scope_access(&context, &scope_id)?;
    let local_root = payload
        .local_root
        .or_else(|| source.local_root.clone())
        .ok_or_else(|| ApiError::bad_request("local_root is required for project document sync"))?;
    let existing = state
        .kernel
        .list_project_documents(ListProjectDocumentsRequest {
            source_id: source.id.clone(),
            limit: 500,
            query: None,
            context: Some(context.clone()),
        })
        .await
        .map_err(api_error_from_anyhow)?;
    let snapshots = existing
        .iter()
        .map(|document| ProjectDocumentSnapshot {
            canonical_uri: document.canonical_uri.clone(),
            content_hash: document.content_hash.clone(),
        })
        .collect::<Vec<_>>();
    let plan = LocalProjectDocumentSyncEngine::new(PathBuf::from(local_root))
        .scan(&snapshots)
        .map_err(|error| {
            ApiError::bad_request(format!("failed to scan local project documents: {error}"))
        })?;
    let planned_documents = plan
        .documents
        .iter()
        .map(project_document_draft_response)
        .collect::<Vec<_>>();
    let dry_run = payload.dry_run.unwrap_or(false);
    if dry_run {
        return Ok(Json(ProjectDocumentSyncResponse {
            dry_run,
            planned_documents,
            imported: Vec::new(),
            missing: plan
                .missing
                .into_iter()
                .map(missing_project_document_response)
                .collect(),
            conflicts: plan
                .conflicts
                .into_iter()
                .map(project_document_conflict_response)
                .collect(),
        }));
    }

    let result = state
        .kernel
        .apply_project_document_sync_plan(ApplyProjectDocumentSyncPlanRequest {
            source_id: source.id,
            scope_id,
            plan,
            context: Some(context),
        })
        .await
        .map_err(api_error_from_anyhow)?;
    Ok(Json(ProjectDocumentSyncResponse {
        dry_run,
        planned_documents,
        imported: result
            .imported
            .into_iter()
            .map(project_document_response)
            .collect(),
        missing: result
            .missing
            .into_iter()
            .map(missing_project_document_response)
            .collect(),
        conflicts: result
            .conflicts
            .into_iter()
            .map(project_document_conflict_response)
            .collect(),
    }))
}

async fn upsert_agent_context(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Json(payload): Json<UpsertAgentContextHttpRequest>,
) -> Result<(StatusCode, Json<AgentContextResponse>), ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let scope_id = payload
        .scope_id
        .map(ScopeId::from_string)
        .unwrap_or_else(|| context.owner_scope_id.clone());
    ensure_context_scope_access(&context, &scope_id)?;
    let expires_at = parse_optional_timestamp(payload.expires_at.as_deref())?;
    let mut request =
        UpsertAgentContextRequest::new(scope_id, payload.session_id, payload.title, payload.body);
    request.task_id = payload.task_id;
    request.labels = payload.labels;
    request.expires_at = expires_at;
    request.context = Some(context);

    let agent_context = state
        .kernel
        .upsert_agent_context(request)
        .await
        .map_err(api_error_from_anyhow)?;
    Ok((
        StatusCode::CREATED,
        Json(agent_context_response(agent_context)),
    ))
}

async fn list_agent_contexts(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Query(query): Query<ListAgentContextsQuery>,
) -> Result<Json<Vec<AgentContextResponse>>, ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let scope_id = query
        .scope_id
        .map(ScopeId::from_string)
        .unwrap_or_else(|| context.owner_scope_id.clone());
    ensure_context_scope_access(&context, &scope_id)?;
    let mut request = ListAgentContextsRequest::new(scope_id, query.session_id);
    request.task_id = query.task_id;
    request.limit = query.limit.unwrap_or(50).clamp(1, 500);
    request.context = Some(context);

    let contexts = state
        .kernel
        .list_agent_contexts(request)
        .await
        .map_err(api_error_from_anyhow)?;
    Ok(Json(
        contexts
            .into_iter()
            .map(agent_context_response)
            .collect::<Vec<_>>(),
    ))
}

async fn delete_agent_context(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Path(context_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    state
        .kernel
        .delete_agent_context(AgentContextId::from_string(context_id), Some(&context))
        .await
        .map_err(api_error_from_anyhow)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn promote_agent_context(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Path(context_id): Path<String>,
    Json(payload): Json<PromoteAgentContextHttpRequest>,
) -> Result<(StatusCode, Json<CreateMemoryResponse>), ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let mut request = PromoteAgentContextRequest::new(AgentContextId::from_string(context_id));
    request.memory_kind = payload
        .memory_kind
        .as_deref()
        .map(parse_memory_kind)
        .transpose()?;
    request.visibility = parse_visibility(payload.visibility.as_deref())?;
    request.sensitivity = parse_sensitivity(payload.sensitivity.as_deref())?;
    request.context = Some(context);

    let result = state
        .kernel
        .promote_agent_context(request)
        .await
        .map_err(api_error_from_anyhow)?;
    Ok((StatusCode::CREATED, Json(remember_text_response(result))))
}

async fn meta(State(state): State<HttpAppState>) -> Json<ApiMetadata> {
    Json(state.metadata.clone())
}

async fn create_memory(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateMemoryRequest>,
) -> Result<(StatusCode, Json<CreateMemoryResponse>), ApiError> {
    let context = resolve_http_context(&state, &headers).await?;
    validate_text_field_len("body", &payload.body, MAX_MEMORY_BODY_CHARS)?;
    let scope_id = payload
        .scope_id
        .map(ScopeId::from_string)
        .unwrap_or_else(|| state.default_scope_id.clone());
    let artifact_kind = parse_artifact_kind(payload.artifact_kind.as_deref())?;
    let memory_kind = payload
        .memory_kind
        .as_deref()
        .map(parse_memory_kind)
        .transpose()?;
    let visibility = parse_visibility(payload.visibility.as_deref())?;
    let sensitivity = parse_sensitivity(payload.sensitivity.as_deref())?;

    let result = state
        .kernel
        .remember_text(RememberTextRequest {
            scope_id,
            title: payload.title,
            body: payload.body,
            artifact_kind,
            memory_kind,
            source_refs: payload.source_refs,
            visibility,
            sensitivity,
            context,
        })
        .await
        .map_err(api_error_from_anyhow)?;

    Ok((
        StatusCode::CREATED,
        Json(CreateMemoryResponse {
            artifact_id: result.artifact.id.as_str().to_string(),
            memory_id: result.memory.id.as_str().to_string(),
            scope_id: result.memory.scope_id.as_str().to_string(),
            owner_scope_id: result.memory.owner_scope_id.as_str().to_string(),
            title: result.memory.title.clone(),
            body: result.memory.body.clone(),
            language_code: result.memory.language_code.clone(),
            memory_kind: memory_kind_label(result.memory.kind).to_string(),
            memory_state: result.memory.state.as_str().to_string(),
            evidence_count: result.memory.evidence_count,
            wrote_pg: result.wrote_pg,
            wrote_markdown: result.wrote_markdown,
        }),
    ))
}

async fn create_image(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateImageRequest>,
) -> Result<(StatusCode, Json<CreateImageResponse>), ApiError> {
    let context = resolve_http_context(&state, &headers).await?;
    if let Some(body) = payload.body.as_deref() {
        validate_text_field_len("body", body, MAX_MEMORY_BODY_CHARS)?;
    }
    let scope_id = payload
        .scope_id
        .map(ScopeId::from_string)
        .unwrap_or_else(|| state.default_scope_id.clone());
    let memory_kind = payload
        .memory_kind
        .as_deref()
        .map(parse_memory_kind)
        .transpose()?;
    let visibility = parse_visibility(payload.visibility.as_deref())?;
    let sensitivity = parse_sensitivity(payload.sensitivity.as_deref())?;
    let bytes = STANDARD
        .decode(payload.image_base64.as_bytes())
        .map_err(|_| ApiError::bad_request("invalid image_base64 payload"))?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(ApiError::bad_request(format!(
            "image payload exceeds {} bytes",
            MAX_IMAGE_BYTES
        )));
    }

    let result = state
        .kernel
        .remember_image(RememberImageRequest {
            scope_id,
            title: payload.title,
            body: payload.body,
            bytes,
            media_type: payload.media_type,
            file_extension: payload.file_extension,
            memory_kind,
            source_refs: payload.source_refs,
            visibility,
            sensitivity,
            context,
        })
        .await
        .map_err(api_error_from_anyhow)?;
    let asset_id = result.asset.reference.asset_id.clone();
    let asset_uri = result.asset.reference.uri();
    let vision_caption = result.vision.as_ref().map(|vision| vision.caption.clone());
    let vision_model_alias = result
        .vision
        .as_ref()
        .map(|vision| vision.model_alias.clone());
    let llm_notice = result
        .vision
        .as_ref()
        .and_then(|vision| vision.switch_notice.as_ref())
        .map(|notice| notice.message.clone());

    Ok((
        StatusCode::CREATED,
        Json(CreateImageResponse {
            asset_id,
            asset_uri,
            artifact_id: result.artifact.id.as_str().to_string(),
            memory_id: result.memory.id.as_str().to_string(),
            scope_id: result.memory.scope_id.as_str().to_string(),
            owner_scope_id: result.memory.owner_scope_id.as_str().to_string(),
            title: result.memory.title.clone(),
            body: result.memory.body.clone(),
            language_code: result.memory.language_code.clone(),
            memory_kind: memory_kind_label(result.memory.kind).to_string(),
            memory_state: result.memory.state.as_str().to_string(),
            evidence_count: result.memory.evidence_count,
            vision_caption,
            vision_model_alias,
            llm_notice,
            wrote_pg: result.wrote_pg,
            wrote_markdown: result.wrote_markdown,
        }),
    ))
}

async fn promote_memory(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Json(payload): Json<PromoteMemoryHttpRequest>,
) -> Result<(StatusCode, Json<PromoteMemoryHttpResponse>), ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let result = state
        .kernel
        .promote_memory_by_id_for_context(
            &context,
            ScopeId::from_string(payload.source_scope_id),
            MemoryId::from_string(payload.memory_id),
            PromoteMemoryRequest {
                source_scope_type: parse_scope_type(&payload.source_scope_type)?,
                target_scope_id: ScopeId::from_string(payload.target_scope_id),
                target_scope_type: parse_scope_type(&payload.target_scope_type)?,
                target_visibility: parse_visibility(Some(payload.target_visibility.as_str()))?,
            },
        )
        .await
        .map_err(api_error_from_anyhow)?;

    Ok((
        StatusCode::CREATED,
        Json(PromoteMemoryHttpResponse {
            memory_id: result.memory.id.as_str().to_string(),
            scope_id: result.memory.scope_id.as_str().to_string(),
            owner_scope_id: result.memory.owner_scope_id.as_str().to_string(),
            published_from_scope_id: result
                .memory
                .published_from_scope_id
                .as_ref()
                .map(|scope| scope.as_str().to_string()),
            title: result.memory.title.clone(),
            body: result.memory.body.clone(),
            memory_kind: memory_kind_label(result.memory.kind).to_string(),
            memory_state: result.memory.state.as_str().to_string(),
            visibility: visibility_label(result.memory.visibility).to_string(),
            language_code: result.memory.language_code.clone(),
            wrote_pg: result.wrote_pg,
            wrote_markdown: result.wrote_markdown,
        }),
    ))
}

async fn inspect_memory_lifecycle(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Path((scope_id, memory_id)): Path<(String, String)>,
    Query(query): Query<InspectLifecycleQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let context = resolve_http_context(&state, &headers).await?;
    let result = state
        .kernel
        .inspect_memory_lifecycle(InspectMemoryLifecycleRequest {
            scope_id: ScopeId::from_string(scope_id),
            memory_id: MemoryId::from_string(memory_id),
            query: query.query,
            context,
        })
        .await
        .map_err(api_error_from_anyhow)?;

    Ok(Json(json!({
        "memory": memory_to_summary(&result.memory),
        "record": lifecycle_record_json(&result.record),
        "explanation": result.explanation.map(|explanation| json!({
            "record_id": explanation.record_id,
            "score": explanation.score,
            "matched_scope": explanation.matched_scope,
            "matched_layer": explanation.matched_layer.as_str(),
            "matched_type": explanation.matched_type.as_str(),
            "status": explanation.status.as_str(),
            "confidence": explanation.confidence.as_str(),
            "reason": explanation.reason,
        })),
    })))
}

async fn change_memory_lifecycle_status(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Path((scope_id, memory_id)): Path<(String, String)>,
    Json(payload): Json<LifecycleStatusHttpRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let status = parse_record_status(
        payload
            .status
            .as_deref()
            .ok_or_else(|| ApiError::bad_request("status is required"))?,
    )?;
    lifecycle_status_result(state, context, scope_id, memory_id, payload, status).await
}

async fn forget_memory_lifecycle(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Path((scope_id, memory_id)): Path<(String, String)>,
    Json(payload): Json<LifecycleStatusHttpRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    lifecycle_status_result(
        state,
        context,
        scope_id,
        memory_id,
        payload,
        MemoryRecordStatus::Forgotten,
    )
    .await
}

async fn restore_memory_lifecycle(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Path((scope_id, memory_id)): Path<(String, String)>,
    Json(payload): Json<LifecycleStatusHttpRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    lifecycle_status_result(
        state,
        context,
        scope_id,
        memory_id,
        payload,
        MemoryRecordStatus::Active,
    )
    .await
}

async fn lifecycle_status_result(
    state: HttpAppState,
    context: RequestContext,
    scope_id: String,
    memory_id: String,
    payload: LifecycleStatusHttpRequest,
    status: MemoryRecordStatus,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = state
        .kernel
        .change_memory_lifecycle_status(ChangeMemoryLifecycleStatusRequest {
            scope_id: ScopeId::from_string(scope_id),
            memory_id: MemoryId::from_string(memory_id),
            status,
            reason: payload
                .reason
                .unwrap_or_else(|| "lifecycle update".to_string()),
            actor: payload
                .actor
                .unwrap_or_else(|| context.principal_id.clone()),
            context: Some(context),
        })
        .await
        .map_err(api_error_from_anyhow)?;

    Ok(Json(json!({
        "memory": memory_to_summary(&result.memory),
        "record": lifecycle_record_json(&result.record),
        "audit": {
            "action": result.audit_event.action,
            "actor": result.audit_event.actor,
            "record_id": result.audit_event.record_id,
            "before_status": result.audit_event.before_status.map(|status| status.as_str()),
            "after_status": result.audit_event.after_status.map(|status| status.as_str()),
            "reason": result.audit_event.reason,
            "created_at": format_timestamp(result.audit_event.created_at),
        },
        "wrote_pg": result.wrote_pg,
        "wrote_markdown": result.wrote_markdown,
    })))
}

async fn list_lifecycle_audit(
    State(state): State<HttpAppState>,
    _headers: HeaderMap,
    Query(query): Query<LifecycleAuditQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let events = state
        .kernel
        .list_lifecycle_audit_events(
            query.scope_id.map(ScopeId::from_string),
            query.memory_id.map(MemoryId::from_string),
            query.limit.unwrap_or(100),
        )
        .await
        .map_err(api_error_from_anyhow)?;

    Ok(Json(json!({
        "events": events.into_iter().map(|event| json!({
            "id": event.id,
            "scope_id": event.scope_id,
            "memory_id": event.memory_id,
            "action": event.action,
            "actor": event.actor,
            "before_status": event.before_status,
            "after_status": event.after_status,
            "reason": event.reason,
            "created_at": format_timestamp(event.created_at),
        })).collect::<Vec<_>>(),
    })))
}

async fn lifecycle_report(
    State(state): State<HttpAppState>,
    _headers: HeaderMap,
    Query(query): Query<LifecycleReportQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let report = state
        .kernel
        .memory_health_report(
            query.scope_id.map(ScopeId::from_string),
            query.limit.unwrap_or(500),
        )
        .await
        .map_err(api_error_from_anyhow)?;

    Ok(Json(json!({
        "scope_id": report.scope_id.as_ref().map(|scope| scope.as_str()),
        "total": report.total,
        "active": report.active,
        "candidate": report.candidate,
        "needs_review": report.needs_review,
        "archived": report.archived,
        "deprecated": report.deprecated,
        "forgotten": report.forgotten,
        "deleted": report.deleted,
        "restricted": report.restricted,
        "stale": report.stale,
        "source_backed": report.source_backed,
        "generated_at": format_timestamp(report.generated_at),
    })))
}

async fn benchmark_report(
    Query(query): Query<ReportInputQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let input_dir = report_input_dir(query.input_dir, "tests/reports/benchmark/latest");
    let summary_path = input_dir.join("summary.md");
    let metrics_path = input_dir.join("metrics.json");
    let summary = read_report_text(&summary_path)?;
    let metrics = read_report_json(&metrics_path)?;

    Ok(Json(json!({
        "input_dir": input_dir.display().to_string(),
        "summary": summary,
        "metrics": metrics,
    })))
}

async fn benchmark_run(
    State(state): State<HttpAppState>,
    Json(payload): Json<BenchmarkRunHttpRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let suite_name = payload.suite.unwrap_or_else(|| "meat-code-zh".to_string());
    let suite = BenchmarkSuiteKind::from_name(&suite_name)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let scope_id = ScopeId::from_string(
        payload
            .scope_id
            .unwrap_or_else(|| state.default_scope_id.as_str().to_string()),
    );
    let output_dir = payload
        .output_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| "tests/reports/benchmark/latest".into());
    let output = state
        .kernel
        .run_benchmark(BenchmarkRunRequest::new(suite, scope_id, output_dir))
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok(Json(benchmark_run_output_json(&output)))
}

async fn benchmark_runs(
    Query(query): Query<ReportInputQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let input_dir = report_input_dir(query.input_dir, "tests/reports/benchmark");
    let mut runs = Vec::new();
    if input_dir.join("metrics.json").exists() {
        runs.push(benchmark_run_summary_from_dir(&input_dir)?);
    }
    let entries = fs::read_dir(&input_dir).map_err(|error| {
        ApiError::bad_request(format!("failed to read {}: {error}", input_dir.display()))
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            ApiError::bad_request(format!(
                "failed to read {} entry: {error}",
                input_dir.display()
            ))
        })?;
        let path = entry.path();
        if path.is_dir() && path.join("metrics.json").exists() {
            runs.push(benchmark_run_summary_from_dir(&path)?);
        }
    }
    runs.sort_by(|left, right| {
        left["input_dir"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["input_dir"].as_str().unwrap_or_default())
    });

    Ok(Json(json!({
        "input_dir": input_dir.display().to_string(),
        "run_count": runs.len(),
        "runs": runs,
    })))
}

async fn benchmark_failures(
    Query(query): Query<ReportInputQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let input_dir = report_input_dir(query.input_dir, "tests/reports/benchmark/latest");
    let failures_path = input_dir.join("failures.jsonl");
    let raw = read_report_text(&failures_path)?;
    let failures = raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str::<serde_json::Value>(line).map_err(|error| {
                ApiError::bad_request(format!(
                    "failed to parse {} JSONL row: {error}",
                    failures_path.display()
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(json!({
        "input_dir": input_dir.display().to_string(),
        "failure_count": failures.len(),
        "failures": failures,
    })))
}

async fn trace_latest(
    State(state): State<HttpAppState>,
    Json(payload): Json<TraceLatestHttpRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    validate_text_field_len("query", &payload.query, MAX_QUERY_CHARS)?;
    let mut search = SearchContextRequest::new(
        ScopeId::from_string(
            payload
                .scope_id
                .unwrap_or_else(|| state.default_scope_id.as_str().to_string()),
        ),
        payload.query,
    );
    search.limit = payload.limit.unwrap_or(10);
    let mut trace_request = TraceSearchContextRequest::new(search);
    trace_request.budget = RecallTraceBudget {
        max_records: payload.max_records.unwrap_or(5),
        max_chars: payload.max_chars.unwrap_or(2_000),
    };
    trace_request.include_debug_candidates = payload.debug_candidates.unwrap_or(false);
    let output_dir = payload
        .output_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| "tests/reports/trace/latest".into());
    let result = state
        .kernel
        .search_context_with_trace(trace_request)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let paths = write_recall_trace_report(&output_dir, &result)
        .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok(Json(trace_result_json(&result, &paths)))
}

async fn trace_inspect(
    Query(query): Query<TraceInspectQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let input_dir = report_input_dir(query.input_dir, "tests/reports/trace/latest");
    let trace_path = input_dir.join("trace.json");
    let explanation_path = input_dir.join("explanation.md");
    let trace = read_report_json(&trace_path)?;
    if let Some(expected_trace_id) = query.trace_id.as_deref() {
        let actual_trace_id = trace
            .pointer("/trace/id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if actual_trace_id != expected_trace_id {
            return Err(ApiError::bad_request(format!(
                "trace id mismatch: expected {expected_trace_id}, found {actual_trace_id}"
            )));
        }
    }
    let explanation = fs::read_to_string(&explanation_path).ok();

    Ok(Json(json!({
        "input_dir": input_dir.display().to_string(),
        "trace": trace,
        "explanation": explanation,
    })))
}

async fn health_report(
    State(state): State<HttpAppState>,
    Query(query): Query<LifecycleReportQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let report = state
        .kernel
        .memory_health_report(
            query.scope_id.map(ScopeId::from_string),
            query.limit.unwrap_or(500),
        )
        .await
        .map_err(api_error_from_anyhow)?;

    Ok(Json(health_json(&report)))
}

async fn passport_export(
    State(state): State<HttpAppState>,
    Json(payload): Json<PassportExportHttpRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let scope_id = payload
        .scope_id
        .map(ScopeId::from_string)
        .unwrap_or_else(|| state.default_scope_id.clone());
    let mut request = MemoryPassportExportRequest::new(scope_id);
    if let Some(limit) = payload.limit {
        request.limit = limit;
    }
    if let Some(redact_sensitive) = payload.redact_sensitive {
        request.redact_sensitive = redact_sensitive;
    }
    let output_dir = payload
        .output_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| "tests/reports/passport/latest".into());

    let bundle = state
        .kernel
        .export_memory_passport(request)
        .await
        .map_err(api_error_from_anyhow)?;
    let paths =
        write_memory_passport_bundle(&output_dir, &bundle).map_err(api_error_from_anyhow)?;

    Ok(Json(passport_export_json(&bundle, &paths)))
}

async fn passport_import(
    State(state): State<HttpAppState>,
    Json(payload): Json<PassportImportHttpRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if payload.input_dir.trim().is_empty() {
        return Err(ApiError::bad_request("input_dir is required"));
    }

    let result = state
        .kernel
        .import_memory_passport(MemoryPassportImportRequest {
            input_dir: PathBuf::from(payload.input_dir),
            target_scope_id: payload.target_scope_id.map(ScopeId::from_string),
            dry_run: payload.dry_run.unwrap_or(true),
            context: None,
        })
        .await
        .map_err(api_error_from_anyhow)?;

    Ok(Json(passport_import_json(&result)))
}

async fn passport_manifest(
    Query(query): Query<ReportInputQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let input_dir = report_input_dir(query.input_dir, "tests/reports/passport/latest");
    let verification = verify_memory_passport_bundle(&input_dir).map_err(api_error_from_anyhow)?;

    Ok(Json(verification_json(&verification)))
}

async fn compat_report(
    State(state): State<HttpAppState>,
    Query(query): Query<CompatReportQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let scope_id = query
        .scope_id
        .map(ScopeId::from_string)
        .unwrap_or_else(|| state.default_scope_id.clone());
    let report = build_competitor_compatibility_report(scope_id).map_err(api_error_from_anyhow)?;

    Ok(Json(compatibility_report_json(&report)))
}

async fn compat_connector_dry_run(
    Query(query): Query<ConnectorDryRunQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let connector = required_connector(query.connector)?;
    let root_path = required_connector_root_path(query.root_path)?;
    let mut request = ConnectorDryRunRequest::new(connector, PathBuf::from(root_path));
    if let Some(max_items) = query.max_items {
        request.max_items = max_items;
    }
    let report = run_connector_dry_run(request).map_err(api_error_from_anyhow)?;

    Ok(Json(connector_dry_run_json(&report)))
}

async fn compat_connector_sync_plan(
    State(state): State<HttpAppState>,
    Query(query): Query<ConnectorSyncPlanQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let connector = required_connector(query.connector)?;
    let root_path = required_connector_root_path(query.root_path)?;
    let scope_id = query
        .scope_id
        .map(ScopeId::from_string)
        .unwrap_or_else(|| state.default_scope_id.clone());
    let mut request = ConnectorSyncPlanRequest::new(connector, PathBuf::from(root_path), scope_id);
    if let Some(max_items) = query.max_items {
        request.max_items = max_items;
    }
    request.allow_remote_fetch = query.allow_remote_fetch.unwrap_or(false);
    let output = build_connector_sync_plan(request).map_err(api_error_from_anyhow)?;

    Ok(Json(connector_sync_plan_json(&output.report)))
}

async fn compat_connector_import_draft(
    State(state): State<HttpAppState>,
    Query(query): Query<ConnectorImportDraftQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let connector = required_connector(query.connector)?;
    let root_path = required_connector_root_path(query.root_path)?;
    let scope_id = query
        .scope_id
        .map(ScopeId::from_string)
        .unwrap_or_else(|| state.default_scope_id.clone());
    let mut request =
        ConnectorImportDraftRequest::new(connector, PathBuf::from(root_path), scope_id);
    if let Some(max_items) = query.max_items {
        request.max_items = max_items;
    }
    request.proposal_mode = query.proposal.unwrap_or(false);
    let report = build_connector_import_draft_report(request).map_err(api_error_from_anyhow)?;

    Ok(Json(connector_import_draft_json(&report)))
}

async fn compat_connector_proposal_queue(
    State(state): State<HttpAppState>,
    Query(query): Query<ConnectorProposalQueueQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let connector = required_connector(query.connector)?;
    let root_path = required_connector_root_path(query.root_path)?;
    let scope_id = query
        .scope_id
        .map(ScopeId::from_string)
        .unwrap_or_else(|| state.default_scope_id.clone());
    let mut request =
        ConnectorProposalQueueRequest::new(connector, PathBuf::from(root_path), scope_id);
    if let Some(max_items) = query.max_items {
        request.max_items = max_items;
    }
    let report = build_connector_proposal_queue_report(request).map_err(api_error_from_anyhow)?;
    let store_path = if query.persist.unwrap_or(false) {
        Some(connector_proposal_store_required(&state)?.save(&report)?)
    } else {
        None
    };

    Ok(Json(connector_proposal_queue_http_json(
        &report,
        store_path,
        state.connector_proposal_store.is_some(),
    )))
}

async fn compat_connector_proposal_queue_get(
    State(state): State<HttpAppState>,
    Path(queue_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let report = connector_proposal_store_required(&state)?.load(&queue_id)?;

    Ok(Json(connector_proposal_queue_http_json(
        &report, None, true,
    )))
}

async fn compat_connector_proposal_apply_plan(
    State(state): State<HttpAppState>,
    Query(query): Query<ConnectorProposalApplyPlanQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let approved_queue_item_ids =
        required_csv_values(query.approved_queue_item_ids, "approved_queue_item_ids")?;
    let confirmation_token = query
        .confirmation_token
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request("confirmation_token is required"))?;
    let stored_queue_id = query.queue_id.filter(|value| !value.trim().is_empty());
    let report = if let Some(queue_id) = stored_queue_id.as_deref() {
        let queue = connector_proposal_store_required(&state)?.load(queue_id)?;
        if let Some(connector) = query.connector.filter(|value| !value.trim().is_empty()) {
            if connector != queue.connector {
                return Err(ApiError::bad_request(format!(
                    "queue_id {queue_id} belongs to connector {}, not {connector}",
                    queue.connector
                )));
            }
        }
        build_connector_proposal_apply_plan_report_from_queue(
            queue,
            approved_queue_item_ids,
            confirmation_token,
        )
        .map_err(api_error_from_anyhow)?
    } else {
        let connector = required_connector(query.connector)?;
        let root_path = required_connector_root_path(query.root_path)?;
        let scope_id = query
            .scope_id
            .map(ScopeId::from_string)
            .unwrap_or_else(|| state.default_scope_id.clone());
        let mut request = ConnectorProposalApplyPlanRequest::new(
            connector,
            PathBuf::from(root_path),
            scope_id,
            approved_queue_item_ids,
            confirmation_token,
        );
        if let Some(max_items) = query.max_items {
            request.max_items = max_items;
        }
        build_connector_proposal_apply_plan_report(request).map_err(api_error_from_anyhow)?
    };

    let payload = if let Some(queue_id) = stored_queue_id.as_deref() {
        attach_connector_proposal_store_metadata(
            connector_proposal_apply_plan_json(&report),
            queue_id,
        )
    } else {
        connector_proposal_apply_plan_json(&report)
    };

    Ok(Json(payload))
}

async fn compat_connector_proposal_apply_plan_apply(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Json(payload): Json<ConnectorProposalApplyPlanApplyRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    if payload.approved_queue_item_ids.is_empty() {
        return Err(ApiError::bad_request("approved_queue_item_ids is required"));
    }
    let confirmation_token = if payload.confirmation_token.trim().is_empty() {
        return Err(ApiError::bad_request("confirmation_token is required"));
    } else {
        payload.confirmation_token
    };
    let max_items = payload.max_items.unwrap_or(100);
    let stored_queue_id = payload.queue_id.filter(|value| !value.trim().is_empty());
    let (report, scope_id) = if let Some(queue_id) = stored_queue_id.as_deref() {
        let queue = connector_proposal_store_required(&state)?.load(queue_id)?;
        if let Some(connector) = payload.connector.filter(|value| !value.trim().is_empty()) {
            if connector != queue.connector {
                return Err(ApiError::bad_request(format!(
                    "queue_id {queue_id} belongs to connector {}, not {connector}",
                    queue.connector
                )));
            }
        }
        let scope_id = if let Some(scope_id) = payload.scope_id {
            ScopeId::from_string(scope_id)
        } else {
            connector_queue_selected_scope(&queue, &payload.approved_queue_item_ids)?
        };
        ensure_context_scope_access(&context, &scope_id)?;
        let report = build_connector_proposal_apply_plan_report_from_queue(
            queue,
            payload.approved_queue_item_ids,
            confirmation_token,
        )
        .map_err(api_error_from_anyhow)?;
        (report, scope_id)
    } else {
        let connector = required_connector(payload.connector)?;
        let root_path = required_connector_root_path(payload.root_path)?;
        let scope_id = payload
            .scope_id
            .map(ScopeId::from_string)
            .unwrap_or_else(|| context.owner_scope_id.clone());
        ensure_context_scope_access(&context, &scope_id)?;
        let mut plan_request = ConnectorProposalApplyPlanRequest::new(
            connector,
            PathBuf::from(root_path),
            scope_id.clone(),
            payload.approved_queue_item_ids,
            confirmation_token,
        );
        plan_request.max_items = max_items;
        let report = build_connector_proposal_apply_plan_report(plan_request)
            .map_err(api_error_from_anyhow)?;
        (report, scope_id)
    };
    let mut apply_request =
        ConnectorProposalApplyExecutorRequest::new(report.clone(), scope_id, context);
    apply_request.source_id = payload.source_id.map(SourceId::from_string);
    apply_request.max_items = max_items;
    let execution = apply_connector_proposal_apply_plan(&state.kernel, apply_request)
        .await
        .map_err(api_error_from_anyhow)?;

    let payload = connector_proposal_apply_plan_execution_json(&report, &execution);
    let payload = if let Some(queue_id) = stored_queue_id.as_deref() {
        attach_connector_proposal_store_metadata(payload, queue_id)
    } else {
        payload
    };

    Ok(Json(payload))
}

fn required_connector(raw: Option<String>) -> Result<String, ApiError> {
    raw.filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request("connector is required"))
}

fn required_connector_root_path(raw: Option<String>) -> Result<String, ApiError> {
    raw.filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request("root_path is required"))
}

fn required_csv_values(raw: Option<String>, field: &'static str) -> Result<Vec<String>, ApiError> {
    let values = raw
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request(format!("{field} is required")))?
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if values.is_empty() {
        return Err(ApiError::bad_request(format!("{field} is required")));
    }
    Ok(values)
}

fn connector_queue_selected_scope(
    queue: &ConnectorProposalQueueReport,
    approved_queue_item_ids: &[String],
) -> Result<ScopeId, ApiError> {
    let mut selected_scope_id: Option<ScopeId> = None;
    for queue_item_id in approved_queue_item_ids {
        let item = queue
            .queue_items
            .iter()
            .find(|item| item.queue_item_id == *queue_item_id)
            .ok_or_else(|| {
                ApiError::bad_request(format!(
                    "unknown connector proposal queue item in store: {queue_item_id}"
                ))
            })?;
        match &selected_scope_id {
            Some(scope_id) if *scope_id != item.scope_id => {
                return Err(ApiError::bad_request(
                    "approved connector queue items span multiple scopes",
                ));
            }
            Some(_) => {}
            None => selected_scope_id = Some(item.scope_id.clone()),
        }
    }
    selected_scope_id.ok_or_else(|| ApiError::bad_request("approved_queue_item_ids is required"))
}

fn report_input_dir(input_dir: Option<String>, default_dir: &str) -> PathBuf {
    input_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| default_dir.into())
}

fn read_report_text(path: &PathBuf) -> Result<String, ApiError> {
    fs::read_to_string(path).map_err(|error| {
        ApiError::bad_request(format!("failed to read {}: {error}", path.display()))
    })
}

fn read_report_json(path: &PathBuf) -> Result<serde_json::Value, ApiError> {
    let raw = read_report_text(path)?;
    serde_json::from_str(&raw).map_err(|error| {
        ApiError::bad_request(format!("failed to parse {}: {error}", path.display()))
    })
}

fn benchmark_run_output_json(output: &BenchmarkRunOutput) -> serde_json::Value {
    json!({
        "suite": output.suite,
        "run": output.run,
        "metrics": output.run.metrics,
        "cases": output.cases,
        "report_paths": {
            "summary": output.report_paths.summary.display().to_string(),
            "metrics": output.report_paths.metrics.display().to_string(),
            "failures": output.report_paths.failures.display().to_string(),
            "latency": output.report_paths.latency.display().to_string(),
            "leakage": output.report_paths.leakage.display().to_string(),
        }
    })
}

fn benchmark_run_summary_from_dir(input_dir: &FsPath) -> Result<serde_json::Value, ApiError> {
    let metrics_path = input_dir.join("metrics.json");
    let metrics = read_report_json(&metrics_path)?;
    Ok(json!({
        "name": input_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("."),
        "input_dir": input_dir.display().to_string(),
        "summary_exists": input_dir.join("summary.md").exists(),
        "failure_detail_exists": input_dir.join("failures.jsonl").exists(),
        "run": metrics.get("run").cloned().unwrap_or_else(|| json!({})),
        "metrics": metrics.get("metrics").cloned().unwrap_or_else(|| metrics.clone()),
    }))
}

fn trace_result_json(
    result: &TraceSearchContextResult,
    paths: &RecallTraceReportPaths,
) -> serde_json::Value {
    json!({
        "trace": result.trace,
        "budget_pack": result.budget_pack,
        "explanation": result.explanation,
        "report_paths": {
            "trace": paths.trace.display().to_string(),
            "explanation": paths.explanation.display().to_string(),
        }
    })
}

fn passport_export_json(
    bundle: &MemoryPassportBundle,
    paths: &MemoryPassportPaths,
) -> serde_json::Value {
    let mut value = bundle_json(bundle);
    if let Some(object) = value.as_object_mut() {
        object.insert("report_paths".to_string(), passport_paths_json(paths));
    }
    value
}

fn passport_import_json(result: &MemoryPassportImportResult) -> serde_json::Value {
    json!({
        "manifest": result.manifest,
        "verified": result.verified,
        "target_scope_id": result.target_scope_id.as_str(),
        "imported_count": result.imported_count,
        "skipped_count": result.skipped_count,
        "id_mappings": result.id_mappings.iter().map(|mapping| json!({
            "original_memory_id": mapping.original_memory_id.as_str(),
            "imported_memory_id": mapping.imported_memory_id.as_str(),
        })).collect::<Vec<_>>(),
    })
}

fn passport_paths_json(paths: &MemoryPassportPaths) -> serde_json::Value {
    json!({
        "passport": paths.passport.display().to_string(),
        "manifest": paths.manifest.display().to_string(),
        "memories": paths.memories.display().to_string(),
        "evidence": paths.evidence.display().to_string(),
        "markdown": paths.markdown.display().to_string(),
    })
}

async fn search_context(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Json(payload): Json<SearchContextHttpRequest>,
) -> Result<Json<SearchContextHttpResponse>, ApiError> {
    let context = resolve_http_context(&state, &headers).await?;
    validate_text_field_len("query", &payload.query, MAX_QUERY_CHARS)?;
    let scope_id = payload
        .scope_id
        .map(ScopeId::from_string)
        .unwrap_or_else(|| state.default_scope_id.clone());
    let mut request = SearchContextRequest::new(scope_id.clone(), payload.query);
    if let Some(limit) = payload.limit {
        request.limit = limit;
    }
    request.context = context;

    let bundle = state
        .kernel
        .search_context(request)
        .await
        .map_err(api_error_from_anyhow)?;
    let memories = bundle
        .memories
        .iter()
        .map(memory_to_summary)
        .collect::<Vec<_>>();

    Ok(Json(SearchContextHttpResponse {
        query: bundle.query,
        scope_id: bundle.scope_id.as_str().to_string(),
        memory_count: memories.len(),
        memories,
    }))
}

async fn resolve_http_context(
    state: &HttpAppState,
    headers: &HeaderMap,
) -> Result<Option<RequestContext>, ApiError> {
    let raw_key = headers
        .get("x-meat-memory-key")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
                .map(str::to_string)
        });

    let Some(raw_key) = raw_key else {
        if state.require_key {
            return Err(ApiError {
                status: StatusCode::UNAUTHORIZED,
                message: "meat memory key is required".to_string(),
            });
        }
        return Ok(None);
    };

    let context = state
        .kernel
        .resolve_access_key_context(&raw_key)
        .await
        .map_err(api_error_from_anyhow)?;
    if context.is_none() && state.require_key {
        return Err(ApiError {
            status: StatusCode::UNAUTHORIZED,
            message: "invalid meat memory key".to_string(),
        });
    }
    Ok(context)
}

async fn resolve_required_http_context(
    state: &HttpAppState,
    headers: &HeaderMap,
) -> Result<RequestContext, ApiError> {
    resolve_http_context(state, headers)
        .await?
        .ok_or_else(|| ApiError {
            status: StatusCode::UNAUTHORIZED,
            message: "meat memory key is required".to_string(),
        })
}

async fn get_memory_source_for_context(
    state: &HttpAppState,
    context: &RequestContext,
    source_id: SourceId,
) -> Result<MemorySource, ApiError> {
    let source = state
        .kernel
        .get_memory_source(source_id)
        .await
        .map_err(api_error_from_anyhow)?
        .ok_or_else(|| ApiError::not_found("memory source not found"))?;
    ensure_context_scope_access(context, &source.owner_scope_id)?;
    Ok(source)
}

fn ensure_context_scope_access(
    context: &RequestContext,
    scope_id: &ScopeId,
) -> Result<(), ApiError> {
    if context.owner_scope_id != *scope_id {
        return Err(ApiError {
            status: StatusCode::FORBIDDEN,
            message: "scope access forbidden for current meat memory key".to_string(),
        });
    }
    Ok(())
}

fn validate_text_field_len(field: &str, value: &str, max_chars: usize) -> Result<(), ApiError> {
    let len = value.chars().count();
    if len > max_chars {
        return Err(ApiError::bad_request(format!(
            "{field} exceeds {max_chars} characters"
        )));
    }
    Ok(())
}

async fn browse_memories(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Query(query): Query<BrowseMemoriesQuery>,
) -> Result<Json<ExplorerMemoriesResponse>, ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    let scope_id = match query.scope_id.as_deref() {
        Some(raw_scope_id) => {
            let scope_id = ScopeId::from_string(raw_scope_id);
            ensure_context_scope_access(&context, &scope_id)?;
            Some(scope_id)
        }
        None => Some(context.owner_scope_id.clone()),
    };
    let limit = query.limit.unwrap_or(240).clamp(1, 500);
    let memories = state
        .kernel
        .browse_memories(scope_id.clone(), limit)
        .await
        .map_err(api_error_from_anyhow)?;
    let items = memories
        .iter()
        .map(memory_to_explorer_item)
        .collect::<Vec<_>>();

    Ok(Json(ExplorerMemoriesResponse {
        total_count: items.len(),
        filtered_scope: scope_id.map(|scope| scope.as_str().to_string()),
        ownership_groups: facet_counts(&items, |item| {
            (item.ownership_key.clone(), item.ownership_label.clone())
        }),
        agent_groups: facet_counts(&items, |item| {
            (item.agent_key.clone(), item.agent_label.clone())
        }),
        scope_groups: facet_counts(&items, |item| {
            (item.scope_id.clone(), item.scope_id.clone())
        }),
        memories: items,
    }))
}

async fn chat_with_memory_assistant(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Json(payload): Json<AssistantChatRequest>,
) -> Result<Json<AssistantChatResponse>, ApiError> {
    let context = resolve_required_http_context(&state, &headers).await?;
    validate_text_field_len("prompt", &payload.prompt, MAX_PROMPT_CHARS)?;
    let requested_scope = match payload.scope_id.as_deref() {
        Some(raw_scope_id) => {
            let scope_id = ScopeId::from_string(raw_scope_id);
            ensure_context_scope_access(&context, &scope_id)?;
            Some(scope_id)
        }
        None => Some(context.owner_scope_id.clone()),
    };
    let limit = payload.limit.unwrap_or(6).clamp(1, 12);
    let memories = state
        .kernel
        .browse_memories(requested_scope, 300)
        .await
        .map_err(api_error_from_anyhow)?;
    let items = memories
        .iter()
        .map(memory_to_explorer_item)
        .collect::<Vec<_>>();
    let prompt = payload.prompt.trim();
    if prompt.is_empty() {
        return Err(ApiError::bad_request("assistant prompt cannot be empty"));
    }

    let filtered = filter_explorer_items(
        &items,
        payload.ownership.as_deref(),
        payload.agent.as_deref(),
        payload.memory_id.as_deref(),
        Some(prompt),
    );
    let citations = filtered
        .iter()
        .take(limit)
        .map(|item| AssistantCitation {
            memory_id: item.memory_id.clone(),
            title: item.title.clone(),
            scope_id: item.scope_id.clone(),
            ownership_label: item.ownership_label.clone(),
            agent_label: item.agent_label.clone(),
        })
        .collect::<Vec<_>>();

    Ok(Json(AssistantChatResponse {
        answer: compose_assistant_answer(prompt, &filtered, payload.memory_id.as_deref()),
        matched_count: filtered.len(),
        citations,
    }))
}

fn memory_to_summary(memory: &Memory) -> MemorySummary {
    MemorySummary {
        memory_id: memory.id.as_str().to_string(),
        title: memory.title.clone(),
        body: memory.body.clone(),
        memory_kind: memory_kind_label(memory.kind).to_string(),
        memory_state: memory.state.as_str().to_string(),
        evidence_count: memory.evidence_count,
    }
}

fn lifecycle_record_json(record: &MemoryRecord) -> serde_json::Value {
    json!({
        "record_id": record.record_id.as_str(),
        "native_id": record.native_id.as_str(),
        "native_kind": record.native_kind.as_str(),
        "layer": record.layer.as_str(),
        "scope_id": record.scope_id.as_str(),
        "owner_scope_id": record.owner_scope_id.as_ref().map(|scope| scope.as_str()),
        "record_type": record.record_type.as_str(),
        "title": record.title.as_str(),
        "summary": record.summary.as_deref(),
        "source_kind": record.source_kind.as_str(),
        "source_ref": record.source_ref.as_deref(),
        "confidence": record.confidence.as_str(),
        "status": record.status.as_str(),
        "visibility": visibility_label(record.visibility),
        "sensitivity": sensitivity_label(record.sensitivity),
        "importance": record.importance,
        "freshness": record.freshness,
        "stability": record.stability,
        "created_at": format_timestamp(record.created_at),
        "updated_at": format_timestamp(record.updated_at),
    })
}

fn access_key_response(
    access_key: memory_domain::AccessKey,
    raw_key: Option<String>,
) -> AccessKeyResponse {
    AccessKeyResponse {
        key_id: access_key.id.as_str().to_string(),
        raw_key,
        name: access_key.display_name,
        source_id: access_key
            .source_id
            .map(|source_id| source_id.as_str().to_string()),
        source: access_key.source_kind.as_str().to_string(),
        owner_principal_id: access_key.owner_principal_id,
        owner_scope_id: access_key.owner_scope_id.as_str().to_string(),
        scope_kind: access_key.scope_kind.as_str().to_string(),
        storage_mode: access_key.storage_mode.as_str().to_string(),
        isolated: access_key.is_fully_isolated,
        isolation_group_id: access_key.isolation_group_id,
        status: access_key.status.as_str().to_string(),
        created_at: format_timestamp(access_key.created_at),
        last_used_at: access_key.last_used_at.map(format_timestamp),
    }
}

fn memory_source_response(source: MemorySource) -> MemorySourceResponse {
    MemorySourceResponse {
        source_id: source.id.as_str().to_string(),
        source_kind: source.source_kind,
        name: source.display_name,
        owner_principal_id: source.owner_principal_id,
        owner_scope_id: source.owner_scope_id.as_str().to_string(),
        source_uri: source.source_uri,
        sync_mode: source.sync_mode.as_str().to_string(),
        local_root: source.local_root,
        status: source.status.as_str().to_string(),
        created_at: format_timestamp(source.created_at),
        updated_at: format_timestamp(source.updated_at),
    }
}

fn project_document_response(document: ProjectDocument) -> ProjectDocumentResponse {
    ProjectDocumentResponse {
        document_id: document.id.as_str().to_string(),
        source_id: document.source_id.as_str().to_string(),
        scope_id: document.scope_id.as_str().to_string(),
        local_path: document.local_path,
        canonical_uri: document.canonical_uri,
        title: document.title,
        content_hash: document.content_hash,
        last_seen_mtime: document.last_seen_mtime.map(format_timestamp),
        sync_state: document.sync_state.as_str().to_string(),
        conflict_state: document.conflict_state.as_str().to_string(),
        artifact_id: document
            .artifact_id
            .map(|artifact_id| artifact_id.as_str().to_string()),
        memory_id: document
            .memory_id
            .map(|memory_id| memory_id.as_str().to_string()),
        metadata: document.metadata,
        created_at: format_timestamp(document.created_at),
        updated_at: format_timestamp(document.updated_at),
    }
}

fn project_document_draft_response(
    document: &LocalProjectDocumentDraft,
) -> ProjectDocumentDraftResponse {
    ProjectDocumentDraftResponse {
        canonical_uri: document.canonical_uri.clone(),
        local_path: document.local_path.to_string_lossy().to_string(),
        title: document.title.clone(),
        content_hash: document.content_hash.clone(),
        sync_state: document.sync_state.as_str().to_string(),
        metadata: document.metadata.clone(),
    }
}

fn missing_project_document_response(
    document: MissingProjectDocument,
) -> MissingProjectDocumentResponse {
    MissingProjectDocumentResponse {
        canonical_uri: document.canonical_uri,
        sync_state: document.sync_state.as_str().to_string(),
    }
}

fn project_document_conflict_response(
    conflict: ProjectDocumentConflictReport,
) -> ProjectDocumentConflictResponse {
    ProjectDocumentConflictResponse {
        canonical_uri: conflict.canonical_uri,
        sync_state: conflict.sync_state.as_str().to_string(),
        conflict_state: conflict.conflict_state.as_str().to_string(),
        reason: conflict.reason,
    }
}

fn agent_context_response(agent_context: AgentContext) -> AgentContextResponse {
    AgentContextResponse {
        context_id: agent_context.id.as_str().to_string(),
        source_id: agent_context
            .source_id
            .map(|source_id| source_id.as_str().to_string()),
        key_id: agent_context
            .key_id
            .map(|key_id| key_id.as_str().to_string()),
        scope_id: agent_context.scope_id.as_str().to_string(),
        session_id: agent_context.session_id,
        task_id: agent_context.task_id,
        layer: agent_context.layer.as_str().to_string(),
        title: agent_context.title,
        body: agent_context.body,
        labels: agent_context.labels,
        expires_at: agent_context.expires_at.map(format_timestamp),
        created_at: format_timestamp(agent_context.created_at),
        updated_at: format_timestamp(agent_context.updated_at),
    }
}

fn remember_text_response(result: RememberTextResult) -> CreateMemoryResponse {
    CreateMemoryResponse {
        artifact_id: result.artifact.id.as_str().to_string(),
        memory_id: result.memory.id.as_str().to_string(),
        scope_id: result.memory.scope_id.as_str().to_string(),
        owner_scope_id: result.memory.owner_scope_id.as_str().to_string(),
        title: result.memory.title.clone(),
        body: result.memory.body.clone(),
        language_code: result.memory.language_code.clone(),
        memory_kind: memory_kind_label(result.memory.kind).to_string(),
        memory_state: result.memory.state.as_str().to_string(),
        evidence_count: result.memory.evidence_count,
        wrote_pg: result.wrote_pg,
        wrote_markdown: result.wrote_markdown,
    }
}

fn memory_to_explorer_item(memory: &Memory) -> ExplorerMemoryItem {
    let (ownership_key, ownership_label) = derive_ownership(memory);
    let (agent_key, agent_label) = detect_agent(memory);
    ExplorerMemoryItem {
        memory_id: memory.id.as_str().to_string(),
        scope_id: memory.scope_id.as_str().to_string(),
        title: memory.title.clone(),
        body: memory.body.clone(),
        preview: preview_text(&memory.body, 140),
        memory_kind: memory_kind_label(memory.kind).to_string(),
        memory_state: memory.state.as_str().to_string(),
        visibility: visibility_label(memory.visibility).to_string(),
        sensitivity: sensitivity_label(memory.sensitivity).to_string(),
        evidence_count: memory.evidence_count,
        created_at: format_timestamp(memory.created_at),
        updated_at: format_timestamp(memory.updated_at),
        ownership_key: ownership_key.to_string(),
        ownership_label: ownership_label.to_string(),
        agent_key: agent_key.to_string(),
        agent_label: agent_label.to_string(),
        source_label: format!(
            "{} / {} / {}",
            ownership_label,
            agent_label,
            memory.scope_id.as_str()
        ),
    }
}

fn facet_counts<F>(items: &[ExplorerMemoryItem], key_fn: F) -> Vec<ExplorerFacet>
where
    F: Fn(&ExplorerMemoryItem) -> (String, String),
{
    let mut counts = BTreeMap::<String, ExplorerFacet>::new();
    for item in items {
        let (key, label) = key_fn(item);
        counts
            .entry(key.clone())
            .and_modify(|facet| facet.count += 1)
            .or_insert(ExplorerFacet {
                key,
                label,
                count: 1,
            });
    }
    counts.into_values().collect()
}

fn filter_explorer_items<'a>(
    items: &'a [ExplorerMemoryItem],
    ownership: Option<&str>,
    agent: Option<&str>,
    memory_id: Option<&str>,
    prompt: Option<&str>,
) -> Vec<&'a ExplorerMemoryItem> {
    let prompt_terms = search_terms(prompt.unwrap_or_default());
    let mut scored = items
        .iter()
        .filter(|item| {
            ownership.is_none_or(|value| value == item.ownership_key)
                && agent.is_none_or(|value| value == item.agent_key)
                && memory_id.is_none_or(|value| value == item.memory_id)
        })
        .map(|item| {
            let haystack = format!(
                "{}\n{}\n{}\n{}\n{}",
                item.title, item.body, item.scope_id, item.agent_label, item.ownership_label
            )
            .to_lowercase();
            let score = if prompt_terms.is_empty() {
                0
            } else {
                prompt_terms
                    .iter()
                    .filter(|term| haystack.contains(term.as_str()))
                    .count()
            };
            (score, item)
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.updated_at.cmp(&left.1.updated_at))
    });

    let mut filtered = scored
        .into_iter()
        .filter(|(score, _)| prompt_terms.is_empty() || *score > 0)
        .map(|(_, item)| item)
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        filtered = items
            .iter()
            .filter(|item| {
                ownership.is_none_or(|value| value == item.ownership_key)
                    && agent.is_none_or(|value| value == item.agent_key)
            })
            .collect::<Vec<_>>();
        filtered.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    }

    filtered
}

fn compose_assistant_answer(
    prompt: &str,
    items: &[&ExplorerMemoryItem],
    selected_memory_id: Option<&str>,
) -> String {
    if items.is_empty() {
        return "当前没有可参考的 memory。你可以先放宽左侧筛选，或者在中间区域挑选一条记忆后再继续问。".to_string();
    }

    let mut lines = Vec::new();
    if let Some(memory_id) = selected_memory_id {
        lines.push(format!(
            "我结合你当前选中的记忆 `{memory_id}` 和工作台里的相关结果，整理出这些要点："
        ));
    } else {
        lines.push(format!(
            "我围绕“{prompt}”从当前 Memory 工作台里整理出这些信息："
        ));
    }

    for (index, item) in items.iter().take(4).enumerate() {
        lines.push(format!(
            "{}. {}（{} / {} / {}）: {}",
            index + 1,
            item.title,
            item.scope_id,
            item.ownership_label,
            item.agent_label,
            preview_text(&item.body, 110)
        ));
    }

    lines.push(
        "如果你愿意，我可以继续按团队/个人、某个 Agent，或者某个 scope 继续缩小范围。".to_string(),
    );
    lines.join("\n")
}

fn derive_ownership(memory: &Memory) -> (&'static str, &'static str) {
    let scope_key = memory.scope_id.as_str().to_lowercase();
    if matches!(
        memory.visibility,
        Visibility::Team | Visibility::Organization
    ) || ["team", "org", "workspace", "project", "shared"]
        .iter()
        .any(|needle| scope_key.contains(needle))
    {
        ("team", "团队")
    } else {
        ("personal", "个人")
    }
}

fn detect_agent(memory: &Memory) -> (&'static str, &'static str) {
    let haystack = format!(
        "{}\n{}\n{}",
        memory.scope_id.as_str(),
        memory.title,
        memory.body
    )
    .to_lowercase();
    let patterns = [
        ("codex", "Codex", &["codex"][..]),
        (
            "claude-code",
            "Claude Code",
            &["claude code", "claude-code", "claude"][..],
        ),
        ("trae", "TRAE", &["trae"][..]),
        ("qoder", "Qoder", &["qoder", "qoderwork"][..]),
        ("openclaw", "OpenClaw", &["openclaw"][..]),
        ("cowork", "CoWork", &["cowork"][..]),
    ];

    for (key, label, needles) in patterns {
        if needles.iter().any(|needle| haystack.contains(needle)) {
            return (key, label);
        }
    }

    ("unknown", "未标注 Agent")
}

fn preview_text(raw: &str, max_chars: usize) -> String {
    let normalized = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let char_count = normalized.chars().count();
    if char_count <= max_chars {
        return normalized;
    }

    let preview = normalized.chars().take(max_chars).collect::<String>();
    format!("{preview}…")
}

fn search_terms(raw: &str) -> Vec<String> {
    let mut terms = raw
        .split_whitespace()
        .map(|term| term.trim().to_lowercase())
        .filter(|term| !term.is_empty())
        .collect::<Vec<_>>();
    if terms.is_empty() {
        let fallback = raw.trim().to_lowercase();
        if !fallback.is_empty() {
            terms.push(fallback);
        }
    }
    terms.sort();
    terms.dedup();
    terms
}

fn format_timestamp(value: OffsetDateTime) -> String {
    value
        .format(&Rfc3339)
        .unwrap_or_else(|_| value.unix_timestamp().to_string())
}

fn parse_optional_timestamp(raw: Option<&str>) -> Result<Option<OffsetDateTime>, ApiError> {
    raw.map(|value| {
        OffsetDateTime::parse(value, &Rfc3339)
            .map_err(|_| ApiError::bad_request(format!("invalid timestamp: {value}")))
    })
    .transpose()
}

fn visibility_label(visibility: Visibility) -> &'static str {
    match visibility {
        Visibility::Private => "private",
        Visibility::Project => "project",
        Visibility::Team => "team",
        Visibility::Organization => "organization",
    }
}

fn sensitivity_label(sensitivity: Sensitivity) -> &'static str {
    match sensitivity {
        Sensitivity::Public => "public",
        Sensitivity::Internal => "internal",
        Sensitivity::Private => "private",
        Sensitivity::Restricted => "restricted",
    }
}

fn parse_artifact_kind(raw: Option<&str>) -> Result<ArtifactKind, ApiError> {
    match raw.unwrap_or("message") {
        "message" => Ok(ArtifactKind::Message),
        "document" => Ok(ArtifactKind::Document),
        "code_diff" => Ok(ArtifactKind::CodeDiff),
        "code_file_snapshot" => Ok(ArtifactKind::CodeFileSnapshot),
        "terminal_output" => Ok(ArtifactKind::TerminalOutput),
        "image" => Ok(ArtifactKind::Image),
        "audio" => Ok(ArtifactKind::Audio),
        "video" => Ok(ArtifactKind::Video),
        "tool_result" => Ok(ArtifactKind::ToolResult),
        "web_page" => Ok(ArtifactKind::WebPage),
        other => Err(ApiError::bad_request(format!(
            "unsupported artifact_kind: {other}"
        ))),
    }
}

fn parse_memory_kind(raw: &str) -> Result<MemoryKind, ApiError> {
    match raw {
        "fact" => Ok(MemoryKind::Fact),
        "preference" => Ok(MemoryKind::Preference),
        "decision" => Ok(MemoryKind::Decision),
        "procedure" => Ok(MemoryKind::Procedure),
        "constraint" => Ok(MemoryKind::Constraint),
        "risk" => Ok(MemoryKind::Risk),
        "summary" => Ok(MemoryKind::Summary),
        "insight" => Ok(MemoryKind::Insight),
        other => Err(ApiError::bad_request(format!(
            "unsupported memory_kind: {other}"
        ))),
    }
}

fn parse_visibility(raw: Option<&str>) -> Result<Visibility, ApiError> {
    match raw.unwrap_or("private") {
        "private" => Ok(Visibility::Private),
        "project" => Ok(Visibility::Project),
        "team" => Ok(Visibility::Team),
        "organization" => Ok(Visibility::Organization),
        other => Err(ApiError::bad_request(format!(
            "unsupported visibility: {other}"
        ))),
    }
}

fn parse_scope_type(raw: &str) -> Result<ScopeType, ApiError> {
    match raw {
        "org" => Ok(ScopeType::Org),
        "team" => Ok(ScopeType::Team),
        "workspace" => Ok(ScopeType::Workspace),
        "project" => Ok(ScopeType::Project),
        "user" => Ok(ScopeType::User),
        "session" => Ok(ScopeType::Session),
        other => Err(ApiError::bad_request(format!(
            "unsupported scope_type: {other}"
        ))),
    }
}

fn parse_sensitivity(raw: Option<&str>) -> Result<Sensitivity, ApiError> {
    match raw.unwrap_or("internal") {
        "public" => Ok(Sensitivity::Public),
        "internal" => Ok(Sensitivity::Internal),
        "private" => Ok(Sensitivity::Private),
        "restricted" => Ok(Sensitivity::Restricted),
        other => Err(ApiError::bad_request(format!(
            "unsupported sensitivity: {other}"
        ))),
    }
}

fn parse_record_status(raw: &str) -> Result<MemoryRecordStatus, ApiError> {
    match raw {
        "candidate" => Ok(MemoryRecordStatus::Candidate),
        "active" => Ok(MemoryRecordStatus::Active),
        "needs_review" | "conflicted" => Ok(MemoryRecordStatus::NeedsReview),
        "archived" => Ok(MemoryRecordStatus::Archived),
        "deprecated" => Ok(MemoryRecordStatus::Deprecated),
        "forgotten" => Ok(MemoryRecordStatus::Forgotten),
        "deleted" => Ok(MemoryRecordStatus::Deleted),
        other => Err(ApiError::bad_request(format!(
            "unsupported lifecycle status: {other}"
        ))),
    }
}

fn parse_key_source(raw: &str) -> Result<KeySourceKind, ApiError> {
    KeySourceKind::parse(raw)
        .map_err(|_| ApiError::bad_request(format!("unsupported source: {raw}")))
}

fn parse_key_scope(raw: &str) -> Result<KeyScopeKind, ApiError> {
    KeyScopeKind::parse(raw)
        .map_err(|_| ApiError::bad_request(format!("unsupported scope_kind: {raw}")))
}

fn parse_storage_mode(raw: &str) -> Result<StorageMode, ApiError> {
    StorageMode::parse(raw)
        .map_err(|_| ApiError::bad_request(format!("unsupported storage_mode: {raw}")))
}

fn parse_key_status(raw: &str) -> Result<AccessKeyStatus, ApiError> {
    AccessKeyStatus::parse(raw)
        .map_err(|_| ApiError::bad_request(format!("unsupported key status: {raw}")))
}

fn parse_source_sync_mode(raw: &str) -> Result<SourceSyncMode, ApiError> {
    SourceSyncMode::parse(raw)
        .map_err(|_| ApiError::bad_request(format!("unsupported source sync_mode: {raw}")))
}

fn parse_document_sync_state(raw: &str) -> Result<DocumentSyncState, ApiError> {
    DocumentSyncState::parse(raw)
        .map_err(|_| ApiError::bad_request(format!("unsupported document sync_state: {raw}")))
}

fn parse_document_conflict_state(raw: &str) -> Result<DocumentConflictState, ApiError> {
    DocumentConflictState::parse(raw)
        .map_err(|_| ApiError::bad_request(format!("unsupported document conflict_state: {raw}")))
}

fn api_error_from_domain(error: memory_domain::DomainError) -> ApiError {
    ApiError::bad_request(error.to_string())
}

fn api_error_from_anyhow(error: anyhow::Error) -> ApiError {
    let message = error.to_string();
    if message.contains("forbidden") {
        return ApiError {
            status: StatusCode::FORBIDDEN,
            message,
        };
    }
    if message.contains("denied by policy") {
        return ApiError {
            status: StatusCode::FORBIDDEN,
            message,
        };
    }
    if message == "meat memory key is required" || message.contains("invalid meat memory key") {
        return ApiError {
            status: StatusCode::UNAUTHORIZED,
            message,
        };
    }
    if message.contains("unsupported")
        || message.contains("unknown")
        || message.contains("empty")
        || message.contains("Invalid")
        || message.contains("exceeds")
        || message.contains("requires scope_id")
    {
        return ApiError::bad_request(message);
    }

    error!(error = %message, "internal http error");
    ApiError::internal("internal server error")
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

fn build_console_page(metadata: &ApiMetadata) -> String {
    let service = escape_html(&metadata.service);
    let version = escape_html(&metadata.version);
    let default_scope = escape_html(&metadata.default_scope);
    let metadata_json = serde_json::to_string(metadata)
        .unwrap_or_else(|_| "{}".to_string())
        .replace('<', "\\u003c");
    let template = r##"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>__SERVICE__ Memory Workspace</title>
  <style>
    :root {
      color-scheme: light;
      --shell-bg:
        radial-gradient(circle at 10% 0%, rgba(255, 226, 191, 0.95), transparent 32%),
        radial-gradient(circle at 100% 10%, rgba(199, 232, 255, 0.65), transparent 28%),
        linear-gradient(180deg, #f6f3ec 0%, #efe6d8 100%);
      --ink: #1f2937;
      --muted: #6b7280;
      --line: rgba(120, 53, 15, 0.16);
      --panel: rgba(255, 253, 249, 0.74);
      --shadow: 0 24px 56px rgba(15, 23, 42, 0.08);
    }
    * { box-sizing: border-box; }
    html, body { min-height: 100%; }
    body {
      margin: 0;
      color: var(--ink);
      background: var(--shell-bg);
      font-family: "Alibaba PuHuiTi", "PingFang SC", "Helvetica Neue", sans-serif;
    }
    .workspace-shell {
      width: 100%;
      padding: 20px 18px 34px;
    }
    .hero-tags {
      display: flex;
      flex-wrap: wrap;
      gap: 10px;
    }
    .hero-tag,
    .chip,
    .citation,
    .memory-meta-tag {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      padding: 6px 10px;
      border-radius: 999px;
      border: 1px solid rgba(120, 53, 15, 0.18);
      background: rgba(255, 248, 240, 0.82);
      font-size: 13px;
      color: rgba(31, 41, 55, 0.82);
    }
    .workspace-grid {
      display: flex;
      flex-wrap: wrap;
      gap: 18px;
      align-items: start;
    }
    .column {
      min-width: 0;
      display: grid;
      gap: 16px;
    }
    .left-rail {
      flex: 0 0 302px;
      max-width: 302px;
    }
    .center-pane {
      flex: 1 1 520px;
    }
    .right-rail {
      flex: 0 0 352px;
      max-width: 352px;
    }
    .section-title {
      font-weight: 700;
      letter-spacing: 0.02em;
    }
    .card {
      backdrop-filter: blur(12px);
      background: var(--panel);
      border: 1px solid var(--line);
      box-shadow: var(--shadow);
      border-radius: 22px;
      padding: 18px;
      min-width: 0;
      overflow: hidden;
    }
    .card-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      margin-bottom: 14px;
    }
    .card-title {
      margin: 0;
      font-size: 18px;
      font-weight: 700;
    }
    .muted {
      color: var(--muted);
    }
    .stats-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 10px;
      margin-bottom: 16px;
    }
    .stat-card {
      border-radius: 18px;
      border: 1px solid rgba(120, 53, 15, 0.14);
      background: rgba(255, 255, 255, 0.55);
      padding: 12px;
    }
    .stat-label {
      color: var(--muted);
      font-size: 12px;
      margin-bottom: 6px;
    }
    .stat-value {
      font-size: 24px;
      font-weight: 700;
    }
    .segmented {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
      margin-top: 10px;
    }
    .segmented button,
    .scope-button,
    .toolbar-button,
    .send-button {
      appearance: none;
      border: 1px solid rgba(120, 53, 15, 0.18);
      background: rgba(255, 255, 255, 0.78);
      color: var(--ink);
      border-radius: 14px;
      padding: 10px 12px;
      font: inherit;
      cursor: pointer;
      transition: transform 160ms ease, box-shadow 160ms ease, border-color 160ms ease;
    }
    .segmented button.active,
    .scope-button.active,
    .chip.active,
    .toolbar-button.primary,
    .send-button {
      background: linear-gradient(135deg, #9a3412 0%, #c2410c 100%);
      color: #fff;
      border-color: rgba(154, 52, 18, 0.35);
      box-shadow: 0 14px 28px rgba(154, 52, 18, 0.18);
    }
    .filter-group {
      display: grid;
      gap: 10px;
      margin-top: 16px;
    }
    .chip-row {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
    }
    .chip {
      cursor: pointer;
      user-select: none;
    }
    .scope-list {
      display: grid;
      gap: 8px;
    }
    .scope-button {
      width: 100%;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 10px;
      text-align: left;
      border: 1px solid rgba(120, 53, 15, 0.18);
      border-radius: 14px;
      background: rgba(255, 255, 255, 0.78);
      box-shadow: none;
      padding: 12px 14px;
    }
    .scope-button.active {
      background: linear-gradient(135deg, #9a3412 0%, #c2410c 100%);
    }
    .scope-summary {
      min-width: 0;
    }
    .scope-name {
      font-weight: 700;
      margin-bottom: 4px;
      overflow-wrap: anywhere;
    }
    .scope-count {
      font-size: 12px;
      color: var(--muted);
    }
    .scope-button.active .scope-count {
      color: rgba(255, 255, 255, 0.82);
    }
    .toolbar {
      display: flex;
      flex-wrap: wrap;
      gap: 12px;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 14px;
    }
    .toolbar-actions {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      align-items: center;
    }
    .search-input,
    .chat-input {
      width: 100%;
      border-radius: 14px;
      border: 1px solid rgba(120, 53, 15, 0.18);
      padding: 12px 14px;
      font: inherit;
      color: var(--ink);
      background: rgba(255, 255, 255, 0.85);
    }
    .memory-list {
      display: grid;
      gap: 12px;
      max-height: 52vh;
      overflow: auto;
      padding-right: 4px;
    }
    .memory-item {
      border-radius: 18px;
      border: 1px solid rgba(120, 53, 15, 0.16);
      background: rgba(255, 255, 255, 0.62);
      padding: 14px;
      cursor: pointer;
      transition: transform 160ms ease, box-shadow 160ms ease, border-color 160ms ease;
    }
    .memory-item:hover,
    .segmented button:hover,
    .scope-button:hover,
    .toolbar-button:hover,
    .send-button:hover,
    .chip:hover {
      transform: translateY(-1px);
      box-shadow: 0 14px 28px rgba(120, 53, 15, 0.12);
    }
    .memory-item.active {
      border-color: rgba(154, 52, 18, 0.45);
      box-shadow: 0 18px 42px rgba(154, 52, 18, 0.16);
    }
    .memory-item-top {
      display: flex;
      align-items: flex-start;
      justify-content: space-between;
      gap: 12px;
      margin-bottom: 10px;
    }
    .memory-item-title {
      font-weight: 700;
      margin-bottom: 4px;
    }
    .memory-preview,
    .memory-expanded-body,
    .chat-bubble {
      white-space: pre-wrap;
      line-height: 1.7;
      overflow-wrap: anywhere;
      word-break: break-word;
    }
    .memory-preview {
      color: rgba(31, 41, 55, 0.78);
      margin-top: 8px;
    }
    .memory-expanded {
      margin-top: 12px;
      border-top: 1px solid rgba(120, 53, 15, 0.12);
      padding-top: 14px;
    }
    .memory-expanded-title {
      margin: 0 0 6px;
      font-size: 22px;
      font-family: "Iowan Old Style", "Times New Roman", serif;
    }
    .memory-expanded-meta {
      margin-bottom: 10px;
    }
    .memory-expanded-body {
      color: rgba(31, 41, 55, 0.9);
    }
    .chat-note {
      margin-bottom: 12px;
    }
    .chat-stack {
      display: grid;
      gap: 14px;
      max-height: 58vh;
      overflow: auto;
      padding-right: 4px;
      margin-bottom: 12px;
    }
    .chat-row {
      display: grid;
      gap: 8px;
    }
    .chat-row.user {
      justify-items: end;
    }
    .chat-row.assistant {
      justify-items: start;
    }
    .chat-bubble {
      max-width: 100%;
      border-radius: 18px;
      padding: 12px 14px;
      border: 1px solid rgba(120, 53, 15, 0.16);
      background: rgba(255, 255, 255, 0.78);
    }
    .chat-row.user .chat-bubble {
      background: linear-gradient(135deg, #9a3412 0%, #c2410c 100%);
      color: #fff;
    }
    .citation-strip,
    .memory-meta,
    .memory-inline-tags {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
    }
    .chat-composer {
      display: grid;
      gap: 10px;
      padding-top: 10px;
      border-top: 1px solid rgba(120, 53, 15, 0.12);
    }
    .empty,
    .status-box {
      border-radius: 16px;
      border: 1px dashed rgba(120, 53, 15, 0.22);
      padding: 16px;
      color: var(--muted);
      background: rgba(255, 255, 255, 0.45);
    }
    .metric-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 10px;
      margin-bottom: 14px;
    }
    .metric-block {
      border-radius: 16px;
      border: 1px solid rgba(120, 53, 15, 0.14);
      background: rgba(255, 255, 255, 0.58);
      padding: 12px;
    }
    .metric-block .stat-value {
      font-size: 22px;
    }
    .metric-block-wide {
      grid-column: 1 / -1;
    }
    .metric-strip {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      margin-top: 10px;
    }
    .hidden {
      display: none !important;
    }
    .sr-only {
      position: absolute;
      width: 1px;
      height: 1px;
      padding: 0;
      margin: -1px;
      overflow: hidden;
      clip: rect(0, 0, 0, 0);
      white-space: nowrap;
      border: 0;
    }
    @media (max-width: 1380px) {
      .right-rail {
        flex: 1 1 100%;
        max-width: none;
      }
    }
    @media (max-width: 980px) {
      .workspace-shell {
        padding: 14px;
      }
      .left-rail,
      .center-pane,
      .right-rail {
        flex: 1 1 100%;
        max-width: none;
      }
      .memory-list,
      .chat-stack {
        max-height: none;
      }
      .stats-grid {
        grid-template-columns: 1fr;
      }
    }
  </style>
</head>
<body>
  <div class="workspace-shell">
    <span class="sr-only">__SERVICE__ Browser Console</span>
    <div class="workspace-grid">
      <div class="column left-rail">
        <section class="card">
          <div class="card-header">
            <h2 class="card-title">来源分类</h2>
          </div>

          <div class="stats-grid">
            <div class="stat-card">
              <div class="stat-label">Memory 总数</div>
              <div class="stat-value" id="total-count">0</div>
            </div>
            <div class="stat-card">
              <div class="stat-label">当前筛选</div>
              <div class="stat-value" id="filtered-count">0</div>
            </div>
          </div>

          <div class="filter-group">
            <div class="section-title">归属</div>
            <div class="segmented" id="ownership-filter">
              <button type="button" data-value="all" class="active">全部</button>
              <button type="button" data-value="team">团队</button>
              <button type="button" data-value="personal">个人</button>
            </div>
          </div>

          <div class="filter-group">
            <div class="card-header">
              <div class="section-title">Agent</div>
              <button type="button" class="toolbar-button" id="reset-agent">全部</button>
            </div>
            <div class="chip-row" id="agent-chips">
              <span class="chip active" data-value="all">全部 Agent</span>
            </div>
          </div>

          <div class="filter-group">
            <div class="card-header">
              <div class="section-title">Scope</div>
              <button type="button" class="toolbar-button" id="reset-scope">清空</button>
            </div>
            <div class="scope-list" id="scope-list">
              <button type="button" class="scope-button active" data-value="all">
                <span>全部 Scope</span>
                <span>0</span>
              </button>
            </div>
          </div>
        </section>
      </div>

      <div class="column center-pane">
        <section class="card">
          <div class="toolbar">
            <input id="search-input" class="search-input" placeholder="搜索标题、正文、scope 或 Agent" />
            <div class="toolbar-actions">
              <span class="memory-meta-tag">来源接口 /api/v1/explorer/memories</span>
              <button type="button" class="toolbar-button primary" id="refresh-workspace">刷新</button>
            </div>
          </div>

          <div id="list-status" class="status-box">正在加载 memory 列表...</div>
          <div id="memory-list" class="memory-list hidden"></div>
          <div id="memory-empty" class="empty hidden">当前筛选下没有可显示的 memory。</div>
        </section>
      </div>

      <div class="column right-rail">
        <section class="card">
          <div class="card-header">
            <h2 class="card-title">监控概览</h2>
            <span class="memory-meta-tag">/metrics · /api/v1/metrics/keys</span>
          </div>
          <div id="metrics-status" class="status-box">正在加载监控数据...</div>
          <div id="metrics-panel" class="hidden">
            <div class="metric-grid">
              <div class="metric-block">
                <div class="stat-label">搜索 p95</div>
                <div class="stat-value" id="search-p95">0ms</div>
              </div>
              <div class="metric-block">
                <div class="stat-label">写入 p95</div>
                <div class="stat-value" id="write-p95">0ms</div>
              </div>
              <div class="metric-block">
                <div class="stat-label">搜索命中率</div>
                <div class="stat-value" id="search-hit-rate">0%</div>
              </div>
              <div class="metric-block">
                <div class="stat-label">Key 成功率</div>
                <div class="stat-value" id="key-success-rate">0%</div>
              </div>
              <div class="metric-block metric-block-wide">
                <div class="stat-label">最近统计</div>
                <div class="metric-strip">
                  <span class="memory-meta-tag" id="search-volume">搜索 0</span>
                  <span class="memory-meta-tag" id="write-volume">写入 0</span>
                  <span class="memory-meta-tag" id="key-volume">Key 操作 0</span>
                </div>
              </div>
              <div class="metric-block metric-block-wide">
                <div class="stat-label">存储模式分布</div>
                <div class="metric-strip">
                  <span class="memory-meta-tag" id="mode-file">file 0</span>
                  <span class="memory-meta-tag" id="mode-vector">vector 0</span>
                  <span class="memory-meta-tag" id="mode-all">all 0</span>
                </div>
              </div>
              <div class="metric-block metric-block-wide">
                <div class="stat-label">V2.4 Memory 能力</div>
                <div class="metric-strip">
                  <span class="memory-meta-tag" id="source-volume">Source 0</span>
                  <span class="memory-meta-tag" id="context-volume">Context 0</span>
                  <span class="memory-meta-tag" id="docs-volume">Docs 0</span>
                  <span class="memory-meta-tag" id="docs-conflicts">Conflicts 0</span>
                </div>
              </div>
            </div>
          </div>
        </section>

        <section class="card">
          <div class="card-header">
            <h2 class="card-title">Benchmark Console</h2>
            <span class="memory-meta-tag">/api/v1/benchmark/run · report · failures</span>
          </div>
          <div class="chat-composer">
            <select id="benchmark-suite" class="chat-input">
              <option value="meat-code-zh">meat-code-zh</option>
              <option value="locomo">locomo</option>
              <option value="longmemeval">longmemeval</option>
              <option value="beam">beam</option>
              <option value="memory-1m">memory-1m</option>
            </select>
            <input id="benchmark-scope-id" class="chat-input" placeholder="scope_id" value="__DEFAULT_SCOPE__" />
            <input id="benchmark-output-dir" class="chat-input" placeholder="output_dir" value="tests/reports/benchmark/latest" />
          </div>
          <div class="chat-composer" style="margin-top: 10px;">
            <input id="benchmark-input-dir" class="chat-input" placeholder="input_dir" value="tests/reports/benchmark/latest" />
          </div>
          <div class="chip-row" style="margin-top: 10px;">
            <button type="button" class="send-button" id="benchmark-run">Run</button>
            <button type="button" class="toolbar-button" id="benchmark-report">Load Report</button>
            <button type="button" class="toolbar-button" id="benchmark-failures">Failures</button>
            <button type="button" class="toolbar-button" id="benchmark-runs">Runs</button>
          </div>
          <div id="benchmark-status" class="status-box hidden" style="margin-top: 12px;"></div>
          <div id="benchmark-meta" class="memory-expanded hidden" style="margin-top: 12px;"></div>
          <pre id="benchmark-output" class="memory-expanded-body hidden" style="margin-top: 12px; white-space: pre-wrap;"></pre>
        </section>

        <section class="card">
          <div class="card-header">
            <h2 class="card-title">Trace Console</h2>
            <span class="memory-meta-tag">/api/v1/recall/traces/latest · inspect</span>
          </div>
          <div class="chat-composer">
            <input id="trace-scope-id" class="chat-input" placeholder="scope_id" value="__DEFAULT_SCOPE__" />
            <input id="trace-query" class="chat-input" placeholder="query" value="memory trace" />
          </div>
          <div class="chat-composer" style="margin-top: 10px;">
            <input id="trace-output-dir" class="chat-input" placeholder="output_dir" value="tests/reports/trace/latest" />
            <input id="trace-input-dir" class="chat-input" placeholder="input_dir" value="tests/reports/trace/latest" />
            <input id="trace-id" class="chat-input" placeholder="trace_id for inspect" />
          </div>
          <div class="chat-composer" style="margin-top: 10px;">
            <input id="trace-max-records" class="chat-input" placeholder="max_records" value="5" />
            <input id="trace-max-chars" class="chat-input" placeholder="max_chars" value="2000" />
          </div>
          <label class="memory-meta-tag" style="margin-top: 10px;">
            <input id="trace-debug-candidates" type="checkbox" />
            debug candidates
          </label>
          <div class="chip-row" style="margin-top: 10px;">
            <button type="button" class="send-button" id="trace-latest">Generate Trace</button>
            <button type="button" class="toolbar-button" id="trace-inspect">Inspect</button>
          </div>
          <div id="trace-status" class="status-box hidden" style="margin-top: 12px;"></div>
          <div id="trace-meta" class="memory-expanded hidden" style="margin-top: 12px;"></div>
          <pre id="trace-output" class="memory-expanded-body hidden" style="margin-top: 12px; white-space: pre-wrap;"></pre>
        </section>

        <section class="card">
          <div class="card-header">
            <h2 class="card-title">Health Console</h2>
            <span class="memory-meta-tag">/api/v1/health/report</span>
          </div>
          <div class="chat-composer">
            <input id="health-scope-id" class="chat-input" placeholder="scope_id" value="__DEFAULT_SCOPE__" />
            <input id="health-limit" class="chat-input" placeholder="limit" value="500" />
            <button type="button" class="send-button" id="health-load">Load Health</button>
          </div>
          <div id="health-status" class="status-box hidden" style="margin-top: 12px;"></div>
          <div id="health-meta" class="memory-expanded hidden" style="margin-top: 12px;"></div>
          <pre id="health-output" class="memory-expanded-body hidden" style="margin-top: 12px; white-space: pre-wrap;"></pre>
        </section>

        <section class="card">
          <div class="card-header">
            <h2 class="card-title">Passport Console</h2>
            <span class="memory-meta-tag">/api/v1/passports/export · import · manifest</span>
          </div>
          <div class="chat-composer">
            <input id="passport-scope-id" class="chat-input" placeholder="scope_id" value="__DEFAULT_SCOPE__" />
            <input id="passport-output-dir" class="chat-input" placeholder="output_dir" value="tests/reports/passport/latest" />
            <input id="passport-limit" class="chat-input" placeholder="limit" value="100" />
          </div>
          <div class="chat-composer" style="margin-top: 10px;">
            <input id="passport-input-dir" class="chat-input" placeholder="input_dir" value="tests/reports/passport/latest" />
            <input id="passport-target-scope-id" class="chat-input" placeholder="target_scope_id for import" />
          </div>
          <div class="chip-row" style="margin-top: 10px;">
            <label class="memory-meta-tag">
              <input id="passport-redact-sensitive" type="checkbox" checked />
              redact sensitive
            </label>
            <label class="memory-meta-tag">
              <input id="passport-dry-run" type="checkbox" checked />
              dry run import
            </label>
          </div>
          <div class="chip-row" style="margin-top: 10px;">
            <button type="button" class="toolbar-button" id="passport-export">Export</button>
            <button type="button" class="toolbar-button" id="passport-verify">Verify Manifest</button>
            <button type="button" class="send-button" id="passport-import">Import</button>
          </div>
          <div id="passport-status" class="status-box hidden" style="margin-top: 12px;"></div>
          <div id="passport-meta" class="memory-expanded hidden" style="margin-top: 12px;"></div>
          <pre id="passport-output" class="memory-expanded-body hidden" style="margin-top: 12px; white-space: pre-wrap;"></pre>
        </section>

        <section class="card">
          <div class="card-header">
            <h2 class="card-title">AI 对话区</h2>
          </div>
          <div id="chat-note" class="chat-note muted">
            当前对话会自动结合左侧筛选和中间选中的 memory，基于本地 Memory 内容给出总结。
          </div>
          <div id="chat-stack" class="chat-stack"></div>
          <form id="chat-form" class="chat-composer">
            <input id="chat-input" class="chat-input" placeholder="问点什么，例如：团队里最近有哪些关于发布流程的记忆？" />
            <button type="submit" class="send-button">发送</button>
          </form>
        </section>

        <section class="card">
          <div class="card-header">
            <h2 class="card-title">Projection Debug</h2>
            <span class="memory-meta-tag">source/document</span>
            <span class="memory-meta-tag">connector dry-run /api/v1/compat/connectors/dry-run</span>
            <span class="memory-meta-tag">sync-plan / import-draft / proposal-queue / apply-plan / POST apply</span>
          </div>
          <div class="chat-note muted">
            填写 raw key、source_id、document_id，直接查看 V2.4 project document markdown projection。
          </div>
          <div class="chat-composer" style="margin-top: 12px;">
            <input id="projection-key" class="chat-input" placeholder="raw key" />
            <button type="button" class="send-button" id="projection-load-sources">载入来源</button>
          </div>
          <div class="chat-composer" style="margin-top: 10px;">
            <input id="projection-source-search" class="chat-input" placeholder="筛选 source" />
          </div>
          <div class="chat-composer" style="margin-top: 10px;">
            <select id="projection-source-select" class="chat-input">
              <option value="">先载入 source 列表</option>
            </select>
          </div>
          <div class="chat-composer" style="margin-top: 10px;">
            <input id="projection-source-id" class="chat-input" placeholder="source_id" />
            <button type="button" class="send-button" id="projection-load-docs">载入文档</button>
          </div>
          <div class="chat-composer" style="margin-top: 10px;">
            <input id="projection-document-search" class="chat-input" placeholder="筛选 document" />
          </div>
          <div class="chat-composer" style="margin-top: 10px;">
            <select id="projection-document-select" class="chat-input">
              <option value="">先载入该 source 的文档列表</option>
            </select>
          </div>
          <div id="projection-doc-actions" class="chip-row" style="margin-top: 10px;"></div>
          <div class="chat-composer" style="margin-top: 10px;">
            <input id="projection-document-id" class="chat-input" placeholder="document_id" />
            <button type="button" class="send-button" id="projection-fetch">查看</button>
          </div>
          <div class="chat-composer" style="margin-top: 10px;">
            <button type="button" class="toolbar-button" id="projection-copy-path">复制路径</button>
            <button type="button" class="toolbar-button" id="projection-copy-markdown">复制 Markdown</button>
          </div>
          <div id="projection-status" class="status-box hidden" style="margin-top: 12px;"></div>
          <div id="projection-meta" class="memory-expanded hidden" style="margin-top: 12px;"></div>
          <pre id="projection-output" class="memory-expanded-body hidden" style="margin-top: 12px; white-space: pre-wrap;"></pre>
          <div class="memory-expanded" style="margin-top: 16px;">
            <div class="memory-expanded-title" style="font-size: 18px;">Connector Review Console</div>
            <div class="chat-note muted">
              用同一组参数查看 dry-run、proposal queue、apply-plan，并在 token + raw key 确认后调用 service-side confirmed executor。
            </div>
            <div class="chat-composer" style="margin-top: 10px;">
              <input id="connector-key" class="chat-input" placeholder="raw key for confirmed apply" />
            </div>
            <div class="chat-composer" style="margin-top: 10px;">
              <select id="connector-name" class="chat-input">
                <option value="chat-export">chat-export</option>
                <option value="markdown-docs">markdown-docs</option>
                <option value="local-git">local-git</option>
                <option value="web-crawler">web-crawler</option>
              </select>
              <input id="connector-root-path" class="chat-input" placeholder="root_path" />
              <input id="connector-scope-id" class="chat-input" placeholder="scope_id" value="__DEFAULT_SCOPE__" />
              <input id="connector-source-id" class="chat-input" placeholder="source_id (optional for confirmed apply)" />
              <input id="connector-max-items" class="chat-input" placeholder="max_items" value="20" />
            </div>
            <label class="chat-note" style="display: inline-flex; align-items: center; gap: 8px; margin-top: 8px;">
              <input id="connector-allow-remote-fetch" type="checkbox" />
              allow remote fetch
            </label>
            <div class="chat-composer" style="margin-top: 10px;">
              <input id="connector-queue-item-ids" class="chat-input" placeholder="approved_queue_item_ids, comma separated" />
              <input id="connector-confirmation-token" class="chat-input" placeholder="confirmation_token" />
              <input id="connector-queue-id" class="chat-input" placeholder="queue_id (optional persisted proposal queue)" />
            </div>
            <div class="chip-row" style="margin-top: 10px;">
              <button type="button" class="toolbar-button" id="connector-dry-run">Dry Run</button>
              <button type="button" class="toolbar-button" id="connector-sync-plan">Sync Plan</button>
              <button type="button" class="toolbar-button" id="connector-proposal-queue">Proposal Queue</button>
              <button type="button" class="toolbar-button" id="connector-persist-queue">Persist Queue</button>
              <button type="button" class="toolbar-button" id="connector-load-queue">Load Queue</button>
              <button type="button" class="toolbar-button" id="connector-apply-plan">Apply Plan</button>
              <button type="button" class="send-button" id="connector-apply-confirm">Confirmed Apply</button>
            </div>
            <div id="connector-status" class="status-box hidden" style="margin-top: 12px;"></div>
            <div id="connector-meta" class="memory-expanded hidden" style="margin-top: 12px;"></div>
            <pre id="connector-output" class="memory-expanded-body hidden" style="margin-top: 12px; white-space: pre-wrap;"></pre>
          </div>
        </section>
      </div>
    </div>
  </div>

  <script>
    const metadata = __METADATA__;
    const PROJECTION_DEBUG_STORAGE_KEY = "meat-memory.projection-debug.v1";
    const CONNECTOR_DEBUG_STORAGE_KEY = "meat-memory.connector-debug.v1";
    const BENCHMARK_DEBUG_STORAGE_KEY = "meat-memory.benchmark-debug.v1";
    const TRACE_DEBUG_STORAGE_KEY = "meat-memory.trace-debug.v1";
    const HEALTH_DEBUG_STORAGE_KEY = "meat-memory.health-debug.v1";
    const PASSPORT_DEBUG_STORAGE_KEY = "meat-memory.passport-debug.v1";
    async function fetchJson(url, options) {
      const response = await fetch(url, options);
      const text = await response.text();
      let payload;
      try {
        payload = text ? JSON.parse(text) : {};
      } catch (_error) {
        payload = { raw: text };
      }
      if (!response.ok) {
        throw new Error(payload.error || JSON.stringify(payload, null, 2));
      }
      return payload;
    }

    const state = {
      workspace: null,
      metrics: null,
      ownershipFilter: "all",
      agentFilter: "all",
      scopeFilter: "scp_meat_memory_v1",
      searchText: "",
      selectedMemoryId: null,
      projectionSources: [],
      projectionDocuments: [],
      projectionSourceSearch: "",
      projectionDocumentSearch: "",
      connectorReport: null,
      benchmarkReport: null,
      traceReport: null,
      healthReport: null,
      passportReport: null,
      chatLoading: false,
      messages: [
        {
          role: "assistant",
          content: "这里是 AI 对话区。你可以基于左侧来源分类和中间选中的 memory，继续追问、汇总或收敛范围。",
          citations: [],
        },
      ],
    };

    const totalCountEl = document.getElementById("total-count");
    const filteredCountEl = document.getElementById("filtered-count");
    const agentChipsEl = document.getElementById("agent-chips");
    const scopeListEl = document.getElementById("scope-list");
    const searchInputEl = document.getElementById("search-input");
    const refreshButtonEl = document.getElementById("refresh-workspace");
    const listStatusEl = document.getElementById("list-status");
    const memoryListEl = document.getElementById("memory-list");
    const memoryEmptyEl = document.getElementById("memory-empty");
    const metricsStatusEl = document.getElementById("metrics-status");
    const metricsPanelEl = document.getElementById("metrics-panel");
    const searchP95El = document.getElementById("search-p95");
    const writeP95El = document.getElementById("write-p95");
    const searchHitRateEl = document.getElementById("search-hit-rate");
    const keySuccessRateEl = document.getElementById("key-success-rate");
    const searchVolumeEl = document.getElementById("search-volume");
    const writeVolumeEl = document.getElementById("write-volume");
    const keyVolumeEl = document.getElementById("key-volume");
    const modeFileEl = document.getElementById("mode-file");
    const modeVectorEl = document.getElementById("mode-vector");
    const modeAllEl = document.getElementById("mode-all");
    const sourceVolumeEl = document.getElementById("source-volume");
    const contextVolumeEl = document.getElementById("context-volume");
    const docsVolumeEl = document.getElementById("docs-volume");
    const docsConflictsEl = document.getElementById("docs-conflicts");
    const chatStackEl = document.getElementById("chat-stack");
    const chatFormEl = document.getElementById("chat-form");
    const chatInputEl = document.getElementById("chat-input");
    const chatNoteEl = document.getElementById("chat-note");
    const projectionKeyEl = document.getElementById("projection-key");
    const projectionLoadSourcesEl = document.getElementById("projection-load-sources");
    const projectionSourceSearchEl = document.getElementById("projection-source-search");
    const projectionSourceSelectEl = document.getElementById("projection-source-select");
    const projectionSourceIdEl = document.getElementById("projection-source-id");
    const projectionLoadDocsEl = document.getElementById("projection-load-docs");
    const projectionDocumentSearchEl = document.getElementById("projection-document-search");
    const projectionDocumentSelectEl = document.getElementById("projection-document-select");
    const projectionDocActionsEl = document.getElementById("projection-doc-actions");
    const projectionDocumentIdEl = document.getElementById("projection-document-id");
    const projectionFetchEl = document.getElementById("projection-fetch");
    const projectionCopyPathEl = document.getElementById("projection-copy-path");
    const projectionCopyMarkdownEl = document.getElementById("projection-copy-markdown");
    const projectionStatusEl = document.getElementById("projection-status");
    const projectionMetaEl = document.getElementById("projection-meta");
    const projectionOutputEl = document.getElementById("projection-output");
    const connectorKeyEl = document.getElementById("connector-key");
    const connectorNameEl = document.getElementById("connector-name");
    const connectorRootPathEl = document.getElementById("connector-root-path");
    const connectorScopeIdEl = document.getElementById("connector-scope-id");
    const connectorSourceIdEl = document.getElementById("connector-source-id");
    const connectorMaxItemsEl = document.getElementById("connector-max-items");
    const connectorAllowRemoteFetchEl = document.getElementById("connector-allow-remote-fetch");
    const connectorQueueItemIdsEl = document.getElementById("connector-queue-item-ids");
    const connectorConfirmationTokenEl = document.getElementById("connector-confirmation-token");
    const connectorQueueIdEl = document.getElementById("connector-queue-id");
    const connectorDryRunEl = document.getElementById("connector-dry-run");
    const connectorSyncPlanEl = document.getElementById("connector-sync-plan");
    const connectorProposalQueueEl = document.getElementById("connector-proposal-queue");
    const connectorPersistQueueEl = document.getElementById("connector-persist-queue");
    const connectorLoadQueueEl = document.getElementById("connector-load-queue");
    const connectorApplyPlanEl = document.getElementById("connector-apply-plan");
    const connectorApplyConfirmEl = document.getElementById("connector-apply-confirm");
    const connectorStatusEl = document.getElementById("connector-status");
    const connectorMetaEl = document.getElementById("connector-meta");
    const connectorOutputEl = document.getElementById("connector-output");
    const benchmarkSuiteEl = document.getElementById("benchmark-suite");
    const benchmarkScopeIdEl = document.getElementById("benchmark-scope-id");
    const benchmarkOutputDirEl = document.getElementById("benchmark-output-dir");
    const benchmarkInputDirEl = document.getElementById("benchmark-input-dir");
    const benchmarkRunEl = document.getElementById("benchmark-run");
    const benchmarkReportEl = document.getElementById("benchmark-report");
    const benchmarkFailuresEl = document.getElementById("benchmark-failures");
    const benchmarkRunsEl = document.getElementById("benchmark-runs");
    const benchmarkStatusEl = document.getElementById("benchmark-status");
    const benchmarkMetaEl = document.getElementById("benchmark-meta");
    const benchmarkOutputEl = document.getElementById("benchmark-output");
    const traceScopeIdEl = document.getElementById("trace-scope-id");
    const traceQueryEl = document.getElementById("trace-query");
    const traceOutputDirEl = document.getElementById("trace-output-dir");
    const traceInputDirEl = document.getElementById("trace-input-dir");
    const traceIdEl = document.getElementById("trace-id");
    const traceMaxRecordsEl = document.getElementById("trace-max-records");
    const traceMaxCharsEl = document.getElementById("trace-max-chars");
    const traceDebugCandidatesEl = document.getElementById("trace-debug-candidates");
    const traceLatestEl = document.getElementById("trace-latest");
    const traceInspectEl = document.getElementById("trace-inspect");
    const traceStatusEl = document.getElementById("trace-status");
    const traceMetaEl = document.getElementById("trace-meta");
    const traceOutputEl = document.getElementById("trace-output");
    const healthScopeIdEl = document.getElementById("health-scope-id");
    const healthLimitEl = document.getElementById("health-limit");
    const healthLoadEl = document.getElementById("health-load");
    const healthStatusEl = document.getElementById("health-status");
    const healthMetaEl = document.getElementById("health-meta");
    const healthOutputEl = document.getElementById("health-output");
    const passportScopeIdEl = document.getElementById("passport-scope-id");
    const passportOutputDirEl = document.getElementById("passport-output-dir");
    const passportLimitEl = document.getElementById("passport-limit");
    const passportInputDirEl = document.getElementById("passport-input-dir");
    const passportTargetScopeIdEl = document.getElementById("passport-target-scope-id");
    const passportRedactSensitiveEl = document.getElementById("passport-redact-sensitive");
    const passportDryRunEl = document.getElementById("passport-dry-run");
    const passportExportEl = document.getElementById("passport-export");
    const passportVerifyEl = document.getElementById("passport-verify");
    const passportImportEl = document.getElementById("passport-import");
    const passportStatusEl = document.getElementById("passport-status");
    const passportMetaEl = document.getElementById("passport-meta");
    const passportOutputEl = document.getElementById("passport-output");

    function escapeHtml(value) {
      return String(value)
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#39;");
    }

    function filteredMemories() {
      const items = (state.workspace && state.workspace.memories) || [];
      return items.filter((item) => {
        if (state.ownershipFilter !== "all" && item.ownership_key !== state.ownershipFilter) {
          return false;
        }
        if (state.agentFilter !== "all" && item.agent_key !== state.agentFilter) {
          return false;
        }
        if (state.scopeFilter !== "all" && item.scope_id !== state.scopeFilter) {
          return false;
        }
        if (state.searchText.trim()) {
          const query = state.searchText.trim().toLowerCase();
          const haystack = [
            item.title,
            item.body,
            item.scope_id,
            item.agent_label,
            item.ownership_label,
          ]
            .join(" ")
            .toLowerCase();
          return haystack.includes(query);
        }
        return true;
      });
    }

    function selectedMemory() {
      return filteredMemories().find((item) => item.memory_id === state.selectedMemoryId) || null;
    }

    function syncSelection() {
      const items = filteredMemories();
      if (!items.length) {
        state.selectedMemoryId = null;
        return;
      }
      const stillSelected = items.some((item) => item.memory_id === state.selectedMemoryId);
      if (!stillSelected) {
        state.selectedMemoryId = items[0].memory_id;
      }
    }

    function percent(value) {
      return (Number(value || 0) * 100).toFixed(0) + "%";
    }

    function metricNumber(value) {
      return String(Number(value || 0));
    }

    function metricLatency(value) {
      return String(Number(value || 0)) + "ms";
    }

    function loadProjectionDebugState() {
      try {
        const raw = localStorage.getItem(PROJECTION_DEBUG_STORAGE_KEY);
        if (!raw) {
          return;
        }
        const persisted = JSON.parse(raw);
        projectionKeyEl.value = String(persisted.rawKey || "");
        projectionSourceIdEl.value = String(persisted.sourceId || "");
        projectionDocumentIdEl.value = String(persisted.documentId || "");
        projectionSourceSearchEl.value = String(persisted.sourceSearch || "");
        projectionDocumentSearchEl.value = String(persisted.documentSearch || "");
        state.projectionSourceSearch = projectionSourceSearchEl.value;
        state.projectionDocumentSearch = projectionDocumentSearchEl.value;
      } catch (_error) {
      }
    }

    function saveProjectionDebugState() {
      try {
        localStorage.setItem(
          PROJECTION_DEBUG_STORAGE_KEY,
          JSON.stringify({
            rawKey: projectionKeyEl.value.trim(),
            sourceId: projectionSourceIdEl.value.trim(),
            documentId: projectionDocumentIdEl.value.trim(),
            sourceSearch: projectionSourceSearchEl.value || "",
            documentSearch: projectionDocumentSearchEl.value || "",
          })
        );
      } catch (_error) {
      }
    }

    function loadConnectorDebugState() {
      try {
        const raw = localStorage.getItem(CONNECTOR_DEBUG_STORAGE_KEY);
        if (!raw) {
          return;
        }
        const persisted = JSON.parse(raw);
        connectorKeyEl.value = String(persisted.rawKey || "");
        connectorNameEl.value = String(persisted.connector || "chat-export");
        connectorRootPathEl.value = String(persisted.rootPath || "");
        connectorScopeIdEl.value = String(persisted.scopeId || metadata.default_scope || "");
        connectorSourceIdEl.value = String(persisted.sourceId || "");
        connectorMaxItemsEl.value = String(persisted.maxItems || "20");
        connectorAllowRemoteFetchEl.checked = Boolean(persisted.allowRemoteFetch);
        connectorQueueItemIdsEl.value = String(persisted.queueItemIds || "");
        connectorConfirmationTokenEl.value = String(persisted.confirmationToken || "");
        connectorQueueIdEl.value = String(persisted.queueId || "");
      } catch (_error) {
      }
    }

    function saveConnectorDebugState() {
      try {
        localStorage.setItem(
          CONNECTOR_DEBUG_STORAGE_KEY,
          JSON.stringify({
            rawKey: connectorKeyEl.value.trim(),
            connector: connectorNameEl.value || "chat-export",
            rootPath: connectorRootPathEl.value.trim(),
            scopeId: connectorScopeIdEl.value.trim(),
            sourceId: connectorSourceIdEl.value.trim(),
            maxItems: connectorMaxItemsEl.value.trim(),
            allowRemoteFetch: connectorAllowRemoteFetchEl.checked,
            queueItemIds: connectorQueueItemIdsEl.value.trim(),
            confirmationToken: connectorConfirmationTokenEl.value.trim(),
            queueId: connectorQueueIdEl.value.trim(),
          })
        );
      } catch (_error) {
      }
    }

    function loadBenchmarkDebugState() {
      try {
        const raw = localStorage.getItem(BENCHMARK_DEBUG_STORAGE_KEY);
        if (!raw) {
          return;
        }
        const persisted = JSON.parse(raw);
        benchmarkSuiteEl.value = String(persisted.suite || "meat-code-zh");
        benchmarkScopeIdEl.value = String(persisted.scopeId || metadata.default_scope || "");
        benchmarkOutputDirEl.value = String(persisted.outputDir || "tests/reports/benchmark/latest");
        benchmarkInputDirEl.value = String(persisted.inputDir || "tests/reports/benchmark/latest");
      } catch (_error) {
      }
    }

    function saveBenchmarkDebugState() {
      try {
        localStorage.setItem(
          BENCHMARK_DEBUG_STORAGE_KEY,
          JSON.stringify({
            suite: benchmarkSuiteEl.value || "meat-code-zh",
            scopeId: benchmarkScopeIdEl.value.trim(),
            outputDir: benchmarkOutputDirEl.value.trim(),
            inputDir: benchmarkInputDirEl.value.trim(),
          })
        );
      } catch (_error) {
      }
    }

    function loadTraceDebugState() {
      try {
        const raw = localStorage.getItem(TRACE_DEBUG_STORAGE_KEY);
        if (!raw) {
          return;
        }
        const persisted = JSON.parse(raw);
        traceScopeIdEl.value = String(persisted.scopeId || metadata.default_scope || "");
        traceQueryEl.value = String(persisted.query || "memory trace");
        traceOutputDirEl.value = String(persisted.outputDir || "tests/reports/trace/latest");
        traceInputDirEl.value = String(persisted.inputDir || "tests/reports/trace/latest");
        traceIdEl.value = String(persisted.traceId || "");
        traceMaxRecordsEl.value = String(persisted.maxRecords || "5");
        traceMaxCharsEl.value = String(persisted.maxChars || "2000");
        traceDebugCandidatesEl.checked = Boolean(persisted.debugCandidates);
      } catch (_error) {
      }
    }

    function saveTraceDebugState() {
      try {
        localStorage.setItem(
          TRACE_DEBUG_STORAGE_KEY,
          JSON.stringify({
            scopeId: traceScopeIdEl.value.trim(),
            query: traceQueryEl.value.trim(),
            outputDir: traceOutputDirEl.value.trim(),
            inputDir: traceInputDirEl.value.trim(),
            traceId: traceIdEl.value.trim(),
            maxRecords: traceMaxRecordsEl.value.trim(),
            maxChars: traceMaxCharsEl.value.trim(),
            debugCandidates: traceDebugCandidatesEl.checked,
          })
        );
      } catch (_error) {
      }
    }

    function loadHealthDebugState() {
      try {
        const raw = localStorage.getItem(HEALTH_DEBUG_STORAGE_KEY);
        if (!raw) {
          return;
        }
        const persisted = JSON.parse(raw);
        healthScopeIdEl.value = String(persisted.scopeId || metadata.default_scope || "");
        healthLimitEl.value = String(persisted.limit || "500");
      } catch (_error) {
      }
    }

    function saveHealthDebugState() {
      try {
        localStorage.setItem(
          HEALTH_DEBUG_STORAGE_KEY,
          JSON.stringify({
            scopeId: healthScopeIdEl.value.trim(),
            limit: healthLimitEl.value.trim(),
          })
        );
      } catch (_error) {
      }
    }

    function loadPassportDebugState() {
      try {
        const raw = localStorage.getItem(PASSPORT_DEBUG_STORAGE_KEY);
        if (!raw) {
          return;
        }
        const persisted = JSON.parse(raw);
        passportScopeIdEl.value = String(persisted.scopeId || metadata.default_scope || "");
        passportOutputDirEl.value = String(persisted.outputDir || "tests/reports/passport/latest");
        passportLimitEl.value = String(persisted.limit || "100");
        passportInputDirEl.value = String(persisted.inputDir || "tests/reports/passport/latest");
        passportTargetScopeIdEl.value = String(persisted.targetScopeId || "");
        passportRedactSensitiveEl.checked = persisted.redactSensitive !== false;
        passportDryRunEl.checked = persisted.dryRun !== false;
      } catch (_error) {
      }
    }

    function savePassportDebugState() {
      try {
        localStorage.setItem(
          PASSPORT_DEBUG_STORAGE_KEY,
          JSON.stringify({
            scopeId: passportScopeIdEl.value.trim(),
            outputDir: passportOutputDirEl.value.trim(),
            limit: passportLimitEl.value.trim(),
            inputDir: passportInputDirEl.value.trim(),
            targetScopeId: passportTargetScopeIdEl.value.trim(),
            redactSensitive: passportRedactSensitiveEl.checked,
            dryRun: passportDryRunEl.checked,
          })
        );
      } catch (_error) {
      }
    }

    function filteredProjectionSources() {
      const query = state.projectionSourceSearch.trim().toLowerCase();
      const sources = state.projectionSources || [];
      if (!query) {
        return sources;
      }
      return sources.filter((source) => {
        return [
          source.source_id,
          source.name,
          source.source_kind,
          source.local_root,
        ]
          .filter(Boolean)
          .join(" ")
          .toLowerCase()
          .includes(query);
      });
    }

    function filteredProjectionDocuments() {
      const query = state.projectionDocumentSearch.trim().toLowerCase();
      const documents = state.projectionDocuments || [];
      if (!query) {
        return documents;
      }
      return documents.filter((document) => {
        return [
          document.document_id,
          document.title,
          document.canonical_uri,
          document.local_path,
        ]
          .filter(Boolean)
          .join(" ")
          .toLowerCase()
          .includes(query);
      });
    }

    function renderProjectionSources() {
      const sources = filteredProjectionSources();
      if (!sources.length) {
        projectionSourceSelectEl.innerHTML = '<option value="">' +
          (state.projectionSources.length ? '没有匹配的 source' : '先载入 source 列表') +
          '</option>';
        return;
      }
      projectionSourceSelectEl.innerHTML =
        '<option value="">选择一个 source_id</option>' +
        sources
          .map((source) => {
            const id = String(source.source_id || "");
            const name = String(source.name || id);
            return '<option value="' + escapeHtml(id) + '">' + escapeHtml(name + " [" + id + "]") + '</option>';
          })
          .join("");
      if (projectionSourceIdEl.value) {
        projectionSourceSelectEl.value = projectionSourceIdEl.value;
      }
    }

    function renderProjectionDocuments() {
      const documents = filteredProjectionDocuments();
      if (!documents.length) {
        projectionDocumentSelectEl.innerHTML = '<option value="">' +
          (state.projectionDocuments.length ? '没有匹配的文档' : '先载入该 source 的文档列表') +
          '</option>';
        projectionDocActionsEl.innerHTML = "";
        return;
      }
      projectionDocumentSelectEl.innerHTML =
        '<option value="">选择一个 document_id</option>' +
        documents
          .map((document) => {
            const id = String(document.document_id || "");
            const title = String(document.title || id);
            return '<option value="' + escapeHtml(id) + '">' + escapeHtml(title + " [" + id + "]") + '</option>';
          })
          .join("");
      projectionDocActionsEl.innerHTML = documents
        .slice(0, 12)
        .map((document) => {
          const id = String(document.document_id || "");
          const title = String(document.title || id);
          return '<button type="button" class="toolbar-button projection-doc-button" data-document-id="' +
            escapeHtml(id) +
            '">' +
            escapeHtml(title) +
            "</button>";
        })
        .join("");
      projectionDocActionsEl.querySelectorAll(".projection-doc-button").forEach((button) => {
        button.addEventListener("click", () => {
          projectionDocumentIdEl.value = button.dataset.documentId || "";
          projectionDocumentSelectEl.value = button.dataset.documentId || "";
          loadProjection();
        });
      });
      if (projectionDocumentIdEl.value) {
        projectionDocumentSelectEl.value = projectionDocumentIdEl.value;
      }
    }

    function renderOwnershipFilter() {
      const buttons = document.querySelectorAll("#ownership-filter button");
      buttons.forEach((button) => {
        button.classList.toggle("active", button.dataset.value === state.ownershipFilter);
      });
    }

    function renderAgentFilters() {
      const groups = (state.workspace && state.workspace.agent_groups) || [];
      const rows = ['<span class="chip' + (state.agentFilter === "all" ? ' active' : '') + '" data-value="all">全部 Agent</span>'];
      groups.forEach((item) => {
        const active = state.agentFilter === item.key ? " active" : "";
        rows.push(
          '<span class="chip' + active + '" data-value="' + escapeHtml(item.key) + '">' +
          escapeHtml(item.label) + " (" + item.count + ")" +
          "</span>"
        );
      });
      agentChipsEl.innerHTML = rows.join("");
      agentChipsEl.querySelectorAll(".chip").forEach((chip) => {
        chip.addEventListener("click", () => {
          state.agentFilter = chip.dataset.value || "all";
          syncSelection();
          render();
        });
      });
    }

    function renderScopeFilters() {
      const groups = (state.workspace && state.workspace.scope_groups) || [];
      const allCount = groups.reduce((sum, item) => sum + item.count, 0);
      const rows = [
        '<button type="button" class="scope-button' + (state.scopeFilter === "all" ? ' active' : '') + '" data-value="all">' +
          '<span class="scope-summary"><span class="scope-name">全部 Scope</span><span class="scope-count">恢复全部记录</span></span>' +
          "<span>" + allCount + "</span></button>",
      ];
      groups.slice(0, 10).forEach((item) => {
        const active = state.scopeFilter === item.key ? " active" : "";
        rows.push(
          '<button type="button" class="scope-button' + active + '" data-value="' + escapeHtml(item.key) + '">' +
            '<span class="scope-summary"><span class="scope-name">' + escapeHtml(item.label) + '</span><span class="scope-count">筛选中间列表</span></span>' +
            "<span>" + item.count + "</span></button>"
        );
      });
      scopeListEl.innerHTML = rows.join("");
      scopeListEl.querySelectorAll(".scope-button").forEach((button) => {
        button.addEventListener("click", () => {
          const nextScope = button.dataset.value || "all";
          if (nextScope === "all") {
            state.scopeFilter = "all";
          } else {
            state.scopeFilter = nextScope;
          }
          syncSelection();
          render();
        });
      });
    }

    function renderList() {
      const items = filteredMemories();
      totalCountEl.textContent = String((state.workspace && state.workspace.total_count) || 0);
      filteredCountEl.textContent = String(items.length);

      if (!state.workspace) {
        listStatusEl.textContent = "正在加载 memory 列表...";
        listStatusEl.classList.remove("hidden");
        memoryListEl.classList.add("hidden");
        memoryEmptyEl.classList.add("hidden");
        return;
      }

      listStatusEl.classList.add("hidden");
      if (!items.length) {
        memoryListEl.classList.add("hidden");
        memoryEmptyEl.classList.remove("hidden");
        memoryListEl.innerHTML = "";
        return;
      }

      memoryEmptyEl.classList.add("hidden");
      memoryListEl.classList.remove("hidden");
      memoryListEl.innerHTML = items
        .map((item) => {
          const active = item.memory_id === state.selectedMemoryId ? " active" : "";
          const expanded = item.memory_id === state.selectedMemoryId;
          return (
            '<article class="memory-item' + active + '" data-memory-id="' + escapeHtml(item.memory_id) + '">' +
              '<div class="memory-item-top">' +
                "<div>" +
                  '<div class="memory-item-title">' + escapeHtml(item.title) + "</div>" +
                  '<div class="muted">' + escapeHtml(item.scope_id) + "</div>" +
                "</div>" +
                '<span class="memory-meta-tag">' + escapeHtml(item.memory_kind) + "</span>" +
              "</div>" +
              '<div class="memory-inline-tags">' +
                '<span class="memory-meta-tag">' + escapeHtml(item.ownership_label) + "</span>" +
                '<span class="memory-meta-tag">' + escapeHtml(item.agent_label) + "</span>" +
                '<span class="memory-meta-tag">' + escapeHtml(item.visibility) + "</span>" +
                '<span class="memory-meta-tag">evidence ' + item.evidence_count + "</span>" +
              "</div>" +
              '<div class="memory-preview">' + escapeHtml(item.preview) + "</div>" +
              '<div class="muted" style="margin-top: 8px;">Updated ' + escapeHtml(item.updated_at) + "</div>" +
              (expanded
                ? '<div class="memory-expanded">' +
                    '<div class="memory-expanded-title">' + escapeHtml(item.title) + "</div>" +
                    '<div class="memory-expanded-meta muted">' + escapeHtml(item.scope_id) + " · " + escapeHtml(item.updated_at) + "</div>" +
                    '<div class="memory-meta">' +
                      '<span class="memory-meta-tag">' + escapeHtml(item.source_label) + "</span>" +
                      '<span class="memory-meta-tag">' + escapeHtml(item.memory_state) + "</span>" +
                      '<span class="memory-meta-tag">' + escapeHtml(item.sensitivity) + "</span>" +
                    "</div>" +
                    '<div class="memory-expanded-body">' + escapeHtml(item.body) + "</div>" +
                  "</div>"
                : "") +
            "</article>"
          );
        })
        .join("");

      memoryListEl.querySelectorAll(".memory-item").forEach((item) => {
        item.addEventListener("click", () => {
          state.selectedMemoryId = item.dataset.memoryId || null;
          render();
        });
      });
    }

    function renderChat() {
      const memory = selectedMemory();
      chatNoteEl.textContent = memory
        ? "当前对话会自动结合左侧筛选和中间选中的 memory，当前已选中：" + memory.title
        : "当前对话会自动结合左侧筛选和中间选中的 memory，当前未选中具体 memory，将按筛选范围回答。";

      chatStackEl.innerHTML = state.messages
        .map((message) => {
          const citations = (message.citations || [])
            .map((citation) =>
              '<span class="citation">' +
              escapeHtml(citation.title) +
              " · " +
              escapeHtml(citation.scope_id) +
              "</span>"
            )
            .join("");
          return (
            '<div class="chat-row ' + escapeHtml(message.role) + '">' +
              '<div class="chat-bubble">' + escapeHtml(message.content) + "</div>" +
              (citations ? '<div class="citation-strip">' + citations + "</div>" : "") +
            "</div>"
          );
        })
        .join("");
      chatStackEl.scrollTop = chatStackEl.scrollHeight;
    }

    function renderMetrics() {
      if (!state.metrics) {
        metricsStatusEl.textContent = "正在加载监控数据...";
        metricsStatusEl.classList.remove("hidden");
        metricsPanelEl.classList.add("hidden");
        return;
      }

      const metrics = state.metrics;
      const search = metrics.search || {};
      const write = metrics.write || {};
      const key = metrics.key || {};
      const v24 = metrics.v2_4 || {};

      searchP95El.textContent = metricLatency(search.latency && search.latency.p95_ms);
      writeP95El.textContent = metricLatency(write.latency && write.latency.p95_ms);
      searchHitRateEl.textContent = percent(search.hit_rate);
      const keySuccessRate = key.keyed_operations
        ? key.successful_operations / key.keyed_operations
        : 0;
      keySuccessRateEl.textContent = percent(keySuccessRate);
      searchVolumeEl.textContent = "搜索 " + metricNumber(search.total_queries);
      writeVolumeEl.textContent = "写入 " + metricNumber(write.total_requests);
      keyVolumeEl.textContent = "Key 操作 " + metricNumber(key.keyed_operations);
      modeFileEl.textContent = "file " + metricNumber(key.file_mode_operations);
      modeVectorEl.textContent = "vector " + metricNumber(key.vector_mode_operations);
      modeAllEl.textContent = "all " + metricNumber(key.all_mode_operations);
      sourceVolumeEl.textContent = "Source " + metricNumber(v24.source_operations);
      contextVolumeEl.textContent = "Context " + metricNumber(v24.context_operations);
      docsVolumeEl.textContent = "Docs " + metricNumber(v24.docs_operations);
      docsConflictsEl.textContent = "Conflicts " + metricNumber(v24.docs_conflicts);

      metricsStatusEl.classList.add("hidden");
      metricsPanelEl.classList.remove("hidden");
    }

    function render() {
      renderOwnershipFilter();
      renderAgentFilters();
      renderScopeFilters();
      renderList();
      renderMetrics();
      renderChat();
    }

    async function loadWorkspace() {
      listStatusEl.textContent = "正在加载 memory 列表...";
      listStatusEl.classList.remove("hidden");
      memoryListEl.classList.add("hidden");
      memoryEmptyEl.classList.add("hidden");
      metricsStatusEl.textContent = "正在加载监控数据...";
      metricsStatusEl.classList.remove("hidden");
      metricsPanelEl.classList.add("hidden");

      try {
        const [workspace, metrics, keyMetrics] = await Promise.all([
          fetchJson("/api/v1/explorer/memories?limit=300"),
          fetchJson("/metrics"),
          fetchJson("/api/v1/metrics/keys"),
        ]);
        state.workspace = workspace;
        state.metrics = keyMetrics && keyMetrics.key ? keyMetrics : metrics;
        const scopeExists = (state.workspace.scope_groups || []).some(
          (item) => item.key === state.scopeFilter
        );
        if (!scopeExists) {
          state.scopeFilter = "all";
        }
        syncSelection();
        render();
      } catch (error) {
        state.workspace = { total_count: 0, agent_groups: [], scope_groups: [], memories: [] };
        state.metrics = null;
        state.selectedMemoryId = null;
        listStatusEl.textContent = "当前无法加载工作台：" + String(error);
        metricsStatusEl.textContent = "当前无法加载监控：" + String(error);
        render();
        listStatusEl.classList.remove("hidden");
        metricsStatusEl.classList.remove("hidden");
      }
    }

    async function submitChat(event) {
      event.preventDefault();
      const prompt = chatInputEl.value.trim();
      if (!prompt || state.chatLoading) {
        return;
      }

      state.messages.push({ role: "user", content: prompt, citations: [] });
      state.chatLoading = true;
      chatInputEl.value = "";
      renderChat();

      try {
        const memory = selectedMemory();
        const result = await fetchJson("/api/v1/assistant/chat", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({
            prompt,
            ownership: state.ownershipFilter === "all" ? null : state.ownershipFilter,
            agent: state.agentFilter === "all" ? null : state.agentFilter,
            scope_id: state.scopeFilter === "all" ? null : state.scopeFilter,
            memory_id: memory ? memory.memory_id : null,
            limit: 6,
          }),
        });
        state.messages.push({
          role: "assistant",
          content: result.answer,
          citations: result.citations || [],
        });
      } catch (error) {
        state.messages.push({
          role: "assistant",
          content: "当前无法生成回复：" + String(error),
          citations: [],
        });
      } finally {
        state.chatLoading = false;
        renderChat();
      }
    }

    async function loadProjection() {
      const rawKey = projectionKeyEl.value.trim();
      const sourceId = projectionSourceIdEl.value.trim();
      const documentId = projectionDocumentIdEl.value.trim();
      if (!rawKey || !sourceId || !documentId) {
        projectionStatusEl.textContent = "请先填写 raw key、source_id、document_id。";
        projectionStatusEl.classList.remove("hidden");
        projectionMetaEl.classList.add("hidden");
        projectionMetaEl.innerHTML = "";
        projectionOutputEl.classList.add("hidden");
        projectionOutputEl.textContent = "";
        return;
      }

      projectionStatusEl.textContent = "正在加载 projection...";
      projectionStatusEl.classList.remove("hidden");
      projectionMetaEl.classList.add("hidden");
      projectionMetaEl.innerHTML = "";
      projectionOutputEl.classList.add("hidden");
      projectionOutputEl.textContent = "";

      try {
        const payload = await fetchJson(
          "/api/v1/sources/" + encodeURIComponent(sourceId) + "/documents/" + encodeURIComponent(documentId) + "/projection",
          {
            headers: {
              "x-meat-memory-key": rawKey,
            },
          }
        );
        projectionStatusEl.textContent = "Projection: " + String(payload.projection_path || "");
        projectionCopyPathEl.dataset.value = String(payload.projection_path || "");
        projectionCopyMarkdownEl.dataset.value = String(payload.markdown || "");
        projectionMetaEl.innerHTML =
          '<div class="memory-expanded-title">' + escapeHtml(String(payload.document && payload.document.title || "")) + '</div>' +
          '<div class="memory-expanded-meta muted">' +
            escapeHtml(String(payload.document && payload.document.document_id || "")) +
            " · " +
            escapeHtml(String(payload.document && payload.document.scope_id || "")) +
          '</div>' +
          '<div class="memory-meta">' +
            '<span class="memory-meta-tag">source ' + escapeHtml(String(payload.document && payload.document.source_id || "")) + '</span>' +
            '<span class="memory-meta-tag">sync ' + escapeHtml(String(payload.document && payload.document.sync_state || "")) + '</span>' +
            '<span class="memory-meta-tag">conflict ' + escapeHtml(String(payload.document && payload.document.conflict_state || "")) + '</span>' +
          '</div>' +
          '<div class="memory-inline-tags">' +
            '<span class="memory-meta-tag">' + escapeHtml(String(payload.document && payload.document.canonical_uri || "")) + '</span>' +
            '<span class="memory-meta-tag">' + escapeHtml(String(payload.document && payload.document.local_path || "")) + '</span>' +
          '</div>';
        projectionMetaEl.classList.remove("hidden");
        projectionOutputEl.textContent = String(payload.markdown || "");
        projectionOutputEl.classList.remove("hidden");
      } catch (error) {
        projectionStatusEl.textContent = "加载 projection 失败：" + String(error);
        projectionCopyPathEl.dataset.value = "";
        projectionCopyMarkdownEl.dataset.value = "";
        projectionMetaEl.classList.add("hidden");
        projectionMetaEl.innerHTML = "";
        projectionOutputEl.classList.add("hidden");
        projectionOutputEl.textContent = "";
      }
    }

    async function loadProjectionSources() {
      const rawKey = projectionKeyEl.value.trim();
      if (!rawKey) {
        projectionStatusEl.textContent = "请先填写 raw key。";
        projectionStatusEl.classList.remove("hidden");
        return;
      }

      projectionStatusEl.textContent = "正在加载 source 列表...";
      projectionStatusEl.classList.remove("hidden");
      state.projectionSources = [];
      projectionSourceSelectEl.innerHTML = '<option value="">正在加载...</option>';

      try {
        const payload = await fetchJson("/api/v1/sources?limit=100", {
          headers: {
            "x-meat-memory-key": rawKey,
          },
        });
        const sources = Array.isArray(payload) ? payload : [];
        state.projectionSources = sources;
        if (!sources.length) {
          projectionSourceSelectEl.innerHTML = '<option value="">暂无 source</option>';
          projectionStatusEl.textContent = "当前 key 下暂无 source。";
          return;
        }
        renderProjectionSources();
        if (projectionSourceIdEl.value) {
          projectionSourceSelectEl.value = projectionSourceIdEl.value;
        }
        projectionStatusEl.textContent = "已载入 " + sources.length + " 个 source。";
      } catch (error) {
        state.projectionSources = [];
        projectionSourceSelectEl.innerHTML = '<option value="">加载失败</option>';
        projectionStatusEl.textContent = "加载 source 列表失败：" + String(error);
      }
    }

    async function loadProjectionDocuments() {
      const rawKey = projectionKeyEl.value.trim();
      const sourceId = projectionSourceIdEl.value.trim();
      if (!rawKey || !sourceId) {
        projectionStatusEl.textContent = "请先填写 raw key 和 source_id。";
        projectionStatusEl.classList.remove("hidden");
        return;
      }

      projectionStatusEl.textContent = "正在加载该 source 的文档列表...";
      projectionStatusEl.classList.remove("hidden");
      state.projectionDocuments = [];
      projectionDocumentSelectEl.innerHTML = '<option value="">正在加载...</option>';
      projectionDocActionsEl.innerHTML = "";

      try {
        const payload = await fetchJson(
          "/api/v1/sources/" + encodeURIComponent(sourceId) + "/documents?limit=100",
          {
            headers: {
              "x-meat-memory-key": rawKey,
            },
          }
        );
        const documents = Array.isArray(payload) ? payload : [];
        state.projectionDocuments = documents;
        if (!documents.length) {
          projectionDocumentSelectEl.innerHTML = '<option value="">该 source 暂无文档</option>';
          projectionDocActionsEl.innerHTML = "";
          projectionStatusEl.textContent = "该 source 暂无 project documents。";
          return;
        }
        renderProjectionDocuments();
        if (projectionDocumentIdEl.value) {
          projectionDocumentSelectEl.value = projectionDocumentIdEl.value;
        }
        projectionStatusEl.textContent = "已载入 " + documents.length + " 个文档，可直接选择。";
      } catch (error) {
        state.projectionDocuments = [];
        projectionDocumentSelectEl.innerHTML = '<option value="">加载失败</option>';
        projectionDocActionsEl.innerHTML = "";
        projectionStatusEl.textContent = "加载文档列表失败：" + String(error);
      }
    }

    async function restoreProjectionDebugSession() {
      if (!projectionKeyEl.value.trim()) {
        return;
      }
      await loadProjectionSources();
      if (!projectionSourceIdEl.value.trim()) {
        return;
      }
      await loadProjectionDocuments();
      if (!projectionDocumentIdEl.value.trim()) {
        return;
      }
      await loadProjection();
    }

    async function copyProjectionValue(button, emptyMessage, successPrefix) {
      const value = String(button.dataset.value || "");
      if (!value) {
        projectionStatusEl.textContent = emptyMessage;
        projectionStatusEl.classList.remove("hidden");
        return;
      }
      try {
        await navigator.clipboard.writeText(value);
        projectionStatusEl.textContent = successPrefix;
      } catch (error) {
        projectionStatusEl.textContent = "复制失败：" + String(error);
      }
      projectionStatusEl.classList.remove("hidden");
    }

    function setConnectorStatus(message) {
      connectorStatusEl.textContent = message;
      connectorStatusEl.classList.remove("hidden");
    }

    function connectorScopeId() {
      return connectorScopeIdEl.value.trim() || metadata.default_scope || "scp_meat_memory_v1";
    }

    function connectorMaxItems() {
      const raw = connectorMaxItemsEl.value.trim();
      if (!raw) {
        return null;
      }
      const parsed = Number.parseInt(raw, 10);
      if (!Number.isFinite(parsed) || parsed <= 0) {
        throw new Error("max_items 必须是正整数。");
      }
      return parsed;
    }

    function connectorQueueItemIds() {
      return connectorQueueItemIdsEl.value
        .split(",")
        .map((item) => item.trim())
        .filter(Boolean);
    }

    function requireConnectorRootPath() {
      const rootPath = connectorRootPathEl.value.trim();
      if (!rootPath) {
        throw new Error("请先填写 root_path。");
      }
      return rootPath;
    }

    function connectorQueryUrl(path, options) {
      const queueId = connectorQueueIdEl.value.trim();
      const allowQueueIdOnly = options && options.allowQueueIdOnly && queueId;
      const rootPath = allowQueueIdOnly ? connectorRootPathEl.value.trim() : requireConnectorRootPath();
      const maxItems = connectorMaxItems();
      const params = new URLSearchParams();
      if (options && options.includeQueueId && queueId) {
        params.set("queue_id", queueId);
      }
      if (rootPath) {
        params.set("connector", connectorNameEl.value || "chat-export");
        params.set("root_path", rootPath);
      }
      if (options && options.includeScope) {
        params.set("scope_id", connectorScopeId());
      }
      if (options && options.persist) {
        params.set("persist", "true");
      }
      if (options && options.includeReview) {
        const queueItemIds = connectorQueueItemIds();
        const confirmationToken = connectorConfirmationTokenEl.value.trim();
        if (!queueItemIds.length || !confirmationToken) {
          throw new Error("请先填写 approved_queue_item_ids 和 confirmation_token。");
        }
        params.set("approved_queue_item_ids", queueItemIds.join(","));
        params.set("confirmation_token", confirmationToken);
      }
      if (maxItems) {
        params.set("max_items", String(maxItems));
      }
      if (connectorAllowRemoteFetchEl.checked) {
        params.set("allow_remote_fetch", "true");
      }
      return path + "?" + params.toString();
    }

    function renderConnectorReport(payload) {
      state.connectorReport = payload;
      if (payload.queue_id) {
        connectorQueueIdEl.value = String(payload.queue_id);
        saveConnectorDebugState();
      }

      const queueItems = Array.isArray(payload.queue_items) ? payload.queue_items : [];
      if (queueItems.length) {
        const first = queueItems[0] || {};
        if (!connectorQueueItemIdsEl.value.trim() && first.queue_item_id) {
          connectorQueueItemIdsEl.value = String(first.queue_item_id);
        }
        if (!connectorConfirmationTokenEl.value.trim() && first.review_token) {
          connectorConfirmationTokenEl.value = String(first.review_token);
        }
        saveConnectorDebugState();
      }

      const execution = payload.execution || {};
      const tags = [
        ["schema", payload.schema_version],
        ["connector", payload.connector],
        ["mode", payload.mode],
        ["queue", payload.queue_item_count],
        ["selected", payload.selected_count],
        ["applicable", payload.applicable_count],
        ["applied", execution.applied_memory_count || execution.applied_document_count],
      ].filter((item) => item[1] !== undefined && item[1] !== null && item[1] !== "");

      connectorMetaEl.innerHTML =
        '<div class="memory-expanded-title">Connector Report</div>' +
        '<div class="memory-meta">' +
        tags
          .map((item) => {
            return '<span class="memory-meta-tag">' +
              escapeHtml(item[0] + " " + String(item[1])) +
              "</span>";
          })
          .join("") +
        "</div>";
      connectorMetaEl.classList.remove("hidden");
      connectorOutputEl.textContent = JSON.stringify(payload, null, 2);
      connectorOutputEl.classList.remove("hidden");
    }

    async function loadConnectorReport(label, path, options) {
      saveConnectorDebugState();
      setConnectorStatus("正在执行 " + label + "...");
      connectorMetaEl.classList.add("hidden");
      connectorMetaEl.innerHTML = "";
      connectorOutputEl.classList.add("hidden");
      connectorOutputEl.textContent = "";

      try {
        const payload = await fetchJson(connectorQueryUrl(path, options));
        renderConnectorReport(payload);
        setConnectorStatus(label + " 已完成。");
      } catch (error) {
        setConnectorStatus(label + " 失败：" + String(error));
      }
    }

    async function loadStoredConnectorQueue() {
      saveConnectorDebugState();
      setConnectorStatus("正在加载 Persisted Queue...");
      connectorMetaEl.classList.add("hidden");
      connectorMetaEl.innerHTML = "";
      connectorOutputEl.classList.add("hidden");
      connectorOutputEl.textContent = "";

      try {
        const queueId = connectorQueueIdEl.value.trim();
        if (!queueId) {
          throw new Error("请先填写 queue_id。");
        }
        const payload = await fetchJson(
          "/api/v1/compat/connectors/proposal-queue/" + encodeURIComponent(queueId)
        );
        renderConnectorReport(payload);
        setConnectorStatus("Persisted Queue 已加载。");
      } catch (error) {
        setConnectorStatus("Persisted Queue 加载失败：" + String(error));
      }
    }

    async function applyConnectorReport() {
      saveConnectorDebugState();
      setConnectorStatus("正在执行 Confirmed Apply...");
      connectorMetaEl.classList.add("hidden");
      connectorMetaEl.innerHTML = "";
      connectorOutputEl.classList.add("hidden");
      connectorOutputEl.textContent = "";

      try {
        const rawKey = connectorKeyEl.value.trim();
        const queueId = connectorQueueIdEl.value.trim();
        const rootPath = queueId ? connectorRootPathEl.value.trim() : requireConnectorRootPath();
        const maxItems = connectorMaxItems();
        const queueItemIds = connectorQueueItemIds();
        const confirmationToken = connectorConfirmationTokenEl.value.trim();
        if (!rawKey) {
          throw new Error("请先填写 raw key。");
        }
        if (!queueItemIds.length || !confirmationToken) {
          throw new Error("请先填写 approved_queue_item_ids 和 confirmation_token。");
        }

        const body = {
          approved_queue_item_ids: queueItemIds,
          confirmation_token: confirmationToken,
        };
        if (queueId) {
          body.queue_id = queueId;
        }
        if (rootPath) {
          body.connector = connectorNameEl.value || "chat-export";
          body.root_path = rootPath;
        }
        if (!queueId || connectorScopeIdEl.value.trim()) {
          body.scope_id = connectorScopeId();
        }
        const sourceId = connectorSourceIdEl.value.trim();
        if (sourceId) {
          body.source_id = sourceId;
        }
        if (maxItems) {
          body.max_items = maxItems;
        }

        const payload = await fetchJson("/api/v1/compat/connectors/proposal-apply-plan/apply", {
          method: "POST",
          headers: {
            "content-type": "application/json",
            "x-meat-memory-key": rawKey,
          },
          body: JSON.stringify(body),
        });
        renderConnectorReport(payload);
        setConnectorStatus("Confirmed Apply 已完成。");
        loadWorkspace();
      } catch (error) {
        setConnectorStatus("Confirmed Apply 失败：" + String(error));
      }
    }

    function setBenchmarkStatus(message) {
      benchmarkStatusEl.textContent = message;
      benchmarkStatusEl.classList.remove("hidden");
    }

    function renderBenchmarkReport(label, payload) {
      state.benchmarkReport = payload;
      const metrics = payload.metrics || {};
      const run = payload.run || {};
      const suite = payload.suite || (metrics && metrics.suite) || {};
      const tags = [
        ["operation", label],
        ["suite", suite.name],
        ["run", run.run_id || run.id],
        ["cases", metrics.case_count],
        ["recall@1", metrics.recall_at_1],
        ["recall@5", metrics.recall_at_5],
        ["failures", payload.failure_count || metrics.failure_count],
        ["runs", payload.run_count],
      ].filter((item) => item[1] !== undefined && item[1] !== null && item[1] !== "");

      benchmarkMetaEl.innerHTML =
        '<div class="memory-expanded-title">Benchmark Report</div>' +
        '<div class="memory-meta">' +
        tags
          .map((item) => {
            return '<span class="memory-meta-tag">' +
              escapeHtml(item[0] + " " + String(item[1])) +
              "</span>";
          })
          .join("") +
        "</div>";
      benchmarkMetaEl.classList.remove("hidden");
      benchmarkOutputEl.textContent = JSON.stringify(payload, null, 2);
      benchmarkOutputEl.classList.remove("hidden");
    }

    async function runBenchmark() {
      saveBenchmarkDebugState();
      setBenchmarkStatus("正在运行 Benchmark...");
      benchmarkMetaEl.classList.add("hidden");
      benchmarkMetaEl.innerHTML = "";
      benchmarkOutputEl.classList.add("hidden");
      benchmarkOutputEl.textContent = "";

      try {
        const outputDir = benchmarkOutputDirEl.value.trim() || "tests/reports/benchmark/latest";
        const payload = await fetchJson("/api/v1/benchmark/run", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({
            suite: benchmarkSuiteEl.value || "meat-code-zh",
            scope_id: benchmarkScopeIdEl.value.trim() || metadata.default_scope,
            output_dir: outputDir,
          }),
        });
        benchmarkInputDirEl.value = outputDir;
        saveBenchmarkDebugState();
        renderBenchmarkReport("run", payload);
        setBenchmarkStatus("Benchmark run 已完成。");
      } catch (error) {
        setBenchmarkStatus("Benchmark run 失败：" + String(error));
      }
    }

    async function loadBenchmarkReport() {
      saveBenchmarkDebugState();
      setBenchmarkStatus("正在加载 Benchmark Report...");
      benchmarkMetaEl.classList.add("hidden");
      benchmarkMetaEl.innerHTML = "";
      benchmarkOutputEl.classList.add("hidden");
      benchmarkOutputEl.textContent = "";

      try {
        const inputDir = benchmarkInputDirEl.value.trim();
        if (!inputDir) {
          throw new Error("请先填写 input_dir。");
        }
        const params = new URLSearchParams();
        params.set("input_dir", inputDir);
        const payload = await fetchJson("/api/v1/benchmark/report?" + params.toString());
        renderBenchmarkReport("report", payload);
        setBenchmarkStatus("Benchmark report 已加载。");
      } catch (error) {
        setBenchmarkStatus("Benchmark report 加载失败：" + String(error));
      }
    }

    async function loadBenchmarkFailures() {
      saveBenchmarkDebugState();
      setBenchmarkStatus("正在加载 Benchmark Failures...");
      benchmarkMetaEl.classList.add("hidden");
      benchmarkMetaEl.innerHTML = "";
      benchmarkOutputEl.classList.add("hidden");
      benchmarkOutputEl.textContent = "";

      try {
        const inputDir = benchmarkInputDirEl.value.trim();
        if (!inputDir) {
          throw new Error("请先填写 input_dir。");
        }
        const params = new URLSearchParams();
        params.set("input_dir", inputDir);
        const payload = await fetchJson("/api/v1/benchmark/failures?" + params.toString());
        renderBenchmarkReport("failures", payload);
        setBenchmarkStatus("Benchmark failures 已加载。");
      } catch (error) {
        setBenchmarkStatus("Benchmark failures 加载失败：" + String(error));
      }
    }

    async function loadBenchmarkRuns() {
      saveBenchmarkDebugState();
      setBenchmarkStatus("正在加载 Benchmark Runs...");
      benchmarkMetaEl.classList.add("hidden");
      benchmarkMetaEl.innerHTML = "";
      benchmarkOutputEl.classList.add("hidden");
      benchmarkOutputEl.textContent = "";

      try {
        const inputDir = benchmarkInputDirEl.value.trim() || "tests/reports/benchmark";
        const params = new URLSearchParams();
        params.set("input_dir", inputDir);
        const payload = await fetchJson("/api/v1/benchmark/runs?" + params.toString());
        renderBenchmarkReport("runs", payload);
        setBenchmarkStatus("Benchmark runs 已加载。");
      } catch (error) {
        setBenchmarkStatus("Benchmark runs 加载失败：" + String(error));
      }
    }

    function setTraceStatus(message) {
      traceStatusEl.textContent = message;
      traceStatusEl.classList.remove("hidden");
    }

    function positiveIntegerFromInput(element, label) {
      const raw = element.value.trim();
      if (!raw) {
        return null;
      }
      const parsed = Number.parseInt(raw, 10);
      if (!Number.isFinite(parsed) || parsed <= 0) {
        throw new Error(label + " 必须是正整数。");
      }
      return parsed;
    }

    function renderTraceReport(label, payload) {
      state.traceReport = payload;
      const trace = payload.trace || {};
      const budget = payload.budget_pack || {};
      const tags = [
        ["operation", label],
        ["trace", trace.id],
        ["query", trace.query],
        ["records", budget.used_records],
        ["chars", budget.used_chars],
        ["trimmed", budget.trimmed_items],
      ].filter((item) => item[1] !== undefined && item[1] !== null && item[1] !== "");

      traceMetaEl.innerHTML =
        '<div class="memory-expanded-title">Recall Trace</div>' +
        '<div class="memory-meta">' +
        tags
          .map((item) => {
            return '<span class="memory-meta-tag">' +
              escapeHtml(item[0] + " " + String(item[1])) +
              "</span>";
          })
          .join("") +
        "</div>";
      traceMetaEl.classList.remove("hidden");
      traceOutputEl.textContent = JSON.stringify(payload, null, 2);
      traceOutputEl.classList.remove("hidden");
    }

    async function generateTrace() {
      saveTraceDebugState();
      setTraceStatus("正在生成 Recall Trace...");
      traceMetaEl.classList.add("hidden");
      traceMetaEl.innerHTML = "";
      traceOutputEl.classList.add("hidden");
      traceOutputEl.textContent = "";

      try {
        const query = traceQueryEl.value.trim();
        if (!query) {
          throw new Error("请先填写 query。");
        }
        const outputDir = traceOutputDirEl.value.trim() || "tests/reports/trace/latest";
        const body = {
          scope_id: traceScopeIdEl.value.trim() || metadata.default_scope,
          query,
          output_dir: outputDir,
          debug_candidates: traceDebugCandidatesEl.checked,
        };
        const maxRecords = positiveIntegerFromInput(traceMaxRecordsEl, "max_records");
        const maxChars = positiveIntegerFromInput(traceMaxCharsEl, "max_chars");
        if (maxRecords) {
          body.max_records = maxRecords;
        }
        if (maxChars) {
          body.max_chars = maxChars;
        }
        const payload = await fetchJson("/api/v1/recall/traces/latest", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(body),
        });
        if (payload.trace && payload.trace.id) {
          traceIdEl.value = String(payload.trace.id);
        }
        traceInputDirEl.value = outputDir;
        saveTraceDebugState();
        renderTraceReport("latest", payload);
        setTraceStatus("Recall Trace 已生成。");
      } catch (error) {
        setTraceStatus("Recall Trace 生成失败：" + String(error));
      }
    }

    async function inspectTrace() {
      saveTraceDebugState();
      setTraceStatus("正在读取 Recall Trace...");
      traceMetaEl.classList.add("hidden");
      traceMetaEl.innerHTML = "";
      traceOutputEl.classList.add("hidden");
      traceOutputEl.textContent = "";

      try {
        const inputDir = traceInputDirEl.value.trim();
        if (!inputDir) {
          throw new Error("请先填写 input_dir。");
        }
        const params = new URLSearchParams();
        params.set("input_dir", inputDir);
        const traceId = traceIdEl.value.trim();
        if (traceId) {
          params.set("trace_id", traceId);
        }
        const payload = await fetchJson("/api/v1/recall/traces/inspect?" + params.toString());
        renderTraceReport("inspect", payload.trace || payload);
        setTraceStatus("Recall Trace 已读取。");
      } catch (error) {
        setTraceStatus("Recall Trace 读取失败：" + String(error));
      }
    }

    function setHealthStatus(message) {
      healthStatusEl.textContent = message;
      healthStatusEl.classList.remove("hidden");
    }

    function healthLimit() {
      const raw = healthLimitEl.value.trim();
      if (!raw) {
        return null;
      }
      const parsed = Number.parseInt(raw, 10);
      if (!Number.isFinite(parsed) || parsed <= 0) {
        throw new Error("limit 必须是正整数。");
      }
      return parsed;
    }

    function renderHealthReport(payload) {
      state.healthReport = payload;
      const tags = [
        ["scope", payload.scope_id],
        ["total", payload.total],
        ["active", payload.active],
        ["risks", Array.isArray(payload.risks) ? payload.risks.length : undefined],
        ["secret", payload.secret_findings],
        ["high-risk", payload.high_risk_secret_findings],
        ["review", payload.needs_review],
      ].filter((item) => item[1] !== undefined && item[1] !== null && item[1] !== "");

      healthMetaEl.innerHTML =
        '<div class="memory-expanded-title">Health Report</div>' +
        '<div class="memory-meta">' +
        tags
          .map((item) => {
            return '<span class="memory-meta-tag">' +
              escapeHtml(item[0] + " " + String(item[1])) +
              "</span>";
          })
          .join("") +
        "</div>";
      healthMetaEl.classList.remove("hidden");
      healthOutputEl.textContent = JSON.stringify(payload, null, 2);
      healthOutputEl.classList.remove("hidden");
    }

    async function loadHealthReport() {
      saveHealthDebugState();
      setHealthStatus("正在加载 Health Report...");
      healthMetaEl.classList.add("hidden");
      healthMetaEl.innerHTML = "";
      healthOutputEl.classList.add("hidden");
      healthOutputEl.textContent = "";

      try {
        const params = new URLSearchParams();
        const scopeId = healthScopeIdEl.value.trim();
        if (scopeId) {
          params.set("scope_id", scopeId);
        }
        const limit = healthLimit();
        if (limit) {
          params.set("limit", String(limit));
        }
        const payload = await fetchJson("/api/v1/health/report?" + params.toString());
        renderHealthReport(payload);
        const riskCount = Array.isArray(payload.risks) ? payload.risks.length : 0;
        setHealthStatus("Health Report 已加载，风险项 " + riskCount + " 个。");
      } catch (error) {
        setHealthStatus("Health Report 加载失败：" + String(error));
      }
    }

    function setPassportStatus(message) {
      passportStatusEl.textContent = message;
      passportStatusEl.classList.remove("hidden");
    }

    function passportLimit() {
      const raw = passportLimitEl.value.trim();
      if (!raw) {
        return null;
      }
      const parsed = Number.parseInt(raw, 10);
      if (!Number.isFinite(parsed) || parsed <= 0) {
        throw new Error("limit 必须是正整数。");
      }
      return parsed;
    }

    function renderPassportReport(label, payload) {
      state.passportReport = payload;
      const manifest = payload.manifest || {};
      const tags = [
        ["operation", label],
        ["scope", manifest.source_scope_id],
        ["target", payload.target_scope_id],
        ["valid", payload.valid],
        ["verified", payload.verified],
        ["objects", manifest.object_count],
        ["memories", Array.isArray(payload.memories) ? payload.memories.length : undefined],
        ["imported", payload.imported_count],
        ["skipped", payload.skipped_count],
      ].filter((item) => item[1] !== undefined && item[1] !== null && item[1] !== "");

      passportMetaEl.innerHTML =
        '<div class="memory-expanded-title">Passport Report</div>' +
        '<div class="memory-meta">' +
        tags
          .map((item) => {
            return '<span class="memory-meta-tag">' +
              escapeHtml(item[0] + " " + String(item[1])) +
              "</span>";
          })
          .join("") +
        "</div>";
      passportMetaEl.classList.remove("hidden");
      passportOutputEl.textContent = JSON.stringify(payload, null, 2);
      passportOutputEl.classList.remove("hidden");
    }

    async function exportPassport() {
      savePassportDebugState();
      setPassportStatus("正在导出 Passport...");
      passportMetaEl.classList.add("hidden");
      passportMetaEl.innerHTML = "";
      passportOutputEl.classList.add("hidden");
      passportOutputEl.textContent = "";

      try {
        const outputDir = passportOutputDirEl.value.trim() || "tests/reports/passport/latest";
        const body = {
          scope_id: passportScopeIdEl.value.trim() || metadata.default_scope,
          output_dir: outputDir,
          redact_sensitive: passportRedactSensitiveEl.checked,
        };
        const limit = passportLimit();
        if (limit) {
          body.limit = limit;
        }
        const payload = await fetchJson("/api/v1/passports/export", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(body),
        });
        passportInputDirEl.value = outputDir;
        savePassportDebugState();
        renderPassportReport("export", payload);
        setPassportStatus("Passport export 已完成。");
      } catch (error) {
        setPassportStatus("Passport export 失败：" + String(error));
      }
    }

    async function verifyPassport() {
      savePassportDebugState();
      setPassportStatus("正在校验 Passport manifest...");
      passportMetaEl.classList.add("hidden");
      passportMetaEl.innerHTML = "";
      passportOutputEl.classList.add("hidden");
      passportOutputEl.textContent = "";

      try {
        const inputDir = passportInputDirEl.value.trim();
        if (!inputDir) {
          throw new Error("请先填写 input_dir。");
        }
        const params = new URLSearchParams();
        params.set("input_dir", inputDir);
        const payload = await fetchJson("/api/v1/passports/manifest?" + params.toString());
        renderPassportReport("manifest", payload);
        setPassportStatus(payload.valid ? "Passport manifest 校验通过。" : "Passport manifest 校验未通过。");
      } catch (error) {
        setPassportStatus("Passport manifest 校验失败：" + String(error));
      }
    }

    async function importPassport() {
      savePassportDebugState();
      setPassportStatus("正在导入 Passport...");
      passportMetaEl.classList.add("hidden");
      passportMetaEl.innerHTML = "";
      passportOutputEl.classList.add("hidden");
      passportOutputEl.textContent = "";

      try {
        const inputDir = passportInputDirEl.value.trim();
        if (!inputDir) {
          throw new Error("请先填写 input_dir。");
        }
        const body = {
          input_dir: inputDir,
          dry_run: passportDryRunEl.checked,
        };
        const targetScopeId = passportTargetScopeIdEl.value.trim();
        if (targetScopeId) {
          body.target_scope_id = targetScopeId;
        }
        const payload = await fetchJson("/api/v1/passports/import", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(body),
        });
        renderPassportReport("import", payload);
        setPassportStatus(passportDryRunEl.checked ? "Passport import dry-run 已完成。" : "Passport import 已完成。");
        if (!passportDryRunEl.checked) {
          loadWorkspace();
        }
      } catch (error) {
        setPassportStatus("Passport import 失败：" + String(error));
      }
    }

    document.querySelectorAll("#ownership-filter button").forEach((button) => {
      button.addEventListener("click", () => {
        state.ownershipFilter = button.dataset.value || "all";
        syncSelection();
        render();
      });
    });

    document.getElementById("reset-agent").addEventListener("click", () => {
      state.agentFilter = "all";
      syncSelection();
      render();
    });

    document.getElementById("reset-scope").addEventListener("click", () => {
      state.scopeFilter = "all";
      syncSelection();
      render();
    });

    searchInputEl.addEventListener("input", (event) => {
      state.searchText = event.target.value || "";
      syncSelection();
      render();
    });

    projectionKeyEl.addEventListener("input", saveProjectionDebugState);
    projectionSourceIdEl.addEventListener("input", saveProjectionDebugState);
    projectionDocumentIdEl.addEventListener("input", saveProjectionDebugState);
    [
      connectorKeyEl,
      connectorNameEl,
      connectorRootPathEl,
      connectorScopeIdEl,
      connectorSourceIdEl,
      connectorMaxItemsEl,
      connectorAllowRemoteFetchEl,
      connectorQueueItemIdsEl,
      connectorConfirmationTokenEl,
      connectorQueueIdEl,
    ].forEach((element) => {
      element.addEventListener("input", saveConnectorDebugState);
      element.addEventListener("change", saveConnectorDebugState);
    });
    [
      benchmarkSuiteEl,
      benchmarkScopeIdEl,
      benchmarkOutputDirEl,
      benchmarkInputDirEl,
    ].forEach((element) => {
      element.addEventListener("input", saveBenchmarkDebugState);
      element.addEventListener("change", saveBenchmarkDebugState);
    });
    [
      traceScopeIdEl,
      traceQueryEl,
      traceOutputDirEl,
      traceInputDirEl,
      traceIdEl,
      traceMaxRecordsEl,
      traceMaxCharsEl,
      traceDebugCandidatesEl,
    ].forEach((element) => {
      element.addEventListener("input", saveTraceDebugState);
      element.addEventListener("change", saveTraceDebugState);
    });
    [
      healthScopeIdEl,
      healthLimitEl,
    ].forEach((element) => {
      element.addEventListener("input", saveHealthDebugState);
      element.addEventListener("change", saveHealthDebugState);
    });
    [
      passportScopeIdEl,
      passportOutputDirEl,
      passportLimitEl,
      passportInputDirEl,
      passportTargetScopeIdEl,
      passportRedactSensitiveEl,
      passportDryRunEl,
    ].forEach((element) => {
      element.addEventListener("input", savePassportDebugState);
      element.addEventListener("change", savePassportDebugState);
    });

    refreshButtonEl.addEventListener("click", loadWorkspace);
    chatFormEl.addEventListener("submit", submitChat);
    projectionLoadSourcesEl.addEventListener("click", loadProjectionSources);
    projectionLoadDocsEl.addEventListener("click", loadProjectionDocuments);
    projectionSourceSearchEl.addEventListener("input", (event) => {
      state.projectionSourceSearch = event.target.value || "";
      saveProjectionDebugState();
      renderProjectionSources();
    });
    projectionDocumentSearchEl.addEventListener("input", (event) => {
      state.projectionDocumentSearch = event.target.value || "";
      saveProjectionDebugState();
      renderProjectionDocuments();
    });
    projectionFetchEl.addEventListener("click", loadProjection);
    projectionSourceSelectEl.addEventListener("change", () => {
      projectionSourceIdEl.value = projectionSourceSelectEl.value || "";
      state.projectionDocuments = [];
      projectionDocumentSelectEl.innerHTML = '<option value="">先载入该 source 的文档列表</option>';
      projectionDocActionsEl.innerHTML = "";
      projectionDocumentIdEl.value = "";
      saveProjectionDebugState();
    });
    projectionDocumentSelectEl.addEventListener("change", () => {
      projectionDocumentIdEl.value = projectionDocumentSelectEl.value || "";
      saveProjectionDebugState();
    });
    projectionCopyPathEl.addEventListener("click", () => {
      copyProjectionValue(projectionCopyPathEl, "当前没有可复制的 projection 路径。", "已复制 projection 路径。");
    });
    projectionCopyMarkdownEl.addEventListener("click", () => {
      copyProjectionValue(projectionCopyMarkdownEl, "当前没有可复制的 Markdown 内容。", "已复制 Markdown 内容。");
    });
    connectorDryRunEl.addEventListener("click", () => {
      loadConnectorReport("Dry Run", "/api/v1/compat/connectors/dry-run", { includeScope: false, includeReview: false });
    });
    connectorSyncPlanEl.addEventListener("click", () => {
      loadConnectorReport("Sync Plan", "/api/v1/compat/connectors/sync-plan", { includeScope: true, includeReview: false });
    });
    connectorProposalQueueEl.addEventListener("click", () => {
      loadConnectorReport("Proposal Queue", "/api/v1/compat/connectors/proposal-queue", { includeScope: true, includeReview: false });
    });
    connectorPersistQueueEl.addEventListener("click", () => {
      loadConnectorReport("Persist Queue", "/api/v1/compat/connectors/proposal-queue", { includeScope: true, includeReview: false, persist: true });
    });
    connectorLoadQueueEl.addEventListener("click", loadStoredConnectorQueue);
    connectorApplyPlanEl.addEventListener("click", () => {
      loadConnectorReport("Apply Plan", "/api/v1/compat/connectors/proposal-apply-plan", { includeScope: true, includeReview: true, includeQueueId: true, allowQueueIdOnly: true });
    });
    connectorApplyConfirmEl.addEventListener("click", applyConnectorReport);
    benchmarkRunEl.addEventListener("click", runBenchmark);
    benchmarkReportEl.addEventListener("click", loadBenchmarkReport);
    benchmarkFailuresEl.addEventListener("click", loadBenchmarkFailures);
    benchmarkRunsEl.addEventListener("click", loadBenchmarkRuns);
    traceLatestEl.addEventListener("click", generateTrace);
    traceInspectEl.addEventListener("click", inspectTrace);
    healthLoadEl.addEventListener("click", loadHealthReport);
    passportExportEl.addEventListener("click", exportPassport);
    passportVerifyEl.addEventListener("click", verifyPassport);
    passportImportEl.addEventListener("click", importPassport);

    loadProjectionDebugState();
    loadConnectorDebugState();
    loadBenchmarkDebugState();
    loadTraceDebugState();
    loadHealthDebugState();
    loadPassportDebugState();
    render();
    loadWorkspace();
    restoreProjectionDebugSession();
  </script>
</body>
</html>
"##;

    template
        .replace("__SERVICE__", &service)
        .replace("__VERSION__", &version)
        .replace("__DEFAULT_SCOPE__", &default_scope)
        .replace("__METADATA__", &metadata_json)
}

fn escape_html(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;
