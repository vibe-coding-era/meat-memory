use anyhow::Result;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use memory_domain::{
    AgentContext, AgentContextId, ArtifactKind, ContextBundle, DistillationProfile,
    DistillationProfileId, DistillationProfileLevel, DistillationProfileStatus, Entity, EntityType,
    Memory, MemoryId, MemoryKind, MemoryProposal, MemoryRecord, MemoryRecordStatus, MemoryRelation,
    MemorySource, ProjectDocument, ProposalId, Relation, RelationState, RelationType, ScopeId,
    ScopeType, Sensitivity, SourceId, Visibility,
};
use memory_kernel::{
    ApplyMemoryProposalRequest, ApplyProjectDocumentSyncPlanRequest, ApproveMemoryProposalRequest,
    ChangeMemoryLifecycleStatusRequest, ComposedDistillationProfile, ConnectorDryRunRequest,
    ConnectorImportDraftRequest, ConnectorProposalQueueRequest, ConnectorSyncPlanRequest,
    DistillationCandidate, DistillationPromptSegment, DistillationSessionOverride,
    GetMemoryProposalRequest, GetMemoryTimelineRequest, InspectMemoryLifecycleRequest, Kernel,
    ListAgentContextsRequest, ListDistillationProfilesRequest, ListMemoryProposalsRequest,
    ListMemoryVersionsRequest, ListProjectDocumentsRequest, MemoryTimeline,
    PreviewDistillationRequest, PreviewDistillationResult, PromoteAgentContextRequest,
    PromoteMemoryRequest, RejectMemoryProposalRequest, RememberTextRequest, ReviewActorKind,
    RollbackMemoryRequest, RollbackMemoryResult, SearchContextRequest, TimelineAuditEvent,
    TimelineEvent, TimelineEventKind, TimelineVersion, UpsertAgentContextRequest,
    UpsertDistillationProfileRequest, build_competitor_compatibility_report,
    build_connector_import_draft_report, build_connector_proposal_queue_report,
    build_connector_sync_plan, compatibility_report_json, connector_dry_run_json,
    connector_import_draft_json, connector_proposal_queue_json, connector_sync_plan_json,
    health_json, run_connector_dry_run, verification_json, verify_memory_passport_bundle,
};
use memory_observability::operation_span;
use memory_sync::{LocalProjectDocumentSyncEngine, ProjectDocumentSnapshot};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{fs, path::PathBuf, sync::Arc};
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
        name: "memory.proposals.list",
        description: "List V2.8 memory governance proposals. Optional: scope_id, limit.",
    },
    ToolSpec {
        name: "memory.proposals.inspect",
        description: "Inspect one V2.8 memory governance proposal. Required arguments: proposal_id.",
    },
    ToolSpec {
        name: "memory.proposals.approve",
        description: "Approve a V2.8 memory proposal with review policy enforcement. Required arguments: key, proposal_id. Optional: actor, actor_kind, user_authorized.",
    },
    ToolSpec {
        name: "memory.proposals.reject",
        description: "Reject a V2.8 memory proposal. Required arguments: key, proposal_id. Optional: actor.",
    },
    ToolSpec {
        name: "memory.proposals.apply",
        description: "Apply an approved V2.8 memory proposal with review policy enforcement. Required arguments: key, proposal_id. Optional: actor, actor_kind, user_authorized.",
    },
    ToolSpec {
        name: "memory.versions.list",
        description: "List version snapshots for one memory. Required arguments: memory_id. Optional: scope_id, limit.",
    },
    ToolSpec {
        name: "memory.timeline.get",
        description: "Return one memory timeline including versions, relations, proposals, and audit events. Required arguments: memory_id. Optional: scope_id, limit.",
    },
    ToolSpec {
        name: "memory.version.rollback",
        description: "Rollback one memory to a previous version. Required arguments: key, memory_id, target_version, reason. Optional: scope_id, actor.",
    },
    ToolSpec {
        name: "memory.profile.list",
        description: "List distillation profiles. Optional: scope_id, limit.",
    },
    ToolSpec {
        name: "memory.profile.upsert",
        description: "Create or update a distillation profile. Required arguments: key, name, prompt_text. Optional: profile_id, scope_id, profile_level, status, focus_topics, prefer_memory_kinds, created_by.",
    },
    ToolSpec {
        name: "memory.distill.preview",
        description: "Preview profile-guided memory distillation without writing candidates. Required arguments: input, evidence_refs. Optional: scope_id, prompt_text, focus_topics, prefer_memory_kinds.",
    },
    ToolSpec {
        name: "memory.lifecycle.inspect",
        description: "Inspect lifecycle metadata and recall explanation for a memory. Required arguments: memory_id. Optional: key, scope_id, query.",
    },
    ToolSpec {
        name: "memory.lifecycle.status",
        description: "Change lifecycle status for a memory. Required arguments: key, memory_id, status. Optional: scope_id, reason, actor.",
    },
    ToolSpec {
        name: "memory.lifecycle.forget",
        description: "Soft-forget a memory so it is excluded from recall. Required arguments: key, memory_id. Optional: scope_id, reason, actor.",
    },
    ToolSpec {
        name: "memory.lifecycle.restore",
        description: "Restore a soft-forgotten memory to active. Required arguments: key, memory_id. Optional: scope_id, reason, actor.",
    },
    ToolSpec {
        name: "memory.lifecycle.report",
        description: "Return a memory health report. Optional: scope_id, limit.",
    },
    ToolSpec {
        name: "memory.benchmark.report",
        description: "Read the latest V2.9 benchmark report without running a benchmark. Optional: input_dir.",
    },
    ToolSpec {
        name: "memory.trace.inspect",
        description: "Read a V2.9 recall trace report. Optional: input_dir, trace_id.",
    },
    ToolSpec {
        name: "memory.health.report",
        description: "Return the V2.9 memory health summary including risks and suggested actions. Optional: scope_id, limit.",
    },
    ToolSpec {
        name: "memory.passport.manifest",
        description: "Inspect and verify a V2.9 Memory Passport manifest without importing it. Optional: input_dir.",
    },
    ToolSpec {
        name: "memory.compat.report",
        description: "Return the V2.95 Supermemory / mem0 / MemoryLake compatibility mapping. Optional: scope_id.",
    },
    ToolSpec {
        name: "memory.connectors.dry_run",
        description: "Run a V2.97 connector dry-run without writing memory. Required arguments: connector, root_path. Optional: max_items.",
    },
    ToolSpec {
        name: "memory.connectors.sync_plan",
        description: "Build a V2.97 connector sync-plan without applying it. Required arguments: connector, root_path. Optional: scope_id, max_items.",
    },
    ToolSpec {
        name: "memory.connectors.import_draft",
        description: "Build a V2.97 connector import draft without writing memory. Required arguments: connector, root_path. Optional: scope_id, max_items, proposal.",
    },
    ToolSpec {
        name: "memory.connectors.proposal_queue",
        description: "Build a V2.97 connector proposal queue without applying or writing memory. Required arguments: connector, root_path. Optional: scope_id, max_items.",
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
    "memory.proposals.list",
    "memory.proposals.inspect",
    "memory.proposals.approve",
    "memory.proposals.reject",
    "memory.proposals.apply",
    "memory.versions.list",
    "memory.timeline.get",
    "memory.version.rollback",
    "memory.profile.list",
    "memory.profile.upsert",
    "memory.distill.preview",
    "memory.lifecycle.inspect",
    "memory.lifecycle.status",
    "memory.lifecycle.forget",
    "memory.lifecycle.restore",
    "memory.lifecycle.report",
    "memory.benchmark.report",
    "memory.trace.inspect",
    "memory.health.report",
    "memory.passport.manifest",
    "memory.compat.report",
    "memory.connectors.dry_run",
    "memory.connectors.sync_plan",
    "memory.connectors.import_draft",
    "memory.connectors.proposal_queue",
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
                "memory.proposals.list" => {
                    self.handle_proposals_list(&trace_id, request.arguments)
                        .await?
                }
                "memory.proposals.inspect" => {
                    self.handle_proposals_inspect(&trace_id, request.arguments)
                        .await?
                }
                "memory.proposals.approve" => {
                    self.handle_proposals_approve(&trace_id, request.arguments)
                        .await?
                }
                "memory.proposals.reject" => {
                    self.handle_proposals_reject(&trace_id, request.arguments)
                        .await?
                }
                "memory.proposals.apply" => {
                    self.handle_proposals_apply(&trace_id, request.arguments)
                        .await?
                }
                "memory.versions.list" => {
                    self.handle_versions_list(&trace_id, request.arguments)
                        .await?
                }
                "memory.timeline.get" => {
                    self.handle_timeline_get(&trace_id, request.arguments)
                        .await?
                }
                "memory.version.rollback" => {
                    self.handle_version_rollback(&trace_id, request.arguments)
                        .await?
                }
                "memory.profile.list" => {
                    self.handle_profile_list(&trace_id, request.arguments)
                        .await?
                }
                "memory.profile.upsert" => {
                    self.handle_profile_upsert(&trace_id, request.arguments)
                        .await?
                }
                "memory.distill.preview" => {
                    self.handle_distill_preview(&trace_id, request.arguments)
                        .await?
                }
                "memory.lifecycle.inspect" => {
                    self.handle_lifecycle_inspect(&trace_id, request.arguments)
                        .await?
                }
                "memory.lifecycle.status" => {
                    self.handle_lifecycle_status(&trace_id, request.arguments, None)
                        .await?
                }
                "memory.lifecycle.forget" => {
                    self.handle_lifecycle_status(
                        &trace_id,
                        request.arguments,
                        Some(MemoryRecordStatus::Forgotten),
                    )
                    .await?
                }
                "memory.lifecycle.restore" => {
                    self.handle_lifecycle_status(
                        &trace_id,
                        request.arguments,
                        Some(MemoryRecordStatus::Active),
                    )
                    .await?
                }
                "memory.lifecycle.report" => {
                    self.handle_lifecycle_report(&trace_id, request.arguments)
                        .await?
                }
                "memory.benchmark.report" => {
                    self.handle_benchmark_report(&trace_id, request.arguments)?
                }
                "memory.trace.inspect" => {
                    self.handle_trace_inspect(&trace_id, request.arguments)?
                }
                "memory.health.report" => {
                    self.handle_health_report(&trace_id, request.arguments)
                        .await?
                }
                "memory.passport.manifest" => {
                    self.handle_passport_manifest(&trace_id, request.arguments)?
                }
                "memory.compat.report" => {
                    self.handle_compat_report(&trace_id, request.arguments)?
                }
                "memory.connectors.dry_run" => {
                    self.handle_connector_dry_run(&trace_id, request.arguments)?
                }
                "memory.connectors.sync_plan" => {
                    self.handle_connector_sync_plan(&trace_id, request.arguments)?
                }
                "memory.connectors.import_draft" => {
                    self.handle_connector_import_draft(&trace_id, request.arguments)?
                }
                "memory.connectors.proposal_queue" => {
                    self.handle_connector_proposal_queue(&trace_id, request.arguments)?
                }
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

    async fn handle_proposals_list(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<ProposalListToolArgs>(arguments)?;
        let scope_id = payload
            .scope_id
            .map(ScopeId::from_string)
            .unwrap_or_else(|| self.default_scope_id.clone());
        let mut request = ListMemoryProposalsRequest::new(Some(scope_id.clone()));
        request.limit = payload.limit.unwrap_or(request.limit);
        let proposals = self
            .kernel
            .list_memory_proposals(request)
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.proposals.list".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "scope_id": scope_id.as_str(),
                "proposal_count": proposals.len(),
                "proposals": proposals.iter().map(memory_proposal_payload).collect::<Vec<_>>(),
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_proposals_inspect(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<ProposalInspectToolArgs>(arguments)?;
        let proposal = self
            .kernel
            .get_memory_proposal(GetMemoryProposalRequest {
                proposal_id: ProposalId::from_string(payload.proposal_id),
            })
            .await
            .map_err(map_kernel_error)?
            .ok_or_else(|| McpError::not_found("memory proposal not found"))?;

        Ok(ToolCallResponse {
            tool: "memory.proposals.inspect".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "proposal": memory_proposal_payload(&proposal),
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_proposals_approve(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<ProposalDecisionToolArgs>(arguments)?;
        let context = self.resolve_required_key(payload.key.as_deref()).await?;
        let proposal_id = ProposalId::from_string(payload.proposal_id);
        let proposal = self
            .kernel
            .get_memory_proposal(GetMemoryProposalRequest {
                proposal_id: proposal_id.clone(),
            })
            .await
            .map_err(map_kernel_error)?
            .ok_or_else(|| McpError::not_found("memory proposal not found"))?;
        ensure_proposal_scope_access(&context, &proposal)?;

        let proposal = self
            .kernel
            .approve_memory_proposal(ApproveMemoryProposalRequest {
                proposal_id,
                actor: payload
                    .actor
                    .unwrap_or_else(|| context.principal_id.clone()),
                actor_kind: payload
                    .actor_kind
                    .as_deref()
                    .map(parse_review_actor_kind)
                    .transpose()?
                    .unwrap_or(ReviewActorKind::Agent),
                has_user_authorization: payload.user_authorized.unwrap_or(false),
            })
            .await
            .map_err(map_kernel_error)?
            .ok_or_else(|| McpError::not_found("memory proposal not found"))?;

        Ok(ToolCallResponse {
            tool: "memory.proposals.approve".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "proposal": memory_proposal_payload(&proposal),
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_proposals_reject(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<ProposalRejectToolArgs>(arguments)?;
        let context = self.resolve_required_key(payload.key.as_deref()).await?;
        let proposal_id = ProposalId::from_string(payload.proposal_id);
        let proposal = self
            .kernel
            .get_memory_proposal(GetMemoryProposalRequest {
                proposal_id: proposal_id.clone(),
            })
            .await
            .map_err(map_kernel_error)?
            .ok_or_else(|| McpError::not_found("memory proposal not found"))?;
        ensure_proposal_scope_access(&context, &proposal)?;

        let proposal = self
            .kernel
            .reject_memory_proposal(RejectMemoryProposalRequest {
                proposal_id,
                actor: payload
                    .actor
                    .unwrap_or_else(|| context.principal_id.clone()),
            })
            .await
            .map_err(map_kernel_error)?
            .ok_or_else(|| McpError::not_found("memory proposal not found"))?;

        Ok(ToolCallResponse {
            tool: "memory.proposals.reject".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "proposal": memory_proposal_payload(&proposal),
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_proposals_apply(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<ProposalDecisionToolArgs>(arguments)?;
        let context = self.resolve_required_key(payload.key.as_deref()).await?;
        let proposal_id = ProposalId::from_string(payload.proposal_id);
        let proposal = self
            .kernel
            .get_memory_proposal(GetMemoryProposalRequest {
                proposal_id: proposal_id.clone(),
            })
            .await
            .map_err(map_kernel_error)?
            .ok_or_else(|| McpError::not_found("memory proposal not found"))?;
        ensure_proposal_scope_access(&context, &proposal)?;

        let proposal = self
            .kernel
            .apply_memory_proposal(ApplyMemoryProposalRequest {
                proposal_id,
                actor: payload
                    .actor
                    .unwrap_or_else(|| context.principal_id.clone()),
                actor_kind: payload
                    .actor_kind
                    .as_deref()
                    .map(parse_review_actor_kind)
                    .transpose()?
                    .unwrap_or(ReviewActorKind::Agent),
                has_user_authorization: payload.user_authorized.unwrap_or(false),
            })
            .await
            .map_err(map_kernel_error)?
            .ok_or_else(|| McpError::not_found("memory proposal not found"))?;

        Ok(ToolCallResponse {
            tool: "memory.proposals.apply".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "proposal": memory_proposal_payload(&proposal),
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_versions_list(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<VersionsListToolArgs>(arguments)?;
        let scope_id = payload
            .scope_id
            .map(ScopeId::from_string)
            .unwrap_or_else(|| self.default_scope_id.clone());
        let memory_id = MemoryId::from_string(payload.memory_id);
        let mut request = ListMemoryVersionsRequest::new(scope_id.clone(), memory_id.clone());
        request.limit = payload.limit.unwrap_or(request.limit);
        let versions = self
            .kernel
            .list_memory_versions(request)
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.versions.list".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "scope_id": scope_id.as_str(),
                "memory_id": memory_id.as_str(),
                "version_count": versions.len(),
                "versions": versions.iter().map(memory_version_payload).collect::<Vec<_>>(),
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_timeline_get(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<TimelineGetToolArgs>(arguments)?;
        let scope_id = payload
            .scope_id
            .map(ScopeId::from_string)
            .unwrap_or_else(|| self.default_scope_id.clone());
        let memory_id = MemoryId::from_string(payload.memory_id);
        let mut request = GetMemoryTimelineRequest::new(scope_id.clone(), memory_id.clone());
        request.limit = payload.limit.unwrap_or(request.limit);
        let timeline = self
            .kernel
            .get_memory_timeline(request)
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.timeline.get".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "scope_id": scope_id.as_str(),
                "memory_id": memory_id.as_str(),
                "timeline": timeline.as_ref().map(memory_timeline_payload),
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_version_rollback(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<VersionRollbackToolArgs>(arguments)?;
        let context = self.resolve_required_key(payload.key.as_deref()).await?;
        let scope_id = payload
            .scope_id
            .map(ScopeId::from_string)
            .unwrap_or_else(|| context.owner_scope_id.clone());
        if scope_id != context.owner_scope_id {
            return Err(McpError::forbidden(
                "scope access forbidden for current meat memory key",
            ));
        }
        let result = self
            .kernel
            .rollback_memory(RollbackMemoryRequest {
                scope_id,
                memory_id: MemoryId::from_string(payload.memory_id),
                target_version: payload.target_version,
                actor: payload
                    .actor
                    .unwrap_or_else(|| context.principal_id.clone()),
                reason: payload.reason,
            })
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.version.rollback".to_string(),
            trace_id: trace_id.to_string(),
            data: rollback_payload(&result),
            warnings: Vec::new(),
        })
    }

    async fn handle_profile_list(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<ProfileListToolArgs>(arguments)?;
        let scope_id = payload
            .scope_id
            .map(ScopeId::from_string)
            .unwrap_or_else(|| self.default_scope_id.clone());
        let mut request = ListDistillationProfilesRequest::new(Some(scope_id.clone()));
        request.limit = payload.limit.unwrap_or(request.limit);
        let profiles = self
            .kernel
            .list_distillation_profiles(request)
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.profile.list".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "scope_id": scope_id.as_str(),
                "profile_count": profiles.len(),
                "profiles": profiles.iter().map(distillation_profile_payload).collect::<Vec<_>>(),
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_profile_upsert(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<ProfileUpsertToolArgs>(arguments)?;
        let context = self.resolve_required_key(payload.key.as_deref()).await?;
        let owner_scope_id = context.owner_scope_id.clone();
        let request = profile_upsert_request(
            payload,
            context.principal_id.clone(),
            owner_scope_id.clone(),
        )?;
        if let Some(scope_id) = request.scope_id.as_ref() {
            if *scope_id != owner_scope_id {
                return Err(McpError::forbidden(
                    "scope access forbidden for current meat memory key",
                ));
            }
        }
        let profile = self
            .kernel
            .upsert_distillation_profile(request)
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.profile.upsert".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "profile": distillation_profile_payload(&profile),
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_distill_preview(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<DistillPreviewToolArgs>(arguments)?;
        let result = self
            .kernel
            .preview_distillation(PreviewDistillationRequest {
                scope_id: payload
                    .scope_id
                    .clone()
                    .map(ScopeId::from_string)
                    .unwrap_or_else(|| self.default_scope_id.clone()),
                input: payload.input.clone(),
                evidence_refs: payload.evidence_refs.clone(),
                session_override: build_distillation_session_override(&payload)?,
            })
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.distill.preview".to_string(),
            trace_id: trace_id.to_string(),
            data: distillation_preview_payload(&result),
            warnings: Vec::new(),
        })
    }

    async fn handle_lifecycle_inspect(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<LifecycleInspectToolArgs>(arguments)?;
        let context = self.resolve_optional_key(payload.key.as_deref()).await?;
        let result = self
            .kernel
            .inspect_memory_lifecycle(InspectMemoryLifecycleRequest {
                scope_id: payload
                    .scope_id
                    .map(ScopeId::from_string)
                    .unwrap_or_else(|| self.default_scope_id.clone()),
                memory_id: MemoryId::from_string(payload.memory_id),
                query: payload.query,
                context,
            })
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.lifecycle.inspect".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "memory": memory_payload(&result.memory),
                "record": lifecycle_record_payload(&result.record),
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
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_lifecycle_status(
        &self,
        trace_id: &str,
        arguments: Value,
        forced_status: Option<MemoryRecordStatus>,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<LifecycleStatusToolArgs>(arguments)?;
        let context = self.resolve_required_key(payload.key.as_deref()).await?;
        let status = match forced_status {
            Some(status) => status,
            None => parse_record_status(payload.status.as_deref().ok_or_else(|| {
                McpError::invalid_arguments("status is required for memory.lifecycle.status")
            })?)?,
        };
        let result = self
            .kernel
            .change_memory_lifecycle_status(ChangeMemoryLifecycleStatusRequest {
                scope_id: payload
                    .scope_id
                    .map(ScopeId::from_string)
                    .unwrap_or_else(|| context.owner_scope_id.clone()),
                memory_id: MemoryId::from_string(payload.memory_id),
                status,
                reason: payload
                    .reason
                    .unwrap_or_else(|| "mcp lifecycle update".to_string()),
                actor: payload
                    .actor
                    .unwrap_or_else(|| context.principal_id.clone()),
                context: Some(context),
            })
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: match forced_status {
                Some(MemoryRecordStatus::Forgotten) => "memory.lifecycle.forget".to_string(),
                Some(MemoryRecordStatus::Active) => "memory.lifecycle.restore".to_string(),
                _ => "memory.lifecycle.status".to_string(),
            },
            trace_id: trace_id.to_string(),
            data: json!({
                "memory": memory_payload(&result.memory),
                "record": lifecycle_record_payload(&result.record),
                "audit": {
                    "action": result.audit_event.action,
                    "actor": result.audit_event.actor,
                    "record_id": result.audit_event.record_id,
                    "before_status": result.audit_event.before_status.map(|status| status.as_str()),
                    "after_status": result.audit_event.after_status.map(|status| status.as_str()),
                    "reason": result.audit_event.reason,
                    "created_at": result.audit_event.created_at,
                },
                "wrote_pg": result.wrote_pg,
                "wrote_markdown": result.wrote_markdown,
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_lifecycle_report(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<LifecycleReportToolArgs>(arguments)?;
        let report = self
            .kernel
            .memory_health_report(
                payload.scope_id.map(ScopeId::from_string),
                payload.limit.unwrap_or(500),
            )
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.lifecycle.report".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
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
                "generated_at": report.generated_at,
            }),
            warnings: Vec::new(),
        })
    }

    fn handle_benchmark_report(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<ReportInputToolArgs>(arguments)?;
        let input_dir = report_input_dir(payload.input_dir, "tests/reports/benchmark/latest");
        let summary = read_report_text(&input_dir.join("summary.md"))?;
        let metrics = read_report_json(&input_dir.join("metrics.json"))?;

        Ok(ToolCallResponse {
            tool: "memory.benchmark.report".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "input_dir": input_dir.display().to_string(),
                "summary": summary,
                "metrics": metrics,
            }),
            warnings: Vec::new(),
        })
    }

    fn handle_trace_inspect(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<TraceInspectToolArgs>(arguments)?;
        let input_dir = report_input_dir(payload.input_dir, "tests/reports/trace/latest");
        let trace = read_report_json(&input_dir.join("trace.json"))?;
        if let Some(expected_trace_id) = payload.trace_id.as_deref() {
            let actual_trace_id = trace
                .pointer("/trace/id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if actual_trace_id != expected_trace_id {
                return Err(McpError::invalid_arguments(format!(
                    "trace id mismatch: expected {expected_trace_id}, found {actual_trace_id}"
                )));
            }
        }
        let explanation = fs::read_to_string(input_dir.join("explanation.md")).ok();

        Ok(ToolCallResponse {
            tool: "memory.trace.inspect".to_string(),
            trace_id: trace_id.to_string(),
            data: json!({
                "input_dir": input_dir.display().to_string(),
                "trace": trace,
                "explanation": explanation,
            }),
            warnings: Vec::new(),
        })
    }

    async fn handle_health_report(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<LifecycleReportToolArgs>(arguments)?;
        let report = self
            .kernel
            .memory_health_report(
                payload.scope_id.map(ScopeId::from_string),
                payload.limit.unwrap_or(500),
            )
            .await
            .map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.health.report".to_string(),
            trace_id: trace_id.to_string(),
            data: health_json(&report),
            warnings: Vec::new(),
        })
    }

    fn handle_passport_manifest(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<ReportInputToolArgs>(arguments)?;
        let input_dir = report_input_dir(payload.input_dir, "tests/reports/passport/latest");
        let verification = verify_memory_passport_bundle(&input_dir).map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.passport.manifest".to_string(),
            trace_id: trace_id.to_string(),
            data: verification_json(&verification),
            warnings: Vec::new(),
        })
    }

    fn handle_compat_report(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<CompatReportToolArgs>(arguments)?;
        let scope_id = payload
            .scope_id
            .map(ScopeId::from_string)
            .unwrap_or_else(|| self.default_scope_id.clone());
        let report = build_competitor_compatibility_report(scope_id).map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.compat.report".to_string(),
            trace_id: trace_id.to_string(),
            data: compatibility_report_json(&report),
            warnings: Vec::new(),
        })
    }

    fn handle_connector_dry_run(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<ConnectorDryRunToolArgs>(arguments)?;
        let mut request = ConnectorDryRunRequest::new(payload.connector, payload.root_path);
        if let Some(max_items) = payload.max_items {
            request.max_items = max_items;
        }
        let report = run_connector_dry_run(request).map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.connectors.dry_run".to_string(),
            trace_id: trace_id.to_string(),
            data: connector_dry_run_json(&report),
            warnings: Vec::new(),
        })
    }

    fn handle_connector_sync_plan(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<ConnectorSyncPlanToolArgs>(arguments)?;
        let scope_id = payload
            .scope_id
            .map(ScopeId::from_string)
            .unwrap_or_else(|| self.default_scope_id.clone());
        let mut request =
            ConnectorSyncPlanRequest::new(payload.connector, payload.root_path, scope_id);
        if let Some(max_items) = payload.max_items {
            request.max_items = max_items;
        }
        let output = build_connector_sync_plan(request).map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.connectors.sync_plan".to_string(),
            trace_id: trace_id.to_string(),
            data: connector_sync_plan_json(&output.report),
            warnings: Vec::new(),
        })
    }

    fn handle_connector_import_draft(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<ConnectorImportDraftToolArgs>(arguments)?;
        let scope_id = payload
            .scope_id
            .map(ScopeId::from_string)
            .unwrap_or_else(|| self.default_scope_id.clone());
        let mut request =
            ConnectorImportDraftRequest::new(payload.connector, payload.root_path, scope_id);
        if let Some(max_items) = payload.max_items {
            request.max_items = max_items;
        }
        request.proposal_mode = payload.proposal.unwrap_or(false);
        let report = build_connector_import_draft_report(request).map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.connectors.import_draft".to_string(),
            trace_id: trace_id.to_string(),
            data: connector_import_draft_json(&report),
            warnings: Vec::new(),
        })
    }

    fn handle_connector_proposal_queue(
        &self,
        trace_id: &str,
        arguments: Value,
    ) -> Result<ToolCallResponse, McpError> {
        let payload = parse_arguments::<ConnectorProposalQueueToolArgs>(arguments)?;
        let scope_id = payload
            .scope_id
            .map(ScopeId::from_string)
            .unwrap_or_else(|| self.default_scope_id.clone());
        let mut request =
            ConnectorProposalQueueRequest::new(payload.connector, payload.root_path, scope_id);
        if let Some(max_items) = payload.max_items {
            request.max_items = max_items;
        }
        let report = build_connector_proposal_queue_report(request).map_err(map_kernel_error)?;

        Ok(ToolCallResponse {
            tool: "memory.connectors.proposal_queue".to_string(),
            trace_id: trace_id.to_string(),
            data: connector_proposal_queue_json(&report),
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
struct LifecycleInspectToolArgs {
    scope_id: Option<String>,
    memory_id: String,
    query: Option<String>,
    key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct LifecycleStatusToolArgs {
    scope_id: Option<String>,
    memory_id: String,
    status: Option<String>,
    reason: Option<String>,
    actor: Option<String>,
    key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct LifecycleReportToolArgs {
    scope_id: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct ReportInputToolArgs {
    input_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct TraceInspectToolArgs {
    input_dir: Option<String>,
    trace_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CompatReportToolArgs {
    scope_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ConnectorDryRunToolArgs {
    connector: String,
    root_path: String,
    max_items: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct ConnectorSyncPlanToolArgs {
    connector: String,
    root_path: String,
    scope_id: Option<String>,
    max_items: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct ConnectorImportDraftToolArgs {
    connector: String,
    root_path: String,
    scope_id: Option<String>,
    max_items: Option<usize>,
    proposal: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct ConnectorProposalQueueToolArgs {
    connector: String,
    root_path: String,
    scope_id: Option<String>,
    max_items: Option<usize>,
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

#[derive(Debug, Clone, Deserialize)]
struct ProposalListToolArgs {
    scope_id: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct ProposalInspectToolArgs {
    proposal_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ProposalDecisionToolArgs {
    proposal_id: String,
    actor: Option<String>,
    actor_kind: Option<String>,
    user_authorized: Option<bool>,
    key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ProposalRejectToolArgs {
    proposal_id: String,
    actor: Option<String>,
    key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct VersionsListToolArgs {
    scope_id: Option<String>,
    memory_id: String,
    limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct TimelineGetToolArgs {
    scope_id: Option<String>,
    memory_id: String,
    limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct VersionRollbackToolArgs {
    scope_id: Option<String>,
    memory_id: String,
    target_version: i32,
    actor: Option<String>,
    reason: String,
    key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ProfileListToolArgs {
    scope_id: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct ProfileUpsertToolArgs {
    profile_id: Option<String>,
    scope_id: Option<String>,
    profile_level: Option<String>,
    level: Option<String>,
    status: Option<String>,
    name: String,
    prompt_text: String,
    #[serde(default)]
    focus_topics: Vec<String>,
    #[serde(default)]
    prefer_memory_kinds: Vec<String>,
    created_by: Option<String>,
    key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct DistillPreviewToolArgs {
    scope_id: Option<String>,
    input: String,
    #[serde(default)]
    evidence_refs: Vec<String>,
    prompt_text: Option<String>,
    #[serde(default)]
    focus_topics: Vec<String>,
    #[serde(default)]
    prefer_memory_kinds: Vec<String>,
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

fn report_input_dir(input_dir: Option<String>, default_dir: &str) -> PathBuf {
    input_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| default_dir.into())
}

fn read_report_text(path: &PathBuf) -> Result<String, McpError> {
    fs::read_to_string(path).map_err(|error| {
        McpError::invalid_arguments(format!("failed to read {}: {error}", path.display()))
    })
}

fn read_report_json(path: &PathBuf) -> Result<Value, McpError> {
    let raw = read_report_text(path)?;
    serde_json::from_str(&raw).map_err(|error| {
        McpError::invalid_arguments(format!("failed to parse {}: {error}", path.display()))
    })
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

fn parse_record_status(raw: &str) -> Result<MemoryRecordStatus, McpError> {
    Ok(match raw {
        "candidate" => MemoryRecordStatus::Candidate,
        "active" => MemoryRecordStatus::Active,
        "needs_review" | "conflicted" => MemoryRecordStatus::NeedsReview,
        "archived" => MemoryRecordStatus::Archived,
        "deprecated" => MemoryRecordStatus::Deprecated,
        "forgotten" => MemoryRecordStatus::Forgotten,
        "deleted" => MemoryRecordStatus::Deleted,
        other => {
            return Err(McpError::invalid_arguments(format!(
                "unsupported lifecycle status: {other}"
            )));
        }
    })
}

fn parse_review_actor_kind(raw: &str) -> Result<ReviewActorKind, McpError> {
    Ok(match raw {
        "user" => ReviewActorKind::User,
        "agent" => ReviewActorKind::Agent,
        "system" => ReviewActorKind::System,
        other => {
            return Err(McpError::invalid_arguments(format!(
                "unsupported review actor kind: {other}"
            )));
        }
    })
}

fn parse_distillation_profile_level(raw: &str) -> Result<DistillationProfileLevel, McpError> {
    Ok(match raw {
        "user_global" | "global" => DistillationProfileLevel::UserGlobal,
        "project" => DistillationProfileLevel::Project,
        other => {
            return Err(McpError::invalid_arguments(format!(
                "unsupported distillation profile level: {other}"
            )));
        }
    })
}

fn parse_distillation_profile_status(raw: &str) -> Result<DistillationProfileStatus, McpError> {
    Ok(match raw {
        "active" => DistillationProfileStatus::Active,
        "archived" => DistillationProfileStatus::Archived,
        other => {
            return Err(McpError::invalid_arguments(format!(
                "unsupported distillation profile status: {other}"
            )));
        }
    })
}

fn profile_upsert_request(
    args: ProfileUpsertToolArgs,
    created_by_default: String,
    default_scope_id: ScopeId,
) -> Result<UpsertDistillationProfileRequest, McpError> {
    let profile_level = parse_distillation_profile_level(
        args.profile_level
            .as_deref()
            .or(args.level.as_deref())
            .unwrap_or("project"),
    )?;
    let scope_id = match (profile_level, args.scope_id.map(ScopeId::from_string)) {
        (DistillationProfileLevel::Project, Some(scope_id)) => Some(scope_id),
        (DistillationProfileLevel::Project, None) => Some(default_scope_id),
        (DistillationProfileLevel::UserGlobal, scope_id) => scope_id,
    };

    Ok(UpsertDistillationProfileRequest {
        profile_id: args
            .profile_id
            .map(DistillationProfileId::from_string)
            .unwrap_or_default(),
        scope_id,
        profile_level,
        status: args
            .status
            .as_deref()
            .map(parse_distillation_profile_status)
            .transpose()?
            .unwrap_or(DistillationProfileStatus::Active),
        name: args.name,
        prompt_text: args.prompt_text,
        focus_topics: args.focus_topics,
        prefer_memory_kinds: args
            .prefer_memory_kinds
            .iter()
            .map(|value| parse_memory_kind(value))
            .collect::<Result<Vec<_>, _>>()?,
        created_by: args.created_by.unwrap_or(created_by_default),
    })
}

fn build_distillation_session_override(
    args: &DistillPreviewToolArgs,
) -> Result<Option<DistillationSessionOverride>, McpError> {
    if args.prompt_text.is_none()
        && args.focus_topics.is_empty()
        && args.prefer_memory_kinds.is_empty()
    {
        return Ok(None);
    }

    let mut session_override =
        DistillationSessionOverride::new(args.prompt_text.clone().unwrap_or_default());
    for topic in &args.focus_topics {
        session_override = session_override.add_focus_topic(topic.clone());
    }
    for memory_kind in &args.prefer_memory_kinds {
        session_override = session_override.prefer_memory_kind(parse_memory_kind(memory_kind)?);
    }
    Ok(Some(session_override))
}

fn map_kernel_error(error: anyhow::Error) -> McpError {
    let message = error.to_string();
    if message.contains("denied by policy") {
        return McpError::forbidden(message);
    }
    if message.contains("required") || message.contains("invalid meat memory key") {
        return McpError::unauthorized(message);
    }
    if message.contains("forbidden") {
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

fn lifecycle_record_payload(record: &MemoryRecord) -> Value {
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

fn ensure_proposal_scope_access(
    context: &memory_domain::RequestContext,
    proposal: &MemoryProposal,
) -> Result<(), McpError> {
    if context.owner_scope_id != proposal.scope_id {
        return Err(McpError::forbidden(
            "proposal access forbidden for current meat memory key",
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

fn memory_proposal_payload(proposal: &MemoryProposal) -> Value {
    json!({
        "proposal_id": proposal.id.as_str(),
        "scope_id": proposal.scope_id.as_str(),
        "proposal_type": proposal.proposal_type.as_str(),
        "status": proposal.status.as_str(),
        "review_level": proposal.review_level.as_str(),
        "subject_memory_id": proposal.subject_memory_id.as_ref().map(|memory_id| memory_id.as_str()),
        "target_memory_ids": proposal
            .target_memory_ids
            .iter()
            .map(|memory_id| memory_id.as_str())
            .collect::<Vec<_>>(),
        "reason": proposal.reason,
        "evidence": proposal.evidence,
        "decided_by": proposal.decided_by,
        "decided_at": proposal.decided_at,
        "applied_at": proposal.applied_at,
        "created_at": proposal.created_at,
        "updated_at": proposal.updated_at,
    })
}

fn memory_version_payload(version: &TimelineVersion) -> Value {
    json!({
        "memory_id": version.memory_id.as_str(),
        "version": version.version,
        "title": version.title,
        "body": version.body,
        "change_kind": version.change_kind,
        "actor": version.actor,
        "reason": version.reason,
        "source_proposal_id": version.source_proposal_id.as_ref().map(|proposal_id| proposal_id.as_str()),
        "created_at": version.created_at,
    })
}

fn memory_relation_payload(relation: &MemoryRelation) -> Value {
    json!({
        "relation_id": relation.id.as_str(),
        "scope_id": relation.scope_id.as_str(),
        "from_memory_id": relation.from_memory_id.as_str(),
        "to_memory_id": relation.to_memory_id.as_str(),
        "relation_type": relation.relation_type.as_str(),
        "confidence": relation.confidence,
        "source_kind": relation.source_kind.as_str(),
        "source_proposal_id": relation.source_proposal_id.as_ref().map(|proposal_id| proposal_id.as_str()),
        "created_at": relation.created_at,
    })
}

fn timeline_audit_event_payload(event: &TimelineAuditEvent) -> Value {
    json!({
        "memory_id": event.memory_id.as_ref().map(|memory_id| memory_id.as_str()),
        "action": event.action,
        "actor": event.actor,
        "reason": event.reason,
        "created_at": event.created_at,
    })
}

fn timeline_event_payload(event: &TimelineEvent) -> Value {
    json!({
        "kind": timeline_event_kind_label(event.kind),
        "action": event.action,
        "occurred_at": event.occurred_at,
        "memory_id": event.memory_id.as_ref().map(|memory_id| memory_id.as_str()),
        "proposal_id": event.proposal_id.as_ref().map(|proposal_id| proposal_id.as_str()),
        "relation_id": event.relation_id.as_ref().map(|relation_id| relation_id.as_str()),
        "version": event.version,
    })
}

fn memory_timeline_payload(timeline: &MemoryTimeline) -> Value {
    json!({
        "memory_id": timeline.memory_id.as_str(),
        "versions": timeline.versions.iter().map(memory_version_payload).collect::<Vec<_>>(),
        "relations": timeline.relations.iter().map(memory_relation_payload).collect::<Vec<_>>(),
        "audit_events": timeline
            .audit_events
            .iter()
            .map(timeline_audit_event_payload)
            .collect::<Vec<_>>(),
        "proposals": timeline.proposals.iter().map(memory_proposal_payload).collect::<Vec<_>>(),
        "events": timeline.events.iter().map(timeline_event_payload).collect::<Vec<_>>(),
    })
}

fn rollback_payload(result: &RollbackMemoryResult) -> Value {
    json!({
        "memory_id": result.plan.memory_id.as_str(),
        "target_version": result.plan.target_version,
        "new_version": result.plan.new_version,
        "title": result.plan.title,
        "body": result.plan.body,
        "change_kind": result.plan.change_kind,
        "actor": result.plan.actor,
        "reason": result.plan.reason,
        "current_title": result.memory.title,
        "current_body": result.memory.body,
    })
}

fn distillation_profile_payload(profile: &DistillationProfile) -> Value {
    json!({
        "profile_id": profile.id.as_str(),
        "scope_id": profile.scope_id.as_ref().map(|scope_id| scope_id.as_str()),
        "profile_level": distillation_profile_level_label(profile.profile_level),
        "status": distillation_profile_status_label(profile.status),
        "name": profile.name,
        "prompt_text": profile.prompt_text,
        "focus_topics": profile.focus_topics,
        "prefer_memory_kinds": profile
            .prefer_memory_kinds
            .iter()
            .copied()
            .map(memory_kind_label)
            .collect::<Vec<_>>(),
        "created_by": profile.created_by,
        "created_at": profile.created_at,
        "updated_at": profile.updated_at,
    })
}

fn distillation_preview_payload(result: &PreviewDistillationResult) -> Value {
    json!({
        "run": {
            "run_id": result.preview.run.id.as_str(),
            "profile_id": result.preview.run.profile_id.as_ref().map(|profile_id| profile_id.as_str()),
            "scope_id": result.preview.run.scope_id.as_str(),
            "input_hash": result.preview.run.input_hash,
            "preview": result.preview.run.preview,
            "created_at": result.preview.run.created_at,
        },
        "profile": composed_distillation_profile_payload(&result.profile),
        "candidates": result
            .preview
            .candidates
            .iter()
            .map(distillation_candidate_payload)
            .collect::<Vec<_>>(),
        "discarded": result.preview.discarded,
        "warnings": result.preview.warnings,
        "stored_in_pg": result.stored_in_pg,
    })
}

fn composed_distillation_profile_payload(profile: &ComposedDistillationProfile) -> Value {
    json!({
        "scope_id": profile.scope_id.as_str(),
        "prompt_segments": profile
            .prompt_segments
            .iter()
            .map(distillation_prompt_segment_payload)
            .collect::<Vec<_>>(),
        "focus_topics": profile.focus_topics,
        "prefer_memory_kinds": profile
            .prefer_memory_kinds
            .iter()
            .copied()
            .map(memory_kind_label)
            .collect::<Vec<_>>(),
        "source_profile_ids": profile
            .source_profile_ids
            .iter()
            .map(|profile_id| profile_id.as_str())
            .collect::<Vec<_>>(),
        "safety_rules": profile.safety_rules,
    })
}

fn distillation_prompt_segment_payload(segment: &DistillationPromptSegment) -> Value {
    json!({
        "layer": segment.layer,
        "text": segment.text,
    })
}

fn distillation_candidate_payload(candidate: &DistillationCandidate) -> Value {
    json!({
        "title": candidate.title,
        "memory_kind": memory_kind_label(candidate.memory_kind),
        "body": candidate.body,
        "confidence": candidate.confidence,
        "why_keep": candidate.why_keep,
        "evidence_refs": candidate.evidence_refs,
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

fn timeline_event_kind_label(kind: TimelineEventKind) -> &'static str {
    match kind {
        TimelineEventKind::Version => "version",
        TimelineEventKind::Proposal => "proposal",
        TimelineEventKind::Relation => "relation",
        TimelineEventKind::Audit => "audit",
    }
}

fn distillation_profile_level_label(level: DistillationProfileLevel) -> &'static str {
    match level {
        DistillationProfileLevel::UserGlobal => "user_global",
        DistillationProfileLevel::Project => "project",
    }
}

fn distillation_profile_status_label(status: DistillationProfileStatus) -> &'static str {
    match status {
        DistillationProfileStatus::Active => "active",
        DistillationProfileStatus::Archived => "archived",
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;
