use memory_core::domain::{
    Artifact, ArtifactKind, Entity, EntityType, Episode, EpisodeKind, EpisodeState, Memory,
    MemoryKind, MemoryScores, Relation, RelationType, Scope, ScopeHierarchyValidator, ScopeId,
    ScopeType,
};

#[test]
fn artifact_hashes_same_payload_consistently() {
    let first = Artifact::new(
        ScopeId::new(),
        ArtifactKind::Message,
        "same payload",
        vec!["source://one".to_string()],
    )
    .unwrap();
    let second = Artifact::new(
        ScopeId::new(),
        ArtifactKind::Message,
        "same payload",
        vec!["source://two".to_string()],
    )
    .unwrap();

    assert_eq!(first.content_hash, second.content_hash);
}

#[test]
fn episode_rejects_invalid_transition() {
    let mut episode =
        Episode::new(ScopeId::new(), EpisodeKind::CodingTask, "Build kernel").unwrap();
    assert!(episode.transition_to(EpisodeState::Closed).is_err());
}

#[test]
fn memory_scores_must_stay_in_range() {
    let memory = Memory::new(ScopeId::new(), MemoryKind::Fact, "title", "body").unwrap();
    assert!(
        memory
            .with_scores(MemoryScores {
                confidence: -0.1,
                ..MemoryScores::default()
            })
            .is_err()
    );
}

#[test]
fn entity_normalized_key_is_machine_friendly() {
    let entity = Entity::new(ScopeId::new(), EntityType::Project, "Meat Memory v2").unwrap();
    assert_eq!(entity.normalized_key, "meatmemoryv2");
}

#[test]
fn relation_requires_evidence_before_activation() {
    let subject = Entity::new(ScopeId::new(), EntityType::Service, "Memory API").unwrap();
    let object = Entity::new(ScopeId::new(), EntityType::Service, "Postgres").unwrap();
    let mut relation = Relation::new(RelationType::DependsOn, subject.id, object.id);
    assert!(relation.activate().is_err());
}

#[test]
fn scope_hierarchy_validator_accepts_simple_tree() {
    let org = Scope::new(ScopeType::Org, "Acme", "org/acme", None).unwrap();
    let team = Scope::new(
        ScopeType::Team,
        "Platform",
        "org/acme/team/platform",
        Some(org.id.clone()),
    )
    .unwrap();

    assert!(ScopeHierarchyValidator::validate(&[org, team]).is_ok());
}
