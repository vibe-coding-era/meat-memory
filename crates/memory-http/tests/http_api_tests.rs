use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use memory_domain::ScopeId;
use memory_http::{ApiFeatureFlags, ApiMetadata, HttpAppState, build_router};
use memory_kernel::Kernel;
use std::{env, sync::Arc};
use tempfile::tempdir;
use tower::ServiceExt;

fn test_database_url() -> String {
    env::var("MEAT_MEMORY_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5433/meat_memory_dev".into())
}

async fn build_test_app() -> axum::Router {
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
}
