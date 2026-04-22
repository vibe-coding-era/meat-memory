use memory_domain::{
    DocumentConflictState, DocumentSyncState, KeyScopeKind, KeySourceKind, MemoryId,
    MemoryRecordStatus, MemorySource, MemoryState, ScopeId, ScopeType, Sensitivity, SourceSyncMode,
    StorageMode, Visibility,
};
use memory_kernel::{
    ApplyProjectDocumentSyncPlanRequest, ChangeMemoryLifecycleStatusRequest,
    CreateAccessKeyRequest, ImportProjectDocumentRequest, InspectMemoryLifecycleRequest, Kernel,
    ListAgentContextsRequest, ListProjectDocumentsRequest, PromoteAgentContextRequest,
    PromoteMemoryRequest, RememberTextRequest, SearchContextRequest, UpsertAgentContextRequest,
};
use memory_store_md::MarkdownStore;
use memory_sync::{LocalProjectDocumentSyncEngine, ProjectDocumentSnapshot};
use std::env;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;
use tempfile::tempdir;

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

#[tokio::test]
async fn remember_search_publish_flow_works_with_pg_and_markdown() {
    if !local_pg_test_port_available() {
        return;
    }
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
    if !local_pg_test_port_available() {
        return;
    }
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

#[tokio::test]
async fn search_context_graph_expansion_finds_related_memories_without_leaking_other_isolation_groups()
 {
    if !local_pg_test_port_available() {
        return;
    }
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
    let key_a = kernel
        .create_access_key(CreateAccessKeyRequest {
            raw_key: Some(format!("mmk_kernel_graph_a_{}", scope_id.as_str())),
            display_name: "graph a".to_string(),
            source_id: None,
            source_kind: KeySourceKind::Cli,
            owner_principal_id: "alice".to_string(),
            owner_scope_id: scope_id.clone(),
            scope_kind: KeyScopeKind::Personal,
            storage_mode: StorageMode::All,
            is_fully_isolated: true,
        })
        .await
        .unwrap();
    let context = kernel
        .resolve_access_key_context(&key_a.raw_key)
        .await
        .unwrap()
        .unwrap();

    let mut seed = RememberTextRequest::new(
        scope_id.clone(),
        "Project Meat Memory uses Service Gateway for context routing.",
    );
    seed.title = Some("Gateway relation".to_string());
    seed.context = Some(context.clone());
    kernel.remember_text(seed).await.unwrap();

    let mut neighbor = RememberTextRequest::new(
        scope_id.clone(),
        "Service Gateway deployment playbook for release windows.",
    );
    neighbor.title = Some("Gateway deployment".to_string());
    neighbor.context = Some(context.clone());
    kernel.remember_text(neighbor).await.unwrap();

    let mut hidden = RememberTextRequest::new(
        scope_id.clone(),
        "Service Gateway secret memory for bob only.",
    );
    hidden.title = Some("Bob hidden gateway".to_string());
    let key_b = kernel
        .create_access_key(CreateAccessKeyRequest {
            raw_key: Some(format!("mmk_kernel_graph_b_{}", scope_id.as_str())),
            display_name: "graph b".to_string(),
            source_id: None,
            source_kind: KeySourceKind::Cli,
            owner_principal_id: "bob".to_string(),
            owner_scope_id: scope_id.clone(),
            scope_kind: KeyScopeKind::Personal,
            storage_mode: StorageMode::All,
            is_fully_isolated: true,
        })
        .await
        .unwrap();
    hidden.context = Some(
        kernel
            .resolve_access_key_context(&key_b.raw_key)
            .await
            .unwrap()
            .unwrap(),
    );
    kernel.remember_text(hidden).await.unwrap();

    let mut search = SearchContextRequest::new(scope_id, "project meat memory");
    search.context = Some(context);
    let bundle = kernel.search_context(search).await.unwrap();

    assert_eq!(bundle.memories.len(), 2);
    assert_eq!(bundle.memories[0].title, "Gateway relation");
    assert!(
        bundle
            .memories
            .iter()
            .any(|memory| memory.title == "Gateway deployment")
    );
    assert!(
        !bundle
            .memories
            .iter()
            .any(|memory| memory.title == "Bob hidden gateway")
    );
}

#[tokio::test]
async fn lifecycle_governance_flow_forgets_restores_reports_and_audits() {
    if !local_pg_test_port_available() {
        return;
    }
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
        "V2.7 lifecycle memories can be inspected, forgotten, restored, and reported.",
    );
    request.title = Some("V2.7 lifecycle governance".to_string());
    request.source_refs = vec!["agent-context://ctx_lifecycle_flow".to_string()];
    let remembered = kernel.remember_text(request).await.unwrap();

    let inspected = kernel
        .inspect_memory_lifecycle(InspectMemoryLifecycleRequest {
            scope_id: scope_id.clone(),
            memory_id: remembered.memory.id.clone(),
            query: Some("lifecycle".to_string()),
            context: None,
        })
        .await
        .unwrap();
    assert_eq!(
        inspected.record.source_ref.as_deref(),
        Some("agent-context://ctx_lifecycle_flow")
    );
    assert!(
        inspected
            .explanation
            .unwrap()
            .reason
            .contains("query matched")
    );

    let forgotten = kernel
        .change_memory_lifecycle_status(ChangeMemoryLifecycleStatusRequest {
            scope_id: scope_id.clone(),
            memory_id: remembered.memory.id.clone(),
            status: MemoryRecordStatus::Forgotten,
            reason: "user requested soft forget".to_string(),
            actor: "test".to_string(),
            context: None,
        })
        .await
        .unwrap();
    assert_eq!(forgotten.memory.state, MemoryState::Forgotten);
    assert_eq!(forgotten.record.status, MemoryRecordStatus::Forgotten);

    let hidden = kernel
        .search_context(SearchContextRequest::new(scope_id.clone(), "lifecycle"))
        .await
        .unwrap();
    assert!(hidden.memories.is_empty());

    let restored = kernel
        .change_memory_lifecycle_status(ChangeMemoryLifecycleStatusRequest {
            scope_id: scope_id.clone(),
            memory_id: remembered.memory.id.clone(),
            status: MemoryRecordStatus::Active,
            reason: "needed again".to_string(),
            actor: "test".to_string(),
            context: None,
        })
        .await
        .unwrap();
    assert_eq!(restored.memory.state, MemoryState::Active);

    let report = kernel
        .memory_health_report(Some(scope_id.clone()), 50)
        .await
        .unwrap();
    assert_eq!(report.total, 1);
    assert_eq!(report.active, 1);
    assert_eq!(report.source_backed, 1);

    let audit = kernel
        .list_lifecycle_audit_events(Some(scope_id), Some(remembered.memory.id), 10)
        .await
        .unwrap();
    assert!(
        audit
            .iter()
            .any(|event| event.after_status.as_deref() == Some("forgotten"))
    );
    assert!(
        audit
            .iter()
            .any(|event| event.after_status.as_deref() == Some("active"))
    );
}

#[tokio::test]
async fn agent_context_flow_promotes_short_term_context_to_memory() {
    if !local_pg_test_port_available() {
        return;
    }
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
    let key = kernel
        .create_access_key(CreateAccessKeyRequest {
            raw_key: Some(format!("mmk_kernel_context_{}", scope_id.as_str())),
            display_name: "agent context key".to_string(),
            source_id: None,
            source_kind: KeySourceKind::Cli,
            owner_principal_id: "alice".to_string(),
            owner_scope_id: scope_id.clone(),
            scope_kind: KeyScopeKind::Personal,
            storage_mode: StorageMode::All,
            is_fully_isolated: false,
        })
        .await
        .unwrap();
    let context = kernel
        .resolve_access_key_context(&key.raw_key)
        .await
        .unwrap()
        .unwrap();

    let mut upsert = UpsertAgentContextRequest::new(
        scope_id.clone(),
        "session-v2-4-kernel",
        "Current V2.4 kernel task",
        "Short term context says the next step is Kernel service wiring.",
    );
    upsert.task_id = Some("V2.4-KER-001".to_string());
    upsert.labels = vec!["v2.4".to_string(), "kernel".to_string()];
    upsert.context = Some(context.clone());
    let agent_context = kernel.upsert_agent_context(upsert).await.unwrap();
    assert_eq!(agent_context.key_id, Some(context.key_id.clone()));

    let mut list = ListAgentContextsRequest::new(scope_id.clone(), "session-v2-4-kernel");
    list.task_id = Some("V2.4-KER-001".to_string());
    list.context = Some(context.clone());
    let contexts = kernel.list_agent_contexts(list).await.unwrap();
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].title, "Current V2.4 kernel task");

    let mut promote = PromoteAgentContextRequest::new(agent_context.id.clone());
    promote.context = Some(context.clone());
    let remembered = kernel.promote_agent_context(promote).await.unwrap();
    assert_eq!(remembered.memory.kind, memory_domain::MemoryKind::Summary);
    assert!(remembered.wrote_pg);
    assert!(remembered.wrote_markdown);

    let mut search = SearchContextRequest::new(scope_id.clone(), "Kernel service wiring");
    search.context = Some(context.clone());
    let bundle = kernel.search_context(search).await.unwrap();
    assert_eq!(bundle.memories.len(), 1);
    assert_eq!(bundle.memories[0].title, "Current V2.4 kernel task");

    kernel
        .delete_agent_context(agent_context.id, Some(&context))
        .await
        .unwrap();
    let mut list_after_delete = ListAgentContextsRequest::new(scope_id, "session-v2-4-kernel");
    list_after_delete.context = Some(context);
    assert!(
        kernel
            .list_agent_contexts(list_after_delete)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn project_document_kernel_flow_imports_lists_and_reports_conflicts() {
    if !local_pg_test_port_available() {
        return;
    }
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
    let key = kernel
        .create_access_key(CreateAccessKeyRequest {
            raw_key: Some(format!("mmk_kernel_docs_{}", scope_id.as_str())),
            display_name: "project docs key".to_string(),
            source_id: None,
            source_kind: KeySourceKind::Cli,
            owner_principal_id: "alice".to_string(),
            owner_scope_id: scope_id.clone(),
            scope_kind: KeyScopeKind::Personal,
            storage_mode: StorageMode::All,
            is_fully_isolated: false,
        })
        .await
        .unwrap();
    let context = kernel
        .resolve_access_key_context(&key.raw_key)
        .await
        .unwrap()
        .unwrap();

    let source = MemorySource::new("cli", "kernel-docs-source", "alice", scope_id.clone())
        .unwrap()
        .with_source_uri("file:///Users/Rou/dev_projects/meat-memory/docs")
        .unwrap()
        .with_local_root("/Users/Rou/dev_projects/meat-memory/docs")
        .unwrap()
        .with_sync_mode(SourceSyncMode::IndexOnly);
    let source = kernel
        .upsert_memory_source(source, Some(&context))
        .await
        .unwrap();

    let mut import = ImportProjectDocumentRequest::new(
        source.id.clone(),
        scope_id.clone(),
        "file:///Users/Rou/dev_projects/meat-memory/docs/meat-memory-scheme-v2_4.md",
        "V2.4 Scheme",
        "V2.4 introduces short-term context and mid-term project document memory.",
    );
    import.local_path =
        Some("/Users/Rou/dev_projects/meat-memory/docs/meat-memory-scheme-v2_4.md".to_string());
    import.sync_state = DocumentSyncState::Conflicted;
    import.conflict_state = DocumentConflictState::BothChanged;
    import.context = Some(context.clone());

    let document = kernel.import_project_document(import).await.unwrap();
    assert_eq!(document.title, "V2.4 Scheme");
    assert!(document.artifact_id.is_some());
    assert_eq!(document.sync_state, DocumentSyncState::Conflicted);
    let markdown_store = MarkdownStore::new(tempdir.path()).unwrap();
    let projection = markdown_store
        .find_project_document_markdown(&source.id, &document.id)
        .unwrap()
        .expect("project document projection should exist");
    assert_eq!(projection.frontmatter.kind, "project_document");
    assert_eq!(projection.frontmatter.sync_state, "conflicted");
    assert_eq!(projection.frontmatter.conflict_state, "both_changed");
    assert!(
        projection
            .body
            .contains("V2.4 introduces short-term context")
    );

    let mut list = ListProjectDocumentsRequest::new(source.id.clone());
    list.query = Some("scheme".to_string());
    list.context = Some(context.clone());
    let documents = kernel.list_project_documents(list).await.unwrap();
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].canonical_uri, document.canonical_uri);

    let conflicts = kernel
        .list_project_document_conflicts(source.id.clone(), 10, Some(&context))
        .await
        .unwrap();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts[0].conflict_state,
        DocumentConflictState::BothChanged
    );

    let sources = kernel
        .list_memory_sources(scope_id, 10, Some(&context))
        .await
        .unwrap();
    assert_eq!(sources.len(), 1);
}

#[tokio::test]
async fn project_document_sync_plan_imports_local_documents_through_kernel() {
    if !local_pg_test_port_available() {
        return;
    }
    let tempdir = tempdir().unwrap();
    let docs_root = tempdir.path().join("docs");
    std::fs::create_dir_all(&docs_root).unwrap();
    std::fs::write(
        docs_root.join("sync-plan.md"),
        "# Sync Plan\nV2.4 sync plan imports local project docs.",
    )
    .unwrap();
    let kernel = Kernel::builder()
        .with_postgres_url(&test_database_url())
        .await
        .unwrap()
        .with_markdown_root(tempdir.path().join("markdown"))
        .unwrap()
        .build()
        .unwrap();
    let scope_id = ScopeId::new();
    let key = kernel
        .create_access_key(CreateAccessKeyRequest {
            raw_key: Some(format!("mmk_kernel_sync_docs_{}", scope_id.as_str())),
            display_name: "project docs sync key".to_string(),
            source_id: None,
            source_kind: KeySourceKind::Cli,
            owner_principal_id: "alice".to_string(),
            owner_scope_id: scope_id.clone(),
            scope_kind: KeyScopeKind::Personal,
            storage_mode: StorageMode::All,
            is_fully_isolated: false,
        })
        .await
        .unwrap();
    let context = kernel
        .resolve_access_key_context(&key.raw_key)
        .await
        .unwrap()
        .unwrap();
    let source = MemorySource::new("cli", "kernel-sync-docs-source", "alice", scope_id.clone())
        .unwrap()
        .with_local_root(docs_root.to_string_lossy())
        .unwrap()
        .with_sync_mode(SourceSyncMode::IndexOnly);
    let source = kernel
        .upsert_memory_source(source, Some(&context))
        .await
        .unwrap();

    let plan = LocalProjectDocumentSyncEngine::new(&docs_root)
        .scan(&[ProjectDocumentSnapshot {
            canonical_uri: "file:///missing-sync-doc.md".to_string(),
            content_hash: "old".to_string(),
        }])
        .unwrap();
    let result = kernel
        .apply_project_document_sync_plan(ApplyProjectDocumentSyncPlanRequest {
            source_id: source.id.clone(),
            scope_id: scope_id.clone(),
            plan,
            context: Some(context.clone()),
        })
        .await
        .unwrap();

    assert_eq!(result.imported.len(), 1);
    assert_eq!(result.imported[0].title, "Sync Plan");
    assert_eq!(result.imported[0].sync_state, DocumentSyncState::Changed);
    assert_eq!(result.missing.len(), 1);
    assert_eq!(result.conflicts.len(), 1);
    assert_eq!(result.conflicts[0].sync_state, DocumentSyncState::Deleted);

    let documents = kernel
        .list_project_documents(ListProjectDocumentsRequest {
            source_id: source.id,
            limit: 10,
            query: Some("sync plan".to_string()),
            context: Some(context),
        })
        .await
        .unwrap();
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].title, "Sync Plan");
}
