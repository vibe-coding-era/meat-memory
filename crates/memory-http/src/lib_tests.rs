use super::{
    ApiError, ApiFeatureFlags, ApiMetadata, HttpAppState, api_error_from_anyhow,
    build_console_page, build_router, distillation_profile_level_label,
    distillation_profile_status_label, escape_html, has_route, memory_kind_label,
    memory_to_summary, parse_artifact_kind, parse_distillation_profile_level,
    parse_distillation_profile_status, parse_memory_kind, parse_review_actor_kind,
    parse_review_level, parse_review_policy_action, parse_sensitivity, parse_visibility,
    review_policy_decision_label,
};
use anyhow::anyhow;
use axum::{
    body::{Body, to_bytes},
    http::Request,
    response::IntoResponse,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use memory_domain::{
    ArtifactKind, DistillationProfileLevel, DistillationProfileStatus, Memory, MemoryKind,
    MemoryProposal, MemoryRelation, MemoryRelationSourceKind, MemoryRelationType, MemoryState,
    ProposalStatus, ProposalType, ReviewLevel, ScopeId, Sensitivity, Visibility,
};
use memory_kernel::{Kernel, ReviewActorKind, ReviewPolicyAction, ReviewPolicyDecision};
use memory_models::{
    CapabilityRoute, DeploymentTarget, ModelCapability, ModelDescriptor, ModelRegistry, Provider,
    ProviderDescriptor,
};
use memory_store_pg::PgStore as TestPgStore;
use std::collections::BTreeSet;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs};
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

async fn test_pg_store() -> TestPgStore {
    let store = TestPgStore::connect(&test_database_url()).await.unwrap();
    store.migrate().await.unwrap();
    store
}

async fn seed_active_memory(
    store: &TestPgStore,
    scope_id: &ScopeId,
    title: &str,
    body: &str,
    kind: MemoryKind,
) -> Memory {
    store
        .seed_scope(
            scope_id,
            scope_id.as_str(),
            &format!("default/scopes/{}", scope_id.as_str()),
        )
        .await
        .unwrap();

    let mut memory = Memory::new(scope_id.clone(), kind, title, body).unwrap();
    memory.activate().unwrap();
    store.insert_memory(&memory).await.unwrap();
    memory
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
    assert!(has_route("/api/v1/benchmark/report"));
    assert!(has_route("/api/v1/recall/traces/inspect"));
    assert!(has_route("/api/v1/health/report"));
    assert!(has_route("/api/v1/passports/manifest"));
    assert!(has_route("/api/v1/compat/report"));
    assert!(has_route("/api/v1/compat/connectors/dry-run"));
    assert!(has_route("/api/v1/compat/connectors/sync-plan"));
    assert!(has_route("/api/v1/compat/connectors/import-draft"));
    assert!(has_route("/api/v1/compat/connectors/proposal-queue"));
    assert!(has_route("/api/v1/agent-contexts"));
    assert!(has_route("/api/v1/agent-contexts/{context_id}"));
    assert!(has_route("/api/v1/agent-contexts/{context_id}/promote"));
    assert!(has_route("/api/v1/proposals"));
    assert!(has_route("/api/v1/proposals/{proposal_id}"));
    assert!(has_route("/api/v1/proposals/{proposal_id}/approve"));
    assert!(has_route("/api/v1/proposals/{proposal_id}/reject"));
    assert!(has_route("/api/v1/proposals/{proposal_id}/apply"));
    assert!(has_route("/api/v1/proposals/review-policy/evaluate"));
    assert!(has_route(
        "/api/v1/memories/{scope_id}/{memory_id}/versions"
    ));
    assert!(has_route(
        "/api/v1/memories/{scope_id}/{memory_id}/timeline"
    ));
    assert!(has_route(
        "/api/v1/memories/{scope_id}/{memory_id}/rollback"
    ));
    assert!(has_route("/api/v1/distillation/profiles"));
    assert!(has_route("/api/v1/distillation/profiles/{profile_id}"));
    assert!(has_route("/api/v1/distillation/preview"));
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
    assert!(text.contains("/api/v1/compat/connectors/dry-run"));
    assert!(text.contains("sync-plan / import-draft / proposal-queue"));
}

#[tokio::test]
async fn v29_surface_http_reports_compat_and_health() {
    let tempdir = tempdir().unwrap();
    let app = build_router(test_state(tempdir.path()));

    let compat = app
        .clone()
        .oneshot(
            Request::get("/api/v1/compat/report?scope_id=scp_http_compat")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(compat.status(), axum::http::StatusCode::OK);
    let compat_payload = response_json(compat).await;
    assert_eq!(compat_payload["schema_version"], "2.95");
    assert_eq!(
        compat_payload["coverage_gate"]["new_feature_test_coverage_required"],
        "100%"
    );

    let health = app
        .oneshot(
            Request::get("/api/v1/health/report?scope_id=scp_http_compat")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(health.status(), axum::http::StatusCode::OK);
    let health_payload = response_json(health).await;
    assert_eq!(health_payload["scope_id"], "scp_http_compat");
    assert!(health_payload["risks"].is_array());
}

#[tokio::test]
async fn v297_http_connector_dry_run_returns_report() {
    let tempdir = tempdir().unwrap();
    fs::write(
        tempdir.path().join("README.md"),
        "# HTTP Connector\n\nDry-run fixture.",
    )
    .unwrap();
    let app = build_router(test_state(tempdir.path()));
    let uri = format!(
        "/api/v1/compat/connectors/dry-run?connector=markdown-docs&root_path={}&max_items=5",
        tempdir.path().display()
    );

    let response = app
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["schema_version"], "2.97-A");
    assert_eq!(payload["connector"], "markdown-docs");
    assert_eq!(payload["mode"], "dry_run");
    assert_eq!(payload["candidate_count"], 1);
    assert_eq!(
        payload["coverage_gate"]["new_feature_test_coverage_required"],
        "100%"
    );
}

#[tokio::test]
async fn v297_http_connector_sync_plan_and_import_draft_return_reports() {
    let tempdir = tempdir().unwrap();
    let docs_dir = tempdir.path().join("docs");
    let chat_dir = tempdir.path().join("chat");
    fs::create_dir_all(&docs_dir).unwrap();
    fs::create_dir_all(&chat_dir).unwrap();
    fs::write(
        docs_dir.join("README.md"),
        "# HTTP Sync\n\nSync-plan fixture.",
    )
    .unwrap();
    fs::write(
        chat_dir.join("chat.json"),
        serde_json::json!({
            "id": "http_chat",
            "title": "HTTP chat import",
            "messages": [
                {"role": "user", "content": "Capture this HTTP connector import draft."},
                {"role": "assistant", "content": "Return a reviewable draft."}
            ]
        })
        .to_string(),
    )
    .unwrap();
    let app = build_router(test_state(tempdir.path()));
    let sync_uri = format!(
        "/api/v1/compat/connectors/sync-plan?connector=markdown-docs&root_path={}&scope_id=scp_http_connector&max_items=5",
        docs_dir.display()
    );
    let import_uri = format!(
        "/api/v1/compat/connectors/import-draft?connector=chat-export&root_path={}&scope_id=scp_http_connector&proposal=true&max_items=5",
        chat_dir.display()
    );

    let sync = app
        .clone()
        .oneshot(Request::get(sync_uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(sync.status(), axum::http::StatusCode::OK);
    let sync_payload = response_json(sync).await;
    assert_eq!(sync_payload["schema_version"], "2.97-A");
    assert_eq!(sync_payload["connector"], "markdown-docs");
    assert_eq!(sync_payload["mode"], "sync_plan");
    assert_eq!(sync_payload["planned_count"], 1);
    assert_eq!(
        sync_payload["coverage_gate"]["new_feature_test_coverage_required"],
        "100%"
    );

    let import = app
        .oneshot(Request::get(import_uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(import.status(), axum::http::StatusCode::OK);
    let import_payload = response_json(import).await;
    assert_eq!(import_payload["schema_version"], "2.97-A");
    assert_eq!(import_payload["connector"], "chat-export");
    assert_eq!(import_payload["mode"], "import_draft");
    assert_eq!(import_payload["draft_count"], 1);
    assert_eq!(import_payload["proposal_draft_count"], 1);
    assert_eq!(import_payload["import_policy"]["writes_memory"], false);
}

#[tokio::test]
async fn v297_http_connector_proposal_queue_returns_report() {
    let tempdir = tempdir().unwrap();
    fs::write(
        tempdir.path().join("chat.json"),
        serde_json::json!({
            "id": "http_queue",
            "title": "HTTP queue import",
            "messages": [
                {"role": "user", "content": "Queue this HTTP connector import."},
                {"role": "assistant", "content": "Return a review queue item."}
            ]
        })
        .to_string(),
    )
    .unwrap();
    let app = build_router(test_state(tempdir.path()));
    let uri = format!(
        "/api/v1/compat/connectors/proposal-queue?connector=chat-export&root_path={}&scope_id=scp_http_connector&max_items=5",
        tempdir.path().display()
    );

    let response = app
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["schema_version"], "2.97-A");
    assert_eq!(payload["connector"], "chat-export");
    assert_eq!(payload["mode"], "proposal_queue");
    assert_eq!(payload["queue_item_count"], 1);
    assert_eq!(payload["queue_items"][0]["proposal_type"], "distill_upsert");
    assert_eq!(payload["queue_policy"]["writes_memory"], false);
    assert_eq!(
        payload["coverage_gate"]["new_feature_test_coverage_required"],
        "100%"
    );
}

#[tokio::test]
async fn v29_surface_http_reads_benchmark_and_trace_reports() {
    let tempdir = tempdir().unwrap();
    let app = build_router(test_state(tempdir.path()));
    let benchmark_dir = tempdir.path().join("benchmark");
    let trace_dir = tempdir.path().join("trace");
    fs::create_dir_all(&benchmark_dir).unwrap();
    fs::create_dir_all(&trace_dir).unwrap();
    fs::write(benchmark_dir.join("summary.md"), "# Benchmark Summary\n").unwrap();
    fs::write(
        benchmark_dir.join("metrics.json"),
        serde_json::json!({"suite":{"name":"meat-code-zh"},"metrics":{"recall_at_1":1.0}})
            .to_string(),
    )
    .unwrap();
    fs::write(
        trace_dir.join("trace.json"),
        serde_json::json!({"trace":{"id":"rtr_http"},"budget_pack":{"used_chars":42}}).to_string(),
    )
    .unwrap();
    fs::write(trace_dir.join("explanation.md"), "# Explanation\n").unwrap();

    let benchmark = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/v1/benchmark/report?input_dir={}",
                benchmark_dir.display()
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(benchmark.status(), axum::http::StatusCode::OK);
    let benchmark_payload = response_json(benchmark).await;
    assert_eq!(
        benchmark_payload["metrics"]["suite"]["name"],
        "meat-code-zh"
    );
    assert!(
        benchmark_payload["summary"]
            .as_str()
            .unwrap()
            .contains("Benchmark Summary")
    );

    let trace = app
        .oneshot(
            Request::get(format!(
                "/api/v1/recall/traces/inspect?input_dir={}&trace_id=rtr_http",
                trace_dir.display()
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(trace.status(), axum::http::StatusCode::OK);
    let trace_payload = response_json(trace).await;
    assert_eq!(trace_payload["trace"]["trace"]["id"], "rtr_http");
    assert!(
        trace_payload["explanation"]
            .as_str()
            .unwrap()
            .contains("Explanation")
    );
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
async fn review_policy_endpoint_evaluates_v28_boundaries() {
    let tempdir = tempdir().unwrap();
    let app = build_router(test_state(tempdir.path()));
    let cases = [
        (
            r#"{"review_level":"auto","action":"approve","actor_kind":"system"}"#,
            "allow",
            "auto proposal may be approved automatically",
        ),
        (
            r#"{"review_level":"required","action":"approve","actor_kind":"agent"}"#,
            "require_user_approval",
            "required proposal needs explicit user approval",
        ),
        (
            r#"{"review_level":"blocked","action":"apply","actor_kind":"user","has_user_authorization":true}"#,
            "deny",
            "blocked proposal cannot be applied",
        ),
    ];

    for (body, decision, reason) in cases {
        let response = app
            .clone()
            .oneshot(
                Request::post("/api/v1/proposals/review-policy/evaluate")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let payload = response_json(response).await;
        assert_eq!(payload["decision"], decision);
        assert_eq!(payload["reason"], reason);
    }
}

#[tokio::test]
async fn distillation_preview_endpoint_uses_session_prompt_and_default_scope() {
    let tempdir = tempdir().unwrap();
    let app = build_router(test_state(tempdir.path()));

    let response = app
        .oneshot(
            Request::post("/api/v1/distillation/preview")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"input":"V2.8 should keep review decisions with evidence.","evidence_refs":["agent-context://ctx_1"],"prompt_text":"Prefer durable review decisions.","focus_topics":["review"],"prefer_memory_kinds":["decision"]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["run"]["scope_id"], "scp_http_default");
    assert_eq!(payload["run"]["preview"], true);
    assert!(
        payload["run"]["run_id"]
            .as_str()
            .unwrap()
            .starts_with("drn_")
    );
    assert!(
        payload["run"]["input_hash"]
            .as_str()
            .unwrap()
            .starts_with("len:")
    );
    assert_eq!(payload["profile"]["focus_topics"][0], "review");
    assert_eq!(payload["profile"]["prefer_memory_kinds"][0], "decision");
    assert_eq!(
        payload["profile"]["prompt_segments"][1]["layer"],
        "session_override"
    );
    assert_eq!(
        payload["profile"]["prompt_segments"][1]["text"],
        "Prefer durable review decisions."
    );
    assert_eq!(payload["candidates"][0]["memory_kind"], "decision");
    assert_eq!(
        payload["candidates"][0]["evidence_refs"][0],
        "agent-context://ctx_1"
    );
    assert_eq!(payload["discarded"], serde_json::json!([]));
    assert!(payload["warnings"][0].as_str().unwrap().contains("preview"));
}

#[tokio::test]
async fn distillation_preview_endpoint_accepts_explicit_scope_and_default_profile() {
    let tempdir = tempdir().unwrap();
    let app = build_router(test_state(tempdir.path()));

    let response = app
        .oneshot(
            Request::post("/api/v1/distillation/preview")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"scope_id":"scp_v28","input":"Remember this stable fact.","evidence_refs":["agent-context://ctx_2"]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["run"]["scope_id"], "scp_v28");
    assert_eq!(
        payload["profile"]["prompt_segments"][0]["layer"],
        "system_base"
    );
    assert_eq!(payload["profile"]["focus_topics"], serde_json::json!([]));
    assert_eq!(
        payload["profile"]["prefer_memory_kinds"],
        serde_json::json!([])
    );
    assert_eq!(payload["candidates"][0]["memory_kind"], "summary");
    assert_eq!(
        payload["candidates"][0]["why_keep"],
        "profile-guided preview candidate"
    );
}

#[tokio::test]
async fn distillation_profile_endpoints_roundtrip_and_feed_preview() {
    if !local_pg_test_port_available() {
        return;
    }
    let tempdir = tempdir().unwrap();
    let app = build_router(test_state_with_pg(tempdir.path()).await);
    let project_scope_id = ScopeId::new();
    let global_profile_id = memory_domain::DistillationProfileId::new();
    let project_profile_id = memory_domain::DistillationProfileId::new();

    let global = app
        .clone()
        .oneshot(
            Request::put(format!(
                "/api/v1/distillation/profiles/{}",
                global_profile_id.as_str()
            ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"profile_level":"user_global","status":"active","name":"HTTP global","prompt_text":"Keep governance guidance.","focus_topics":["governance"],"prefer_memory_kinds":[],"created_by":"user"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(global.status(), axum::http::StatusCode::OK);
    let global_payload = response_json(global).await;
    assert_eq!(global_payload["profile_id"], global_profile_id.as_str());
    assert_eq!(global_payload["profile_level"], "user_global");

    let project = app
        .clone()
        .oneshot(
            Request::put(format!(
                "/api/v1/distillation/profiles/{}",
                project_profile_id.as_str()
            ))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"scope_id":"{}","profile_level":"project","status":"active","name":"HTTP project","prompt_text":"Prefer rollout constraints.","focus_topics":["rollout"],"prefer_memory_kinds":["constraint"],"created_by":"user"}}"#,
                    project_scope_id.as_str()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(project.status(), axum::http::StatusCode::OK);
    let project_payload = response_json(project).await;
    assert_eq!(project_payload["scope_id"], project_scope_id.as_str());
    assert_eq!(project_payload["prefer_memory_kinds"][0], "constraint");

    let scoped = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/v1/distillation/profiles?scope_id={}",
                project_scope_id.as_str()
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(scoped.status(), axum::http::StatusCode::OK);
    let scoped_payload = response_json(scoped).await;
    assert_eq!(scoped_payload.as_array().unwrap().len(), 1);
    assert_eq!(scoped_payload[0]["profile_id"], project_profile_id.as_str());

    let preview = app
        .oneshot(
            Request::post("/api/v1/distillation/preview")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"scope_id":"{}","input":"Keep rollback evidence visible.","evidence_refs":["agent-context://ctx_http"]}}"#,
                    project_scope_id.as_str()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(preview.status(), axum::http::StatusCode::OK);
    let preview_payload = response_json(preview).await;
    assert_eq!(
        preview_payload["profile"]["prompt_segments"][1]["layer"],
        "user_global"
    );
    assert_eq!(
        preview_payload["profile"]["prompt_segments"][2]["layer"],
        "project"
    );
    assert_eq!(
        preview_payload["candidates"][0]["memory_kind"],
        "constraint"
    );
}

#[tokio::test]
async fn memory_rollback_endpoint_restores_prior_version_from_pg() {
    if !local_pg_test_port_available() {
        return;
    }
    let tempdir = tempdir().unwrap();
    let app = build_router(test_state_with_pg(tempdir.path()).await);
    let store = test_pg_store().await;
    let scope_id = ScopeId::new();
    let mut memory = seed_active_memory(
        &store,
        &scope_id,
        "Rollback HTTP v1",
        "Rollback HTTP original body.",
        MemoryKind::Decision,
    )
    .await;
    memory.title = "Rollback HTTP v2".to_string();
    memory.body = "Rollback HTTP updated body.".to_string();
    memory.updated_at = time::OffsetDateTime::now_utc();
    store.upsert_memory(&memory).await.unwrap();

    let rollback = app
        .clone()
        .oneshot(
            Request::post(format!(
                "/api/v1/memories/{}/{}/rollback",
                scope_id.as_str(),
                memory.id.as_str()
            ))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"target_version":1,"actor":"user","reason":"restore original wording"}"#,
            ))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rollback.status(), axum::http::StatusCode::OK);
    let rollback_payload = response_json(rollback).await;
    assert_eq!(rollback_payload["target_version"], 1);
    assert_eq!(rollback_payload["new_version"], 3);
    assert_eq!(rollback_payload["change_kind"], "rollback");
    assert_eq!(rollback_payload["title"], "Rollback HTTP v1");

    let restored = store
        .get_memory(&scope_id, &memory.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(restored.title, "Rollback HTTP v1");
    assert_eq!(restored.body, "Rollback HTTP original body.");

    let versions = app
        .oneshot(
            Request::get(format!(
                "/api/v1/memories/{}/{}/versions",
                scope_id.as_str(),
                memory.id.as_str()
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(versions.status(), axum::http::StatusCode::OK);
    let versions_payload = response_json(versions).await;
    assert_eq!(versions_payload[0]["version"], 3);
    assert_eq!(versions_payload[0]["change_kind"], "rollback");
    assert_eq!(versions_payload[0]["reason"], "restore original wording");
}

#[tokio::test]
async fn proposal_review_endpoints_roundtrip_from_pg() {
    if !local_pg_test_port_available() {
        return;
    }
    let tempdir = tempdir().unwrap();
    let app = build_router(test_state_with_pg(tempdir.path()).await);
    let store = test_pg_store().await;
    let scope_id = ScopeId::new();
    store
        .seed_scope(
            &scope_id,
            scope_id.as_str(),
            &format!("default/scopes/{}", scope_id.as_str()),
        )
        .await
        .unwrap();
    let subject = seed_active_memory(
        &store,
        &scope_id,
        "HTTP proposal subject",
        "Newer proposal subject body.",
        MemoryKind::Decision,
    )
    .await;
    let target = seed_active_memory(
        &store,
        &scope_id,
        "HTTP proposal target",
        "Older proposal target body.",
        MemoryKind::Decision,
    )
    .await;

    let mut approve_proposal = MemoryProposal::new(
        scope_id.clone(),
        ProposalType::Supersede,
        ReviewLevel::Required,
        "http proposal requires user approval",
    )
    .unwrap()
    .with_subject_memory(subject.id.clone());
    approve_proposal.add_target_memory(target.id.clone());
    approve_proposal
        .add_evidence("required change should not be agent-approved silently".to_string())
        .unwrap();
    store
        .upsert_memory_proposal(&approve_proposal)
        .await
        .unwrap();
    let reject_proposal = MemoryProposal::new(
        scope_id.clone(),
        ProposalType::ConflictMark,
        ReviewLevel::Suggested,
        "http proposal should be rejected",
    )
    .unwrap();
    store
        .upsert_memory_proposal(&reject_proposal)
        .await
        .unwrap();

    let listed = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/v1/proposals?scope_id={}&limit=10",
                scope_id.as_str()
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(listed.status(), axum::http::StatusCode::OK);
    let listed_payload = response_json(listed).await;
    let listed_ids = listed_payload
        .as_array()
        .unwrap()
        .iter()
        .map(|proposal| proposal["proposal_id"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert!(listed_ids.contains(approve_proposal.id.as_str()));
    assert!(listed_ids.contains(reject_proposal.id.as_str()));

    let inspected = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/v1/proposals/{}",
                approve_proposal.id.as_str()
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(inspected.status(), axum::http::StatusCode::OK);
    let inspected_payload = response_json(inspected).await;
    assert_eq!(inspected_payload["status"], "open");
    assert_eq!(inspected_payload["review_level"], "required");

    let forbidden = app
        .clone()
        .oneshot(
            Request::post(format!(
                "/api/v1/proposals/{}/approve",
                approve_proposal.id.as_str()
            ))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"actor":"agent","actor_kind":"agent","has_user_authorization":false}"#,
            ))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(forbidden.status(), axum::http::StatusCode::FORBIDDEN);

    let approved = app
        .clone()
        .oneshot(
            Request::post(format!(
                "/api/v1/proposals/{}/approve",
                approve_proposal.id.as_str()
            ))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"actor":"user","actor_kind":"user","has_user_authorization":false}"#,
            ))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(approved.status(), axum::http::StatusCode::OK);
    let approved_payload = response_json(approved).await;
    assert_eq!(approved_payload["status"], "approved");
    assert_eq!(approved_payload["decided_by"], "user");

    let applied = app
        .clone()
        .oneshot(
            Request::post(format!(
                "/api/v1/proposals/{}/apply",
                approve_proposal.id.as_str()
            ))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"actor":"system","actor_kind":"system","has_user_authorization":false}"#,
            ))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(applied.status(), axum::http::StatusCode::OK);
    let applied_payload = response_json(applied).await;
    assert_eq!(applied_payload["status"], "applied");

    let timeline = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/v1/memories/{}/{}/timeline",
                scope_id.as_str(),
                subject.id.as_str()
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(timeline.status(), axum::http::StatusCode::OK);
    let timeline_payload = response_json(timeline).await;
    assert_eq!(timeline_payload["relations"].as_array().unwrap().len(), 1);
    assert!(
        timeline_payload["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["action"] == "proposal.applied")
    );

    let rejected = app
        .clone()
        .oneshot(
            Request::post(format!(
                "/api/v1/proposals/{}/reject",
                reject_proposal.id.as_str()
            ))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"actor":"user"}"#))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rejected.status(), axum::http::StatusCode::OK);
    let rejected_payload = response_json(rejected).await;
    assert_eq!(rejected_payload["status"], "rejected");

    let missing = app
        .oneshot(
            Request::get("/api/v1/proposals/prp_missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn memory_versions_and_timeline_endpoints_roundtrip_from_pg() {
    if !local_pg_test_port_available() {
        return;
    }
    let tempdir = tempdir().unwrap();
    let app = build_router(test_state_with_pg(tempdir.path()).await);
    let store = test_pg_store().await;
    let scope_id = ScopeId::new();
    let mut subject = seed_active_memory(
        &store,
        &scope_id,
        "HTTP timeline subject",
        "Initial HTTP timeline body.",
        MemoryKind::Decision,
    )
    .await;
    let target = seed_active_memory(
        &store,
        &scope_id,
        "HTTP timeline target",
        "Legacy HTTP timeline body.",
        MemoryKind::Decision,
    )
    .await;

    subject.title = "HTTP timeline subject v2".to_string();
    subject.body = "Updated HTTP timeline body.".to_string();
    subject.updated_at = time::OffsetDateTime::now_utc();
    store.upsert_memory(&subject).await.unwrap();

    let mut proposal = MemoryProposal::new(
        scope_id.clone(),
        ProposalType::Supersede,
        ReviewLevel::Required,
        "http timeline supersede proposal",
    )
    .unwrap()
    .with_subject_memory(subject.id.clone());
    proposal.add_target_memory(target.id.clone());
    proposal
        .add_evidence("same scope with newer rollback guidance".to_string())
        .unwrap();
    proposal.approve("user").unwrap();
    store.upsert_memory_proposal(&proposal).await.unwrap();
    sqlx::query(
        "UPDATE memory_versions SET source_proposal_id = $1 WHERE memory_id = $2 AND version = 2",
    )
    .bind(proposal.id.as_str())
    .bind(subject.id.as_str())
    .execute(store.pool())
    .await
    .unwrap();

    let relation = MemoryRelation::new(
        scope_id.clone(),
        subject.id.clone(),
        target.id.clone(),
        MemoryRelationType::Supersedes,
        MemoryRelationSourceKind::System,
    )
    .with_source_proposal_id(proposal.id.clone());
    store.insert_memory_relation(&relation).await.unwrap();
    store
        .insert_lifecycle_audit_event(
            &scope_id,
            &subject.id,
            "memory.status.changed",
            "system",
            Some("active"),
            Some("deprecated"),
            Some("superseded in http timeline test"),
            time::OffsetDateTime::now_utc(),
        )
        .await
        .unwrap();

    let versions = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/v1/memories/{}/{}/versions",
                scope_id.as_str(),
                subject.id.as_str()
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(versions.status(), axum::http::StatusCode::OK);
    let versions_payload = response_json(versions).await;
    assert_eq!(versions_payload.as_array().unwrap().len(), 2);
    assert_eq!(versions_payload[0]["version"], 2);
    assert_eq!(versions_payload[0]["title"], "HTTP timeline subject v2");
    assert_eq!(
        versions_payload[0]["source_proposal_id"],
        proposal.id.as_str()
    );
    assert_eq!(versions_payload[1]["version"], 1);

    let timeline = app
        .oneshot(
            Request::get(format!(
                "/api/v1/memories/{}/{}/timeline",
                scope_id.as_str(),
                subject.id.as_str()
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(timeline.status(), axum::http::StatusCode::OK);
    let timeline_payload = response_json(timeline).await;
    assert_eq!(timeline_payload["memory_id"], subject.id.as_str());
    assert_eq!(timeline_payload["versions"].as_array().unwrap().len(), 2);
    assert_eq!(timeline_payload["relations"].as_array().unwrap().len(), 1);
    assert_eq!(timeline_payload["proposals"].as_array().unwrap().len(), 1);
    assert_eq!(
        timeline_payload["audit_events"].as_array().unwrap().len(),
        1
    );
    assert_eq!(timeline_payload["proposals"][0]["status"], "approved");
    assert_eq!(
        timeline_payload["proposals"][0]["status"],
        ProposalStatus::Approved.as_str()
    );
    assert!(
        timeline_payload["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["action"] == "proposal.approved")
    );
    assert!(
        timeline_payload["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["action"] == "relation.supersedes")
    );
    assert!(
        timeline_payload["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["action"] == "audit.memory.status.changed")
    );
}

#[tokio::test]
async fn memory_versions_and_timeline_endpoints_require_pg_or_existing_memory() {
    let tempdir = tempdir().unwrap();
    let app = build_router(test_state(tempdir.path()));
    let scope_id = ScopeId::new();

    let created = app
        .clone()
        .oneshot(
            Request::post("/api/v1/memories")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"scope_id":"{}","title":"Local only memory","body":"Stored in markdown only."}}"#,
                    scope_id.as_str()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), axum::http::StatusCode::CREATED);
    let created_payload = response_json(created).await;
    let memory_id = created_payload["memory_id"].as_str().unwrap();

    let versions = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/v1/memories/{}/{}/versions",
                scope_id.as_str(),
                memory_id
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        versions.status(),
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        response_json(versions).await["error"],
        "internal server error"
    );

    let missing_timeline = app
        .oneshot(
            Request::get(format!(
                "/api/v1/memories/{}/mem_missing/timeline",
                scope_id.as_str()
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_timeline.status(), axum::http::StatusCode::NOT_FOUND);
    assert_eq!(
        response_json(missing_timeline).await["error"],
        "memory not found"
    );
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
        (
            "/api/v1/proposals/review-policy/evaluate",
            r#"{"review_level":"unknown","action":"approve","actor_kind":"agent"}"#,
            axum::http::StatusCode::BAD_REQUEST,
            "unsupported review_level: unknown",
        ),
        (
            "/api/v1/proposals/review-policy/evaluate",
            r#"{"review_level":"auto","action":"publish","actor_kind":"agent"}"#,
            axum::http::StatusCode::BAD_REQUEST,
            "unsupported review_policy_action: publish",
        ),
        (
            "/api/v1/proposals/review-policy/evaluate",
            r#"{"review_level":"auto","action":"approve","actor_kind":"robot"}"#,
            axum::http::StatusCode::BAD_REQUEST,
            "unsupported review_actor_kind: robot",
        ),
        (
            "/api/v1/distillation/preview",
            r#"{"input":" ","evidence_refs":["agent-context://ctx_1"]}"#,
            axum::http::StatusCode::BAD_REQUEST,
            "distillation input cannot be empty",
        ),
        (
            "/api/v1/distillation/preview",
            r#"{"input":"Keep this decision.","evidence_refs":[]}"#,
            axum::http::StatusCode::BAD_REQUEST,
            "distillation evidence_refs cannot be empty",
        ),
        (
            "/api/v1/distillation/preview",
            r#"{"input":"Keep this decision.","evidence_refs":["agent-context://ctx_1"],"prefer_memory_kinds":["bogus"]}"#,
            axum::http::StatusCode::BAD_REQUEST,
            "unsupported memory_kind: bogus",
        ),
        (
            "/api/v1/distillation/profiles/dpf_invalid",
            r#"{"profile_level":"workspace","name":"Invalid","prompt_text":"prompt","created_by":"user"}"#,
            axum::http::StatusCode::BAD_REQUEST,
            "unsupported distillation_profile_level: workspace",
        ),
        (
            "/api/v1/distillation/profiles/dpf_invalid",
            r#"{"profile_level":"user_global","status":"draft","name":"Invalid","prompt_text":"prompt","created_by":"user"}"#,
            axum::http::StatusCode::BAD_REQUEST,
            "unsupported distillation_profile_status: draft",
        ),
        (
            "/api/v1/distillation/profiles/dpf_invalid",
            r#"{"profile_level":"project","name":"Invalid","prompt_text":"prompt","created_by":"user"}"#,
            axum::http::StatusCode::BAD_REQUEST,
            "project profile requires scope_id",
        ),
    ];

    for (path, body, status, message) in cases {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(if path.contains("/distillation/profiles/") {
                        "PUT"
                    } else {
                        "POST"
                    })
                    .uri(path)
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            status,
            "unexpected status for path={path} body={body}"
        );
        let payload = response_json(response).await;
        assert_eq!(
            payload["error"], message,
            "unexpected error for path={path}"
        );
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
    let review_levels = [
        ("auto", ReviewLevel::Auto),
        ("suggested", ReviewLevel::Suggested),
        ("required", ReviewLevel::Required),
        ("blocked", ReviewLevel::Blocked),
    ];
    let review_policy_actions = [
        ("approve", ReviewPolicyAction::Approve),
        ("apply", ReviewPolicyAction::Apply),
    ];
    let distillation_profile_levels = [
        (
            "user_global",
            DistillationProfileLevel::UserGlobal,
            "user_global",
        ),
        ("project", DistillationProfileLevel::Project, "project"),
    ];
    let distillation_profile_statuses = [
        ("active", DistillationProfileStatus::Active, "active"),
        ("archived", DistillationProfileStatus::Archived, "archived"),
    ];
    let review_actor_kinds = [
        ("user", ReviewActorKind::User),
        ("agent", ReviewActorKind::Agent),
        ("system", ReviewActorKind::System),
    ];
    let review_policy_decisions = [
        (ReviewPolicyDecision::Allow, "allow"),
        (
            ReviewPolicyDecision::RequireUserApproval,
            "require_user_approval",
        ),
        (ReviewPolicyDecision::Deny, "deny"),
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
    for (raw, expected) in review_levels {
        assert_eq!(parse_review_level(raw).unwrap(), expected);
    }
    for (raw, expected) in review_policy_actions {
        assert_eq!(parse_review_policy_action(raw).unwrap(), expected);
    }
    for (raw, expected, label) in distillation_profile_levels {
        assert_eq!(parse_distillation_profile_level(raw).unwrap(), expected);
        assert_eq!(distillation_profile_level_label(expected), label);
    }
    for (raw, expected, label) in distillation_profile_statuses {
        assert_eq!(parse_distillation_profile_status(raw).unwrap(), expected);
        assert_eq!(distillation_profile_status_label(expected), label);
    }
    for (raw, expected) in review_actor_kinds {
        assert_eq!(parse_review_actor_kind(raw).unwrap(), expected);
    }
    for (decision, label) in review_policy_decisions {
        assert_eq!(review_policy_decision_label(decision), label);
    }
}

#[test]
fn maps_anyhow_errors_to_expected_http_statuses() {
    let forbidden = api_error_from_anyhow(anyhow!("write denied by policy"));
    let unsupported = api_error_from_anyhow(anyhow!("unsupported memory_kind"));
    let unknown = api_error_from_anyhow(anyhow!("unknown provider"));
    let empty = api_error_from_anyhow(anyhow!("empty request body"));
    let invalid = api_error_from_anyhow(anyhow!("Invalid media type"));
    let missing_scope = api_error_from_anyhow(anyhow!("project profile requires scope_id"));
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
    assert_eq!(missing_scope.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(missing_scope.message, "project profile requires scope_id");
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
