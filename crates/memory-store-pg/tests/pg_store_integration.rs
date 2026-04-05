use memory_domain::{Artifact, ArtifactKind, Memory, MemoryKind, ScopeId};
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
    store
        .seed_scope(
            &scope_id,
            "Integration Scope",
            &format!("integration/{}", scope_id.as_str()),
        )
        .await
        .unwrap();

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
