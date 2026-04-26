use super::{
    DistillPreviewToolArgs, McpServer, McpTransport, ProfileUpsertToolArgs, TOOL_SPECS,
    ToolCallRequest, build_distillation_session_override, build_router, context_bundle_payload,
    distillation_preview_payload, distillation_profile_payload, entity_type_label,
    map_kernel_error, memory_kind_label, memory_proposal_payload, memory_timeline_payload,
    memory_version_payload, parse_arguments, parse_artifact_kind, parse_distillation_profile_level,
    parse_distillation_profile_status, parse_memory_kind, parse_review_actor_kind,
    parse_scope_type, parse_sensitivity, parse_visibility, profile_upsert_request,
    relation_state_label, relation_type_label, rollback_payload, sensitivity_label, tool_supported,
    visibility_label,
};
use axum::{body::Body, http::Request};
use memory_domain::{
    ArtifactKind, ContextBundle, DistillationProfile, DistillationProfileId,
    DistillationProfileLevel, DistillationProfileStatus, Entity, EntityType, KeyScopeKind,
    KeySourceKind, Memory, MemoryId, MemoryKind, MemoryProposal, MemoryRelation,
    MemoryRelationSourceKind, MemoryRelationType, ProposalId, ProposalStatus, ProposalType,
    Relation, RelationState, RelationType, ReviewLevel, ScopeId, ScopeType, Sensitivity,
    StorageMode, Visibility,
};
use memory_kernel::{
    ComposedDistillationProfile, CreateAccessKeyRequest, DistillationPreviewService,
    DistillationPromptSegment, Kernel, PreviewDistillationResult, ReviewActorKind,
    RollbackMemoryResult, RollbackPlan, TimelineAuditEvent, TimelineEvent, TimelineEventKind,
    TimelineVersion,
};
use serde::Deserialize;
use std::env;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;
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
    env::var("MEAT_MEMORY_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5433/meat_memory_dev".into())
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

    let entity = Entity::new(scope_id.clone(), EntityType::Project, "Meat Memory").expect("entity");
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
    assert!(tool_supported("memory.proposals.list"));
    assert!(tool_supported("memory.proposals.inspect"));
    assert!(tool_supported("memory.proposals.approve"));
    assert!(tool_supported("memory.proposals.reject"));
    assert!(tool_supported("memory.proposals.apply"));
    assert!(tool_supported("memory.versions.list"));
    assert!(tool_supported("memory.timeline.get"));
    assert!(tool_supported("memory.version.rollback"));
    assert!(tool_supported("memory.profile.list"));
    assert!(tool_supported("memory.profile.upsert"));
    assert!(tool_supported("memory.distill.preview"));
    assert!(tool_supported("memory.lifecycle.inspect"));
    assert!(tool_supported("memory.lifecycle.status"));
    assert!(tool_supported("memory.lifecycle.forget"));
    assert!(tool_supported("memory.lifecycle.restore"));
    assert!(tool_supported("memory.lifecycle.report"));
    assert_eq!(TOOL_SPECS.len(), 28);
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
    if !local_pg_test_port_available() {
        return;
    }
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

#[tokio::test]
async fn dispatch_lifecycle_tools_inspect_forget_restore_and_report() {
    if !local_pg_test_port_available() {
        return;
    }
    let tempdir = tempdir().unwrap();
    let (server, kernel) = test_server_with_pg(tempdir.path()).await;
    let scope_id = ScopeId::new();
    let raw_key = create_test_key(&kernel, scope_id.as_str()).await;

    let remembered = server
        .dispatch(ToolCallRequest {
            name: "memory.remember".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str(),
                "title": "MCP lifecycle",
                "body": "Lifecycle MCP tools inspect, forget, restore, and report.",
                "memory_kind": "summary",
                "source_refs": ["agent-context://ctx_mcp_lifecycle"],
                "key": raw_key
            }),
        })
        .await
        .unwrap();

    let inspected = server
        .dispatch(ToolCallRequest {
            name: "memory.lifecycle.inspect".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str(),
                "memory_id": remembered.data["memory_id"],
                "query": "lifecycle"
            }),
        })
        .await
        .unwrap();
    assert_eq!(inspected.tool, "memory.lifecycle.inspect");
    assert_eq!(inspected.data["record"]["source_kind"], "conversation");
    assert!(
        inspected.data["explanation"]["reason"]
            .as_str()
            .unwrap()
            .contains("query matched")
    );

    let forgotten = server
        .dispatch(ToolCallRequest {
            name: "memory.lifecycle.forget".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str(),
                "memory_id": remembered.data["memory_id"],
                "reason": "mcp test",
                "key": raw_key
            }),
        })
        .await
        .unwrap();
    assert_eq!(forgotten.data["record"]["status"], "forgotten");

    let restored = server
        .dispatch(ToolCallRequest {
            name: "memory.lifecycle.restore".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str(),
                "memory_id": remembered.data["memory_id"],
                "reason": "mcp restore",
                "key": raw_key
            }),
        })
        .await
        .unwrap();
    assert_eq!(restored.data["record"]["status"], "active");

    let report = server
        .dispatch(ToolCallRequest {
            name: "memory.lifecycle.report".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str()
            }),
        })
        .await
        .unwrap();
    assert_eq!(report.tool, "memory.lifecycle.report");
    assert_eq!(report.data["total"], 1);
    assert_eq!(report.data["active"], 1);
}

#[tokio::test]
async fn dispatch_v28_governance_tools_cover_policy_timeline_rollback_profile_and_preview() {
    if !local_pg_test_port_available() {
        return;
    }
    let tempdir = tempdir().unwrap();
    let (server, kernel) = test_server_with_pg(tempdir.path()).await;
    let scope_id = ScopeId::new();
    let raw_key = create_test_key(&kernel, scope_id.as_str()).await;
    let store = memory_store_pg::PgStore::connect(&test_database_url())
        .await
        .unwrap();
    store
        .seed_scope(
            &scope_id,
            scope_id.as_str(),
            &format!("default/scopes/{}", scope_id.as_str()),
        )
        .await
        .unwrap();

    let mut memory = Memory::new(
        scope_id.clone(),
        MemoryKind::Decision,
        "MCP V2.8 original",
        "Original body",
    )
    .unwrap();
    memory.activate().unwrap();
    store.insert_memory(&memory).await.unwrap();
    memory.title = "MCP V2.8 edited".to_string();
    memory.body = "Edited body".to_string();
    store.upsert_memory(&memory).await.unwrap();

    let mut proposal = MemoryProposal::new(
        scope_id.clone(),
        ProposalType::Merge,
        ReviewLevel::Required,
        "MCP required proposal must honor user authorization",
    )
    .unwrap()
    .with_subject_memory(memory.id.clone());
    proposal.id = ProposalId::from_string(format!("prp_mcp_v28_{}", scope_id.as_str()));
    proposal
        .add_evidence("seeded MCP V2.8 proposal".to_string())
        .unwrap();
    store.upsert_memory_proposal(&proposal).await.unwrap();

    let listed = server
        .dispatch(ToolCallRequest {
            name: "memory.proposals.list".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str(),
                "limit": 10
            }),
        })
        .await
        .unwrap();
    assert_eq!(listed.tool, "memory.proposals.list");
    assert_eq!(listed.data["proposal_count"], 1);

    let inspected = server
        .dispatch(ToolCallRequest {
            name: "memory.proposals.inspect".to_string(),
            arguments: serde_json::json!({
                "proposal_id": proposal.id.as_str()
            }),
        })
        .await
        .unwrap();
    assert_eq!(inspected.data["proposal"]["review_level"], "required");

    let denied = server
        .dispatch(ToolCallRequest {
            name: "memory.proposals.approve".to_string(),
            arguments: serde_json::json!({
                "key": raw_key,
                "proposal_id": proposal.id.as_str(),
                "actor_kind": "agent"
            }),
        })
        .await
        .expect_err("agent approval without explicit user authorization should fail");
    assert_eq!(denied.status, axum::http::StatusCode::FORBIDDEN);

    let mut reject_proposal = MemoryProposal::new(
        scope_id.clone(),
        ProposalType::Archive,
        ReviewLevel::Suggested,
        "MCP reject proposal covers user review decline",
    )
    .unwrap()
    .with_subject_memory(memory.id.clone());
    reject_proposal.id =
        ProposalId::from_string(format!("prp_mcp_v28_reject_{}", scope_id.as_str()));
    store
        .upsert_memory_proposal(&reject_proposal)
        .await
        .unwrap();
    let rejected = server
        .dispatch(ToolCallRequest {
            name: "memory.proposals.reject".to_string(),
            arguments: serde_json::json!({
                "key": raw_key,
                "proposal_id": reject_proposal.id.as_str(),
                "actor": "mcp-reviewer"
            }),
        })
        .await
        .unwrap();
    assert_eq!(rejected.data["proposal"]["status"], "rejected");
    assert_eq!(rejected.data["proposal"]["decided_by"], "mcp-reviewer");

    let approved = server
        .dispatch(ToolCallRequest {
            name: "memory.proposals.approve".to_string(),
            arguments: serde_json::json!({
                "key": raw_key,
                "proposal_id": proposal.id.as_str(),
                "actor_kind": "agent",
                "user_authorized": true
            }),
        })
        .await
        .unwrap();
    assert_eq!(approved.data["proposal"]["status"], "approved");

    let applied = server
        .dispatch(ToolCallRequest {
            name: "memory.proposals.apply".to_string(),
            arguments: serde_json::json!({
                "key": raw_key,
                "proposal_id": proposal.id.as_str(),
                "actor_kind": "agent",
                "user_authorized": true
            }),
        })
        .await
        .unwrap();
    assert_eq!(applied.data["proposal"]["status"], "applied");

    let versions = server
        .dispatch(ToolCallRequest {
            name: "memory.versions.list".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str(),
                "memory_id": memory.id.as_str(),
                "limit": 10
            }),
        })
        .await
        .unwrap();
    assert!(versions.data["versions"].as_array().unwrap().len() >= 3);

    let timeline = server
        .dispatch(ToolCallRequest {
            name: "memory.timeline.get".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str(),
                "memory_id": memory.id.as_str(),
                "limit": 10
            }),
        })
        .await
        .unwrap();
    assert!(
        timeline.data["timeline"]["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["kind"] == "proposal")
    );

    let rollback = server
        .dispatch(ToolCallRequest {
            name: "memory.version.rollback".to_string(),
            arguments: serde_json::json!({
                "key": raw_key,
                "scope_id": scope_id.as_str(),
                "memory_id": memory.id.as_str(),
                "target_version": 1,
                "reason": "restore MCP original"
            }),
        })
        .await
        .unwrap();
    assert_eq!(rollback.data["target_version"], 1);
    assert_eq!(rollback.data["current_title"], "MCP V2.8 original");

    let other_scope_id = ScopeId::new();
    let rollback_forbidden = server
        .dispatch(ToolCallRequest {
            name: "memory.version.rollback".to_string(),
            arguments: serde_json::json!({
                "key": raw_key,
                "scope_id": other_scope_id.as_str(),
                "memory_id": memory.id.as_str(),
                "target_version": 1,
                "reason": "cross-scope rollback should fail"
            }),
        })
        .await
        .unwrap_err();
    assert_eq!(rollback_forbidden.status, axum::http::StatusCode::FORBIDDEN);

    let profile_id = format!("dpf_mcp_v28_{}", scope_id.as_str());
    let profile = server
        .dispatch(ToolCallRequest {
            name: "memory.profile.upsert".to_string(),
            arguments: serde_json::json!({
                "key": raw_key,
                "profile_id": profile_id,
                "scope_id": scope_id.as_str(),
                "profile_level": "project",
                "name": "MCP V2.8 profile",
                "prompt_text": "Prefer proposal governance",
                "focus_topics": ["proposal"],
                "prefer_memory_kinds": ["decision"]
            }),
        })
        .await
        .unwrap();
    assert_eq!(profile.data["profile"]["profile_level"], "project");
    assert_eq!(
        profile.data["profile"]["prefer_memory_kinds"][0],
        "decision"
    );

    let global_profile = server
        .dispatch(ToolCallRequest {
            name: "memory.profile.upsert".to_string(),
            arguments: serde_json::json!({
                "key": raw_key,
                "profile_level": "global",
                "name": "MCP V2.8 global profile",
                "prompt_text": "Prefer stable user guidance"
            }),
        })
        .await
        .unwrap();
    assert_eq!(
        global_profile.data["profile"]["profile_level"],
        "user_global"
    );
    assert_eq!(
        global_profile.data["profile"]["scope_id"],
        serde_json::Value::Null
    );

    let profile_forbidden = server
        .dispatch(ToolCallRequest {
            name: "memory.profile.upsert".to_string(),
            arguments: serde_json::json!({
                "key": raw_key,
                "scope_id": other_scope_id.as_str(),
                "profile_level": "project",
                "name": "Cross scope profile",
                "prompt_text": "Should be rejected"
            }),
        })
        .await
        .unwrap_err();
    assert_eq!(profile_forbidden.status, axum::http::StatusCode::FORBIDDEN);

    let profiles = server
        .dispatch(ToolCallRequest {
            name: "memory.profile.list".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str(),
                "limit": 10
            }),
        })
        .await
        .unwrap();
    assert_eq!(profiles.data["profile_count"], 1);

    let preview = server
        .dispatch(ToolCallRequest {
            name: "memory.distill.preview".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str(),
                "input": "MCP V2.8 keeps proposal-first governance.",
                "evidence_refs": ["mcp://v28"],
                "prompt_text": "Prefer decision memories",
                "prefer_memory_kinds": ["decision"]
            }),
        })
        .await
        .unwrap();
    assert_eq!(preview.data["candidates"][0]["memory_kind"], "decision");
    assert_eq!(preview.data["stored_in_pg"], true);
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

    let actor_kind_cases = [
        ("user", ReviewActorKind::User),
        ("agent", ReviewActorKind::Agent),
        ("system", ReviewActorKind::System),
    ];
    for (raw, expected) in actor_kind_cases {
        assert_eq!(parse_review_actor_kind(raw).unwrap(), expected);
    }
    assert!(parse_review_actor_kind("bot").is_err());

    assert_eq!(
        parse_distillation_profile_level("user_global").unwrap(),
        DistillationProfileLevel::UserGlobal
    );
    assert_eq!(
        parse_distillation_profile_level("global").unwrap(),
        DistillationProfileLevel::UserGlobal
    );
    assert_eq!(
        parse_distillation_profile_level("project").unwrap(),
        DistillationProfileLevel::Project
    );
    assert!(parse_distillation_profile_level("workspace").is_err());

    assert_eq!(
        parse_distillation_profile_status("active").unwrap(),
        DistillationProfileStatus::Active
    );
    assert_eq!(
        parse_distillation_profile_status("archived").unwrap(),
        DistillationProfileStatus::Archived
    );
    assert!(parse_distillation_profile_status("deleted").is_err());
}

#[test]
fn v28_request_builders_cover_profile_defaults_and_session_override() {
    let default_scope_id = ScopeId::from_string("scp_mcp_v28_builder");
    let project_request = profile_upsert_request(
        ProfileUpsertToolArgs {
            profile_id: Some("dpf_mcp_v28_project_builder".to_string()),
            scope_id: None,
            profile_level: Some("project".to_string()),
            level: None,
            status: Some("archived".to_string()),
            name: "Project profile".to_string(),
            prompt_text: "Keep decisions".to_string(),
            focus_topics: vec!["proposal".to_string()],
            prefer_memory_kinds: vec!["decision".to_string()],
            created_by: None,
            key: None,
        },
        "mcp-default-actor".to_string(),
        default_scope_id.clone(),
    )
    .unwrap();
    assert_eq!(project_request.scope_id, Some(default_scope_id.clone()));
    assert_eq!(project_request.status, DistillationProfileStatus::Archived);
    assert_eq!(project_request.created_by, "mcp-default-actor");
    assert_eq!(
        project_request.prefer_memory_kinds,
        vec![MemoryKind::Decision]
    );

    let global_request = profile_upsert_request(
        ProfileUpsertToolArgs {
            profile_id: None,
            scope_id: None,
            profile_level: None,
            level: Some("global".to_string()),
            status: None,
            name: "Global profile".to_string(),
            prompt_text: "Keep stable preferences".to_string(),
            focus_topics: Vec::new(),
            prefer_memory_kinds: Vec::new(),
            created_by: Some("human".to_string()),
            key: None,
        },
        "mcp-default-actor".to_string(),
        default_scope_id,
    )
    .unwrap();
    assert_eq!(
        global_request.profile_level,
        DistillationProfileLevel::UserGlobal
    );
    assert_eq!(global_request.scope_id, None);
    assert_eq!(global_request.status, DistillationProfileStatus::Active);
    assert_eq!(global_request.created_by, "human");

    let no_override = build_distillation_session_override(&DistillPreviewToolArgs {
        scope_id: None,
        input: "Keep this".to_string(),
        evidence_refs: vec!["mcp://builder".to_string()],
        prompt_text: None,
        focus_topics: Vec::new(),
        prefer_memory_kinds: Vec::new(),
    })
    .unwrap();
    assert!(no_override.is_none());

    let override_profile = build_distillation_session_override(&DistillPreviewToolArgs {
        scope_id: None,
        input: "Keep this".to_string(),
        evidence_refs: vec!["mcp://builder".to_string()],
        prompt_text: Some("Prefer explicit review boundaries".to_string()),
        focus_topics: vec!["review".to_string()],
        prefer_memory_kinds: vec!["constraint".to_string()],
    })
    .unwrap()
    .unwrap();
    assert_eq!(
        override_profile.prompt_text,
        "Prefer explicit review boundaries"
    );
    assert_eq!(override_profile.focus_topics, vec!["review"]);
    assert_eq!(
        override_profile.prefer_memory_kinds,
        vec![MemoryKind::Constraint]
    );
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

#[test]
fn v28_payload_helpers_render_governance_and_distillation_shapes() {
    let scope_id = ScopeId::from_string("scp_mcp_v28_payload");
    let memory_id = MemoryId::from_string("mem_mcp_v28_payload");
    let proposal_id = ProposalId::from_string("prp_mcp_v28_payload");
    let mut proposal = MemoryProposal::new(
        scope_id.clone(),
        ProposalType::Merge,
        ReviewLevel::Suggested,
        "near duplicate should be reviewed",
    )
    .unwrap()
    .with_subject_memory(memory_id.clone());
    proposal.id = proposal_id.clone();
    proposal.status = ProposalStatus::Approved;
    proposal.add_target_memory(MemoryId::from_string("mem_mcp_target"));
    proposal
        .add_evidence("same normalized title".to_string())
        .unwrap();
    let proposal_payload = memory_proposal_payload(&proposal);
    assert_eq!(proposal_payload["proposal_id"], "prp_mcp_v28_payload");
    assert_eq!(proposal_payload["status"], "approved");
    assert_eq!(proposal_payload["target_memory_ids"][0], "mem_mcp_target");

    let version = TimelineVersion {
        memory_id: memory_id.clone(),
        version: 2,
        title: "New title".to_string(),
        body: "New body".to_string(),
        change_kind: "edit".to_string(),
        actor: "agent".to_string(),
        reason: Some("payload test".to_string()),
        source_proposal_id: Some(proposal_id.clone()),
        created_at: time::macros::datetime!(2025-03-04 05:06:07 UTC),
    };
    let version_payload = memory_version_payload(&version);
    assert_eq!(version_payload["version"], 2);
    assert_eq!(version_payload["source_proposal_id"], "prp_mcp_v28_payload");

    let relation = MemoryRelation::new(
        scope_id.clone(),
        memory_id.clone(),
        MemoryId::from_string("mem_old"),
        MemoryRelationType::Supersedes,
        MemoryRelationSourceKind::Agent,
    )
    .with_source_proposal_id(proposal_id.clone());
    let audit = TimelineAuditEvent {
        memory_id: Some(memory_id.clone()),
        action: "memory.rollback".to_string(),
        actor: "agent".to_string(),
        reason: Some("payload rollback".to_string()),
        created_at: time::macros::datetime!(2025-03-04 06:06:07 UTC),
    };
    let event = TimelineEvent {
        kind: TimelineEventKind::Proposal,
        action: "proposal.approved".to_string(),
        occurred_at: time::macros::datetime!(2025-03-04 05:06:07 UTC),
        memory_id: Some(memory_id.clone()),
        proposal_id: Some(proposal_id),
        relation_id: None,
        version: None,
    };
    let timeline_payload = memory_timeline_payload(&memory_kernel::MemoryTimeline {
        memory_id: memory_id.clone(),
        versions: vec![version],
        relations: vec![relation],
        audit_events: vec![audit],
        proposals: vec![proposal],
        events: vec![event],
    });
    assert_eq!(timeline_payload["memory_id"], "mem_mcp_v28_payload");
    assert_eq!(
        timeline_payload["relations"][0]["relation_type"],
        "supersedes"
    );
    assert_eq!(timeline_payload["events"][0]["kind"], "proposal");

    let rollback = rollback_payload(&RollbackMemoryResult {
        plan: RollbackPlan {
            memory_id: memory_id.clone(),
            target_version: 1,
            new_version: 3,
            title: "Old title".to_string(),
            body: "Old body".to_string(),
            change_kind: "rollback",
            actor: "agent".to_string(),
            reason: "payload rollback".to_string(),
        },
        memory: Memory::new(
            scope_id.clone(),
            MemoryKind::Decision,
            "Old title",
            "Old body",
        )
        .unwrap(),
    });
    assert_eq!(rollback["new_version"], 3);
    assert_eq!(rollback["change_kind"], "rollback");

    let mut profile = DistillationProfile::new_project(
        scope_id.clone(),
        "Governance profile",
        "Keep proposal-first decisions",
        "agent",
    )
    .unwrap()
    .add_focus_topic("proposal")
    .unwrap()
    .prefer_memory_kind(MemoryKind::Decision);
    profile.id = DistillationProfileId::from_string("dpf_mcp_v28_payload");
    let profile_payload = distillation_profile_payload(&profile);
    assert_eq!(profile_payload["profile_id"], "dpf_mcp_v28_payload");
    assert_eq!(profile_payload["profile_level"], "project");
    assert_eq!(profile_payload["prefer_memory_kinds"][0], "decision");

    let composed = ComposedDistillationProfile {
        scope_id: scope_id.clone(),
        prompt_segments: vec![DistillationPromptSegment {
            layer: "system_base",
            text: "Preserve evidence".to_string(),
        }],
        focus_topics: vec!["proposal".to_string()],
        prefer_memory_kinds: vec![MemoryKind::Decision],
        source_profile_ids: vec![DistillationProfileId::from_string("dpf_mcp_v28_payload")],
        safety_rules: vec!["must preserve evidence references"],
    };
    let preview = DistillationPreviewService::preview(
        scope_id,
        "Keep proposal-first governance.",
        &["mcp://payload".to_string()],
        &composed,
    )
    .unwrap();
    let preview_payload = distillation_preview_payload(&PreviewDistillationResult {
        preview,
        profile: composed,
        stored_in_pg: false,
    });
    assert_eq!(preview_payload["stored_in_pg"], false);
    assert_eq!(preview_payload["candidates"][0]["memory_kind"], "decision");
    assert_eq!(preview_payload["profile"]["focus_topics"][0], "proposal");
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
