use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use memory_domain::{ArtifactKind, Memory, MemoryKind, ScopeId, Sensitivity, Visibility};
use memory_kernel::{Kernel, RememberImageRequest, RememberTextRequest, SearchContextRequest};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

pub const HTTP_ROUTES: &[&str] = &[
    "/",
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
    pub llm_notice: Option<String>,
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
        .route("/", get(index))
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
            title: result.memory.title.clone(),
            body: result.memory.body.clone(),
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

fn build_console_page(metadata: &ApiMetadata) -> String {
    let service = escape_html(&metadata.service);
    let version = escape_html(&metadata.version);
    let default_scope = escape_html(&metadata.default_scope);
    let pg = if metadata.features.pg {
        "enabled"
    } else {
        "disabled"
    };
    let markdown = if metadata.features.markdown {
        "enabled"
    } else {
        "disabled"
    };
    let mcp = if metadata.features.mcp {
        "enabled"
    } else {
        "disabled"
    };
    let mcp_link = if metadata.features.mcp {
        r#"<a class="pill" href="/mcp/tools" target="_blank" rel="noreferrer">GET /mcp/tools</a>"#
    } else {
        r#"<span class="pill">MCP disabled</span>"#
    };

    format!(
        r#"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{service} Console</title>
  <style>
    :root {{
      color-scheme: light;
      --bg: #f7f4ed;
      --panel: rgba(255, 252, 245, 0.9);
      --line: #d4c7af;
      --ink: #1f2937;
      --muted: #6b7280;
      --accent: #9a3412;
      --accent-soft: #fdedd5;
      --good: #166534;
      --bad: #991b1b;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      font-family: "SF Pro Text", "PingFang SC", "Helvetica Neue", sans-serif;
      background:
        radial-gradient(circle at top left, #fff7ed 0, transparent 36%),
        linear-gradient(160deg, #f7f4ed 0%, #efe5d3 100%);
      color: var(--ink);
    }}
    main {{
      max-width: 1120px;
      margin: 0 auto;
      padding: 32px 20px 48px;
    }}
    .hero {{
      display: grid;
      gap: 16px;
      margin-bottom: 24px;
    }}
    .hero-card {{
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 20px;
      padding: 24px;
      box-shadow: 0 18px 45px rgba(120, 53, 15, 0.08);
      backdrop-filter: blur(8px);
    }}
    h1, h2 {{
      margin: 0 0 12px;
      font-family: "Iowan Old Style", "Times New Roman", serif;
      font-weight: 700;
      letter-spacing: 0.01em;
    }}
    p {{
      margin: 0;
      line-height: 1.6;
    }}
    .subtle {{
      color: var(--muted);
      margin-top: 8px;
    }}
    .grid {{
      display: grid;
      gap: 16px;
      grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
      margin-top: 16px;
    }}
    .metric {{
      background: rgba(255, 255, 255, 0.7);
      border: 1px solid var(--line);
      border-radius: 16px;
      padding: 16px;
    }}
    .metric-label {{
      display: block;
      font-size: 12px;
      letter-spacing: 0.08em;
      text-transform: uppercase;
      color: var(--muted);
      margin-bottom: 8px;
    }}
    .metric-value {{
      font-size: 18px;
      font-weight: 700;
    }}
    .layout {{
      display: grid;
      gap: 16px;
      grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
      margin-top: 24px;
    }}
    section {{
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 18px;
      padding: 20px;
      box-shadow: 0 14px 35px rgba(120, 53, 15, 0.06);
    }}
    form {{
      display: grid;
      gap: 12px;
      margin-top: 16px;
    }}
    label {{
      display: grid;
      gap: 6px;
      font-size: 14px;
      color: var(--muted);
    }}
    input, textarea, button {{
      width: 100%;
      border-radius: 12px;
      border: 1px solid var(--line);
      padding: 12px 14px;
      font: inherit;
      color: var(--ink);
      background: rgba(255, 255, 255, 0.9);
    }}
    textarea {{
      min-height: 120px;
      resize: vertical;
    }}
    button {{
      cursor: pointer;
      border: none;
      color: #fff;
      font-weight: 600;
      background: linear-gradient(135deg, #9a3412 0%, #c2410c 100%);
      box-shadow: 0 10px 24px rgba(154, 52, 18, 0.24);
    }}
    button:hover {{
      filter: brightness(1.04);
    }}
    pre {{
      margin: 16px 0 0;
      padding: 14px;
      border-radius: 14px;
      background: #1f2937;
      color: #e5e7eb;
      overflow: auto;
      min-height: 180px;
    }}
    .links {{
      display: flex;
      flex-wrap: wrap;
      gap: 10px;
      margin-top: 16px;
    }}
    a {{
      color: var(--accent);
      text-decoration: none;
      font-weight: 600;
    }}
    .pill {{
      display: inline-flex;
      align-items: center;
      gap: 8px;
      padding: 8px 12px;
      border-radius: 999px;
      border: 1px solid var(--line);
      background: var(--accent-soft);
    }}
    .status-ok {{ color: var(--good); }}
    .status-bad {{ color: var(--bad); }}
    @media (max-width: 720px) {{
      main {{ padding: 20px 14px 36px; }}
      .hero-card, section {{ padding: 16px; }}
    }}
  </style>
</head>
<body>
  <main>
    <div class="hero">
      <div class="hero-card">
        <h1>{service} Browser Console</h1>
        <p>当前实例已经启动。这个页面提供健康检查、元信息查看、文本记忆写入和上下文搜索，方便你直接在浏览器里试用 V1 能力。</p>
        <p class="subtle">Version {version} · Default Scope {default_scope}</p>
        <div class="grid">
          <div class="metric">
            <span class="metric-label">Postgres</span>
            <div class="metric-value">{pg}</div>
          </div>
          <div class="metric">
            <span class="metric-label">Markdown</span>
            <div class="metric-value">{markdown}</div>
          </div>
          <div class="metric">
            <span class="metric-label">MCP</span>
            <div class="metric-value">{mcp}</div>
          </div>
          <div class="metric">
            <span class="metric-label">Health</span>
            <div id="health-indicator" class="metric-value">checking...</div>
          </div>
        </div>
        <div class="links">
          <a class="pill" href="/healthz" target="_blank" rel="noreferrer">GET /healthz</a>
          <a class="pill" href="/readyz" target="_blank" rel="noreferrer">GET /readyz</a>
          <a class="pill" href="/api/v1/meta" target="_blank" rel="noreferrer">GET /api/v1/meta</a>
          {mcp_link}
        </div>
      </div>
    </div>

    <div class="layout">
      <section>
        <h2>Write Text Memory</h2>
        <p>把一段文本写入 PG 和 Markdown 双存储。默认使用私有、内部可见级别。</p>
        <form id="memory-form">
          <label>Scope ID
            <input name="scope_id" value="{default_scope}" />
          </label>
          <label>Title
            <input name="title" value="浏览器试写" />
          </label>
          <label>Body
            <textarea name="body">这是从浏览器控制台写入的一条测试记忆。</textarea>
          </label>
          <label>Memory Kind
            <input name="memory_kind" value="procedure" />
          </label>
          <button type="submit">Create Memory</button>
        </form>
        <pre id="memory-result">Submit the form to create a memory...</pre>
      </section>

      <section>
        <h2>Search Context</h2>
        <p>按 scope 搜索上下文，验证刚写入的记忆能否被检索出来。</p>
        <form id="search-form">
          <label>Scope ID
            <input name="scope_id" value="{default_scope}" />
          </label>
          <label>Query
            <input name="query" value="浏览器 测试 记忆" />
          </label>
          <label>Limit
            <input name="limit" type="number" min="1" max="20" value="5" />
          </label>
          <button type="submit">Search</button>
        </form>
        <pre id="search-result">Submit the form to search context...</pre>
      </section>
    </div>

    <section style="margin-top:16px;">
      <h2>Runtime Metadata</h2>
      <p>下面是实时拉取的元信息，便于确认服务开关和默认 scope。</p>
      <pre id="meta-result">Loading metadata...</pre>
    </section>
  </main>

  <script>
    const healthIndicator = document.getElementById("health-indicator");
    const metaResult = document.getElementById("meta-result");
    const memoryResult = document.getElementById("memory-result");
    const searchResult = document.getElementById("search-result");

    async function fetchJson(url, options) {{
      const response = await fetch(url, options);
      const text = await response.text();
      let payload;
      try {{
        payload = text ? JSON.parse(text) : {{}};
      }} catch (_error) {{
        payload = {{ raw: text }};
      }}
      if (!response.ok) {{
        throw new Error(JSON.stringify(payload, null, 2));
      }}
      return payload;
    }}

    function prettyPrint(target, payload) {{
      target.textContent = JSON.stringify(payload, null, 2);
    }}

    async function loadSummary() {{
      try {{
        const [health, meta] = await Promise.all([
          fetchJson("/healthz"),
          fetchJson("/api/v1/meta"),
        ]);
        healthIndicator.textContent = health.status || "ok";
        healthIndicator.className = "metric-value status-ok";
        prettyPrint(metaResult, meta);
      }} catch (error) {{
        healthIndicator.textContent = "error";
        healthIndicator.className = "metric-value status-bad";
        metaResult.textContent = String(error);
      }}
    }}

    document.getElementById("memory-form").addEventListener("submit", async (event) => {{
      event.preventDefault();
      const form = new FormData(event.currentTarget);
      const payload = {{
        scope_id: form.get("scope_id"),
        title: form.get("title"),
        body: form.get("body"),
        memory_kind: form.get("memory_kind"),
        visibility: "private",
        sensitivity: "internal",
      }};
      memoryResult.textContent = "Creating memory...";
      try {{
        const result = await fetchJson("/api/v1/memories", {{
          method: "POST",
          headers: {{ "content-type": "application/json" }},
          body: JSON.stringify(payload),
        }});
        prettyPrint(memoryResult, result);
      }} catch (error) {{
        memoryResult.textContent = String(error);
      }}
    }});

    document.getElementById("search-form").addEventListener("submit", async (event) => {{
      event.preventDefault();
      const form = new FormData(event.currentTarget);
      const payload = {{
        scope_id: form.get("scope_id"),
        query: form.get("query"),
        limit: Number(form.get("limit")),
      }};
      searchResult.textContent = "Searching context...";
      try {{
        const result = await fetchJson("/api/v1/context/search", {{
          method: "POST",
          headers: {{ "content-type": "application/json" }},
          body: JSON.stringify(payload),
        }});
        prettyPrint(searchResult, result);
      }} catch (error) {{
        searchResult.textContent = String(error);
      }}
    }});

    loadSummary();
  </script>
</body>
</html>
"#
    )
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
        assert!(has_route("/api/v1/context/search"));
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
        assert!(text.contains("Write Text Memory"));
        assert!(text.contains("/api/v1/context/search"));
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
            },
        };

        let page = build_console_page(&metadata);

        assert!(page.contains("meat&lt;mem&gt;&amp;ory"));
        assert!(page.contains("0.1.0&quot;beta"));
        assert!(page.contains("scp_&#39;default&#39;"));
        assert!(page.contains("enabled"));
        assert!(page.contains("disabled"));
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
