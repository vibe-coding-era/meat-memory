use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, patch, post},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use memory_domain::{
    AccessKeyId, AccessKeyStatus, ArtifactKind, KeyScopeKind, KeySourceKind, Memory, MemoryId,
    MemoryKind, RequestContext, ScopeId, ScopeType, Sensitivity, StorageMode, Visibility,
};
use memory_kernel::{
    CreateAccessKeyRequest, Kernel, PromoteMemoryRequest, RememberImageRequest,
    RememberTextRequest, SearchContextRequest, UpdateAccessKeyRequest,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::BTreeMap, sync::Arc};
use time::format_description::well_known::Rfc3339;

pub const HTTP_ROUTES: &[&str] = &[
    "/",
    "/healthz",
    "/readyz",
    "/livez",
    "/metrics",
    "/api/v1/meta",
    "/api/v1/memories",
    "/api/v1/memories/promote",
    "/api/v1/images",
    "/api/v1/context",
    "/api/v1/context/search",
    "/api/v1/explorer/memories",
    "/api/v1/assistant/chat",
    "/api/v1/keys",
    "/api/v1/keys/{key_id}",
    "/api/v1/keys/{key_id}/rotate",
    "/api/v1/keys/{key_id}/stats",
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
}

impl HttpAppState {
    pub fn new(default_scope_id: ScopeId, metadata: ApiMetadata, kernel: Arc<Kernel>) -> Self {
        Self {
            default_scope_id,
            require_key: metadata.features.require_key,
            metadata,
            kernel,
        }
    }
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
        .route("/api/v1/images", post(create_image))
        .route("/api/v1/context", post(search_context))
        .route("/api/v1/context/search", post(search_context))
        .route("/api/v1/explorer/memories", get(browse_memories))
        .route("/api/v1/assistant/chat", post(chat_with_memory_assistant))
        .route(
            "/api/v1/keys",
            post(create_access_key).get(list_access_keys),
        )
        .route("/api/v1/keys/{key_id}", patch(update_access_key))
        .route("/api/v1/keys/{key_id}/rotate", post(rotate_access_key))
        .route("/api/v1/keys/{key_id}/stats", get(access_key_stats))
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
) -> Result<Json<Vec<AccessKeyResponse>>, ApiError> {
    let keys = state
        .kernel
        .list_access_keys(200)
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
    Path(key_id): Path<String>,
    Json(payload): Json<UpdateAccessKeyHttpRequest>,
) -> Result<Json<AccessKeyResponse>, ApiError> {
    let updated = state
        .kernel
        .update_access_key(UpdateAccessKeyRequest {
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
        })
        .await
        .map_err(api_error_from_anyhow)?;
    Ok(Json(access_key_response(updated, None)))
}

async fn rotate_access_key(
    State(state): State<HttpAppState>,
    Path(key_id): Path<String>,
) -> Result<(StatusCode, Json<AccessKeyResponse>), ApiError> {
    let result = state
        .kernel
        .rotate_access_key(&AccessKeyId::from_string(key_id), None)
        .await
        .map_err(api_error_from_anyhow)?;
    Ok((
        StatusCode::CREATED,
        Json(access_key_response(result.access_key, Some(result.raw_key))),
    ))
}

async fn access_key_stats(
    State(state): State<HttpAppState>,
    Path(key_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let stats = state
        .kernel
        .access_key_usage_stats(&AccessKeyId::from_string(key_id))
        .await
        .map_err(api_error_from_anyhow)?
        .ok_or_else(|| ApiError::not_found("access key not found"))?;
    Ok(Json(json!({ "stats": stats })))
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
    Json(payload): Json<PromoteMemoryHttpRequest>,
) -> Result<(StatusCode, Json<PromoteMemoryHttpResponse>), ApiError> {
    let result = state
        .kernel
        .promote_memory_by_id(
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

async fn search_context(
    State(state): State<HttpAppState>,
    headers: HeaderMap,
    Json(payload): Json<SearchContextHttpRequest>,
) -> Result<Json<SearchContextHttpResponse>, ApiError> {
    let context = resolve_http_context(&state, &headers).await?;
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

async fn browse_memories(
    State(state): State<HttpAppState>,
    Query(query): Query<BrowseMemoriesQuery>,
) -> Result<Json<ExplorerMemoriesResponse>, ApiError> {
    let scope_id = query.scope_id.as_deref().map(ScopeId::from_string);
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
    Json(payload): Json<AssistantChatRequest>,
) -> Result<Json<AssistantChatResponse>, ApiError> {
    let requested_scope = payload.scope_id.as_deref().map(ScopeId::from_string);
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

fn access_key_response(
    access_key: memory_domain::AccessKey,
    raw_key: Option<String>,
) -> AccessKeyResponse {
    AccessKeyResponse {
        key_id: access_key.id.as_str().to_string(),
        raw_key,
        name: access_key.display_name,
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

fn format_timestamp(value: time::OffsetDateTime) -> String {
    value
        .format(&Rfc3339)
        .unwrap_or_else(|_| value.unix_timestamp().to_string())
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

fn api_error_from_anyhow(error: anyhow::Error) -> ApiError {
    let message = error.to_string();
    if message.contains("denied by policy") {
        return ApiError {
            status: StatusCode::FORBIDDEN,
            message,
        };
    }
    if message.contains("unsupported")
        || message.contains("unknown")
        || message.contains("empty")
        || message.contains("Invalid")
    {
        return ApiError::bad_request(message);
    }

    ApiError::internal(message)
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
            </div>
          </div>
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
      </div>
    </div>
  </div>

  <script>
    const metadata = __METADATA__;
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
    const chatStackEl = document.getElementById("chat-stack");
    const chatFormEl = document.getElementById("chat-form");
    const chatInputEl = document.getElementById("chat-input");
    const chatNoteEl = document.getElementById("chat-note");

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

    refreshButtonEl.addEventListener("click", loadWorkspace);
    chatFormEl.addEventListener("submit", submitChat);

    render();
    loadWorkspace();
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
mod tests {
    use super::{
        ApiError, ApiFeatureFlags, ApiMetadata, HttpAppState, api_error_from_anyhow,
        build_console_page, build_router, escape_html, has_route, memory_kind_label,
        memory_to_summary, parse_artifact_kind, parse_memory_kind, parse_sensitivity,
        parse_visibility,
    };
    use anyhow::anyhow;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
        response::IntoResponse,
    };
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use memory_domain::{
        ArtifactKind, Memory, MemoryKind, MemoryState, ScopeId, Sensitivity, Visibility,
    };
    use memory_kernel::Kernel;
    use memory_models::{
        CapabilityRoute, DeploymentTarget, ModelCapability, ModelDescriptor, ModelRegistry,
        Provider, ProviderDescriptor,
    };
    use std::collections::BTreeSet;
    use std::sync::Arc;
    use tempfile::tempdir;
    use tower::ServiceExt;

    async fn response_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn test_state(tempdir: &std::path::Path) -> HttpAppState {
        let kernel = Arc::new(
            Kernel::builder()
                .with_markdown_root(tempdir.join("markdown"))
                .unwrap()
                .with_asset_root(tempdir.join("assets"))
                .unwrap()
                .with_model_registry(test_model_registry(), "zh-CN")
                .unwrap()
                .build()
                .unwrap(),
        );

        HttpAppState::new(
            ScopeId::from_string("scp_http_default"),
            ApiMetadata {
                service: "meat-memory".to_string(),
                version: "0.1.0".to_string(),
                default_scope: "scp_http_default".to_string(),
                features: ApiFeatureFlags {
                    pg: false,
                    markdown: true,
                    http: true,
                    mcp: false,
                    require_key: false,
                },
            },
            kernel,
        )
    }

    fn failover_test_state(tempdir: &std::path::Path) -> HttpAppState {
        let kernel = Arc::new(
            Kernel::builder()
                .with_markdown_root(tempdir.join("markdown"))
                .unwrap()
                .with_asset_root(tempdir.join("assets"))
                .unwrap()
                .with_model_registry(failover_test_model_registry(), "zh-CN")
                .unwrap()
                .build()
                .unwrap(),
        );

        HttpAppState::new(
            ScopeId::from_string("scp_http_default"),
            ApiMetadata {
                service: "meat-memory".to_string(),
                version: "0.1.0".to_string(),
                default_scope: "scp_http_default".to_string(),
                features: ApiFeatureFlags {
                    pg: false,
                    markdown: true,
                    http: true,
                    mcp: false,
                    require_key: false,
                },
            },
            kernel,
        )
    }

    #[tokio::test]
    async fn exposes_http_routes() {
        assert!(has_route("/"));
        assert!(has_route("/healthz"));
        assert!(has_route("/livez"));
        assert!(has_route("/metrics"));
        assert!(has_route("/api/v1/metrics/keys"));
        assert!(has_route("/api/v1/context/search"));
        assert!(has_route("/api/v1/explorer/memories"));
        assert!(has_route("/api/v1/assistant/chat"));
        assert!(has_route("/api/v1/keys/{key_id}"));
        assert!(has_route("/api/v1/keys/{key_id}/rotate"));
        assert!(has_route("/api/v1/keys/{key_id}/stats"));
    }

    #[tokio::test]
    async fn serves_browser_console_at_root() {
        let tempdir = tempdir().unwrap();
        let app = build_router(test_state(tempdir.path()));

        let response = app
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("Browser Console"));
        assert!(text.contains("监控概览"));
        assert!(text.contains("AI 对话区"));
        assert!(text.contains("/api/v1/explorer/memories"));
        assert!(text.contains("/api/v1/assistant/chat"));
        assert!(text.contains("/api/v1/metrics/keys"));
    }

    #[tokio::test]
    async fn creates_memory_through_router() {
        let tempdir = tempdir().unwrap();
        let app = build_router(test_state(tempdir.path()));

        let response = app
            .oneshot(
                Request::post("/api/v1/memories")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"scope_id":"scp_http_scope","title":"Branch policy","body":"The default branch is main."}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::CREATED);
        assert!(
            tempdir
                .path()
                .join("markdown")
                .join("default")
                .join("scopes")
                .join("scp_http_scope")
                .join("MEMORY.md")
                .exists()
        );
    }

    #[tokio::test]
    async fn creates_image_through_router() {
        let tempdir = tempdir().unwrap();
        let app = build_router(test_state(tempdir.path()));
        let payload = format!(
            r#"{{"scope_id":"scp_http_image","title":"UI screenshot","body":"Search result page","media_type":"image/png","image_base64":"{}"}}"#,
            STANDARD.encode([137_u8, 80, 78, 71, 13, 10, 26, 10])
        );

        let response = app
            .oneshot(
                Request::post("/api/v1/images")
                    .header("content-type", "application/json")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::CREATED);
        assert!(tempdir.path().join("assets").join("raw").exists());
    }

    #[tokio::test]
    async fn creates_image_through_router_with_llm_failover_notice() {
        let tempdir = tempdir().unwrap();
        let app = build_router(failover_test_state(tempdir.path()));
        let payload = format!(
            r#"{{"scope_id":"scp_http_image","title":"UI screenshot","body":"Search result page","media_type":"image/png","image_base64":"{}"}}"#,
            STANDARD.encode([137_u8, 80, 78, 71, 13, 10, 26, 10])
        );

        let response = app
            .oneshot(
                Request::post("/api/v1/images")
                    .header("content-type", "application/json")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::CREATED);
        let body = response_json(response).await;
        assert_eq!(body["vision_model_alias"], "claude_vision");
        assert_eq!(
            body["llm_notice"],
            "Gemini Vision LLM 不可用，已经切换到Claude Vision"
        );
    }

    #[tokio::test]
    async fn returns_empty_search_result_without_postgres() {
        let tempdir = tempdir().unwrap();
        let app = build_router(test_state(tempdir.path()));

        let response = app
            .oneshot(
                Request::post("/api/v1/context/search")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"scope_id":"scp_http_scope","query":"branch","limit":5}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn explorer_endpoint_lists_memories_from_markdown_store() {
        let tempdir = tempdir().unwrap();
        let app = build_router(test_state(tempdir.path()));

        app.clone()
            .oneshot(
                Request::post("/api/v1/memories")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"scope_id":"scp_browser_codex","title":"Codex 发布步骤","body":"Codex 团队记录了浏览工作台的发布流程。","memory_kind":"procedure","visibility":"team"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::get("/api/v1/explorer/memories?limit=20")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = response_json(response).await;
        assert_eq!(body["total_count"], 1);
        assert_eq!(body["memories"][0]["title"], "Codex 发布步骤");
        assert_eq!(body["memories"][0]["ownership_key"], "team");
        assert_eq!(body["memories"][0]["agent_key"], "codex");
    }

    #[tokio::test]
    async fn assistant_chat_endpoint_summarizes_matching_memories() {
        let tempdir = tempdir().unwrap();
        let app = build_router(test_state(tempdir.path()));

        let created = app
            .clone()
            .oneshot(
                Request::post("/api/v1/memories")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"scope_id":"scp_browser_claude","title":"Claude Code 评审准则","body":"Claude Code 团队把评审准则整理成长期记忆，要求先列风险再列摘要。","memory_kind":"procedure","visibility":"team"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let created_payload = response_json(created).await;
        let memory_id = created_payload["memory_id"].as_str().unwrap();

        let response = app
            .oneshot(
                Request::post("/api/v1/assistant/chat")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"prompt":"评审准则","memory_id":"{memory_id}","ownership":"team","agent":"claude-code"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let payload = response_json(response).await;
        let answer = payload["answer"].as_str().unwrap();
        assert!(answer.contains("评审准则"));
        assert!(answer.contains("Claude Code"));
        assert_eq!(payload["matched_count"], 1);
        assert_eq!(payload["citations"][0]["memory_id"], memory_id);
    }

    #[tokio::test]
    async fn promote_memory_endpoint_creates_review_candidate_in_target_scope() {
        let tempdir = tempdir().unwrap();
        let app = build_router(test_state(tempdir.path()));

        let created = app
            .clone()
            .oneshot(
                Request::post("/api/v1/memories")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"scope_id":"scp_user_alice","title":"Alice 发布凭证","body":"token: abc123 联系人 alice@example.com","memory_kind":"procedure","visibility":"private","sensitivity":"restricted"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let created_payload = response_json(created).await;
        let memory_id = created_payload["memory_id"].as_str().unwrap();

        let promoted = app
            .oneshot(
                Request::post("/api/v1/memories/promote")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"source_scope_id":"scp_user_alice","memory_id":"{memory_id}","source_scope_type":"user","target_scope_id":"scp_project_demo","target_scope_type":"project","target_visibility":"project"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(promoted.status(), axum::http::StatusCode::CREATED);
        let payload = response_json(promoted).await;
        assert_eq!(payload["scope_id"], "scp_project_demo");
        assert_eq!(payload["owner_scope_id"], "scp_user_alice");
        assert_eq!(payload["published_from_scope_id"], "scp_user_alice");
        assert_eq!(payload["memory_state"], "candidate");
        assert!(payload["body"].as_str().unwrap().contains("[REDACTED]"));
    }

    #[tokio::test]
    async fn exposes_metrics_and_liveness_routes() {
        let tempdir = tempdir().unwrap();
        let app = build_router(test_state(tempdir.path()));

        let livez = app
            .clone()
            .oneshot(Request::get("/livez").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let metrics = app
            .clone()
            .oneshot(Request::get("/metrics").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let key_metrics = app
            .oneshot(
                Request::get("/api/v1/metrics/keys")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(livez.status(), axum::http::StatusCode::OK);
        assert_eq!(metrics.status(), axum::http::StatusCode::OK);
        assert_eq!(key_metrics.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn exposes_health_ready_and_meta_payloads() {
        let tempdir = tempdir().unwrap();
        let app = build_router(test_state(tempdir.path()));

        let healthz = response_json(
            app.clone()
                .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
                .await
                .unwrap(),
        )
        .await;
        let readyz = response_json(
            app.clone()
                .oneshot(Request::get("/readyz").body(Body::empty()).unwrap())
                .await
                .unwrap(),
        )
        .await;
        let meta = response_json(
            app.oneshot(Request::get("/api/v1/meta").body(Body::empty()).unwrap())
                .await
                .unwrap(),
        )
        .await;

        assert_eq!(healthz["status"], "ok");
        assert_eq!(readyz["status"], "ready");
        assert_eq!(readyz["stores"]["pg"], false);
        assert_eq!(readyz["stores"]["markdown"], true);
        assert_eq!(meta["service"], "meat-memory");
        assert_eq!(meta["version"], "0.1.0");
        assert_eq!(meta["default_scope"], "scp_http_default");
        assert_eq!(meta["features"]["http"], true);
        assert_eq!(meta["features"]["mcp"], false);
    }

    #[tokio::test]
    async fn search_context_uses_default_scope_and_limit_when_omitted() {
        let tempdir = tempdir().unwrap();
        let app = build_router(test_state(tempdir.path()));

        let response = app
            .oneshot(
                Request::post("/api/v1/context")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"query":"  Branch Memory  "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let payload = response_json(response).await;
        assert_eq!(payload["query"], "branch memory");
        assert_eq!(payload["scope_id"], "scp_http_default");
        assert_eq!(payload["memory_count"], 0);
        assert_eq!(payload["memories"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn rejects_invalid_payloads_and_policy_violations() {
        let tempdir = tempdir().unwrap();
        let app = build_router(test_state(tempdir.path()));
        let cases = [
            (
                "/api/v1/memories",
                r#"{"body":"hello","artifact_kind":"bogus"}"#,
                axum::http::StatusCode::BAD_REQUEST,
                "unsupported artifact_kind: bogus",
            ),
            (
                "/api/v1/memories",
                r#"{"body":"hello","memory_kind":"bogus"}"#,
                axum::http::StatusCode::BAD_REQUEST,
                "unsupported memory_kind: bogus",
            ),
            (
                "/api/v1/memories",
                r#"{"body":"hello","visibility":"secret"}"#,
                axum::http::StatusCode::BAD_REQUEST,
                "unsupported visibility: secret",
            ),
            (
                "/api/v1/memories",
                r#"{"body":"hello","sensitivity":"secret"}"#,
                axum::http::StatusCode::BAD_REQUEST,
                "unsupported sensitivity: secret",
            ),
            (
                "/api/v1/memories",
                r#"{"body":"blocked","visibility":"organization","sensitivity":"restricted"}"#,
                axum::http::StatusCode::FORBIDDEN,
                "write denied by policy",
            ),
            (
                "/api/v1/images",
                r#"{"media_type":"image/png","image_base64":"%%%"}"#,
                axum::http::StatusCode::BAD_REQUEST,
                "invalid image_base64 payload",
            ),
        ];

        for (path, body, status, message) in cases {
            let response = app
                .clone()
                .oneshot(
                    Request::post(path)
                        .header("content-type", "application/json")
                        .body(Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), status);
            let payload = response_json(response).await;
            assert_eq!(payload["error"], message);
        }
    }

    #[test]
    fn parses_supported_enums_and_labels() {
        let artifact_kinds = [
            ("message", ArtifactKind::Message),
            ("document", ArtifactKind::Document),
            ("code_diff", ArtifactKind::CodeDiff),
            ("code_file_snapshot", ArtifactKind::CodeFileSnapshot),
            ("terminal_output", ArtifactKind::TerminalOutput),
            ("image", ArtifactKind::Image),
            ("audio", ArtifactKind::Audio),
            ("video", ArtifactKind::Video),
            ("tool_result", ArtifactKind::ToolResult),
            ("web_page", ArtifactKind::WebPage),
        ];
        let memory_kinds = [
            ("fact", MemoryKind::Fact, "fact"),
            ("preference", MemoryKind::Preference, "preference"),
            ("decision", MemoryKind::Decision, "decision"),
            ("procedure", MemoryKind::Procedure, "procedure"),
            ("constraint", MemoryKind::Constraint, "constraint"),
            ("risk", MemoryKind::Risk, "risk"),
            ("summary", MemoryKind::Summary, "summary"),
            ("insight", MemoryKind::Insight, "insight"),
        ];
        let visibility_levels = [
            ("private", Visibility::Private),
            ("project", Visibility::Project),
            ("team", Visibility::Team),
            ("organization", Visibility::Organization),
        ];
        let sensitivity_levels = [
            ("public", Sensitivity::Public),
            ("internal", Sensitivity::Internal),
            ("private", Sensitivity::Private),
            ("restricted", Sensitivity::Restricted),
        ];

        assert_eq!(parse_artifact_kind(None).unwrap(), ArtifactKind::Message);
        assert_eq!(parse_visibility(None).unwrap(), Visibility::Private);
        assert_eq!(parse_sensitivity(None).unwrap(), Sensitivity::Internal);

        for (raw, expected) in artifact_kinds {
            assert_eq!(parse_artifact_kind(Some(raw)).unwrap(), expected);
        }
        for (raw, expected, label) in memory_kinds {
            assert_eq!(parse_memory_kind(raw).unwrap(), expected);
            assert_eq!(memory_kind_label(expected), label);
        }
        for (raw, expected) in visibility_levels {
            assert_eq!(parse_visibility(Some(raw)).unwrap(), expected);
        }
        for (raw, expected) in sensitivity_levels {
            assert_eq!(parse_sensitivity(Some(raw)).unwrap(), expected);
        }
    }

    #[test]
    fn maps_anyhow_errors_to_expected_http_statuses() {
        let forbidden = api_error_from_anyhow(anyhow!("write denied by policy"));
        let unsupported = api_error_from_anyhow(anyhow!("unsupported memory_kind"));
        let unknown = api_error_from_anyhow(anyhow!("unknown provider"));
        let empty = api_error_from_anyhow(anyhow!("empty request body"));
        let invalid = api_error_from_anyhow(anyhow!("Invalid media type"));
        let internal = api_error_from_anyhow(anyhow!("database offline"));

        assert_eq!(forbidden.status, axum::http::StatusCode::FORBIDDEN);
        assert_eq!(forbidden.message, "write denied by policy");
        assert_eq!(unsupported.status, axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(unknown.status, axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(empty.status, axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(invalid.status, axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(
            internal.status,
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn builds_console_page_and_escapes_metadata() {
        let metadata = ApiMetadata {
            service: "meat<mem>&ory".to_string(),
            version: r#"0.1.0"beta"#.to_string(),
            default_scope: "scp_'default'".to_string(),
            features: ApiFeatureFlags {
                pg: true,
                markdown: true,
                http: true,
                mcp: false,
                require_key: false,
            },
        };

        let page = build_console_page(&metadata);

        assert!(page.contains("meat&lt;mem&gt;&amp;ory"));
        assert!(page.contains(r#"0.1.0\"beta"#) || page.contains("0.1.0"));
        assert!(page.contains("scp_"));
        assert!(page.contains("Memory Workspace"));
        assert!(page.contains("/api/v1/explorer/memories"));
        assert!(page.contains("/api/v1/assistant/chat"));
        assert_eq!(escape_html("<>&\"'"), "&lt;&gt;&amp;&quot;&#39;");
    }

    #[tokio::test]
    async fn api_error_response_is_json() {
        let bad_request = ApiError::bad_request("bad input").into_response();
        let internal = ApiError::internal("server exploded").into_response();

        assert_eq!(bad_request.status(), axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(
            internal.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(response_json(bad_request).await["error"], "bad input");
        assert_eq!(response_json(internal).await["error"], "server exploded");
    }

    #[test]
    fn converts_memory_to_summary_shape() {
        let mut memory = Memory::new(
            ScopeId::from_string("scp_http_summary"),
            MemoryKind::Procedure,
            "Deploy checklist",
            "Run migrations before restart.",
        )
        .unwrap();
        memory.state = MemoryState::Active;
        memory.evidence_count = 3;

        let summary = memory_to_summary(&memory);

        assert_eq!(summary.memory_id, memory.id.as_str());
        assert_eq!(summary.title, "Deploy checklist");
        assert_eq!(summary.body, "Run migrations before restart.");
        assert_eq!(summary.memory_kind, "procedure");
        assert_eq!(summary.memory_state, "active");
        assert_eq!(summary.evidence_count, 3);
    }

    fn test_model_registry() -> ModelRegistry {
        ModelRegistry::build(
            vec![ProviderDescriptor {
                provider: Provider::Gemini,
                display_name: "Gemini".to_string(),
                base_url: Some("https://generativelanguage.googleapis.com".to_string()),
                api_key_env: Some("GEMINI_API_KEY".to_string()),
                enabled: true,
            }],
            vec![
                ModelDescriptor {
                    alias: "gemini_reasoning".to_string(),
                    provider: Provider::Gemini,
                    remote_model_id: "gemini-2.5-flash".to_string(),
                    display_name: "Gemini Reasoning".to_string(),
                    capabilities: BTreeSet::from([
                        ModelCapability::Reasoning,
                        ModelCapability::Extraction,
                    ]),
                    deployment: DeploymentTarget::Cloud,
                    locale: "zh-CN".to_string(),
                    priority: 100,
                    enabled: true,
                },
                ModelDescriptor {
                    alias: "gemini_vision".to_string(),
                    provider: Provider::Gemini,
                    remote_model_id: "gemini-2.5-flash".to_string(),
                    display_name: "Gemini Vision".to_string(),
                    capabilities: BTreeSet::from([ModelCapability::Vision]),
                    deployment: DeploymentTarget::Cloud,
                    locale: "zh-CN".to_string(),
                    priority: 100,
                    enabled: true,
                },
                ModelDescriptor {
                    alias: "gemini_embedding".to_string(),
                    provider: Provider::Gemini,
                    remote_model_id: "text-embedding-004".to_string(),
                    display_name: "Gemini Embedding".to_string(),
                    capabilities: BTreeSet::from([ModelCapability::Embedding]),
                    deployment: DeploymentTarget::Cloud,
                    locale: "zh-CN".to_string(),
                    priority: 100,
                    enabled: true,
                },
            ],
            [
                (
                    ModelCapability::Reasoning,
                    CapabilityRoute {
                        primary: "gemini_reasoning".to_string(),
                        fallbacks: Vec::new(),
                    },
                ),
                (
                    ModelCapability::Extraction,
                    CapabilityRoute {
                        primary: "gemini_reasoning".to_string(),
                        fallbacks: Vec::new(),
                    },
                ),
                (
                    ModelCapability::Vision,
                    CapabilityRoute {
                        primary: "gemini_vision".to_string(),
                        fallbacks: Vec::new(),
                    },
                ),
                (
                    ModelCapability::Embedding,
                    CapabilityRoute {
                        primary: "gemini_embedding".to_string(),
                        fallbacks: Vec::new(),
                    },
                ),
            ],
        )
        .expect("test registry should build")
    }

    fn failover_test_model_registry() -> ModelRegistry {
        ModelRegistry::build(
            vec![
                ProviderDescriptor {
                    provider: Provider::Gemini,
                    display_name: "Gemini".to_string(),
                    base_url: Some("https://generativelanguage.googleapis.com".to_string()),
                    api_key_env: Some("MEAT_MEMORY_TEST_GEMINI_MISSING".to_string()),
                    enabled: true,
                },
                ProviderDescriptor {
                    provider: Provider::Anthropic,
                    display_name: "Claude".to_string(),
                    base_url: Some("https://api.anthropic.com".to_string()),
                    api_key_env: None,
                    enabled: true,
                },
            ],
            vec![
                ModelDescriptor {
                    alias: "gemini_reasoning".to_string(),
                    provider: Provider::Gemini,
                    remote_model_id: "gemini-2.5-flash".to_string(),
                    display_name: "Gemini Reasoning".to_string(),
                    capabilities: BTreeSet::from([
                        ModelCapability::Reasoning,
                        ModelCapability::Extraction,
                    ]),
                    deployment: DeploymentTarget::Cloud,
                    locale: "zh-CN".to_string(),
                    priority: 100,
                    enabled: true,
                },
                ModelDescriptor {
                    alias: "gemini_vision".to_string(),
                    provider: Provider::Gemini,
                    remote_model_id: "gemini-2.5-flash".to_string(),
                    display_name: "Gemini Vision".to_string(),
                    capabilities: BTreeSet::from([ModelCapability::Vision]),
                    deployment: DeploymentTarget::Cloud,
                    locale: "zh-CN".to_string(),
                    priority: 100,
                    enabled: true,
                },
                ModelDescriptor {
                    alias: "claude_reasoning".to_string(),
                    provider: Provider::Anthropic,
                    remote_model_id: "claude-sonnet-4-5".to_string(),
                    display_name: "Claude Reasoning".to_string(),
                    capabilities: BTreeSet::from([
                        ModelCapability::Reasoning,
                        ModelCapability::Extraction,
                    ]),
                    deployment: DeploymentTarget::Cloud,
                    locale: "zh-CN".to_string(),
                    priority: 90,
                    enabled: true,
                },
                ModelDescriptor {
                    alias: "claude_vision".to_string(),
                    provider: Provider::Anthropic,
                    remote_model_id: "claude-sonnet-4-5".to_string(),
                    display_name: "Claude Vision".to_string(),
                    capabilities: BTreeSet::from([ModelCapability::Vision]),
                    deployment: DeploymentTarget::Cloud,
                    locale: "zh-CN".to_string(),
                    priority: 90,
                    enabled: true,
                },
                ModelDescriptor {
                    alias: "claude_embedding".to_string(),
                    provider: Provider::Anthropic,
                    remote_model_id: "text-embedding-3-large".to_string(),
                    display_name: "Claude Embedding".to_string(),
                    capabilities: BTreeSet::from([ModelCapability::Embedding]),
                    deployment: DeploymentTarget::Cloud,
                    locale: "zh-CN".to_string(),
                    priority: 90,
                    enabled: true,
                },
            ],
            [
                (
                    ModelCapability::Reasoning,
                    CapabilityRoute {
                        primary: "gemini_reasoning".to_string(),
                        fallbacks: vec!["claude_reasoning".to_string()],
                    },
                ),
                (
                    ModelCapability::Extraction,
                    CapabilityRoute {
                        primary: "gemini_reasoning".to_string(),
                        fallbacks: vec!["claude_reasoning".to_string()],
                    },
                ),
                (
                    ModelCapability::Vision,
                    CapabilityRoute {
                        primary: "gemini_vision".to_string(),
                        fallbacks: vec!["claude_vision".to_string()],
                    },
                ),
                (
                    ModelCapability::Embedding,
                    CapabilityRoute {
                        primary: "claude_embedding".to_string(),
                        fallbacks: Vec::new(),
                    },
                ),
            ],
        )
        .expect("failover test registry should build")
    }
}
