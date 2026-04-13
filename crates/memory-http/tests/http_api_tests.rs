#![allow(clippy::await_holding_lock)]

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
use std::{
    env,
    sync::{Arc, Mutex, MutexGuard, OnceLock},
};
use tempfile::tempdir;
use tower::ServiceExt;

fn test_database_url() -> String {
    env::var("MEAT_MEMORY_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5433/meat_memory_dev".into())
}

fn http_test_guard() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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
                require_key: false,
            },
        },
        kernel,
    ))
}

#[tokio::test]
async fn http_create_and_search_flow_returns_persisted_memory() {
    let _guard = http_test_guard();
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
async fn http_key_create_and_authenticated_remember_flow() {
    let _guard = http_test_guard();
    let app = build_test_app().await;
    let key_response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/keys")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"http test key","source":"http","owner_principal_id":"alice","owner_scope_id":"scp_user_http_key","scope_kind":"personal","storage_mode":"all"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(key_response.status(), StatusCode::CREATED);

    let key_bytes = to_bytes(key_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let key_payload = serde_json::from_slice::<serde_json::Value>(&key_bytes).unwrap();
    let raw_key = key_payload["raw_key"].as_str().unwrap();
    assert_eq!(key_payload["storage_mode"], "all");

    let remember_response = app
        .oneshot(
            Request::post("/api/v1/memories")
                .header("content-type", "application/json")
                .header("x-meat-memory-key", raw_key)
                .body(Body::from(
                    r#"{"scope_id":"scp_user_http_key","title":"Keyed memory","body":"Access key write path works.","memory_kind":"fact"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(remember_response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn http_source_routes_manage_multiple_keys_per_source() {
    let _guard = http_test_guard();
    let app = build_test_app().await;
    let owner_scope = ScopeId::new();
    let key_response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/keys")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"name":"source owner key","source":"http","owner_principal_id":"alice","owner_scope_id":"{}","scope_kind":"personal","storage_mode":"all"}}"#,
                    owner_scope.as_str()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(key_response.status(), StatusCode::CREATED);
    let key_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(key_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let raw_key = key_payload["raw_key"].as_str().unwrap();

    let source_response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/sources")
                .header("content-type", "application/json")
                .header("x-meat-memory-key", raw_key)
                .body(Body::from(
                    r#"{"name":"Local project docs","source_kind":"local_docs","source_uri":"file:///tmp/meat-memory","sync_mode":"index_only","local_root":"/tmp/meat-memory"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    let source_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(source_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let source_id = source_payload["source_id"].as_str().unwrap();
    assert_eq!(source_payload["source_kind"], "local_docs");
    assert_eq!(source_payload["owner_scope_id"], owner_scope.as_str());
    assert_eq!(source_payload["sync_mode"], "index_only");

    for name in ["docs writer key", "docs reader key"] {
        let response = app
            .clone()
            .oneshot(
                Request::post(format!("/api/v1/sources/{source_id}/keys"))
                    .header("content-type", "application/json")
                    .header("x-meat-memory-key", raw_key)
                    .body(Body::from(format!(
                        r#"{{"name":"{name}","source":"custom","storage_mode":"vector"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let payload = serde_json::from_slice::<serde_json::Value>(
            &to_bytes(response.into_body(), usize::MAX).await.unwrap(),
        )
        .unwrap();
        assert_eq!(payload["source_id"], source_id);
        assert_eq!(payload["owner_scope_id"], owner_scope.as_str());
        assert_eq!(payload["storage_mode"], "vector");
    }

    let list_sources = app
        .clone()
        .oneshot(
            Request::get("/api/v1/sources?limit=10")
                .header("x-meat-memory-key", raw_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_sources.status(), StatusCode::OK);
    let sources_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(list_sources.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(
        sources_payload
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["source_id"] == source_id)
    );

    let list_keys = app
        .oneshot(
            Request::get(format!("/api/v1/sources/{source_id}/keys?limit=10"))
                .header("x-meat-memory-key", raw_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_keys.status(), StatusCode::OK);
    let keys_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(list_keys.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let source_keys = keys_payload.as_array().unwrap();
    assert_eq!(source_keys.len(), 2);
    assert!(
        source_keys
            .iter()
            .all(|key| key["source_id"].as_str() == Some(source_id))
    );
}

#[tokio::test]
async fn http_agent_context_routes_upsert_list_promote_and_delete() {
    let _guard = http_test_guard();
    let app = build_test_app().await;
    let owner_scope = ScopeId::new();
    let key_response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/keys")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"name":"agent context key","source":"http","owner_principal_id":"alice","owner_scope_id":"{}","scope_kind":"personal","storage_mode":"all"}}"#,
                    owner_scope.as_str()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(key_response.status(), StatusCode::CREATED);
    let key_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(key_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let raw_key = key_payload["raw_key"].as_str().unwrap();
    let session_id = format!("session-{}", owner_scope.as_str());

    let upsert_response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/agent-contexts")
                .header("content-type", "application/json")
                .header("x-meat-memory-key", raw_key)
                .body(Body::from(format!(
                    r#"{{"scope_id":"{}","session_id":"{session_id}","task_id":"task-ctx","title":"HTTP agent scratchpad","body":"Codex is collecting V2.4 API context.","labels":["api","short-term"]}}"#,
                    owner_scope.as_str()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(upsert_response.status(), StatusCode::CREATED);
    let upsert_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(upsert_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let context_id = upsert_payload["context_id"].as_str().unwrap();
    assert_eq!(upsert_payload["scope_id"], owner_scope.as_str());
    assert_eq!(upsert_payload["session_id"], session_id);
    assert_eq!(upsert_payload["layer"], "short_term");
    assert_eq!(upsert_payload["labels"].as_array().unwrap().len(), 2);

    let list_response = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/v1/agent-contexts?scope_id={}&session_id={session_id}&task_id=task-ctx&limit=10",
                owner_scope.as_str()
            ))
            .header("x-meat-memory-key", raw_key)
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(list_payload.as_array().unwrap().len(), 1);
    assert_eq!(list_payload[0]["context_id"], context_id);

    let promote_response = app
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/agent-contexts/{context_id}/promote"))
                .header("content-type", "application/json")
                .header("x-meat-memory-key", raw_key)
                .body(Body::from(
                    r#"{"memory_kind":"summary","visibility":"private","sensitivity":"internal"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(promote_response.status(), StatusCode::CREATED);
    let promote_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(promote_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(promote_payload["scope_id"], owner_scope.as_str());
    assert_eq!(promote_payload["memory_kind"], "summary");
    assert!(
        promote_payload["body"]
            .as_str()
            .unwrap()
            .contains("Codex is collecting")
    );

    let delete_response = app
        .clone()
        .oneshot(
            Request::delete(format!("/api/v1/agent-contexts/{context_id}"))
                .header("x-meat-memory-key", raw_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    let list_after_delete = app
        .oneshot(
            Request::get(format!(
                "/api/v1/agent-contexts?scope_id={}&session_id={session_id}&task_id=task-ctx&limit=10",
                owner_scope.as_str()
            ))
            .header("x-meat-memory-key", raw_key)
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_after_delete.status(), StatusCode::OK);
    let payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(list_after_delete.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(payload.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn http_project_document_sync_scans_imports_and_lists_documents() {
    let _guard = http_test_guard();
    let app = build_test_app().await;
    let docs_root = tempdir().unwrap();
    std::fs::write(
        docs_root.path().join("README.md"),
        "# Project README\nHTTP sync imports project docs.",
    )
    .unwrap();
    std::fs::write(
        docs_root.path().join("runbook.txt"),
        "Runbook says sync should keep local files safe.",
    )
    .unwrap();

    let owner_scope = ScopeId::new();
    let key_response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/keys")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"name":"project docs key","source":"http","owner_principal_id":"alice","owner_scope_id":"{}","scope_kind":"personal","storage_mode":"all"}}"#,
                    owner_scope.as_str()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(key_response.status(), StatusCode::CREATED);
    let key_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(key_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let raw_key = key_payload["raw_key"].as_str().unwrap();

    let source_body = serde_json::json!({
        "name": "HTTP project docs",
        "source_kind": "local_docs",
        "sync_mode": "index_only",
        "local_root": docs_root.path().to_string_lossy(),
    })
    .to_string();
    let source_response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/sources")
                .header("content-type", "application/json")
                .header("x-meat-memory-key", raw_key)
                .body(Body::from(source_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::CREATED);
    let source_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(source_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let source_id = source_payload["source_id"].as_str().unwrap();

    let sync_response = app
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/sources/{source_id}/documents/sync"))
                .header("content-type", "application/json")
                .header("x-meat-memory-key", raw_key)
                .body(Body::from(format!(
                    r#"{{"scope_id":"{}","dry_run":false}}"#,
                    owner_scope.as_str()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(sync_response.status(), StatusCode::OK);
    let sync_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(sync_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(sync_payload["dry_run"], false);
    assert_eq!(
        sync_payload["planned_documents"].as_array().unwrap().len(),
        2
    );
    assert_eq!(sync_payload["imported"].as_array().unwrap().len(), 2);
    assert!(sync_payload["missing"].as_array().unwrap().is_empty());

    let list_response = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/v1/sources/{source_id}/documents?query=README&limit=10"
            ))
            .header("x-meat-memory-key", raw_key)
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(list_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(list_payload.as_array().unwrap().len(), 1);
    assert_eq!(list_payload[0]["title"], "Project README");
    let document_id = list_payload[0]["document_id"].as_str().unwrap();

    let projection_response = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/v1/sources/{source_id}/documents/{document_id}/projection"
            ))
            .header("x-meat-memory-key", raw_key)
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(projection_response.status(), StatusCode::OK);
    let projection_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(projection_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(projection_payload["document"]["document_id"], document_id);
    assert!(
        projection_payload["projection_path"]
            .as_str()
            .unwrap()
            .contains("/sources/")
    );
    assert!(
        projection_payload["markdown"]
            .as_str()
            .unwrap()
            .contains("kind: project_document")
    );
    assert!(
        projection_payload["markdown"]
            .as_str()
            .unwrap()
            .contains("HTTP sync imports project docs.")
    );

    let conflicts_response = app
        .oneshot(
            Request::get(format!(
                "/api/v1/sources/{source_id}/documents/conflicts?limit=10"
            ))
            .header("x-meat-memory-key", raw_key)
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(conflicts_response.status(), StatusCode::OK);
    let conflicts_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(conflicts_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(conflicts_payload.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn http_vector_key_flow_writes_pg_only_and_searches_with_key_context() {
    let _guard = http_test_guard();
    let app = build_test_app().await;
    let scope_id = ScopeId::new();
    let key_response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/keys")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"name":"http vector key","source":"http","owner_principal_id":"alice","owner_scope_id":"{}","scope_kind":"personal","storage_mode":"vector"}}"#,
                    scope_id.as_str()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(key_response.status(), StatusCode::CREATED);

    let key_bytes = to_bytes(key_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let key_payload = serde_json::from_slice::<serde_json::Value>(&key_bytes).unwrap();
    let raw_key = key_payload["raw_key"].as_str().unwrap();

    let remember_response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/memories")
                .header("content-type", "application/json")
                .header("x-meat-memory-key", raw_key)
                .body(Body::from(format!(
                    r#"{{"scope_id":"{}","title":"Vector only memory","body":"Graphite apple vector retrieval works in HTTP.","memory_kind":"fact"}}"#,
                    scope_id.as_str()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(remember_response.status(), StatusCode::CREATED);
    let remember_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(remember_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(remember_payload["wrote_pg"], true);
    assert_eq!(remember_payload["wrote_markdown"], false);

    let search_response = app
        .oneshot(
            Request::post("/api/v1/context/search")
                .header("content-type", "application/json")
                .header("x-meat-memory-key", raw_key)
                .body(Body::from(format!(
                    r#"{{"scope_id":"{}","query":"graphite apple","limit":5}}"#,
                    scope_id.as_str()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(search_response.status(), StatusCode::OK);

    let payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(search_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(payload["memory_count"], 1);
}

#[tokio::test]
async fn http_key_update_rotate_and_stats_routes_work() {
    let _guard = http_test_guard();
    let app = build_test_app().await;
    let key_response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/keys")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"http manage key","source":"http","owner_principal_id":"alice","owner_scope_id":"scp_user_http_manage","scope_kind":"personal","storage_mode":"all"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let key_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(key_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let key_id = key_payload["key_id"].as_str().unwrap();
    let raw_key = key_payload["raw_key"].as_str().unwrap();

    let remember_response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/memories")
                .header("content-type", "application/json")
                .header("x-meat-memory-key", raw_key)
                .body(Body::from(
                    r#"{"scope_id":"scp_user_http_manage","title":"Managed key memory","body":"Key stats should capture HTTP writes.","memory_kind":"fact"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(remember_response.status(), StatusCode::CREATED);

    let update_response = app
        .clone()
        .oneshot(
            Request::patch(format!("/api/v1/keys/{key_id}"))
                .header("x-meat-memory-key", raw_key)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"http manage key updated","storage_mode":"vector","isolated":true}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update_response.status(), StatusCode::OK);
    let update_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(update_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(update_payload["name"], "http manage key updated");
    assert_eq!(update_payload["storage_mode"], "vector");
    assert_eq!(update_payload["isolated"], true);

    let stats_response = app
        .clone()
        .oneshot(
            Request::get(format!("/api/v1/keys/{key_id}/stats"))
                .header("x-meat-memory-key", raw_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(stats_response.status(), StatusCode::OK);
    let stats_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(stats_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(stats_payload["stats"]["total_operations"], 1);

    let rotate_response = app
        .oneshot(
            Request::post(format!("/api/v1/keys/{key_id}/rotate"))
                .header("x-meat-memory-key", raw_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rotate_response.status(), StatusCode::CREATED);
    let rotate_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(rotate_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_ne!(rotate_payload["key_id"], key_payload["key_id"]);
    assert!(
        rotate_payload["raw_key"]
            .as_str()
            .unwrap()
            .starts_with("mmk_")
    );
}

#[tokio::test]
async fn http_root_route_serves_browser_console() {
    let _guard = http_test_guard();
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
    let _guard = http_test_guard();
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
    let _guard = http_test_guard();
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
    let _guard = http_test_guard();
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
    let _guard = http_test_guard();
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
    let _guard = http_test_guard();
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
    let _guard = http_test_guard();
    let app = build_test_app().await;
    let key_response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/keys")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"alice promote key","source":"http","owner_principal_id":"alice","owner_scope_id":"scp_user_http_alice","scope_kind":"personal","storage_mode":"all"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(key_response.status(), StatusCode::CREATED);
    let key_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(key_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let raw_key = key_payload["raw_key"].as_str().unwrap();

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
                .header("x-meat-memory-key", raw_key)
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

#[tokio::test]
async fn http_browse_memories_requires_key() {
    let _guard = http_test_guard();
    let app = build_test_app().await;

    let response = app
        .oneshot(
            Request::get("/api/v1/explorer/memories?limit=20")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn http_key_listing_requires_key() {
    let _guard = http_test_guard();
    let app = build_test_app().await;

    let response = app
        .oneshot(Request::get("/api/v1/keys").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn http_browse_memories_defaults_to_owner_scope() {
    let _guard = http_test_guard();
    let app = build_test_app().await;
    let owner_scope = ScopeId::new();
    let other_scope = ScopeId::new();

    let key_response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/keys")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"name":"browse key","source":"http","owner_principal_id":"alice","owner_scope_id":"{}","scope_kind":"personal","storage_mode":"all"}}"#,
                    owner_scope.as_str()
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    let key_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(key_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let raw_key = key_payload["raw_key"].as_str().unwrap();

    for (scope_id, title) in [
        (owner_scope.as_str(), "Owner memory"),
        (other_scope.as_str(), "Other memory"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post("/api/v1/memories")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"scope_id":"{scope_id}","title":"{title}","body":"{title} body","memory_kind":"fact"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    let response = app
        .oneshot(
            Request::get("/api/v1/explorer/memories")
                .header("x-meat-memory-key", raw_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(response.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(payload["filtered_scope"], owner_scope.as_str());
    assert_eq!(payload["total_count"], 1);
}

#[tokio::test]
async fn http_promote_memory_forbids_scope_mismatch() {
    let _guard = http_test_guard();
    let app = build_test_app().await;

    let created = app
        .clone()
        .oneshot(
            Request::post("/api/v1/memories")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"scope_id":"scp_user_http_alice","title":"Alice secret","body":"token: abc123","memory_kind":"procedure","visibility":"private","sensitivity":"restricted"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let created_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(created.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let memory_id = created_payload["memory_id"].as_str().unwrap();

    let key_response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/keys")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"mallory key","source":"http","owner_principal_id":"mallory","owner_scope_id":"scp_user_http_mallory","scope_kind":"personal","storage_mode":"all"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let key_payload = serde_json::from_slice::<serde_json::Value>(
        &to_bytes(key_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let raw_key = key_payload["raw_key"].as_str().unwrap();

    let promoted = app
        .oneshot(
            Request::post("/api/v1/memories/promote")
                .header("x-meat-memory-key", raw_key)
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"source_scope_id":"scp_user_http_alice","memory_id":"{memory_id}","source_scope_type":"user","target_scope_id":"scp_project_http_demo","target_scope_type":"project","target_visibility":"project"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(promoted.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn http_search_rejects_too_long_query() {
    let _guard = http_test_guard();
    let app = build_test_app().await;
    let long_query = "q".repeat(1_025);

    let response = app
        .oneshot(
            Request::post("/api/v1/context/search")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"scope_id":"scp_long_query","query":"{long_query}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn http_create_memory_rejects_too_large_body() {
    let _guard = http_test_guard();
    let app = build_test_app().await;
    let long_body = "a".repeat(16_001);

    let response = app
        .oneshot(
            Request::post("/api/v1/memories")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"scope_id":"scp_large_body","body":"{long_body}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn http_create_image_rejects_too_large_payload() {
    let _guard = http_test_guard();
    let app = build_test_app().await;
    let oversized = vec![0_u8; 8 * 1024 * 1024 + 1];
    let payload = format!(
        r#"{{"scope_id":"scp_large_image","media_type":"image/png","image_base64":"{}"}}"#,
        STANDARD.encode(oversized)
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

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
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
