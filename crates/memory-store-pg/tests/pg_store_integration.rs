use memory_domain::{
    AccessKey, AccessKeyStatus, Artifact, ArtifactKind, KeyScopeKind, KeySourceKind, Memory,
    MemoryKind, Scope, ScopeId, ScopeType, StorageMode, Visibility, hash_access_key,
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
