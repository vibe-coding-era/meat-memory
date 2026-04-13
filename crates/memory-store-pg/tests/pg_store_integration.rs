use memory_domain::{
    AccessKey, AccessKeyStatus, AgentContext, Artifact, ArtifactKind, DocumentConflictState,
    DocumentSyncState, KeyScopeKind, KeySourceKind, Memory, MemoryKind, MemorySource,
    ProjectDocument, Scope, ScopeId, ScopeType, SourceSyncMode, StorageMode, Visibility,
    hash_access_key,
};
use memory_store_pg::PgStore;

fn test_database_url() -> String {
    std::env::var("MEAT_MEMORY_TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@127.0.0.1:5433/meat_memory_dev".to_string()
    })
}

#[tokio::test]
async fn pg_store_can_migrate_insert_and_search() {
    let store = PgStore::connect(&test_database_url()).await.unwrap();
    store.migrate().await.unwrap();

    let scope_id = ScopeId::new();
    let scope = Scope::new_with_id(
        scope_id.clone(),
        ScopeType::Project,
        "Integration Scope",
        format!("integration/{}", scope_id.as_str()),
        None,
    )
    .unwrap()
    .with_default_visibility(Visibility::Project);
    store.seed_scope_definition(&scope).await.unwrap();

    let artifact = Artifact::new(
        scope_id.clone(),
        ArtifactKind::Message,
        "Codex prefers concise summaries",
        vec!["integration://artifact".to_string()],
    )
    .unwrap();
    let artifact_id = store.insert_artifact(&artifact).await.unwrap();

    let mut memory = Memory::new(
        scope_id.clone(),
        MemoryKind::Preference,
        "Concise summaries",
        "Codex prefers concise summaries in final responses.",
    )
    .unwrap();
    memory.activate().unwrap();
    let memory_id = store.insert_memory(&memory).await.unwrap();

    store
        .link_evidence(&memory_id, &artifact_id, Some("prefers concise summaries"))
        .await
        .unwrap();

    let results = store
        .search_by_keyword(&scope_id, "concise", 10)
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Concise summaries");
    assert_eq!(results[0].evidence_count, 1);
}

#[tokio::test]
async fn pg_store_persists_access_key_and_isolation_links() {
    let store = PgStore::connect(&test_database_url()).await.unwrap();
    store.migrate().await.unwrap();

    let scope_id = ScopeId::new();
    let raw_key = format!("mmk_pg_integration_{}", scope_id.as_str());
    let scope = Scope::new_with_id(
        scope_id.clone(),
        ScopeType::User,
        "Alice",
        format!("integration/user/{}", scope_id.as_str()),
        None,
    )
    .unwrap();
    store.seed_scope_definition(&scope).await.unwrap();

    let access_key = AccessKey::new(
        &raw_key,
        "PG integration",
        KeySourceKind::Cli,
        "alice",
        scope_id.clone(),
        KeyScopeKind::Personal,
        StorageMode::All,
        true,
    )
    .unwrap();
    store.upsert_access_key(&access_key).await.unwrap();

    let mut memory = Memory::new(
        scope_id.clone(),
        MemoryKind::Fact,
        "Isolated memory",
        "Only this isolation group can see graphite apple.",
    )
    .unwrap();
    memory.activate().unwrap();
    let memory_id = store.insert_memory(&memory).await.unwrap();
    store
        .link_memory_key(&memory_id, &access_key.id, &access_key.isolation_group_id)
        .await
        .unwrap();
    let mut embedding = vec![0.0_f32; 1536];
    embedding[42] = 1.0;
    store
        .upsert_memory_embedding(
            &memory_id,
            Some(&access_key.id),
            &access_key.isolation_group_id,
            "integration_embedding",
            &embedding,
        )
        .await
        .unwrap();

    let loaded = store
        .get_access_key_by_hash(&hash_access_key(&raw_key).unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.display_name, "PG integration");

    store
        .record_key_usage(
            Some(&access_key.id),
            KeySourceKind::Cli,
            "remember_text",
            Some(&scope_id),
            StorageMode::All,
            true,
            12,
            None,
        )
        .await
        .unwrap();
    store
        .record_key_usage(
            Some(&access_key.id),
            KeySourceKind::Cli,
            "search_context",
            Some(&scope_id),
            StorageMode::All,
            false,
            24,
            Some("boom"),
        )
        .await
        .unwrap();

    let visible = store
        .search_by_keyword_for_isolation_group(
            &scope_id,
            "graphite",
            &access_key.isolation_group_id,
            5,
        )
        .await
        .unwrap();
    let hidden = store
        .search_by_keyword_for_isolation_group(&scope_id, "graphite", "personal:bob", 5)
        .await
        .unwrap();
    assert_eq!(visible.len(), 1);
    assert!(hidden.is_empty());

    let vector_visible = store
        .search_by_embedding(
            &scope_id,
            &embedding,
            Some(&access_key.isolation_group_id),
            5,
        )
        .await
        .unwrap();
    let vector_hidden = store
        .search_by_embedding(&scope_id, &embedding, Some("personal:bob"), 5)
        .await
        .unwrap();
    assert_eq!(vector_visible.len(), 1);
    assert!(vector_hidden.is_empty());

    let stats = store
        .access_key_usage_stats(&access_key.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stats.total_operations, 2);
    assert_eq!(stats.successful_operations, 1);
    assert_eq!(stats.failed_operations, 1);
    assert_eq!(stats.by_storage_mode[0].label, "all");

    store
        .update_access_key_status(&access_key.id, AccessKeyStatus::Disabled)
        .await
        .unwrap();
    let updated = store
        .get_access_key_by_id(&access_key.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.status, AccessKeyStatus::Disabled);
}

#[tokio::test]
async fn pg_store_persists_v2_4_sources_contexts_and_documents() {
    let store = PgStore::connect(&test_database_url()).await.unwrap();
    store.migrate().await.unwrap();

    let scope_id = ScopeId::new();
    let scope = Scope::new_with_id(
        scope_id.clone(),
        ScopeType::Project,
        "V2.4 Source Scope",
        format!("integration/v2_4/{}", scope_id.as_str()),
        None,
    )
    .unwrap();
    store.seed_scope_definition(&scope).await.unwrap();

    let source = MemorySource::new("cli", "codex-local", "rou", scope_id.clone())
        .unwrap()
        .with_source_uri("agent://codex/local")
        .unwrap()
        .with_local_root("/Users/Rou/dev_projects/meat-memory")
        .unwrap()
        .with_sync_mode(SourceSyncMode::IndexOnly);
    store.upsert_memory_source(&source).await.unwrap();

    let loaded_source = store.get_memory_source(&source.id).await.unwrap().unwrap();
    assert_eq!(loaded_source.display_name, "codex-local");
    assert_eq!(loaded_source.sync_mode, SourceSyncMode::IndexOnly);
    assert_eq!(
        store
            .list_memory_sources(&scope_id, 10)
            .await
            .unwrap()
            .len(),
        1
    );

    let key_a = AccessKey::new(
        &format!("mmk_v24_a_{}", scope_id.as_str()),
        "codex-a",
        KeySourceKind::Cli,
        "rou",
        scope_id.clone(),
        KeyScopeKind::Personal,
        StorageMode::All,
        false,
    )
    .unwrap()
    .with_source_id(source.id.clone());
    let key_b = AccessKey::new(
        &format!("mmk_v24_b_{}", scope_id.as_str()),
        "codex-b",
        KeySourceKind::Cli,
        "rou",
        scope_id.clone(),
        KeyScopeKind::Personal,
        StorageMode::File,
        false,
    )
    .unwrap()
    .with_source_id(source.id.clone());
    store.upsert_access_key(&key_a).await.unwrap();
    store.upsert_access_key(&key_b).await.unwrap();

    let source_keys = store
        .list_access_keys_for_source(&source.id, 10)
        .await
        .unwrap();
    assert_eq!(source_keys.len(), 2);
    assert!(
        source_keys
            .iter()
            .all(|key| key.source_id == Some(source.id.clone()))
    );

    let mut context = AgentContext::new(
        scope_id.clone(),
        "session-v2-4",
        "Current V2.4 task",
        "Implement PG store CRUD for source/context/document.",
    )
    .unwrap();
    context.source_id = Some(source.id.clone());
    context.key_id = Some(key_a.id.clone());
    context.task_id = Some("V2.4-STO-001".to_string());
    context.labels = vec!["v2.4".to_string(), "store".to_string()];
    store.upsert_agent_context(&context).await.unwrap();

    let contexts = store
        .list_agent_contexts(&scope_id, "session-v2-4", Some("V2.4-STO-001"), 10)
        .await
        .unwrap();
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].source_id, Some(source.id.clone()));
    assert_eq!(
        contexts[0].labels,
        vec!["v2.4".to_string(), "store".to_string()]
    );

    store.delete_agent_context(&context.id).await.unwrap();
    assert!(
        store
            .list_agent_contexts(&scope_id, "session-v2-4", None, 10)
            .await
            .unwrap()
            .is_empty()
    );

    let mut document = ProjectDocument::new(
        source.id.clone(),
        scope_id.clone(),
        "file:///Users/Rou/dev_projects/meat-memory/docs/README.md",
        "Docs README",
        "sha256:v24",
    )
    .unwrap();
    document.local_path = Some("/Users/Rou/dev_projects/meat-memory/docs/README.md".to_string());
    document.sync_state = DocumentSyncState::Conflicted;
    document.conflict_state = DocumentConflictState::BothChanged;
    store.upsert_project_document(&document).await.unwrap();

    let loaded_document = store
        .get_project_document(&source.id, &document.canonical_uri)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded_document.title, "Docs README");
    assert_eq!(loaded_document.sync_state, DocumentSyncState::Conflicted);

    let documents = store
        .list_project_documents_for_source(&source.id, 10)
        .await
        .unwrap();
    assert_eq!(documents.len(), 1);

    let conflicts = store
        .list_project_document_conflicts(&source.id, 10)
        .await
        .unwrap();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts[0].conflict_state,
        DocumentConflictState::BothChanged
    );
}
