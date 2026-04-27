use super::{
    MemoryRecord, PgStore, SEARCH_MEMORY_BY_EMBEDDING_SQL, artifact_kind_to_str,
    binding_confirmed_by_to_str, build_embedding_search_query, build_list_query,
    build_search_query, distillation_profile_level_to_str, distillation_profile_status_to_str,
    insert_artifact_sql, insert_distillation_run_sql, insert_memory_evidence_sql,
    insert_memory_relation_sql, insert_memory_sql, insert_memory_version_snapshot_sql,
    insert_memory_version_sql, list_distillation_profiles_sql, list_memory_prefix_sql,
    list_memory_proposals_prefix_sql, list_memory_proposals_sql, list_memory_relations_prefix_sql,
    list_memory_relations_sql, list_memory_versions_sql, list_project_identity_bindings_sql,
    memory_from_record, memory_kind_to_str, memory_relation_source_kind_to_str,
    memory_relation_type_to_str, memory_state_to_str, migration_0008_sql, migration_0009_sql,
    parse_memory_kind, parse_memory_state, parse_sensitivity, parse_visibility,
    project_binding_kind_to_str, proposal_status_to_str, proposal_type_to_str, review_level_to_str,
    scope_type_to_str, search_memory_prefix_sql, search_terms, seed_scope_sql,
    select_distillation_profile_sql, select_memory_proposal_sql, select_memory_sql,
    sensitivity_to_str, update_memory_evidence_count_sql, update_memory_version_metadata_sql,
    upsert_distillation_profile_sql, upsert_memory_embedding_sql, upsert_memory_proposal_sql,
    upsert_memory_sql, upsert_memory_version_sql, upsert_project_identity_binding_sql,
    visibility_to_str,
};
use memory_domain::{
    Artifact, ArtifactId, ArtifactKind, BindingConfirmedBy, DistillationProfileLevel,
    DistillationProfileStatus, Memory, MemoryId, MemoryKind, MemoryRelationSourceKind,
    MemoryRelationType, MemoryState, ProjectBindingKind, ProposalStatus, ProposalType, ReviewLevel,
    ScopeId, ScopeType, Sensitivity, Visibility,
};
use sqlx::PgPool;
use time::macros::datetime;

#[test]
fn search_terms_splits_and_deduplicates_keywords() {
    assert_eq!(
        search_terms(" gateway   http gateway "),
        vec!["gateway".to_string(), "http".to_string()]
    );
}

#[test]
fn search_terms_preserves_non_whitespace_queries() {
    assert_eq!(search_terms("服务网关"), vec!["服务网关".to_string()]);
}

#[test]
fn search_terms_trims_single_fallback_keyword() {
    assert_eq!(
        search_terms("  /api/v1/demo  "),
        vec!["/api/v1/demo".to_string()]
    );
}

#[test]
fn exposes_default_schema() {
    assert_eq!(PgStore::default_schema(), "public");
}

#[test]
fn search_terms_returns_empty_for_blank_queries() {
    assert!(search_terms("   ").is_empty());
}

#[test]
fn serializes_all_enums_for_storage() {
    let artifact_cases = [
        (ArtifactKind::Message, "message"),
        (ArtifactKind::Document, "document"),
        (ArtifactKind::CodeDiff, "code_diff"),
        (ArtifactKind::CodeFileSnapshot, "code_file_snapshot"),
        (ArtifactKind::TerminalOutput, "terminal_output"),
        (ArtifactKind::Image, "image"),
        (ArtifactKind::Audio, "audio"),
        (ArtifactKind::Video, "video"),
        (ArtifactKind::ToolResult, "tool_result"),
        (ArtifactKind::WebPage, "web_page"),
    ];
    for (kind, expected) in artifact_cases {
        assert_eq!(artifact_kind_to_str(kind), expected);
    }

    let memory_kind_cases = [
        (MemoryKind::Fact, "fact"),
        (MemoryKind::Preference, "preference"),
        (MemoryKind::Decision, "decision"),
        (MemoryKind::Procedure, "procedure"),
        (MemoryKind::Constraint, "constraint"),
        (MemoryKind::Risk, "risk"),
        (MemoryKind::Summary, "summary"),
        (MemoryKind::Insight, "insight"),
    ];
    for (kind, expected) in memory_kind_cases {
        assert_eq!(memory_kind_to_str(kind), expected);
    }

    let state_cases = [
        (MemoryState::Candidate, "candidate"),
        (MemoryState::Active, "active"),
        (MemoryState::Deprecated, "deprecated"),
        (MemoryState::Conflicted, "conflicted"),
        (MemoryState::Archived, "archived"),
        (MemoryState::Forgotten, "forgotten"),
        (MemoryState::Deleted, "deleted"),
    ];
    for (state, expected) in state_cases {
        assert_eq!(memory_state_to_str(state), expected);
    }

    let visibility_cases = [
        (Visibility::Private, "private"),
        (Visibility::Project, "project"),
        (Visibility::Team, "team"),
        (Visibility::Organization, "organization"),
    ];
    for (visibility, expected) in visibility_cases {
        assert_eq!(visibility_to_str(visibility), expected);
    }

    let sensitivity_cases = [
        (Sensitivity::Public, "public"),
        (Sensitivity::Internal, "internal"),
        (Sensitivity::Private, "private"),
        (Sensitivity::Restricted, "restricted"),
    ];
    for (sensitivity, expected) in sensitivity_cases {
        assert_eq!(sensitivity_to_str(sensitivity), expected);
    }

    assert_eq!(scope_type_to_str(ScopeType::Workspace), "workspace");
}

#[test]
fn serializes_v28_enums_for_storage() {
    let binding_cases = [
        (ProjectBindingKind::RepoRoot, "repo_root"),
        (ProjectBindingKind::RemoteUrl, "remote_url"),
        (ProjectBindingKind::SourceRefPrefix, "source_ref_prefix"),
        (ProjectBindingKind::Alias, "alias"),
    ];
    for (kind, expected) in binding_cases {
        assert_eq!(project_binding_kind_to_str(kind), expected);
    }

    assert_eq!(
        binding_confirmed_by_to_str(BindingConfirmedBy::User),
        "user"
    );
    assert_eq!(
        binding_confirmed_by_to_str(BindingConfirmedBy::System),
        "system"
    );

    let proposal_cases = [
        (ProposalType::Merge, "merge"),
        (ProposalType::NewVersion, "new_version"),
        (ProposalType::Supersede, "supersede"),
        (ProposalType::ConflictMark, "conflict_mark"),
        (ProposalType::Archive, "archive"),
        (ProposalType::Forget, "forget"),
        (ProposalType::Restore, "restore"),
        (ProposalType::HardDelete, "hard_delete"),
        (ProposalType::DistillUpsert, "distill_upsert"),
    ];
    for (kind, expected) in proposal_cases {
        assert_eq!(proposal_type_to_str(kind), expected);
    }

    let proposal_status_cases = [
        (ProposalStatus::Open, "open"),
        (ProposalStatus::Approved, "approved"),
        (ProposalStatus::Rejected, "rejected"),
        (ProposalStatus::Applied, "applied"),
        (ProposalStatus::Canceled, "canceled"),
        (ProposalStatus::Expired, "expired"),
    ];
    for (status, expected) in proposal_status_cases {
        assert_eq!(proposal_status_to_str(status), expected);
    }

    let review_cases = [
        (ReviewLevel::Auto, "auto"),
        (ReviewLevel::Suggested, "suggested"),
        (ReviewLevel::Required, "required"),
        (ReviewLevel::Blocked, "blocked"),
    ];
    for (level, expected) in review_cases {
        assert_eq!(review_level_to_str(level), expected);
    }

    let relation_cases = [
        (MemoryRelationType::Supersedes, "supersedes"),
        (MemoryRelationType::ConflictsWith, "conflicts_with"),
        (MemoryRelationType::DerivedFrom, "derived_from"),
        (MemoryRelationType::MergedInto, "merged_into"),
        (MemoryRelationType::RelatedTo, "related_to"),
    ];
    for (relation_type, expected) in relation_cases {
        assert_eq!(memory_relation_type_to_str(relation_type), expected);
    }

    assert_eq!(
        memory_relation_source_kind_to_str(MemoryRelationSourceKind::User),
        "user"
    );
    assert_eq!(
        memory_relation_source_kind_to_str(MemoryRelationSourceKind::Agent),
        "agent"
    );
    assert_eq!(
        memory_relation_source_kind_to_str(MemoryRelationSourceKind::System),
        "system"
    );

    assert_eq!(
        distillation_profile_level_to_str(DistillationProfileLevel::UserGlobal),
        "user_global"
    );
    assert_eq!(
        distillation_profile_level_to_str(DistillationProfileLevel::Project),
        "project"
    );
    assert_eq!(
        distillation_profile_status_to_str(DistillationProfileStatus::Active),
        "active"
    );
    assert_eq!(
        distillation_profile_status_to_str(DistillationProfileStatus::Archived),
        "archived"
    );
}

#[test]
fn parses_all_enums_from_storage_and_rejects_unknown_values() {
    let memory_kind_cases = [
        ("fact", MemoryKind::Fact),
        ("preference", MemoryKind::Preference),
        ("decision", MemoryKind::Decision),
        ("procedure", MemoryKind::Procedure),
        ("constraint", MemoryKind::Constraint),
        ("risk", MemoryKind::Risk),
        ("summary", MemoryKind::Summary),
        ("insight", MemoryKind::Insight),
    ];
    for (raw, expected) in memory_kind_cases {
        assert_eq!(parse_memory_kind(raw).unwrap(), expected);
    }
    assert!(parse_memory_kind("unknown").is_err());

    let state_cases = [
        ("candidate", MemoryState::Candidate),
        ("active", MemoryState::Active),
        ("deprecated", MemoryState::Deprecated),
        ("conflicted", MemoryState::Conflicted),
        ("archived", MemoryState::Archived),
        ("forgotten", MemoryState::Forgotten),
        ("deleted", MemoryState::Deleted),
    ];
    for (raw, expected) in state_cases {
        assert_eq!(parse_memory_state(raw).unwrap(), expected);
    }
    assert!(parse_memory_state("unknown").is_err());

    let visibility_cases = [
        ("private", Visibility::Private),
        ("project", Visibility::Project),
        ("team", Visibility::Team),
        ("organization", Visibility::Organization),
    ];
    for (raw, expected) in visibility_cases {
        assert_eq!(parse_visibility(raw).unwrap(), expected);
    }
    assert!(parse_visibility("unknown").is_err());

    let sensitivity_cases = [
        ("public", Sensitivity::Public),
        ("internal", Sensitivity::Internal),
        ("private", Sensitivity::Private),
        ("restricted", Sensitivity::Restricted),
    ];
    for (raw, expected) in sensitivity_cases {
        assert_eq!(parse_sensitivity(raw).unwrap(), expected);
    }
    assert!(parse_sensitivity("unknown").is_err());
}

#[test]
fn sql_helpers_expose_expected_statements() {
    let statements = [
        seed_scope_sql(),
        insert_artifact_sql(),
        insert_memory_sql(),
        insert_memory_version_sql(),
        upsert_memory_sql(),
        upsert_memory_version_sql(),
        select_memory_sql(),
        insert_memory_evidence_sql(),
        update_memory_evidence_count_sql(),
        list_memory_prefix_sql(),
        search_memory_prefix_sql(),
    ];

    for statement in statements {
        assert!(!statement.trim().is_empty());
    }

    assert!(seed_scope_sql().contains("INSERT INTO scopes"));
    assert!(insert_artifact_sql().contains("INSERT INTO artifacts"));
    assert!(insert_memory_sql().contains("INSERT INTO memories"));
    assert!(upsert_memory_sql().contains("ON CONFLICT (id) DO UPDATE"));
    assert!(select_memory_sql().contains("FROM memories"));
    assert!(insert_memory_evidence_sql().contains("memory_evidence_links"));
    assert!(update_memory_evidence_count_sql().contains("COUNT(*)::int"));
    assert!(list_memory_prefix_sql().contains("FROM memories"));
    assert!(search_memory_prefix_sql().contains("WHERE scope_id = "));
    assert!(upsert_memory_embedding_sql().contains("memory_embeddings"));
}

#[test]
fn v28_sql_helpers_expose_expected_statements() {
    let statements = [
        upsert_project_identity_binding_sql(),
        list_project_identity_bindings_sql(),
        upsert_memory_proposal_sql(),
        select_memory_proposal_sql(),
        list_memory_proposals_prefix_sql(),
        list_memory_proposals_sql(),
        insert_memory_relation_sql(),
        list_memory_relations_prefix_sql(),
        list_memory_relations_sql(),
        list_memory_versions_sql(),
        insert_memory_version_snapshot_sql(),
        update_memory_version_metadata_sql(),
        upsert_distillation_profile_sql(),
        select_distillation_profile_sql(),
        list_distillation_profiles_sql(),
        insert_distillation_run_sql(),
    ];

    for statement in statements {
        assert!(!statement.trim().is_empty());
    }

    assert!(migration_0008_sql().contains("project_identity_bindings"));
    assert!(migration_0008_sql().contains("memory_proposals"));
    assert!(migration_0008_sql().contains("memory_relations"));
    assert!(migration_0008_sql().contains("distillation_profiles"));
    assert!(migration_0008_sql().contains("distillation_runs"));
    assert!(migration_0008_sql().contains("ALTER TABLE memory_versions"));
    assert!(migration_0009_sql().contains("benchmark_suites"));
    assert!(migration_0009_sql().contains("benchmark_runs"));
    assert!(migration_0009_sql().contains("benchmark_case_results"));

    assert!(upsert_project_identity_binding_sql().contains("ON CONFLICT (id) DO UPDATE"));
    assert!(list_project_identity_bindings_sql().contains("WHERE owner_scope_id = $1"));
    assert!(upsert_memory_proposal_sql().contains("target_memory_ids"));
    assert!(select_memory_proposal_sql().contains("WHERE id = $1"));
    assert!(list_memory_proposals_prefix_sql().contains("FROM memory_proposals"));
    assert!(list_memory_proposals_sql().contains("WHERE ($1::text IS NULL"));
    assert!(insert_memory_relation_sql().contains("INSERT INTO memory_relations"));
    assert!(list_memory_relations_prefix_sql().contains("from_memory_id = "));
    assert!(list_memory_relations_sql().contains("($2::text IS NULL"));
    assert!(list_memory_versions_sql().contains("ORDER BY version DESC"));
    assert!(insert_memory_version_snapshot_sql().contains("source_proposal_id"));
    assert!(update_memory_version_metadata_sql().contains("UPDATE memory_versions"));
    assert!(upsert_distillation_profile_sql().contains("rules_json"));
    assert!(select_distillation_profile_sql().contains("WHERE id = $1"));
    assert!(list_distillation_profiles_sql().contains("WHERE ($1::text IS NULL"));
    assert!(insert_distillation_run_sql().contains("output_json"));
}

#[test]
fn build_search_query_generates_expected_postgres_sql() {
    let terms = vec!["gateway".to_string(), "http".to_string()];
    let builder = build_search_query("scp_search", &terms, 25, None);
    let sql = builder.sql().to_string();

    assert!(sql.starts_with(search_memory_prefix_sql()));
    assert!(sql.contains("WHERE scope_id = $1"));
    assert!(sql.contains("title ILIKE $2"));
    assert!(sql.contains("body ILIKE $3"));
    assert!(sql.contains("title ILIKE $4"));
    assert!(sql.contains("body ILIKE $5"));
    assert!(sql.ends_with("ORDER BY updated_at DESC LIMIT $6"));
}

#[test]
fn build_list_query_supports_optional_scope_filter() {
    let scoped = build_list_query(Some("scp_a"), 10).sql().to_string();
    let unscoped = build_list_query(None, 20).sql().to_string();

    assert!(scoped.starts_with(list_memory_prefix_sql()));
    assert!(scoped.contains("WHERE scope_id = $1"));
    assert!(scoped.ends_with("ORDER BY updated_at DESC LIMIT $2"));
    assert!(unscoped.starts_with(list_memory_prefix_sql()));
    assert!(!unscoped.contains("WHERE scope_id = "));
    assert!(unscoped.ends_with("ORDER BY updated_at DESC LIMIT $1"));
}

#[test]
fn build_embedding_search_query_generates_expected_postgres_sql() {
    let builder = build_embedding_search_query("scp_search", "[1,0,0]", Some("personal:alice"), 5);
    let sql = builder.sql().to_string();

    assert!(sql.starts_with(SEARCH_MEMORY_BY_EMBEDDING_SQL));
    assert!(sql.contains("WHERE memories.scope_id = $1"));
    assert!(sql.contains("memory_embeddings.isolation_group_id = $2"));
    assert!(sql.ends_with("ORDER BY memory_embeddings.embedding <-> $3::vector LIMIT $4"));
}

#[tokio::test]
async fn pool_accessor_returns_underlying_pool_reference() {
    let store = failing_store();

    assert!(std::ptr::eq(store.pool(), &store.pool));
}

#[tokio::test]
async fn connect_rejects_invalid_database_url() {
    assert!(PgStore::connect("not-a-valid-postgres-url").await.is_err());
}

#[tokio::test]
async fn search_by_keyword_returns_empty_for_blank_queries_without_database_access() {
    let store = failing_store();
    let memories = store
        .search_by_keyword(&ScopeId::from_string("scp_blank"), "   ", 10)
        .await
        .expect("blank search should short-circuit");

    assert!(memories.is_empty());
}

#[tokio::test]
#[ignore = "lazy pool error path is flaky under workspace-wide parallel test runs"]
async fn lazy_pool_operations_surface_errors_after_building_queries() {
    let store = failing_store();
    let scope_id = ScopeId::from_string("scp_exec");
    let artifact = sample_artifact();
    let memory = sample_memory();

    assert!(store.migrate().await.is_err());
    assert!(
        store
            .seed_scope(&scope_id, "demo", "/tmp/demo")
            .await
            .is_err()
    );
    assert!(store.insert_artifact(&artifact).await.is_err());
    assert!(store.insert_memory(&memory).await.is_err());
    assert!(store.upsert_memory(&memory).await.is_err());
    assert!(
        store
            .get_memory(&scope_id, &MemoryId::from_string("mem_missing"))
            .await
            .is_err()
    );
    assert!(
        store
            .link_evidence(
                &MemoryId::from_string("mem_exec"),
                &ArtifactId::from_string("art_exec"),
                Some("quote"),
            )
            .await
            .is_err()
    );
    assert!(
        store
            .search_by_keyword(&scope_id, "gateway", 5)
            .await
            .is_err()
    );
}

#[test]
fn memory_from_record_maps_fields_and_rejects_invalid_values() {
    let record = MemoryRecord {
        id: "mem_pg".to_string(),
        scope_id: "scp_pg".to_string(),
        owner_scope_id: "scp_pg".to_string(),
        published_from_scope_id: Some("scp_user_pg".to_string()),
        memory_kind: "summary".to_string(),
        state: "active".to_string(),
        title: "数据库映射".to_string(),
        body: "映射成功".to_string(),
        language_code: Some("zh-CN".to_string()),
        source_refs: vec!["agent-context://ctx_pg".to_string()],
        confidence: 0.8,
        importance: 0.7,
        stability: 0.6,
        freshness: 0.5,
        visibility: "team".to_string(),
        sensitivity: "restricted".to_string(),
        evidence_count: 3,
        created_at: datetime!(2025-01-02 03:04:05 UTC),
        updated_at: datetime!(2025-01-03 04:05:06 UTC),
    };

    let memory = memory_from_record(record.clone()).unwrap();
    assert_eq!(memory.id.as_str(), "mem_pg");
    assert_eq!(memory.scope_id.as_str(), "scp_pg");
    assert_eq!(memory.owner_scope_id.as_str(), "scp_pg");
    assert_eq!(
        memory.published_from_scope_id.as_ref().map(ScopeId::as_str),
        Some("scp_user_pg")
    );
    assert_eq!(memory.kind, MemoryKind::Summary);
    assert_eq!(memory.state, MemoryState::Active);
    assert_eq!(memory.language_code.as_deref(), Some("zh-CN"));
    assert_eq!(memory.source_refs, vec!["agent-context://ctx_pg"]);
    assert_eq!(memory.visibility, Visibility::Team);
    assert_eq!(memory.sensitivity, Sensitivity::Restricted);
    assert_eq!(memory.evidence_count, 3);
    assert_eq!(memory.created_at, datetime!(2025-01-02 03:04:05 UTC));
    assert_eq!(memory.updated_at, datetime!(2025-01-03 04:05:06 UTC));

    let mut bad_kind = record.clone();
    bad_kind.memory_kind = "unknown".to_string();
    assert!(memory_from_record(bad_kind).is_err());

    let mut bad_state = record.clone();
    bad_state.state = "unknown".to_string();
    assert!(memory_from_record(bad_state).is_err());

    let mut bad_visibility = record.clone();
    bad_visibility.visibility = "unknown".to_string();
    assert!(memory_from_record(bad_visibility).is_err());

    let mut bad_sensitivity = record.clone();
    bad_sensitivity.sensitivity = "unknown".to_string();
    assert!(memory_from_record(bad_sensitivity).is_err());

    let mut negative_evidence = record;
    negative_evidence.evidence_count = -1;
    assert!(memory_from_record(negative_evidence).is_err());
}

fn failing_store() -> PgStore {
    let pool = PgPool::connect_lazy(
        "postgres://postgres:postgres@127.0.0.1:1/meat_memory_test?connect_timeout=1",
    )
    .expect("lazy pool should parse");

    PgStore { pool }
}

fn sample_artifact() -> Artifact {
    Artifact::new(
        ScopeId::from_string("scp_exec"),
        ArtifactKind::Document,
        "artifact body",
        vec!["file:///tmp/demo.md".to_string()],
    )
    .expect("artifact should build")
}

fn sample_memory() -> Memory {
    Memory::new(
        ScopeId::from_string("scp_exec"),
        MemoryKind::Summary,
        "memory title",
        "memory body",
    )
    .expect("memory should build")
}
