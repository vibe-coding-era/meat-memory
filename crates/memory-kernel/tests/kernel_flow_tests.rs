use memory_domain::{MemoryId, MemoryState, ScopeId, ScopeType, Sensitivity, Visibility};
use memory_kernel::{Kernel, PromoteMemoryRequest, RememberTextRequest, SearchContextRequest};
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

#[tokio::test]
async fn markdown_only_get_memory_and_promote_flow_preserves_scope_lineage() {
    let tempdir = tempdir().unwrap();
    let kernel = Kernel::builder()
        .with_markdown_root(tempdir.path())
        .unwrap()
        .build()
        .unwrap();
    let source_scope_id = ScopeId::from_string("scp_user_alice");

    let mut request = RememberTextRequest::new(
        source_scope_id.clone(),
        "token: abc123 联系人 alice@example.com 发布前需要审批",
    );
    request.title = Some("Alice 私有发布凭证".to_string());
    request.visibility = Visibility::Private;
    request.sensitivity = Sensitivity::Restricted;

    let remembered = kernel.remember_text(request).await.unwrap();
    assert!(!remembered.wrote_pg);
    assert!(remembered.wrote_markdown);

    let fetched = kernel
        .get_memory(source_scope_id.clone(), remembered.memory.id.clone())
        .await
        .unwrap()
        .expect("markdown fallback should load remembered memory");
    assert_eq!(fetched.id, remembered.memory.id);
    assert_eq!(fetched.scope_id, source_scope_id);

    let promoted = kernel
        .promote_memory_by_id(
            source_scope_id,
            remembered.memory.id.clone(),
            PromoteMemoryRequest {
                source_scope_type: ScopeType::User,
                target_scope_id: ScopeId::from_string("scp_project_demo"),
                target_scope_type: ScopeType::Project,
                target_visibility: Visibility::Project,
            },
        )
        .await
        .unwrap();

    assert!(!promoted.wrote_pg);
    assert!(promoted.wrote_markdown);
    assert_eq!(promoted.memory.scope_id.as_str(), "scp_project_demo");
    assert_eq!(promoted.memory.owner_scope_id.as_str(), "scp_user_alice");
    assert_eq!(
        promoted
            .memory
            .published_from_scope_id
            .as_ref()
            .map(|scope_id| scope_id.as_str()),
        Some("scp_user_alice")
    );
    assert_eq!(promoted.memory.state, MemoryState::Candidate);
    assert!(promoted.memory.body.contains("[REDACTED]"));

    let promoted_memory = kernel
        .get_memory(
            ScopeId::from_string("scp_project_demo"),
            MemoryId::from_string(promoted.memory.id.as_str()),
        )
        .await
        .unwrap()
        .expect("promoted memory should be readable from markdown store");
    assert_eq!(promoted_memory.scope_id.as_str(), "scp_project_demo");
    assert_eq!(promoted_memory.owner_scope_id.as_str(), "scp_user_alice");
}
