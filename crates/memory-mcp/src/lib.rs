use anyhow::Result;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use memory_domain::{
    AgentContext, AgentContextId, ArtifactKind, ContextBundle, Entity, EntityType, Memory,
    MemoryId, MemoryKind, MemorySource, ProjectDocument, Relation, RelationState, RelationType,
    ScopeId, ScopeType, Sensitivity, SourceId, Visibility,
};
use memory_kernel::{
    ApplyProjectDocumentSyncPlanRequest, Kernel, ListAgentContextsRequest,
    ListProjectDocumentsRequest, PromoteAgentContextRequest, PromoteMemoryRequest,
    RememberTextRequest, SearchContextRequest, UpsertAgentContextRequest,
};
use memory_observability::operation_span;
use memory_sync::{LocalProjectDocumentSyncEngine, ProjectDocumentSnapshot};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};
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
        description: "Create a memory from text content. Required arguments: body. Optional: scope_id, title, artifact_kind, memory_kind, visibility, sensitivity, source_refs.",
    },
    ToolSpec {
        name: "memory.fetch_context",
        description: "Fetch memories plus graph context for an Agent prompt. Required arguments: query. Optional: scope_id, limit.",
    },
    ToolSpec {
        name: "memory.search",
        description: "Search memories in a scope and return graph-aware context. Required arguments: query. Optional: scope_id, limit.",
    },
    ToolSpec {
        name: "memory.publish",
        description: "Promote an existing memory to a broader visibility level. Required arguments: scope_id, memory_id, target_visibility.",
    },
    ToolSpec {
        name: "memory.promote",
        description: "Publish an existing memory into another scope with review-aware promotion. Required arguments: source_scope_id, memory_id, source_scope_type, target_scope_id, target_scope_type, target_visibility.",
    },
    ToolSpec {
        name: "memory.context.upsert",
        description: "Create or refresh short-term Agent context. Required arguments: key, session_id, title, body. Optional: scope_id, task_id, labels.",
    },
    ToolSpec {
        name: "memory.context.list",
        description: "List short-term Agent contexts for a session. Required arguments: key, session_id. Optional: scope_id, task_id, limit.",
    },
    ToolSpec {
        name: "memory.context.promote",
        description: "Promote a short-term Agent context into long-term memory. Required arguments: key, context_id. Optional: memory_kind, visibility, sensitivity.",
    },
    ToolSpec {
        name: "memory.context.delete",
        description: "Delete a short-term Agent context. Required arguments: key, context_id.",
    },
    ToolSpec {
        name: "memory.docs.sync",
        description: "Scan and optionally import local project documents for a source. Required arguments: key, source_id. Optional: scope_id, local_root, dry_run.",
    },
    ToolSpec {
        name: "memory.docs.search",
        description: "Search/list project documents for a source. Required arguments: key, source_id. Optional: query, limit.",
    },
    ToolSpec {
        name: "memory.docs.conflicts",
        description: "List conflicted project documents for a source. Required arguments: key, source_id. Optional: limit.",
    },
];

pub const TOOL_NAMES: &[&str] = &[
    "memory.remember",
    "memory.fetch_context",
    "memory.search",
    "memory.publish",
    "memory.promote",
    "memory.context.upsert",
    "memory.context.list",
    "memory.context.promote",
    "memory.context.delete",
    "memory.docs.sync",
    "memory.docs.search",
    "memory.docs.conflicts",
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
                "memory.promote" => self.handle_promote(&trace_id, request.arguments).await?,
                "memory.context.upsert" => {
                    self.handle_context_upsert(&trace_id, request.arguments)
                        .await?
                }
                "memory.context.list" => {
                    self.handle_context_list(&trace_id, request.arguments)
                        .await?
                }
                "memory.context.promote" => {
                    self.handle_context_promote(&trace_id, request.arguments)
                        .await?
                }
                "memory.context.delete" => {
                    self.handle_context_delete(&trace_id, request.arguments)
                        .await?
                }
                "memory.docs.sync" => self.handle_docs_sync(&trace_id, request.arguments).await?,
                "memory.docs.search" => {
                    self.handle_docs_search(&trace_id, request.arguments)
                        .await?
                }
                "memory.docs.conflicts" => {
                    self.handle_docs_conflicts(&trace_id, request.arguments)
                        .await?
                }
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
        let context = self.resolve_optional_key(payload.key.as_deref()).await?;

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
                context,
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
        let context = self.resolve_optional_key(payload.key.as_deref()).await?;
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
        request.context = context;

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
        let context = self.resolve_required_key(payload.key.as_deref()).await?;
        let scope_id = payload
            .scope_id
            .map(ScopeId::from_string)
            .unwrap_or_else(|| self.default_scope_id.clone());
        let memory_id = MemoryId::from_string(payload.memory_id);
        let target_visibility = parse_visibility(&payload.target_visibility)?;
        let result = self
            .kernel
            .publish_memory_by_id_for_context(&context, scope_id, memory_id, target_visibility)
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

    async fn handle_promote(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<PromoteToolArgs>(arguments)?;
        let context = self.resolve_required_key(payload.key.as_deref()).await?;
        let result = self
            .kernel
            .promote_memory_by_id_for_context(
                &context,
                ScopeId::from_string(payload.source_scope_id),
                MemoryId::from_string(payload.memory_id),
                PromoteMemoryRequest {
                    source_scope_type: parse_scope_type(&payload.source_scope_type)?,
                    target_scope_id: ScopeId::from_string(payload.target_scope_id),
                    target_scope_type: parse_scope_type(&payload.target_scope_type)?,
                    target_visibility: parse_visibility(&payload.target_visibility)?,
                },
            )
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.promote".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "memory_id": result.memory.id.as_str(),
                "scope_id": result.memory.scope_id.as_str(),
                "owner_scope_id": result.memory.owner_scope_id.as_str(),
                "published_from_scope_id": result.memory.published_from_scope_id.as_ref().map(|scope| scope.as_str()),
                "title": result.memory.title,
                "body": result.memory.body,
                "memory_kind": memory_kind_label(result.memory.kind),
                "memory_state": result.memory.state.as_str(),
                "visibility": visibility_label(result.memory.visibility),
                "language_code": result.memory.language_code,
                "wrote_pg": result.wrote_pg,
                "wrote_markdown": result.wrote_markdown,
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_context_upsert(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<ContextUpsertToolArgs>(arguments)?;
        let context = self.resolve_required_key(payload.key.as_deref()).await?;
        let scope_id = payload
            .scope_id
            .map(ScopeId::from_string)
            .unwrap_or_else(|| context.owner_scope_id.clone());
        let mut request = UpsertAgentContextRequest::new(
            scope_id,
            payload.session_id,
            payload.title,
            payload.body,
        );
        request.task_id = payload.task_id;
        request.labels = payload.labels;
        request.context = Some(context);

        let agent_context = self
            .kernel
            .upsert_agent_context(request)
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.context.upsert".to_string(),
            trace_id: trace_id.to_string(),
            data: agent_context_payload(agent_context),
            warnings: Vec::new(),
        })
    }

    async fn handle_context_list(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<ContextListToolArgs>(arguments)?;
        let context = self.resolve_required_key(payload.key.as_deref()).await?;
        let scope_id = payload
            .scope_id
            .map(ScopeId::from_string)
            .unwrap_or_else(|| context.owner_scope_id.clone());
        let mut request = ListAgentContextsRequest::new(scope_id, payload.session_id);
        request.task_id = payload.task_id;
        request.limit = payload.limit.unwrap_or(20);
        request.context = Some(context);

        let contexts = self
            .kernel
            .list_agent_contexts(request)
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.context.list".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "context_count": contexts.len(),
                "contexts": contexts.into_iter().map(agent_context_payload).collect::<Vec<_>>(),
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_context_promote(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<ContextPromoteToolArgs>(arguments)?;
        let context = self.resolve_required_key(payload.key.as_deref()).await?;
        let mut request =
            PromoteAgentContextRequest::new(AgentContextId::from_string(payload.context_id));
        request.memory_kind = payload
            .memory_kind
            .as_deref()
            .map(parse_memory_kind)
            .transpose()?;
        request.visibility = payload
            .visibility
            .as_deref()
            .map(parse_visibility)
            .transpose()?
            .unwrap_or(Visibility::Private);
        request.sensitivity = payload
            .sensitivity
            .as_deref()
            .map(parse_sensitivity)
            .transpose()?
            .unwrap_or(Sensitivity::Internal);
        request.context = Some(context);

        let result = self
            .kernel
            .promote_agent_context(request)
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.context.promote".to_string(),
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
                "wrote_pg": result.wrote_pg,
                "wrote_markdown": result.wrote_markdown,
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_context_delete(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<ContextDeleteToolArgs>(arguments)?;
        let context = self.resolve_required_key(payload.key.as_deref()).await?;
        self.kernel
            .delete_agent_context(
                AgentContextId::from_string(payload.context_id.clone()),
                Some(&context),
            )
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.context.delete".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "context_id": payload.context_id,
                "deleted": true,
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_docs_search(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<DocsSearchToolArgs>(arguments)?;
        let context = self.resolve_required_key(payload.key.as_deref()).await?;
        let source_id = SourceId::from_string(payload.source_id);
        let mut request = ListProjectDocumentsRequest::new(source_id);
        request.limit = payload.limit.unwrap_or(20);
        request.query = payload.query;
        request.context = Some(context);

        let documents = self
            .kernel
            .list_project_documents(request)
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.docs.search".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "document_count": documents.len(),
                "documents": documents.into_iter().map(project_document_payload).collect::<Vec<_>>(),
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_docs_conflicts(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<DocsConflictsToolArgs>(arguments)?;
        let context = self.resolve_required_key(payload.key.as_deref()).await?;
        let documents = self
            .kernel
            .list_project_document_conflicts(
                SourceId::from_string(payload.source_id),
                payload.limit.unwrap_or(20),
                Some(&context),
            )
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.docs.conflicts".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "conflict_count": documents.len(),
                "documents": documents.into_iter().map(project_document_payload).collect::<Vec<_>>(),
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_docs_sync(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<DocsSyncToolArgs>(arguments)?;
        let context = self.resolve_required_key(payload.key.as_deref()).await?;
        let source_id = SourceId::from_string(payload.source_id);
        let source = self
            .kernel
            .get_memory_source(source_id.clone())
            .await
            .map_err(map_kernel_error)?
            .ok_or_else(|| McpError::not_found("memory source not found"))?;
        ensure_source_access(&context, &source)?;
        let scope_id = payload
            .scope_id
            .map(ScopeId::from_string)
            .unwrap_or_else(|| source.owner_scope_id.clone());
        if context.owner_scope_id != scope_id {
            return Err(McpError::forbidden(
                "scope access forbidden for current meat memory key",
            ));
        }
        let local_root = payload
            .local_root
            .or_else(|| source.local_root.clone())
            .ok_or_else(|| McpError::invalid_arguments("local_root is required"))?;
        let existing = self
            .kernel
            .list_project_documents(ListProjectDocumentsRequest {
                source_id: source_id.clone(),
                limit: 500,
                query: None,
                context: Some(context.clone()),
            })
            .await
            .map_err(map_kernel_error)?;
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
                McpError::invalid_arguments(format!(
                    "failed to scan local project documents: {error}"
                ))
            })?;
        let planned_documents = plan
            .documents
            .iter()
            .map(|document| {
                json!({
                    "canonical_uri": document.canonical_uri,
                    "local_path": document.local_path.to_string_lossy(),
                    "title": document.title,
                    "content_hash": document.content_hash,
                    "sync_state": document.sync_state.as_str(),
                })
            })
            .collect::<Vec<_>>();
        let dry_run = payload.dry_run.unwrap_or(false);
        if dry_run {
            return Ok(ToolCallResponse {
                tool: "memory.docs.sync".to_string(),
                trace_id: trace_id.to_string(),
                data: json!({
                    "dry_run": true,
                    "planned_documents": planned_documents,
                    "imported": [],
                    "missing": plan.missing,
                    "conflicts": plan.conflicts,
                }),
                warnings: Vec::new(),
            });
        }

        let result = self
            .kernel
            .apply_project_document_sync_plan(ApplyProjectDocumentSyncPlanRequest {
                source_id,
                scope_id,
                plan,
                context: Some(context),
            })
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.docs.sync".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "dry_run": false,
                "planned_documents": planned_documents,
                "imported": result.imported.into_iter().map(project_document_payload).collect::<Vec<_>>(),
                "missing": result.missing,
                "conflicts": result.conflicts,
            }),
            warnings: Vec::new(),
        })
    }

    async fn resolve_optional_key(
        &self,
        raw_key: Option<&str>,
    ) -> Result<Option<memory_domain::RequestContext>, McpError> {
        let Some(raw_key) = raw_key else {
            return Ok(None);
        };
        self.kernel
            .resolve_access_key_context(raw_key)
            .await
            .map_err(map_kernel_error)
    }

    async fn resolve_required_key(
        &self,
        raw_key: Option<&str>,
    ) -> Result<memory_domain::RequestContext, McpError> {
        self.resolve_optional_key(raw_key)
            .await?
            .ok_or_else(|| McpError::unauthorized("meat memory key is required"))
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

    fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "unauthorized",
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
    key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchToolArgs {
    scope_id: Option<String>,
    query: String,
    limit: Option<usize>,
    key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PublishToolArgs {
    scope_id: Option<String>,
    memory_id: String,
    target_visibility: String,
    key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PromoteToolArgs {
    source_scope_id: String,
    memory_id: String,
    source_scope_type: String,
    target_scope_id: String,
    target_scope_type: String,
    target_visibility: String,
    key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ContextUpsertToolArgs {
    scope_id: Option<String>,
    session_id: String,
    task_id: Option<String>,
    title: String,
    body: String,
    #[serde(default)]
    labels: Vec<String>,
    key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ContextListToolArgs {
    scope_id: Option<String>,
    session_id: String,
    task_id: Option<String>,
    limit: Option<usize>,
    key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ContextPromoteToolArgs {
    context_id: String,
    memory_kind: Option<String>,
    visibility: Option<String>,
    sensitivity: Option<String>,
    key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ContextDeleteToolArgs {
    context_id: String,
    key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct DocsSyncToolArgs {
    source_id: String,
    scope_id: Option<String>,
    local_root: Option<String>,
    dry_run: Option<bool>,
    key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct DocsSearchToolArgs {
    source_id: String,
    query: Option<String>,
    limit: Option<usize>,
    key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct DocsConflictsToolArgs {
    source_id: String,
    limit: Option<usize>,
    key: Option<String>,
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
    headers: HeaderMap,
    Json(mut payload): Json<ToolCallRequest>,
) -> Result<Json<ToolCallResponse>, McpError> {
    if let Some(raw_key) = key_from_headers(&headers) {
        let arguments = payload.arguments.as_object_mut().ok_or_else(|| {
            McpError::invalid_arguments("MCP tool arguments must be a JSON object")
        })?;
        arguments
            .entry("key".to_string())
            .or_insert_with(|| Value::String(raw_key));
    }
    Ok(Json(state.dispatch(payload).await?))
}

fn key_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-meat-memory-key")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
                .map(str::to_string)
        })
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

fn parse_scope_type(raw: &str) -> Result<ScopeType, McpError> {
    Ok(match raw {
        "org" => ScopeType::Org,
        "team" => ScopeType::Team,
        "workspace" => ScopeType::Workspace,
        "project" => ScopeType::Project,
        "user" => ScopeType::User,
        "session" => ScopeType::Session,
        other => {
            return Err(McpError::invalid_arguments(format!(
                "unsupported scope_type: {other}"
            )));
        }
    })
}

fn map_kernel_error(error: anyhow::Error) -> McpError {
    let message = error.to_string();
    if message.contains("required") || message.contains("invalid meat memory key") {
        return McpError::unauthorized(message);
    }
    if message.contains("forbidden") {
        return McpError::forbidden(message);
    }
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

    tracing::error!(error = %message, "internal mcp error");
    McpError::internal("internal server error")
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
        "scope_id": memory.scope_id.as_str(),
        "owner_scope_id": memory.owner_scope_id.as_str(),
        "published_from_scope_id": memory.published_from_scope_id.as_ref().map(|scope| scope.as_str()),
        "title": memory.title,
        "body": memory.body,
        "language_code": memory.language_code,
        "memory_kind": memory_kind_label(memory.kind),
        "memory_state": memory.state.as_str(),
        "visibility": visibility_label(memory.visibility),
        "sensitivity": sensitivity_label(memory.sensitivity),
        "evidence_count": memory.evidence_count,
    })
}

fn agent_context_payload(agent_context: AgentContext) -> Value {
    json!({
        "context_id": agent_context.id.as_str(),
        "source_id": agent_context.source_id.as_ref().map(|source_id| source_id.as_str()),
        "key_id": agent_context.key_id.as_ref().map(|key_id| key_id.as_str()),
        "scope_id": agent_context.scope_id.as_str(),
        "session_id": agent_context.session_id,
        "task_id": agent_context.task_id,
        "layer": agent_context.layer.as_str(),
        "title": agent_context.title,
        "body": agent_context.body,
        "labels": agent_context.labels,
        "expires_at": agent_context.expires_at,
        "created_at": agent_context.created_at,
        "updated_at": agent_context.updated_at,
    })
}

fn project_document_payload(document: ProjectDocument) -> Value {
    json!({
        "document_id": document.id.as_str(),
        "source_id": document.source_id.as_str(),
        "scope_id": document.scope_id.as_str(),
        "local_path": document.local_path,
        "canonical_uri": document.canonical_uri,
        "title": document.title,
        "content_hash": document.content_hash,
        "last_seen_mtime": document.last_seen_mtime,
        "sync_state": document.sync_state.as_str(),
        "conflict_state": document.conflict_state.as_str(),
        "artifact_id": document.artifact_id.as_ref().map(|artifact_id| artifact_id.as_str()),
        "memory_id": document.memory_id.as_ref().map(|memory_id| memory_id.as_str()),
        "created_at": document.created_at,
        "updated_at": document.updated_at,
    })
}

fn ensure_source_access(
    context: &memory_domain::RequestContext,
    source: &MemorySource,
) -> Result<(), McpError> {
    if context.owner_scope_id != source.owner_scope_id {
        return Err(McpError::forbidden(
            "source access forbidden for current meat memory key",
        ));
    }
    Ok(())
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
        McpServer, McpTransport, TOOL_SPECS, ToolCallRequest, build_router, context_bundle_payload,
        entity_type_label, map_kernel_error, memory_kind_label, parse_arguments,
        parse_artifact_kind, parse_memory_kind, parse_scope_type, parse_sensitivity,
        parse_visibility, relation_state_label, relation_type_label, sensitivity_label,
        tool_supported, visibility_label,
    };
    use axum::{body::Body, http::Request};
    use memory_domain::{
        ArtifactKind, ContextBundle, Entity, EntityType, KeyScopeKind, KeySourceKind, Memory,
        MemoryKind, Relation, RelationState, RelationType, ScopeId, ScopeType, Sensitivity,
        StorageMode, Visibility,
    };
    use memory_kernel::{CreateAccessKeyRequest, Kernel};
    use serde::Deserialize;
    use std::env;
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

    fn test_database_url() -> String {
        env::var("MEAT_MEMORY_TEST_DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@127.0.0.1:5433/meat_memory_dev".into()
        })
    }

    async fn test_server_with_pg(tempdir: &std::path::Path) -> (McpServer, Arc<Kernel>) {
        let kernel = Arc::new(
            Kernel::builder()
                .with_postgres_url(&test_database_url())
                .await
                .unwrap()
                .with_markdown_root(tempdir)
                .unwrap()
                .build()
                .unwrap(),
        );

        (
            McpServer::new(
                ScopeId::from_string("scp_mcp_default"),
                "meat-memory",
                "0.1.0",
                kernel.clone(),
            ),
            kernel,
        )
    }

    async fn create_test_key(kernel: &Kernel, owner_scope_id: &str) -> String {
        kernel
            .create_access_key(CreateAccessKeyRequest {
                raw_key: None,
                display_name: format!("key-{owner_scope_id}"),
                source_id: None,
                source_kind: KeySourceKind::Mcp,
                owner_principal_id: owner_scope_id.to_string(),
                owner_scope_id: ScopeId::from_string(owner_scope_id),
                scope_kind: KeyScopeKind::Personal,
                storage_mode: StorageMode::All,
                is_fully_isolated: false,
            })
            .await
            .unwrap()
            .raw_key
    }

    fn sample_bundle() -> ContextBundle {
        let scope_id = ScopeId::from_string("scp_mcp_bundle");
        let mut memory = Memory::new(
            scope_id.clone(),
            MemoryKind::Summary,
            "V1 测试覆盖率",
            "继续补齐单测",
        )
        .expect("memory should build");
        memory.id = memory_domain::MemoryId::from_string("mem_mcp_bundle");
        memory.state = memory_domain::MemoryState::Active;
        memory.visibility = Visibility::Team;
        memory.sensitivity = Sensitivity::Restricted;
        memory.evidence_count = 2;

        let entity =
            Entity::new(scope_id.clone(), EntityType::Project, "Meat Memory").expect("entity");
        let mut relation = Relation::new(
            RelationType::DependsOn,
            entity.id.clone(),
            memory_domain::EntityId::from_string("ent_other"),
        );
        relation.state = RelationState::Active;

        let mut bundle = ContextBundle::empty("测试覆盖率", scope_id);
        bundle.memories.push(memory);
        bundle.entities.push(entity);
        bundle.relations.push(relation);
        bundle
    }

    #[test]
    fn exposes_fetch_context_tool() {
        assert!(tool_supported("memory.fetch_context"));
        assert!(tool_supported("memory.promote"));
        assert!(tool_supported("memory.context.upsert"));
        assert!(tool_supported("memory.context.list"));
        assert!(tool_supported("memory.context.promote"));
        assert!(tool_supported("memory.context.delete"));
        assert!(tool_supported("memory.docs.sync"));
        assert!(tool_supported("memory.docs.search"));
        assert!(tool_supported("memory.docs.conflicts"));
        assert_eq!(TOOL_SPECS.len(), 12);
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

    #[tokio::test]
    async fn dispatch_remember_returns_markdown_write_payload() {
        let tempdir = tempdir().unwrap();
        let server = test_server(tempdir.path());

        let response = server
            .dispatch(ToolCallRequest {
                name: "memory.remember".to_string(),
                arguments: serde_json::json!({
                    "scope_id": "scp_mcp_default",
                    "title": "记住 V1 覆盖率",
                    "body": "优先提升 unit coverage",
                    "artifact_kind": "message",
                    "memory_kind": "summary",
                    "visibility": "private",
                    "sensitivity": "internal"
                }),
            })
            .await
            .unwrap();

        assert_eq!(response.tool, "memory.remember");
        assert_eq!(response.data["scope_id"], "scp_mcp_default");
        assert_eq!(response.data["title"], "记住 V1 覆盖率");
        assert_eq!(response.data["memory_kind"], "summary");
        assert_eq!(response.data["wrote_markdown"], true);
        assert_eq!(response.data["wrote_pg"], false);
    }

    #[tokio::test]
    async fn dispatch_search_and_fetch_context_return_empty_bundle_without_pg() {
        let tempdir = tempdir().unwrap();
        let server = test_server(tempdir.path());

        let search = server
            .dispatch(ToolCallRequest {
                name: "memory.search".to_string(),
                arguments: serde_json::json!({
                    "scope_id": "scp_mcp_default",
                    "query": "覆盖率",
                    "limit": 5
                }),
            })
            .await
            .unwrap();
        let fetch_context = server
            .dispatch(ToolCallRequest {
                name: "memory.fetch_context".to_string(),
                arguments: serde_json::json!({
                    "scope_id": "scp_mcp_default",
                    "query": "覆盖率"
                }),
            })
            .await
            .unwrap();

        assert_eq!(search.tool, "memory.search");
        assert_eq!(search.data["query"], "覆盖率");
        assert_eq!(search.data["memory_count"], 0);
        assert_eq!(search.data["entity_count"], 0);
        assert_eq!(fetch_context.tool, "memory.fetch_context");
        assert_eq!(fetch_context.data["scope_id"], "scp_mcp_default");
        assert_eq!(fetch_context.data["memory_count"], 0);
    }

    #[tokio::test]
    async fn dispatch_publish_returns_not_found_when_memory_is_missing() {
        let tempdir = tempdir().unwrap();
        let server = test_server(tempdir.path());

        let error = server
            .dispatch(ToolCallRequest {
                name: "memory.publish".to_string(),
                arguments: serde_json::json!({
                    "scope_id": "scp_mcp_default",
                    "memory_id": "mem_missing",
                    "target_visibility": "team"
                }),
            })
            .await
            .unwrap_err();

        assert_eq!(error.status, axum::http::StatusCode::UNAUTHORIZED);
        assert_eq!(error.code, "unauthorized");
    }

    #[tokio::test]
    async fn dispatch_promote_creates_target_scope_copy() {
        let tempdir = tempdir().unwrap();
        let (server, kernel) = test_server_with_pg(tempdir.path()).await;
        let raw_key = create_test_key(&kernel, "scp_user_bob").await;

        let remembered = server
            .dispatch(ToolCallRequest {
                name: "memory.remember".to_string(),
                arguments: serde_json::json!({
                    "scope_id": "scp_user_bob",
                    "title": "Bob 团队共享",
                    "body": "password: abc123",
                    "memory_kind": "procedure",
                    "visibility": "private",
                    "sensitivity": "private"
                }),
            })
            .await
            .unwrap();

        let promoted = server
            .dispatch(ToolCallRequest {
                name: "memory.promote".to_string(),
                arguments: serde_json::json!({
                    "source_scope_id": "scp_user_bob",
                    "memory_id": remembered.data["memory_id"],
                    "source_scope_type": "user",
                    "target_scope_id": "scp_project_demo",
                    "target_scope_type": "project",
                    "target_visibility": "project",
                    "key": raw_key
                }),
            })
            .await
            .unwrap();

        assert_eq!(promoted.tool, "memory.promote");
        assert_eq!(promoted.data["scope_id"], "scp_project_demo");
        assert_eq!(promoted.data["owner_scope_id"], "scp_user_bob");
        assert_eq!(promoted.data["published_from_scope_id"], "scp_user_bob");
        assert_eq!(promoted.data["memory_state"], "candidate");
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct ParseFixture {
        value: usize,
    }

    #[test]
    fn parse_helpers_cover_all_variants() {
        let artifact_cases = [
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
        for (raw, expected) in artifact_cases {
            assert_eq!(parse_artifact_kind(raw).unwrap(), expected);
        }
        assert!(parse_artifact_kind("unknown").is_err());

        let memory_kind_cases = [
            ("fact", MemoryKind::Fact),
            ("preference", MemoryKind::Preference),
            ("decision", MemoryKind::Decision),
            ("procedure", MemoryKind::Procedure),
            ("constraint", MemoryKind::Constraint),
            ("risk", MemoryKind::Risk),
            ("summary", MemoryKind::Summary),
            ("insight", MemoryKind::Insight),
        ];
        for (raw, expected) in memory_kind_cases {
            assert_eq!(parse_memory_kind(raw).unwrap(), expected);
            assert_eq!(memory_kind_label(expected), raw);
        }
        assert!(parse_memory_kind("unknown").is_err());

        let visibility_cases = [
            ("private", Visibility::Private),
            ("project", Visibility::Project),
            ("team", Visibility::Team),
            ("organization", Visibility::Organization),
        ];
        for (raw, expected) in visibility_cases {
            assert_eq!(parse_visibility(raw).unwrap(), expected);
            assert_eq!(visibility_label(expected), raw);
        }
        assert!(parse_visibility("unknown").is_err());

        let scope_type_cases = [
            ("org", ScopeType::Org),
            ("team", ScopeType::Team),
            ("workspace", ScopeType::Workspace),
            ("project", ScopeType::Project),
            ("user", ScopeType::User),
            ("session", ScopeType::Session),
        ];
        for (raw, expected) in scope_type_cases {
            assert_eq!(parse_scope_type(raw).unwrap(), expected);
        }
        assert!(parse_scope_type("unknown").is_err());

        let sensitivity_cases = [
            ("public", Sensitivity::Public),
            ("internal", Sensitivity::Internal),
            ("private", Sensitivity::Private),
            ("restricted", Sensitivity::Restricted),
        ];
        for (raw, expected) in sensitivity_cases {
            assert_eq!(parse_sensitivity(raw).unwrap(), expected);
            assert_eq!(sensitivity_label(expected), raw);
        }
        assert!(parse_sensitivity("unknown").is_err());
    }

    #[test]
    fn payload_helpers_cover_all_entity_and_relation_labels() {
        let entity_type_cases = [
            (EntityType::Person, "person"),
            (EntityType::Team, "team"),
            (EntityType::Organization, "organization"),
            (EntityType::Workspace, "workspace"),
            (EntityType::Project, "project"),
            (EntityType::Repository, "repository"),
            (EntityType::Service, "service"),
            (EntityType::Document, "document"),
            (EntityType::Task, "task"),
            (EntityType::Topic, "topic"),
            (EntityType::CodeSymbol, "code_symbol"),
        ];
        for (entity_type, expected) in entity_type_cases {
            assert_eq!(entity_type_label(entity_type), expected);
        }

        let relation_type_cases = [
            (RelationType::MemberOf, "member_of"),
            (RelationType::BelongsTo, "belongs_to"),
            (RelationType::Owns, "owns"),
            (RelationType::DependsOn, "depends_on"),
            (RelationType::Uses, "uses"),
            (RelationType::Implements, "implements"),
            (RelationType::References, "references"),
            (RelationType::DerivedFrom, "derived_from"),
            (RelationType::Documents, "documents"),
        ];
        for (relation_type, expected) in relation_type_cases {
            assert_eq!(relation_type_label(relation_type), expected);
        }

        let relation_state_cases = [
            (RelationState::Candidate, "candidate"),
            (RelationState::Active, "active"),
            (RelationState::Rejected, "rejected"),
            (RelationState::Archived, "archived"),
        ];
        for (state, expected) in relation_state_cases {
            assert_eq!(relation_state_label(state), expected);
        }
    }

    #[test]
    fn parse_arguments_and_kernel_error_mapping_behave_as_expected() {
        assert_eq!(
            parse_arguments::<ParseFixture>(serde_json::json!({ "value": 7 })).unwrap(),
            ParseFixture { value: 7 }
        );
        assert!(parse_arguments::<ParseFixture>(serde_json::json!({ "bad": 7 })).is_err());

        let forbidden = map_kernel_error(anyhow::anyhow!("denied by policy: blocked"));
        assert_eq!(forbidden.status, axum::http::StatusCode::FORBIDDEN);

        let missing = map_kernel_error(anyhow::anyhow!("memory not found"));
        assert_eq!(missing.status, axum::http::StatusCode::NOT_FOUND);

        let invalid = map_kernel_error(anyhow::anyhow!("unsupported memory_kind"));
        assert_eq!(invalid.status, axum::http::StatusCode::BAD_REQUEST);

        let internal = map_kernel_error(anyhow::anyhow!("database unavailable"));
        assert_eq!(
            internal.status,
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn context_bundle_payload_includes_graph_counts_and_labels() {
        let payload = context_bundle_payload(&sample_bundle());

        assert_eq!(payload["query"], "测试覆盖率");
        assert_eq!(payload["scope_id"], "scp_mcp_bundle");
        assert_eq!(payload["memory_count"], 1);
        assert_eq!(payload["entity_count"], 1);
        assert_eq!(payload["relation_count"], 1);
        assert_eq!(payload["memories"][0]["memory_kind"], "summary");
        assert_eq!(payload["memories"][0]["visibility"], "team");
        assert_eq!(payload["memories"][0]["sensitivity"], "restricted");
        assert_eq!(payload["entities"][0]["entity_type"], "project");
        assert_eq!(payload["relations"][0]["relation_type"], "depends_on");
        assert_eq!(payload["relations"][0]["state"], "active");
    }

    #[tokio::test]
    async fn stdio_message_returns_invalid_arguments_envelope_for_bad_json() {
        let tempdir = tempdir().unwrap();
        let server = test_server(tempdir.path());

        let response = server.handle_stdio_message("not-json").await;

        assert!(response.contains("\"code\":\"invalid_arguments\""));
    }

    #[tokio::test]
    async fn stdio_message_returns_success_envelope_for_remember() {
        let tempdir = tempdir().unwrap();
        let server = test_server(tempdir.path());

        let response = server
            .handle_stdio_message(
                r#"{"name":"memory.remember","arguments":{"scope_id":"scp_mcp_default","title":"标题","body":"正文","memory_kind":"fact"}}"#,
            )
            .await;

        assert!(response.contains("\"tool\":\"memory.remember\""));
        assert!(response.contains("\"wrote_markdown\":true"));
    }
}
