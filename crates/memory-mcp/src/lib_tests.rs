use super::{
    McpServer, McpTransport, TOOL_SPECS, ToolCallRequest, build_router, context_bundle_payload,
    entity_type_label, map_kernel_error, memory_kind_label, parse_arguments, parse_artifact_kind,
    parse_memory_kind, parse_scope_type, parse_sensitivity, parse_visibility, relation_state_label,
    relation_type_label, sensitivity_label, tool_supported, visibility_label,
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
    assert!(tool_supported("memory.lifecycle.inspect"));
    assert!(tool_supported("memory.lifecycle.status"));
    assert!(tool_supported("memory.lifecycle.forget"));
    assert!(tool_supported("memory.lifecycle.restore"));
    assert!(tool_supported("memory.lifecycle.report"));
    assert_eq!(TOOL_SPECS.len(), 17);
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
