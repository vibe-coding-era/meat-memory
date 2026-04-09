use memory_domain::{ScopeId, Visibility};
use memory_kernel::{Kernel, RememberTextRequest, SearchContextRequest};
use std::env;
use tempfile::tempdir;

fn test_database_url() -> String {
    env::var("MEAT_MEMORY_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5433/meat_memory_dev".into())
}

#[tokio::test]
async fn remember_search_publish_flow_works_with_pg_and_markdown() {
    let tempdir = tempdir().unwrap();
    let kernel = Kernel::builder()
        .with_postgres_url(&test_database_url())
        .await
        .unwrap()
        .with_markdown_root(tempdir.path())
        .unwrap()
        .build()
        .unwrap();
    let scope_id = ScopeId::new();

    let mut request = RememberTextRequest::new(
        scope_id.clone(),
        "Project Meat Memory uses Service Gateway for HTTP context search.",
    );
    request.title = Some("Gateway relation".to_string());

    let remembered = kernel.remember_text(request).await.unwrap();
    assert!(remembered.wrote_pg);
    assert!(remembered.wrote_markdown);

    let context = kernel
        .search_context(SearchContextRequest::new(scope_id.clone(), "gateway"))
        .await
        .unwrap();
    assert_eq!(context.memories.len(), 1);
    assert!(context.entities.len() >= 2);
    assert!(!context.relations.is_empty());

    let published = kernel
        .publish_memory(remembered.memory, Visibility::Team)
        .await
        .unwrap();
    assert_eq!(published.memory.visibility, Visibility::Team);
    assert!(published.wrote_pg);
    assert!(published.wrote_markdown);

    let raw = std::fs::read_to_string(
        tempdir
            .path()
            .join("default")
            .join("scopes")
            .join(scope_id.as_str())
            .join("MEMORY.md"),
    )
    .unwrap();
    assert!(raw.contains("visibility: team"));
}

#[tokio::test]
async fn remember_search_publish_flow_supports_chinese_context_and_relations() {
    let tempdir = tempdir().unwrap();
    let kernel = Kernel::builder()
        .with_postgres_url(&test_database_url())
        .await
        .unwrap()
        .with_markdown_root(tempdir.path())
        .unwrap()
        .build()
        .unwrap();
    let scope_id = ScopeId::new();

    let mut request = RememberTextRequest::new(
        scope_id.clone(),
        "项目 `服务网关` 依赖 `PostgreSQL`，`服务网关` 记录 `发布手册`。发布前必须通过回归与验收。",
    );
    request.title = Some("中文发布验收规则".to_string());

    let remembered = kernel.remember_text(request).await.unwrap();
    assert!(remembered.wrote_pg);
    assert!(remembered.wrote_markdown);

    let context = kernel
        .search_context(SearchContextRequest::new(scope_id.clone(), "发布前 验收"))
        .await
        .unwrap();
    assert_eq!(context.memories.len(), 1);
    assert!(
        context
            .entities
            .iter()
            .any(|entity| entity.canonical_name == "服务网关")
    );
    assert!(
        context
            .relations
            .iter()
            .any(|relation| relation.relation_type == memory_domain::RelationType::DependsOn)
    );
    assert!(
        context
            .relations
            .iter()
            .any(|relation| relation.relation_type == memory_domain::RelationType::Documents)
    );

    let published = kernel
        .publish_memory(remembered.memory, Visibility::Project)
        .await
        .unwrap();
    assert_eq!(published.memory.visibility, Visibility::Project);
    assert!(published.wrote_pg);
    assert!(published.wrote_markdown);

    let raw = std::fs::read_to_string(
        tempdir
            .path()
            .join("default")
            .join("scopes")
            .join(scope_id.as_str())
            .join("MEMORY.md"),
    )
    .unwrap();
    assert!(raw.contains("中文发布验收规则"));
    assert!(raw.contains("visibility: project"));
}
