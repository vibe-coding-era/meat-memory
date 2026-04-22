use super::{
    ApiError, ApiFeatureFlags, ApiMetadata, HttpAppState, api_error_from_anyhow,
    build_console_page, build_router, escape_html, has_route, memory_kind_label, memory_to_summary,
    parse_artifact_kind, parse_memory_kind, parse_sensitivity, parse_visibility,
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
    CapabilityRoute, DeploymentTarget, ModelCapability, ModelDescriptor, ModelRegistry, Provider,
    ProviderDescriptor,
};
use std::collections::BTreeSet;
use std::env;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;
use tower::ServiceExt;

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
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

async fn test_state_with_pg(tempdir: &std::path::Path) -> HttpAppState {
    let kernel = Arc::new(
        Kernel::builder()
            .with_postgres_url(&test_database_url())
            .await
            .unwrap()
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
                pg: true,
                markdown: true,
                http: true,
                mcp: false,
                require_key: false,
            },
        },
        kernel,
    )
}

async fn create_http_key(app: axum::Router, owner_scope_id: &str) -> String {
    let key_response = app
        .oneshot(
            Request::post("/api/v1/keys")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"name":"test key","source":"http","owner_principal_id":"test","owner_scope_id":"{owner_scope_id}","scope_kind":"personal","storage_mode":"all"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    response_json(key_response).await["raw_key"]
        .as_str()
        .unwrap()
        .to_string()
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
    assert!(has_route(
        "/api/v1/lifecycle/memories/{scope_id}/{memory_id}"
    ));
    assert!(has_route(
        "/api/v1/lifecycle/memories/{scope_id}/{memory_id}/status"
    ));
    assert!(has_route(
        "/api/v1/lifecycle/memories/{scope_id}/{memory_id}/forget"
    ));
    assert!(has_route(
        "/api/v1/lifecycle/memories/{scope_id}/{memory_id}/restore"
    ));
    assert!(has_route("/api/v1/lifecycle/audit"));
    assert!(has_route("/api/v1/lifecycle/report"));
    assert!(has_route("/api/v1/agent-contexts"));
    assert!(has_route("/api/v1/agent-contexts/{context_id}"));
    assert!(has_route("/api/v1/agent-contexts/{context_id}/promote"));
    assert!(has_route("/api/v1/explorer/memories"));
    assert!(has_route("/api/v1/assistant/chat"));
    assert!(has_route("/api/v1/keys/{key_id}"));
    assert!(has_route("/api/v1/keys/{key_id}/rotate"));
    assert!(has_route("/api/v1/keys/{key_id}/stats"));
    assert!(has_route("/api/v1/sources"));
    assert!(has_route("/api/v1/sources/{source_id}"));
    assert!(has_route("/api/v1/sources/{source_id}/keys"));
    assert!(has_route("/api/v1/sources/{source_id}/documents"));
    assert!(has_route(
        "/api/v1/sources/{source_id}/documents/{document_id}/projection"
    ));
    assert!(has_route("/api/v1/sources/{source_id}/documents/import"));
    assert!(has_route("/api/v1/sources/{source_id}/documents/conflicts"));
    assert!(has_route("/api/v1/sources/{source_id}/documents/sync"));
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
    if !local_pg_test_port_available() {
        return;
    }
    let tempdir = tempdir().unwrap();
    let app = build_router(test_state_with_pg(tempdir.path()).await);
    let scope_id = ScopeId::new();
    let raw_key = create_http_key(app.clone(), scope_id.as_str()).await;

    app.clone()
        .oneshot(
            Request::post("/api/v1/memories")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"scope_id":"{}","title":"Codex 发布步骤","body":"Codex 团队记录了浏览工作台的发布流程。","memory_kind":"procedure","visibility":"team"}}"#,
                    scope_id.as_str()
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    let response = app
        .oneshot(
            Request::get("/api/v1/explorer/memories?limit=20")
                .header("x-meat-memory-key", raw_key)
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
    if !local_pg_test_port_available() {
        return;
    }
    let tempdir = tempdir().unwrap();
    let app = build_router(test_state_with_pg(tempdir.path()).await);
    let raw_key = create_http_key(app.clone(), "scp_browser_claude").await;

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
                .header("x-meat-memory-key", raw_key)
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
async fn lifecycle_endpoints_inspect_forget_restore_report_and_audit() {
    if !local_pg_test_port_available() {
        return;
    }
    let tempdir = tempdir().unwrap();
    let app = build_router(test_state_with_pg(tempdir.path()).await);
    let scope_id = ScopeId::new();
    let raw_key = create_http_key(app.clone(), scope_id.as_str()).await;

    let created = app
        .clone()
        .oneshot(
            Request::post("/api/v1/memories")
                .header("x-meat-memory-key", &raw_key)
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"scope_id":"{}","title":"V2.7 lifecycle API","body":"Lifecycle API should inspect, forget, restore, report, and audit.","memory_kind":"summary","source_refs":["agent-context://ctx_http_lifecycle"]}}"#,
                    scope_id.as_str()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), axum::http::StatusCode::CREATED);
    let created_payload = response_json(created).await;
    let memory_id = created_payload["memory_id"].as_str().unwrap();

    let inspected = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/v1/lifecycle/memories/{}/{}?query=lifecycle",
                scope_id.as_str(),
                memory_id
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(inspected.status(), axum::http::StatusCode::OK);
    let inspected_payload = response_json(inspected).await;
    assert_eq!(inspected_payload["record"]["source_kind"], "conversation");
    assert!(
        inspected_payload["explanation"]["reason"]
            .as_str()
            .unwrap()
            .contains("query matched")
    );

    let forgotten = app
        .clone()
        .oneshot(
            Request::post(format!(
                "/api/v1/lifecycle/memories/{}/{}/forget",
                scope_id.as_str(),
                memory_id
            ))
            .header("x-meat-memory-key", &raw_key)
            .header("content-type", "application/json")
            .body(Body::from(r#"{"reason":"http test"}"#))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(forgotten.status(), axum::http::StatusCode::OK);
    let forgotten_payload = response_json(forgotten).await;
    assert_eq!(forgotten_payload["record"]["status"], "forgotten");

    let restored = app
        .clone()
        .oneshot(
            Request::post(format!(
                "/api/v1/lifecycle/memories/{}/{}/restore",
                scope_id.as_str(),
                memory_id
            ))
            .header("x-meat-memory-key", &raw_key)
            .header("content-type", "application/json")
            .body(Body::from(r#"{"reason":"http restore"}"#))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(restored.status(), axum::http::StatusCode::OK);
    let restored_payload = response_json(restored).await;
    assert_eq!(restored_payload["record"]["status"], "active");

    let report = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/v1/lifecycle/report?scope_id={}",
                scope_id.as_str()
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(report.status(), axum::http::StatusCode::OK);
    let report_payload = response_json(report).await;
    assert_eq!(report_payload["total"], 1);
    assert_eq!(report_payload["active"], 1);

    let audit = app
        .oneshot(
            Request::get(format!(
                "/api/v1/lifecycle/audit?scope_id={}&memory_id={}",
                scope_id.as_str(),
                memory_id
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(audit.status(), axum::http::StatusCode::OK);
    let audit_payload = response_json(audit).await;
    assert!(audit_payload["events"].as_array().unwrap().len() >= 2);
}

#[tokio::test]
async fn promote_memory_endpoint_creates_review_candidate_in_target_scope() {
    if !local_pg_test_port_available() {
        return;
    }
    let tempdir = tempdir().unwrap();
    let app = build_router(test_state_with_pg(tempdir.path()).await);
    let raw_key = create_http_key(app.clone(), "scp_user_alice").await;

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
                .header("x-meat-memory-key", raw_key)
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

    let metrics_payload = response_json(metrics).await;
    assert!(metrics_payload["v2_4"].is_object());
    assert!(metrics_payload["v2_4"]["source_operations"].is_number());
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
    let scope_forbidden = api_error_from_anyhow(anyhow!(
        "scope access forbidden for current meat memory key"
    ));
    let internal = api_error_from_anyhow(anyhow!("database offline"));

    assert_eq!(forbidden.status, axum::http::StatusCode::FORBIDDEN);
    assert_eq!(forbidden.message, "write denied by policy");
    assert_eq!(unsupported.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(unknown.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(empty.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(invalid.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(scope_forbidden.status, axum::http::StatusCode::FORBIDDEN);
    assert_eq!(
        scope_forbidden.message,
        "scope access forbidden for current meat memory key"
    );
    assert_eq!(
        internal.status,
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(internal.message, "internal server error");
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
