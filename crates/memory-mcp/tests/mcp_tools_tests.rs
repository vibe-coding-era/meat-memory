use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    response::IntoResponse,
};
use memory_domain::{
    KeyScopeKind, KeySourceKind, MemorySource, ScopeId, SourceSyncMode, StorageMode,
};
use memory_kernel::{CreateAccessKeyRequest, Kernel};
use memory_mcp::{McpServer, ToolCallRequest, build_router};
use std::{
    env,
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    sync::{Arc, OnceLock},
    time::Duration,
};
use tempfile::tempdir;
use tokio::sync::{Mutex, OwnedMutexGuard};
use tower::ServiceExt;

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

async fn mcp_test_guard() -> OwnedMutexGuard<()> {
    static LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();
    LOCK.get_or_init(|| Arc::new(Mutex::new(())))
        .clone()
        .lock_owned()
        .await
}

async fn build_test_server(root: &std::path::Path) -> (McpServer, Arc<Kernel>) {
    let kernel = Arc::new(
        Kernel::builder()
            .with_postgres_url(&test_database_url())
            .await
            .unwrap()
            .with_markdown_root(root)
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

#[tokio::test]
async fn mcp_dispatches_remember_search_fetch_context_and_publish() {
    if !local_pg_test_port_available() {
        return;
    }
    let _guard = mcp_test_guard().await;
    let tempdir = tempdir().unwrap();
    let (server, kernel) = build_test_server(tempdir.path()).await;
    let scope_id = ScopeId::new();
    let raw_key = create_test_key(&kernel, scope_id.as_str()).await;

    let remember = server
        .dispatch(ToolCallRequest {
            name: "memory.remember".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str(),
                "title": "MCP gateway relation",
                "body": "Project Meat Memory uses Service Gateway for agent context search.",
                "memory_kind": "decision"
            }),
        })
        .await
        .unwrap();
    let memory_id = remember.data["memory_id"].as_str().unwrap().to_string();

    assert_eq!(remember.tool, "memory.remember");
    assert_eq!(remember.data["scope_id"], scope_id.as_str());
    assert_eq!(remember.data["memory_kind"], "decision");

    let search = server
        .dispatch(ToolCallRequest {
            name: "memory.search".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str(),
                "query": "gateway agent",
                "limit": 5
            }),
        })
        .await
        .unwrap();

    assert_eq!(search.data["memory_count"], 1);
    assert!(search.data["entity_count"].as_u64().unwrap() >= 2);

    let fetch_context = server
        .dispatch(ToolCallRequest {
            name: "memory.fetch_context".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str(),
                "query": "context search",
                "limit": 5
            }),
        })
        .await
        .unwrap();

    assert_eq!(fetch_context.tool, "memory.fetch_context");
    assert_eq!(fetch_context.data["memory_count"], 1);

    let publish = server
        .dispatch(ToolCallRequest {
            name: "memory.publish".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str(),
                "memory_id": memory_id,
                "target_visibility": "team",
                "key": raw_key
            }),
        })
        .await
        .unwrap();

    assert_eq!(publish.tool, "memory.publish");
    assert_eq!(publish.data["visibility"], "team");
    assert_eq!(publish.data["wrote_pg"], true);
    assert_eq!(publish.data["wrote_markdown"], true);
}

#[tokio::test]
async fn mcp_http_transport_accepts_tool_calls() {
    if !local_pg_test_port_available() {
        return;
    }
    let _guard = mcp_test_guard().await;
    let tempdir = tempdir().unwrap();
    let (server, _kernel) = build_test_server(tempdir.path()).await;
    let app = build_router(server);
    let scope_id = ScopeId::new();

    let response = app
        .oneshot(
            Request::post("/mcp/tools/call")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "name": "memory.remember",
                        "arguments": {
                            "scope_id": scope_id.as_str(),
                            "title": "HTTP MCP remember",
                            "body": "Project Meat Memory uses HTTP MCP transport for remember.",
                            "memory_kind": "decision"
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap();

    assert_eq!(payload["tool"], "memory.remember");
    assert_eq!(payload["data"]["scope_id"], scope_id.as_str());
}

#[tokio::test]
async fn mcp_agent_context_tools_upsert_list_promote_and_delete() {
    if !local_pg_test_port_available() {
        return;
    }
    let _guard = mcp_test_guard().await;
    let tempdir = tempdir().unwrap();
    let (server, kernel) = build_test_server(tempdir.path()).await;
    let scope_id = ScopeId::new();
    let raw_key = create_test_key(&kernel, scope_id.as_str()).await;
    let session_id = format!("mcp-session-{}", scope_id.as_str());

    let upsert = server
        .dispatch(ToolCallRequest {
            name: "memory.context.upsert".to_string(),
            arguments: serde_json::json!({
                "key": raw_key,
                "scope_id": scope_id.as_str(),
                "session_id": session_id,
                "task_id": "mcp-context-task",
                "title": "MCP agent scratchpad",
                "body": "MCP agent context captures a short-term tool result.",
                "labels": ["mcp", "short-term"]
            }),
        })
        .await
        .unwrap();
    let context_id = upsert.data["context_id"].as_str().unwrap().to_string();
    assert_eq!(upsert.tool, "memory.context.upsert");
    assert_eq!(upsert.data["scope_id"], scope_id.as_str());
    assert_eq!(upsert.data["layer"], "short_term");

    let list = server
        .dispatch(ToolCallRequest {
            name: "memory.context.list".to_string(),
            arguments: serde_json::json!({
                "key": raw_key,
                "scope_id": scope_id.as_str(),
                "session_id": session_id,
                "task_id": "mcp-context-task",
                "limit": 5
            }),
        })
        .await
        .unwrap();
    assert_eq!(list.data["context_count"], 1);
    assert_eq!(list.data["contexts"][0]["context_id"], context_id);

    let promoted = server
        .dispatch(ToolCallRequest {
            name: "memory.context.promote".to_string(),
            arguments: serde_json::json!({
                "key": raw_key,
                "context_id": context_id,
                "memory_kind": "summary",
                "visibility": "private",
                "sensitivity": "internal"
            }),
        })
        .await
        .unwrap();
    assert_eq!(promoted.tool, "memory.context.promote");
    assert_eq!(promoted.data["scope_id"], scope_id.as_str());
    assert_eq!(promoted.data["memory_kind"], "summary");
    assert!(
        promoted.data["body"]
            .as_str()
            .unwrap()
            .contains("short-term tool result")
    );

    let deleted = server
        .dispatch(ToolCallRequest {
            name: "memory.context.delete".to_string(),
            arguments: serde_json::json!({
                "key": raw_key,
                "context_id": context_id
            }),
        })
        .await
        .unwrap();
    assert_eq!(deleted.data["deleted"], true);
}

#[tokio::test]
async fn mcp_project_document_tools_sync_search_and_list_conflicts() {
    if !local_pg_test_port_available() {
        return;
    }
    let _guard = mcp_test_guard().await;
    let memory_root = tempdir().unwrap();
    let docs_root = tempdir().unwrap();
    std::fs::write(
        docs_root.path().join("README.md"),
        "# MCP Project README\nMCP docs sync imports project documentation.",
    )
    .unwrap();
    std::fs::write(
        docs_root.path().join("notes.txt"),
        "MCP docs sync keeps conflict reporting explicit.",
    )
    .unwrap();

    let (server, kernel) = build_test_server(memory_root.path()).await;
    let scope_id = ScopeId::new();
    let raw_key = create_test_key(&kernel, scope_id.as_str()).await;
    let source = MemorySource::new(
        "local_docs",
        "MCP project docs",
        scope_id.as_str(),
        scope_id.clone(),
    )
    .unwrap()
    .with_local_root(docs_root.path().to_string_lossy())
    .unwrap()
    .with_sync_mode(SourceSyncMode::IndexOnly);
    let source_id = source.id.as_str().to_string();
    kernel.upsert_memory_source(source, None).await.unwrap();

    let sync = server
        .dispatch(ToolCallRequest {
            name: "memory.docs.sync".to_string(),
            arguments: serde_json::json!({
                "key": raw_key,
                "source_id": source_id,
                "scope_id": scope_id.as_str(),
                "dry_run": false
            }),
        })
        .await
        .unwrap();
    assert_eq!(sync.tool, "memory.docs.sync");
    assert_eq!(sync.data["dry_run"], false);
    assert_eq!(sync.data["planned_documents"].as_array().unwrap().len(), 2);
    assert_eq!(sync.data["imported"].as_array().unwrap().len(), 2);

    let search = server
        .dispatch(ToolCallRequest {
            name: "memory.docs.search".to_string(),
            arguments: serde_json::json!({
                "key": raw_key,
                "source_id": source_id,
                "query": "README",
                "limit": 10
            }),
        })
        .await
        .unwrap();
    assert_eq!(search.data["document_count"], 1);
    assert_eq!(search.data["documents"][0]["title"], "MCP Project README");

    let conflicts = server
        .dispatch(ToolCallRequest {
            name: "memory.docs.conflicts".to_string(),
            arguments: serde_json::json!({
                "key": raw_key,
                "source_id": source_id,
                "limit": 10
            }),
        })
        .await
        .unwrap();
    assert_eq!(conflicts.data["conflict_count"], 0);
}

#[tokio::test]
async fn mcp_supports_chinese_memory_search_and_publish_flow() {
    if !local_pg_test_port_available() {
        return;
    }
    let _guard = mcp_test_guard().await;
    let tempdir = tempdir().unwrap();
    let (server, kernel) = build_test_server(tempdir.path()).await;
    let scope_id = ScopeId::new();
    let raw_key = create_test_key(&kernel, scope_id.as_str()).await;

    let remember = server
        .dispatch(ToolCallRequest {
            name: "memory.remember".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str(),
                "title": "中文验收约定",
                "body": "项目 `服务网关` 依赖 `PostgreSQL`，`服务网关` 记录 `发布手册`。发布前必须通过回归与验收。",
                "memory_kind": "procedure"
            }),
        })
        .await
        .unwrap();
    let memory_id = remember.data["memory_id"].as_str().unwrap().to_string();

    let search = server
        .dispatch(ToolCallRequest {
            name: "memory.search".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str(),
                "query": "回归 验收",
                "limit": 5
            }),
        })
        .await
        .unwrap();

    assert_eq!(search.data["memory_count"], 1);
    assert!(search.data["entity_count"].as_u64().unwrap() >= 2);
    assert!(search.data["relation_count"].as_u64().unwrap() >= 1);

    let publish = server
        .dispatch(ToolCallRequest {
            name: "memory.publish".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str(),
                "memory_id": memory_id,
                "target_visibility": "project",
                "key": raw_key
            }),
        })
        .await
        .unwrap();

    assert_eq!(publish.data["visibility"], "project");
    assert_eq!(publish.data["wrote_pg"], true);
    assert_eq!(publish.data["wrote_markdown"], true);
}

#[tokio::test]
async fn mcp_stdio_transport_serializes_errors() {
    if !local_pg_test_port_available() {
        return;
    }
    let _guard = mcp_test_guard().await;
    let tempdir = tempdir().unwrap();
    let (server, _kernel) = build_test_server(tempdir.path()).await;

    let response = server
        .handle_stdio_message(r#"{"name":"memory.unknown","arguments":{}}"#)
        .await;
    let payload = serde_json::from_str::<serde_json::Value>(&response).unwrap();

    assert_eq!(payload["error"]["code"], "unsupported_tool");
}

#[tokio::test]
async fn mcp_dispatches_promote_and_preserves_scope_lineage() {
    if !local_pg_test_port_available() {
        return;
    }
    let _guard = mcp_test_guard().await;
    let tempdir = tempdir().unwrap();
    let (server, kernel) = build_test_server(tempdir.path()).await;
    let raw_key = create_test_key(&kernel, "scp_user_mcp_alice").await;

    let remembered = server
        .dispatch(ToolCallRequest {
            name: "memory.remember".to_string(),
            arguments: serde_json::json!({
                "scope_id": "scp_user_mcp_alice",
                "title": "Alice MCP 发布凭证",
                "body": "token: abc123 联系人 alice@example.com",
                "memory_kind": "procedure",
                "visibility": "private",
                "sensitivity": "restricted"
            }),
        })
        .await
        .unwrap();

    let promoted = server
        .dispatch(ToolCallRequest {
            name: "memory.promote".to_string(),
            arguments: serde_json::json!({
                "source_scope_id": "scp_user_mcp_alice",
                "memory_id": remembered.data["memory_id"],
                "source_scope_type": "user",
                "target_scope_id": "scp_project_mcp_demo",
                "target_scope_type": "project",
                "target_visibility": "project",
                "key": raw_key
            }),
        })
        .await
        .unwrap();

    assert_eq!(promoted.tool, "memory.promote");
    assert_eq!(promoted.data["scope_id"], "scp_project_mcp_demo");
    assert_eq!(promoted.data["owner_scope_id"], "scp_user_mcp_alice");
    assert_eq!(
        promoted.data["published_from_scope_id"],
        "scp_user_mcp_alice"
    );
    assert_eq!(promoted.data["memory_state"], "candidate");
    assert_eq!(promoted.data["visibility"], "project");
    assert!(
        promoted.data["body"]
            .as_str()
            .unwrap()
            .contains("[REDACTED]")
    );
}

#[tokio::test]
async fn mcp_publish_requires_key_for_sensitive_tool() {
    if !local_pg_test_port_available() {
        return;
    }
    let _guard = mcp_test_guard().await;
    let tempdir = tempdir().unwrap();
    let (server, _kernel) = build_test_server(tempdir.path()).await;
    let scope_id = ScopeId::new();

    let remembered = server
        .dispatch(ToolCallRequest {
            name: "memory.remember".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str(),
                "title": "Publish target",
                "body": "Need key to publish",
                "memory_kind": "fact"
            }),
        })
        .await
        .unwrap();

    let error = server
        .dispatch(ToolCallRequest {
            name: "memory.publish".to_string(),
            arguments: serde_json::json!({
                "scope_id": scope_id.as_str(),
                "memory_id": remembered.data["memory_id"],
                "target_visibility": "project"
            }),
        })
        .await
        .expect_err("publish without key should fail");

    assert_eq!(error.into_response().status(), StatusCode::UNAUTHORIZED);
}
