use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use memory_domain::ScopeId;
use memory_kernel::Kernel;
use memory_mcp::{McpServer, ToolCallRequest, build_router};
use std::{env, sync::Arc};
use tempfile::tempdir;
use tower::ServiceExt;

fn test_database_url() -> String {
    env::var("MEAT_MEMORY_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5433/meat_memory_dev".into())
}

async fn build_test_server(root: &std::path::Path) -> McpServer {
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

    McpServer::new(
        ScopeId::from_string("scp_mcp_default"),
        "meat-memory",
        "0.1.0",
        kernel,
    )
}

#[tokio::test]
async fn mcp_dispatches_remember_search_fetch_context_and_publish() {
    let tempdir = tempdir().unwrap();
    let server = build_test_server(tempdir.path()).await;
    let scope_id = ScopeId::new();

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
                "target_visibility": "team"
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
    let tempdir = tempdir().unwrap();
    let server = build_test_server(tempdir.path()).await;
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
async fn mcp_stdio_transport_serializes_errors() {
    let tempdir = tempdir().unwrap();
    let server = build_test_server(tempdir.path()).await;

    let response = server
        .handle_stdio_message(r#"{"name":"memory.unknown","arguments":{}}"#)
        .await;
    let payload = serde_json::from_str::<serde_json::Value>(&response).unwrap();

    assert_eq!(payload["error"]["code"], "unsupported_tool");
}
