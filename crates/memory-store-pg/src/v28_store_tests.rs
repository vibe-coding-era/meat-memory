use super::{
    parse_distillation_profile_level, parse_distillation_profile_status,
    parse_memory_relation_source_kind, parse_memory_relation_type, parse_proposal_status,
    parse_proposal_type, parse_review_level,
};
use crate::PgStore;
use memory_domain::{
    DistillationProfile, DistillationProfileId, DistillationProfileLevel,
    DistillationProfileStatus, DistillationRun, Memory, MemoryId, MemoryKind, MemoryProposal,
    MemoryRelation, MemoryRelationSourceKind, MemoryRelationType, ProposalStatus, ProposalType,
    ReviewLevel, ScopeId,
};
use serde_json::json;
use sqlx::Row;
use std::env;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

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

async fn test_store() -> PgStore {
    let store = PgStore::connect(&test_database_url()).await.unwrap();
    store.migrate().await.unwrap();
    store
}

async fn seed_active_memory(
    store: &PgStore,
    scope_id: &ScopeId,
    title: &str,
    body: &str,
    kind: MemoryKind,
) -> Memory {
    store
        .seed_scope(
            scope_id,
            scope_id.as_str(),
            &format!("default/scopes/{}", scope_id.as_str()),
        )
        .await
        .unwrap();

    let mut memory = Memory::new(scope_id.clone(), kind, title, body).unwrap();
    memory.activate().unwrap();
    store.insert_memory(&memory).await.unwrap();
    memory
}

#[test]
fn parses_distillation_profile_enums_and_rejects_unknown_values() {
    assert_eq!(
        parse_distillation_profile_level("user_global").unwrap(),
        DistillationProfileLevel::UserGlobal
    );
    assert_eq!(
        parse_distillation_profile_level("project").unwrap(),
        DistillationProfileLevel::Project
    );
    assert!(parse_distillation_profile_level("workspace").is_err());

    assert_eq!(
        parse_distillation_profile_status("active").unwrap(),
        DistillationProfileStatus::Active
    );
    assert_eq!(
        parse_distillation_profile_status("archived").unwrap(),
        DistillationProfileStatus::Archived
    );
    assert!(parse_distillation_profile_status("draft").is_err());
}

#[test]
fn parses_proposal_and_relation_enums_and_rejects_unknown_values() {
    assert_eq!(parse_proposal_type("merge").unwrap(), ProposalType::Merge);
    assert_eq!(
        parse_proposal_type("distill_upsert").unwrap(),
        ProposalType::DistillUpsert
    );
    assert!(parse_proposal_type("publish").is_err());

    assert_eq!(parse_proposal_status("open").unwrap(), ProposalStatus::Open);
    assert_eq!(
        parse_proposal_status("applied").unwrap(),
        ProposalStatus::Applied
    );
    assert!(parse_proposal_status("queued").is_err());

    assert_eq!(parse_review_level("auto").unwrap(), ReviewLevel::Auto);
    assert_eq!(
        parse_review_level("required").unwrap(),
        ReviewLevel::Required
    );
    assert!(parse_review_level("manual").is_err());

    assert_eq!(
        parse_memory_relation_type("supersedes").unwrap(),
        MemoryRelationType::Supersedes
    );
    assert_eq!(
        parse_memory_relation_type("related_to").unwrap(),
        MemoryRelationType::RelatedTo
    );
    assert!(parse_memory_relation_type("linked").is_err());

    assert_eq!(
        parse_memory_relation_source_kind("user").unwrap(),
        MemoryRelationSourceKind::User
    );
    assert_eq!(
        parse_memory_relation_source_kind("system").unwrap(),
        MemoryRelationSourceKind::System
    );
    assert!(parse_memory_relation_source_kind("daemon").is_err());
}

#[tokio::test]
async fn distillation_profile_store_roundtrips_global_and_project_profiles() {
    if !local_pg_test_port_available() {
        return;
    }
    let store = test_store().await;
    let scope_id = ScopeId::new();
    let global =
        DistillationProfile::new_global("Global profile", "Keep durable governance rules.", "user")
            .unwrap()
            .add_focus_topic("governance")
            .unwrap()
            .prefer_memory_kind(MemoryKind::Decision);
    let mut project = DistillationProfile::new_project(
        scope_id.clone(),
        "Project profile",
        "Prefer rollout constraints.",
        "user",
    )
    .unwrap()
    .add_focus_topic("rollout")
    .unwrap()
    .prefer_memory_kind(MemoryKind::Constraint);
    project.archive();

    store.upsert_distillation_profile(&global).await.unwrap();
    store.upsert_distillation_profile(&project).await.unwrap();

    let fetched = store
        .get_distillation_profile(&project.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.id, project.id);
    assert_eq!(fetched.scope_id, Some(scope_id.clone()));
    assert_eq!(fetched.status, DistillationProfileStatus::Archived);
    assert_eq!(fetched.focus_topics, vec!["rollout".to_string()]);
    assert_eq!(fetched.prefer_memory_kinds, vec![MemoryKind::Constraint]);
    assert!(
        store
            .get_distillation_profile(&DistillationProfileId::from_string("dpf_missing"))
            .await
            .unwrap()
            .is_none()
    );

    let scoped = store
        .list_distillation_profiles(Some(&scope_id), 10)
        .await
        .unwrap();
    assert_eq!(scoped.len(), 1);
    assert_eq!(scoped[0].id, project.id);

    let all = store.list_distillation_profiles(None, 200).await.unwrap();
    let all_ids = all
        .iter()
        .map(|profile| profile.id.as_str().to_string())
        .collect::<Vec<_>>();
    assert!(all_ids.contains(&global.id.as_str().to_string()));
    assert!(all_ids.contains(&project.id.as_str().to_string()));
}

#[tokio::test]
async fn distillation_run_store_persists_preview_output() {
    if !local_pg_test_port_available() {
        return;
    }
    let store = test_store().await;
    let scope_id = ScopeId::new();
    let profile = DistillationProfile::new_project(
        scope_id.clone(),
        "Run profile",
        "Track preview runs.",
        "user",
    )
    .unwrap()
    .prefer_memory_kind(MemoryKind::Constraint);
    store.upsert_distillation_profile(&profile).await.unwrap();

    let run =
        DistillationRun::new(Some(profile.id.clone()), scope_id, "len:28:2718", true).unwrap();
    let output_json = json!({
        "candidates": [{
            "title": "Keep rollback evidence visible.",
            "memory_kind": "constraint",
            "body": "Keep rollback evidence visible.",
            "confidence": "explicit",
            "why_keep": "profile-guided preview candidate",
            "evidence_refs": ["agent-context://ctx_store"],
        }],
        "discarded": [],
        "warnings": ["preview only; candidates must be approved before writing memory"],
    });

    store
        .insert_distillation_run(&run, output_json.clone())
        .await
        .unwrap();

    let row = sqlx::query(
        "SELECT profile_id, input_hash, preview, output_json FROM distillation_runs WHERE id = $1",
    )
    .bind(run.id.as_str())
    .fetch_one(store.pool())
    .await
    .unwrap();

    let output = row
        .try_get::<sqlx::types::Json<serde_json::Value>, _>("output_json")
        .unwrap()
        .0;
    assert_eq!(
        row.try_get::<Option<String>, _>("profile_id").unwrap(),
        Some(profile.id.as_str().to_string())
    );
    assert_eq!(
        row.try_get::<String, _>("input_hash").unwrap(),
        run.input_hash
    );
    assert!(row.try_get::<bool, _>("preview").unwrap());
    assert_eq!(output, output_json);
}

#[tokio::test]
async fn proposal_relation_and_version_store_support_timeline_queries() {
    if !local_pg_test_port_available() {
        return;
    }
    let store = test_store().await;
    let scope_id = ScopeId::new();
    let mut subject = seed_active_memory(
        &store,
        &scope_id,
        "Rollback policy",
        "Initial rollback body.",
        MemoryKind::Decision,
    )
    .await;
    let target = seed_active_memory(
        &store,
        &scope_id,
        "Legacy rollback policy",
        "Old rollback body.",
        MemoryKind::Decision,
    )
    .await;

    subject.title = "Rollback policy v2".to_string();
    subject.body = "Updated rollback body.".to_string();
    subject.updated_at = time::OffsetDateTime::now_utc();
    store.upsert_memory(&subject).await.unwrap();

    let mut proposal = MemoryProposal::new(
        scope_id.clone(),
        ProposalType::Supersede,
        ReviewLevel::Required,
        "new rollback policy replaces the legacy variant",
    )
    .unwrap()
    .with_subject_memory(subject.id.clone());
    proposal.add_target_memory(target.id.clone());
    proposal
        .add_evidence("same rollout target with newer rollback guidance".to_string())
        .unwrap();
    proposal.approve("user").unwrap();
    store.upsert_memory_proposal(&proposal).await.unwrap();

    let relation = MemoryRelation::new(
        scope_id.clone(),
        subject.id.clone(),
        target.id.clone(),
        MemoryRelationType::Supersedes,
        MemoryRelationSourceKind::System,
    )
    .with_confidence(0.9)
    .unwrap()
    .with_source_proposal_id(proposal.id.clone());
    store.insert_memory_relation(&relation).await.unwrap();

    let fetched = store
        .get_memory_proposal(&proposal.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.id, proposal.id);
    assert_eq!(fetched.status, ProposalStatus::Approved);
    assert_eq!(fetched.subject_memory_id, Some(subject.id.clone()));
    assert_eq!(fetched.target_memory_ids, vec![target.id.clone()]);

    let proposals = store
        .list_memory_proposals(Some(&scope_id), 10)
        .await
        .unwrap();
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].id, proposal.id);

    let subject_relations = store
        .list_memory_relations(&scope_id, Some(&subject.id), 10)
        .await
        .unwrap();
    assert_eq!(subject_relations.len(), 1);
    assert_eq!(subject_relations[0].id, relation.id);
    let target_relations = store
        .list_memory_relations(&scope_id, Some(&target.id), 10)
        .await
        .unwrap();
    assert_eq!(target_relations.len(), 1);
    assert_eq!(target_relations[0].id, relation.id);
    let all_relations = store
        .list_memory_relations(&scope_id, None, 10)
        .await
        .unwrap();
    assert_eq!(all_relations.len(), 1);

    let versions = store.list_memory_versions(&subject.id, 10).await.unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].memory_id, subject.id);
    assert_eq!(versions[0].version, 2);
    assert_eq!(versions[0].title, "Rollback policy v2");
    assert_eq!(versions[0].body, "Updated rollback body.");
    assert_eq!(versions[0].change_kind, "upsert");
    assert_eq!(versions[0].actor, "system");
    assert_eq!(versions[1].version, 1);
    assert_eq!(versions[1].title, "Rollback policy");
    assert_eq!(versions[1].source_proposal_id, None);

    let applied_version = store
        .insert_memory_version_snapshot(
            &subject.id,
            &subject.title,
            &subject.body,
            "supersede",
            "system",
            Some("applied supersede proposal"),
            Some(&proposal.id),
        )
        .await
        .unwrap();
    assert_eq!(applied_version, 3);
    let versions = store.list_memory_versions(&subject.id, 10).await.unwrap();
    assert_eq!(versions[0].version, 3);
    assert_eq!(versions[0].change_kind, "supersede");
    assert_eq!(versions[0].actor, "system");
    assert_eq!(
        versions[0].reason.as_deref(),
        Some("applied supersede proposal")
    );
    assert_eq!(versions[0].source_proposal_id, Some(proposal.id.clone()));
    store
        .update_memory_version_metadata(
            &subject.id,
            3,
            "rollback",
            "user",
            Some("restored previous wording"),
            None,
        )
        .await
        .unwrap();
    let versions = store.list_memory_versions(&subject.id, 10).await.unwrap();
    assert_eq!(versions[0].change_kind, "rollback");
    assert_eq!(versions[0].actor, "user");
    assert_eq!(
        versions[0].reason.as_deref(),
        Some("restored previous wording")
    );
    assert_eq!(versions[0].source_proposal_id, None);
    let missing_version_error = store
        .update_memory_version_metadata(&subject.id, 99, "rollback", "user", None, None)
        .await
        .unwrap_err();
    assert!(missing_version_error.to_string().contains("not found"));

    assert!(
        store
            .list_memory_versions(&MemoryId::from_string("mem_missing"), 10)
            .await
            .unwrap()
            .is_empty()
    );
}
