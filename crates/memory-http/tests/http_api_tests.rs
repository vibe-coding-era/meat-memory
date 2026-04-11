use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use memory_domain::ScopeId;
use memory_http::{ApiFeatureFlags, ApiMetadata, HttpAppState, build_router};
use memory_kernel::Kernel;
use memory_models::{
    CapabilityRoute, DeploymentTarget, ModelCapability, ModelDescriptor, ModelRegistry, Provider,
    ProviderDescriptor,
};
use std::collections::BTreeSet;
use std::{env, sync::Arc};
use tempfile::tempdir;
use tower::ServiceExt;

fn test_database_url() -> String {
    env::var("MEAT_MEMORY_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5433/meat_memory_dev".into())
}

async fn build_test_app() -> axum::Router {
    build_test_app_with_registry(test_model_registry()).await
}

async fn build_test_app_with_registry(registry: ModelRegistry) -> axum::Router {
    let tempdir = tempdir().unwrap();
    let kernel = Arc::new(
        Kernel::builder()
            .with_postgres_url(&test_database_url())
            .await
            .unwrap()
            .with_markdown_root(tempdir.path().join("markdown"))
            .unwrap()
            .with_asset_root(tempdir.path().join("assets"))
            .unwrap()
            .with_model_registry(registry, "zh-CN")
            .unwrap()
            .build()
            .unwrap(),
    );

    build_router(HttpAppState::new(
        ScopeId::from_string("scp_http_integration"),
        ApiMetadata {
            service: "meat-memory".into(),
            version: "0.1.0".into(),
            default_scope: "scp_http_integration".into(),
            features: ApiFeatureFlags {
                pg: true,
                markdown: true,
                http: true,
                mcp: false,
            },
        },
        kernel,
    ))
}

#[tokio::test]
async fn http_create_and_search_flow_returns_persisted_memory() {
    let app = build_test_app().await;
    let scope_id = ScopeId::new();
    let create_payload = format!(
        r#"{{"scope_id":"{}","title":"HTTP kernel flow","body":"Project Meat Memory uses Service Gateway for HTTP API search.","memory_kind":"decision"}}"#,
        scope_id.as_str()
    );
    let search_payload = format!(
        r#"{{"scope_id":"{}","query":"gateway http","limit":5}}"#,
        scope_id.as_str()
    );

    let create_response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/memories")
                .header("content-type", "application/json")
                .body(Body::from(create_payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);

    let search_response = app
        .oneshot(
            Request::post("/api/v1/context/search")
                .header("content-type", "application/json")
                .body(Body::from(search_payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(search_response.status(), StatusCode::OK);

    let bytes = to_bytes(search_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let payload = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap();
    assert_eq!(payload["memory_count"], 1);
}

#[tokio::test]
async fn http_root_route_serves_browser_console() {
    let app = build_test_app().await;

    let response = app
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();

    assert!(body.contains("Browser Console"));
    assert!(body.contains("AI 对话区"));
    assert!(body.contains("/api/v1/explorer/memories"));
    assert!(body.contains("/api/v1/assistant/chat"));
}

#[tokio::test]
async fn http_create_and_search_flow_supports_chinese_acceptance_corpus() {
    let app = build_test_app().await;
    let scope_id = ScopeId::new();
    let create_payload = format!(
        r#"{{"scope_id":"{}","title":"中文发布验收规则","body":"项目 `服务网关` 依赖 `PostgreSQL`，`服务网关` 记录 `发布手册`。发布前必须通过回归与验收。","memory_kind":"procedure"}}"#,
        scope_id.as_str()
    );
    let search_payload = format!(
        r#"{{"scope_id":"{}","query":"发布前 验收","limit":5}}"#,
        scope_id.as_str()
    );

    let create_response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/memories")
                .header("content-type", "application/json")
                .body(Body::from(create_payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);

    let search_response = app
        .oneshot(
            Request::post("/api/v1/context/search")
                .header("content-type", "application/json")
                .body(Body::from(search_payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(search_response.status(), StatusCode::OK);

    let bytes = to_bytes(search_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let payload = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap();
    assert_eq!(payload["memory_count"], 1);
    assert!(
        payload["memories"][0]["body"]
            .as_str()
            .unwrap()
            .contains("发布前必须通过回归与验收")
    );
}

#[tokio::test]
async fn http_rejects_invalid_memory_kind() {
    let app = build_test_app().await;
    let scope_id = ScopeId::new();
    let payload = format!(
        r#"{{"scope_id":"{}","body":"Invalid kind sample","memory_kind":"nonsense"}}"#,
        scope_id.as_str()
    );

    let response = app
        .oneshot(
            Request::post("/api/v1/memories")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn http_denies_forbidden_publish_level_write() {
    let app = build_test_app().await;
    let scope_id = ScopeId::new();
    let payload = format!(
        r#"{{"scope_id":"{}","body":"Restricted organization note","visibility":"organization","sensitivity":"restricted"}}"#,
        scope_id.as_str()
    );

    let response = app
        .oneshot(
            Request::post("/api/v1/memories")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn http_create_image_flow_returns_asset_uri() {
    let app = build_test_app().await;
    let scope_id = ScopeId::new();
    let payload = format!(
        r#"{{"scope_id":"{}","title":"Search screenshot","body":"Search results for memory graph","media_type":"image/png","image_base64":"{}"}}"#,
        scope_id.as_str(),
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

    assert_eq!(response.status(), StatusCode::CREATED);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap();
    assert!(
        body["asset_uri"]
            .as_str()
            .unwrap()
            .starts_with("asset://raw/sha256/")
    );
    assert!(
        body["vision_caption"]
            .as_str()
            .unwrap()
            .contains("检测到一张 image/png 图片")
    );
}

#[tokio::test]
async fn http_create_image_flow_returns_llm_failover_notice() {
    let app = build_test_app_with_registry(failover_test_model_registry()).await;
    let scope_id = ScopeId::new();
    let payload = format!(
        r#"{{"scope_id":"{}","title":"中文截图","body":"浏览器首页截图","media_type":"image/png","image_base64":"{}"}}"#,
        scope_id.as_str(),
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

    assert_eq!(response.status(), StatusCode::CREATED);

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap();
    assert_eq!(body["vision_model_alias"], "claude_vision");
    assert_eq!(
        body["llm_notice"],
        "Gemini Vision LLM 不可用，已经切换到Claude Vision"
    );
}

#[tokio::test]
async fn http_promote_memory_endpoint_creates_review_candidate_in_target_scope() {
    let app = build_test_app().await;

    let created = app
        .clone()
        .oneshot(
            Request::post("/api/v1/memories")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"scope_id":"scp_user_http_alice","title":"Alice 发布凭证","body":"token: abc123 联系人 alice@example.com","memory_kind":"procedure","visibility":"private","sensitivity":"restricted"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);

    let created_bytes = to_bytes(created.into_body(), usize::MAX).await.unwrap();
    let created_payload = serde_json::from_slice::<serde_json::Value>(&created_bytes).unwrap();
    let memory_id = created_payload["memory_id"].as_str().unwrap();

    let promoted = app
        .oneshot(
            Request::post("/api/v1/memories/promote")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"source_scope_id":"scp_user_http_alice","memory_id":"{memory_id}","source_scope_type":"user","target_scope_id":"scp_project_http_demo","target_scope_type":"project","target_visibility":"project"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(promoted.status(), StatusCode::CREATED);

    let promoted_bytes = to_bytes(promoted.into_body(), usize::MAX).await.unwrap();
    let payload = serde_json::from_slice::<serde_json::Value>(&promoted_bytes).unwrap();
    assert_eq!(payload["scope_id"], "scp_project_http_demo");
    assert_eq!(payload["owner_scope_id"], "scp_user_http_alice");
    assert_eq!(payload["published_from_scope_id"], "scp_user_http_alice");
    assert_eq!(payload["memory_state"], "candidate");
    assert_eq!(payload["visibility"], "project");
    assert!(payload["body"].as_str().unwrap().contains("[REDACTED]"));
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
                priority: 95,
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
                priority: 95,
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
                priority: 95,
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
