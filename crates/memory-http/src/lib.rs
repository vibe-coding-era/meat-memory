use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use memory_domain::{ArtifactKind, Memory, MemoryKind, ScopeId, Sensitivity, Visibility};
use memory_kernel::{Kernel, RememberImageRequest, RememberTextRequest, SearchContextRequest};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

pub const HTTP_ROUTES: &[&str] = &[
    "/healthz",
    "/readyz",
    "/livez",
    "/metrics",
    "/api/v1/meta",
    "/api/v1/memories",
    "/api/v1/images",
    "/api/v1/context",
    "/api/v1/context/search",
];

#[derive(Debug, Clone, Serialize)]
pub struct ApiFeatureFlags {
    pub pg: bool,
    pub markdown: bool,
    pub http: bool,
    pub mcp: bool,
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
}

impl HttpAppState {
    pub fn new(default_scope_id: ScopeId, metadata: ApiMetadata, kernel: Arc<Kernel>) -> Self {
        Self {
            default_scope_id,
            metadata,
            kernel,
        }
    }
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
    pub title: String,
    pub body: String,
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
    pub title: String,
    pub body: String,
    pub memory_kind: String,
    pub memory_state: String,
    pub evidence_count: usize,
    pub vision_caption: Option<String>,
    pub vision_model_alias: Option<String>,
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
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/livez", get(livez))
        .route("/metrics", get(metrics))
        .route("/api/v1/meta", get(meta))
        .route("/api/v1/memories", post(create_memory))
        .route("/api/v1/images", post(create_image))
        .route("/api/v1/context", post(search_context))
        .route("/api/v1/context/search", post(search_context))
        .with_state(state)
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

async fn meta(State(state): State<HttpAppState>) -> Json<ApiMetadata> {
    Json(state.metadata.clone())
}

async fn create_memory(
    State(state): State<HttpAppState>,
    Json(payload): Json<CreateMemoryRequest>,
) -> Result<(StatusCode, Json<CreateMemoryResponse>), ApiError> {
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
        })
        .await
        .map_err(api_error_from_anyhow)?;

    Ok((
        StatusCode::CREATED,
        Json(CreateMemoryResponse {
            artifact_id: result.artifact.id.as_str().to_string(),
            memory_id: result.memory.id.as_str().to_string(),
            scope_id: result.memory.scope_id.as_str().to_string(),
            title: result.memory.title.clone(),
            body: result.memory.body.clone(),
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
    Json(payload): Json<CreateImageRequest>,
) -> Result<(StatusCode, Json<CreateImageResponse>), ApiError> {
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

    Ok((
        StatusCode::CREATED,
        Json(CreateImageResponse {
            asset_id,
            asset_uri,
            artifact_id: result.artifact.id.as_str().to_string(),
            memory_id: result.memory.id.as_str().to_string(),
            scope_id: result.memory.scope_id.as_str().to_string(),
            title: result.memory.title.clone(),
            body: result.memory.body.clone(),
            memory_kind: memory_kind_label(result.memory.kind).to_string(),
            memory_state: result.memory.state.as_str().to_string(),
            evidence_count: result.memory.evidence_count,
            vision_caption,
            vision_model_alias,
            wrote_pg: result.wrote_pg,
            wrote_markdown: result.wrote_markdown,
        }),
    ))
}

async fn search_context(
    State(state): State<HttpAppState>,
    Json(payload): Json<SearchContextHttpRequest>,
) -> Result<Json<SearchContextHttpResponse>, ApiError> {
    let scope_id = payload
        .scope_id
        .map(ScopeId::from_string)
        .unwrap_or_else(|| state.default_scope_id.clone());
    let mut request = SearchContextRequest::new(scope_id.clone(), payload.query);
    if let Some(limit) = payload.limit {
        request.limit = limit;
    }

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

#[cfg(test)]
mod tests {
    use super::{ApiFeatureFlags, ApiMetadata, HttpAppState, build_router, has_route};
    use axum::{body::Body, http::Request};
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use memory_domain::ScopeId;
    use memory_kernel::Kernel;
    use memory_models::{
        CapabilityRoute, DeploymentTarget, ModelCapability, ModelDescriptor, ModelRegistry,
        Provider, ProviderDescriptor,
    };
    use std::collections::BTreeSet;
    use std::sync::Arc;
    use tempfile::tempdir;
    use tower::ServiceExt;

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
                },
            },
            kernel,
        )
    }

    #[tokio::test]
    async fn exposes_http_routes() {
        assert!(has_route("/healthz"));
        assert!(has_route("/livez"));
        assert!(has_route("/metrics"));
        assert!(has_route("/api/v1/context/search"));
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
    async fn exposes_metrics_and_liveness_routes() {
        let tempdir = tempdir().unwrap();
        let app = build_router(test_state(tempdir.path()));

        let livez = app
            .clone()
            .oneshot(Request::get("/livez").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let metrics = app
            .oneshot(Request::get("/metrics").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(livez.status(), axum::http::StatusCode::OK);
        assert_eq!(metrics.status(), axum::http::StatusCode::OK);
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
}
