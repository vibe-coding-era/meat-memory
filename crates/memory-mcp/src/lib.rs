use anyhow::Result;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use memory_domain::{
    ArtifactKind, ContextBundle, Entity, EntityType, Memory, MemoryId, MemoryKind, Relation,
    RelationState, RelationType, ScopeId, Sensitivity, Visibility,
};
use memory_kernel::{Kernel, RememberTextRequest, SearchContextRequest};
use memory_observability::operation_span;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use tracing::{Instrument, info};
use ulid::Ulid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    Stdio,
    Http,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
}

pub const TOOL_SPECS: &[ToolSpec] = &[
    ToolSpec {
        name: "memory.remember",
        description: "Create a memory from text content and persist it to active stores.",
    },
    ToolSpec {
        name: "memory.fetch_context",
        description: "Fetch memories plus graph context for a query in a scope.",
    },
    ToolSpec {
        name: "memory.search",
        description: "Search memories in a scope and return graph-aware context.",
    },
    ToolSpec {
        name: "memory.publish",
        description: "Promote an existing memory to a broader visibility level.",
    },
];

pub const TOOL_NAMES: &[&str] = &[
    "memory.remember",
    "memory.fetch_context",
    "memory.search",
    "memory.publish",
];

#[derive(Clone)]
pub struct McpServer {
    default_scope_id: ScopeId,
    service: String,
    version: String,
    kernel: Arc<Kernel>,
}

impl McpServer {
    pub fn new(
        default_scope_id: ScopeId,
        service: impl Into<String>,
        version: impl Into<String>,
        kernel: Arc<Kernel>,
    ) -> Self {
        Self {
            default_scope_id,
            service: service.into(),
            version: version.into(),
            kernel,
        }
    }

    pub fn service(&self) -> &str {
        &self.service
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn supported_transports(&self) -> [McpTransport; 2] {
        [McpTransport::Stdio, McpTransport::Http]
    }

    pub async fn dispatch(&self, request: ToolCallRequest) -> Result<ToolCallResponse, McpError> {
        let trace_id = Ulid::new().to_string();
        let span = operation_span("mcp", &request.name, None, Some(&request.name));

        async {
            let response = match request.name.as_str() {
                "memory.remember" => self.handle_remember(&trace_id, request.arguments).await?,
                "memory.search" => {
                    self.handle_search("memory.search", &trace_id, request.arguments)
                        .await?
                }
                "memory.fetch_context" => {
                    self.handle_search("memory.fetch_context", &trace_id, request.arguments)
                        .await?
                }
                "memory.publish" => self.handle_publish(&trace_id, request.arguments).await?,
                other => return Err(McpError::unsupported_tool(other)),
            };

            info!(
                trace_id = %trace_id,
                tool = response.tool,
                service = self.service(),
                version = self.version(),
                "mcp tool completed"
            );

            Ok(response)
        }
        .instrument(span)
        .await
    }

    pub async fn handle_stdio_message(&self, request_json: &str) -> String {
        let response = serde_json::from_str::<ToolCallRequest>(request_json)
            .map_err(|error| McpError::invalid_arguments(error.to_string()))
            .map(|request| async move { self.dispatch(request).await });

        let response = match response {
            Ok(fut) => fut.await,
            Err(error) => Err(error),
        };

        let envelope = match response {
            Ok(payload) => json!(payload),
            Err(error) => json!({
                "error": {
                    "code": error.code,
                    "message": error.message,
                }
            }),
        };

        serde_json::to_string(&envelope).unwrap_or_else(|_| {
            r#"{"error":{"code":"serialization_error","message":"failed to serialize MCP response"}}"#
                .to_string()
        })
    }

    async fn handle_remember(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<RememberToolArgs>(arguments)?;
        let artifact_kind = payload
            .artifact_kind
            .as_deref()
            .map(parse_artifact_kind)
            .transpose()?
            .unwrap_or(ArtifactKind::Message);
        let memory_kind = payload
            .memory_kind
            .as_deref()
            .map(parse_memory_kind)
            .transpose()?;
        let visibility = payload
            .visibility
            .as_deref()
            .map(parse_visibility)
            .transpose()?
            .unwrap_or(Visibility::Private);
        let sensitivity = payload
            .sensitivity
            .as_deref()
            .map(parse_sensitivity)
            .transpose()?
            .unwrap_or(Sensitivity::Internal);

        let result = self
            .kernel
            .remember_text(RememberTextRequest {
                scope_id: payload
                    .scope_id
                    .map(ScopeId::from_string)
                    .unwrap_or_else(|| self.default_scope_id.clone()),
                title: payload.title,
                body: payload.body,
                artifact_kind,
                memory_kind,
                source_refs: payload.source_refs,
                visibility,
                sensitivity,
            })
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.remember".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "artifact_id": result.artifact.id.as_str(),
                "memory_id": result.memory.id.as_str(),
                "scope_id": result.memory.scope_id.as_str(),
                "title": result.memory.title,
                "body": result.memory.body,
                "memory_kind": memory_kind_label(result.memory.kind),
                "memory_state": result.memory.state.as_str(),
                "visibility": visibility_label(result.memory.visibility),
                "sensitivity": sensitivity_label(result.memory.sensitivity),
                "evidence_count": result.memory.evidence_count,
                "wrote_pg": result.wrote_pg,
                "wrote_markdown": result.wrote_markdown,
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_search(
        &self,
        tool_name: &str,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<SearchToolArgs>(arguments)?;
        let mut request = SearchContextRequest::new(
            payload
                .scope_id
                .map(ScopeId::from_string)
                .unwrap_or_else(|| self.default_scope_id.clone()),
            payload.query,
        );
        if let Some(limit) = payload.limit {
            request.limit = limit;
        }

        let bundle = self
            .kernel
            .search_context(request)
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: tool_name.to_string(),
            trace_id: trace_id.to_string(),
            data: context_bundle_payload(&bundle),
            warnings: Vec::new(),
        })
    }

    async fn handle_publish(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<PublishToolArgs>(arguments)?;
        let scope_id = payload
            .scope_id
            .map(ScopeId::from_string)
            .unwrap_or_else(|| self.default_scope_id.clone());
        let memory_id = MemoryId::from_string(payload.memory_id);
        let target_visibility = parse_visibility(&payload.target_visibility)?;
        let result = self
            .kernel
            .publish_memory_by_id(scope_id, memory_id, target_visibility)
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.publish".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "memory_id": result.memory.id.as_str(),
                "scope_id": result.memory.scope_id.as_str(),
                "title": result.memory.title,
                "memory_kind": memory_kind_label(result.memory.kind),
                "memory_state": result.memory.state.as_str(),
                "visibility": visibility_label(result.memory.visibility),
                "sensitivity": sensitivity_label(result.memory.sensitivity),
                "wrote_pg": result.wrote_pg,
                "wrote_markdown": result.wrote_markdown,
            }),
            warnings: Vec::new(),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolCallRequest {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolCallResponse {
    pub tool: String,
    pub trace_id: String,
    pub data: Value,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolListResponse {
    pub service: String,
    pub version: String,
    pub transports: Vec<McpTransport>,
    pub tools: Vec<ToolSpec>,
}

#[derive(Debug)]
pub struct McpError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl McpError {
    fn invalid_arguments(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_arguments",
            message: message.into(),
        }
    }

    fn unsupported_tool(tool: &str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "unsupported_tool",
            message: format!("unsupported tool: {tool}"),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: message.into(),
        }
    }

    fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: "forbidden",
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: message.into(),
        }
    }
}

impl IntoResponse for McpError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({
                "error": {
                    "code": self.code,
                    "message": self.message,
                }
            })),
        )
            .into_response()
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RememberToolArgs {
    scope_id: Option<String>,
    title: Option<String>,
    body: String,
    artifact_kind: Option<String>,
    memory_kind: Option<String>,
    #[serde(default)]
    source_refs: Vec<String>,
    visibility: Option<String>,
    sensitivity: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchToolArgs {
    scope_id: Option<String>,
    query: String,
    limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct PublishToolArgs {
    scope_id: Option<String>,
    memory_id: String,
    target_visibility: String,
}

pub fn tool_supported(name: &str) -> bool {
    TOOL_NAMES.contains(&name)
}

pub fn build_router(state: McpServer) -> Router {
    Router::new()
        .route("/mcp/tools", get(list_tools))
        .route("/mcp/tools/call", post(call_tool))
        .with_state(state)
}

async fn list_tools(State(state): State<McpServer>) -> Json<ToolListResponse> {
    Json(ToolListResponse {
        service: state.service().to_string(),
        version: state.version().to_string(),
        transports: state.supported_transports().to_vec(),
        tools: TOOL_SPECS.to_vec(),
    })
}

async fn call_tool(
    State(state): State<McpServer>,
    Json(payload): Json<ToolCallRequest>,
) -> Result<Json<ToolCallResponse>, McpError> {
    Ok(Json(state.dispatch(payload).await?))
}

fn parse_arguments<T>(arguments: Value) -> Result<T, McpError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(arguments)
        .map_err(|error| McpError::invalid_arguments(error.to_string()))
}

fn parse_artifact_kind(raw: &str) -> Result<ArtifactKind, McpError> {
    Ok(match raw {
        "message" => ArtifactKind::Message,
        "document" => ArtifactKind::Document,
        "code_diff" => ArtifactKind::CodeDiff,
        "code_file_snapshot" => ArtifactKind::CodeFileSnapshot,
        "terminal_output" => ArtifactKind::TerminalOutput,
        "image" => ArtifactKind::Image,
        "audio" => ArtifactKind::Audio,
        "video" => ArtifactKind::Video,
        "tool_result" => ArtifactKind::ToolResult,
        "web_page" => ArtifactKind::WebPage,
        other => {
            return Err(McpError::invalid_arguments(format!(
                "unsupported artifact_kind: {other}"
            )));
        }
    })
}

fn parse_memory_kind(raw: &str) -> Result<MemoryKind, McpError> {
    Ok(match raw {
        "fact" => MemoryKind::Fact,
        "preference" => MemoryKind::Preference,
        "decision" => MemoryKind::Decision,
        "procedure" => MemoryKind::Procedure,
        "constraint" => MemoryKind::Constraint,
        "risk" => MemoryKind::Risk,
        "summary" => MemoryKind::Summary,
        "insight" => MemoryKind::Insight,
        other => {
            return Err(McpError::invalid_arguments(format!(
                "unsupported memory_kind: {other}"
            )));
        }
    })
}

fn parse_visibility(raw: &str) -> Result<Visibility, McpError> {
    Ok(match raw {
        "private" => Visibility::Private,
        "project" => Visibility::Project,
        "team" => Visibility::Team,
        "organization" => Visibility::Organization,
        other => {
            return Err(McpError::invalid_arguments(format!(
                "unsupported visibility: {other}"
            )));
        }
    })
}

fn parse_sensitivity(raw: &str) -> Result<Sensitivity, McpError> {
    Ok(match raw {
        "public" => Sensitivity::Public,
        "internal" => Sensitivity::Internal,
        "private" => Sensitivity::Private,
        "restricted" => Sensitivity::Restricted,
        other => {
            return Err(McpError::invalid_arguments(format!(
                "unsupported sensitivity: {other}"
            )));
        }
    })
}

fn map_kernel_error(error: anyhow::Error) -> McpError {
    let message = error.to_string();
    if message.contains("denied by policy") {
        return McpError::forbidden(message);
    }
    if message.contains("memory not found") {
        return McpError::not_found(message);
    }
    if message.contains("unsupported")
        || message.contains("unknown")
        || message.contains("empty")
        || message.contains("Invalid")
    {
        return McpError::invalid_arguments(message);
    }

    McpError::internal(message)
}

fn context_bundle_payload(bundle: &ContextBundle) -> Value {
    json!({
        "query": bundle.query,
        "scope_id": bundle.scope_id.as_str(),
        "generated_at": bundle.generated_at,
        "memory_count": bundle.memories.len(),
        "entity_count": bundle.entities.len(),
        "relation_count": bundle.relations.len(),
        "memories": bundle.memories.iter().map(memory_payload).collect::<Vec<_>>(),
        "entities": bundle.entities.iter().map(entity_payload).collect::<Vec<_>>(),
        "relations": bundle.relations.iter().map(relation_payload).collect::<Vec<_>>(),
    })
}

fn memory_payload(memory: &Memory) -> Value {
    json!({
        "memory_id": memory.id.as_str(),
        "title": memory.title,
        "body": memory.body,
        "memory_kind": memory_kind_label(memory.kind),
        "memory_state": memory.state.as_str(),
        "visibility": visibility_label(memory.visibility),
        "sensitivity": sensitivity_label(memory.sensitivity),
        "evidence_count": memory.evidence_count,
    })
}

fn entity_payload(entity: &Entity) -> Value {
    json!({
        "entity_id": entity.id.as_str(),
        "entity_type": entity_type_label(entity.entity_type),
        "canonical_name": entity.canonical_name,
        "normalized_key": entity.normalized_key,
    })
}

fn relation_payload(relation: &Relation) -> Value {
    json!({
        "relation_id": relation.id.as_str(),
        "relation_type": relation_type_label(relation.relation_type),
        "subject_entity_id": relation.subject_entity_id.as_str(),
        "object_entity_id": relation.object_entity_id.as_str(),
        "state": relation_state_label(relation.state),
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

fn entity_type_label(entity_type: EntityType) -> &'static str {
    match entity_type {
        EntityType::Person => "person",
        EntityType::Team => "team",
        EntityType::Organization => "organization",
        EntityType::Workspace => "workspace",
        EntityType::Project => "project",
        EntityType::Repository => "repository",
        EntityType::Service => "service",
        EntityType::Document => "document",
        EntityType::Task => "task",
        EntityType::Topic => "topic",
        EntityType::CodeSymbol => "code_symbol",
    }
}

fn relation_type_label(relation_type: RelationType) -> &'static str {
    match relation_type {
        RelationType::MemberOf => "member_of",
        RelationType::BelongsTo => "belongs_to",
        RelationType::Owns => "owns",
        RelationType::DependsOn => "depends_on",
        RelationType::Uses => "uses",
        RelationType::Implements => "implements",
        RelationType::References => "references",
        RelationType::DerivedFrom => "derived_from",
        RelationType::Documents => "documents",
    }
}

fn relation_state_label(state: RelationState) -> &'static str {
    match state {
        RelationState::Candidate => "candidate",
        RelationState::Active => "active",
        RelationState::Rejected => "rejected",
        RelationState::Archived => "archived",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        McpServer, McpTransport, TOOL_SPECS, ToolCallRequest, build_router, tool_supported,
    };
    use axum::{body::Body, http::Request};
    use memory_domain::ScopeId;
    use memory_kernel::Kernel;
    use std::sync::Arc;
    use tempfile::tempdir;
    use tower::ServiceExt;

    fn test_server(tempdir: &std::path::Path) -> McpServer {
        let kernel = Arc::new(
            Kernel::builder()
                .with_markdown_root(tempdir)
                .unwrap()
                .build()
                .unwrap(),
        );

        McpServer::new(
            ScopeId::from_string("scp_mcp_default"),
            "meat-memory",
            "0.1.0",
            kernel,
        )
    }

    #[test]
    fn exposes_fetch_context_tool() {
        assert!(tool_supported("memory.fetch_context"));
        assert_eq!(TOOL_SPECS.len(), 4);
    }

    #[test]
    fn exposes_stdio_and_http_transports() {
        let tempdir = tempdir().unwrap();
        let server = test_server(tempdir.path());
        assert_eq!(
            server.supported_transports(),
            [McpTransport::Stdio, McpTransport::Http]
        );
    }

    #[tokio::test]
    async fn router_exposes_tool_listing() {
        let tempdir = tempdir().unwrap();
        let app = build_router(test_server(tempdir.path()));

        let response = app
            .oneshot(Request::get("/mcp/tools").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn dispatch_rejects_unknown_tool() {
        let tempdir = tempdir().unwrap();
        let server = test_server(tempdir.path());

        let error = server
            .dispatch(ToolCallRequest {
                name: "memory.unknown".to_string(),
                arguments: serde_json::json!({}),
            })
            .await
            .unwrap_err();

        assert_eq!(error.status, axum::http::StatusCode::NOT_FOUND);
    }
}
