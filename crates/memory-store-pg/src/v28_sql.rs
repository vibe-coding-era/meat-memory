#[cfg(test)]
use memory_domain::{
    BindingConfirmedBy, DistillationProfileLevel, DistillationProfileStatus,
    MemoryRelationSourceKind, MemoryRelationType, ProjectBindingKind, ProposalStatus, ProposalType,
    ReviewLevel,
};
#[cfg(not(test))]
use memory_domain::{
    DistillationProfileLevel, DistillationProfileStatus, MemoryRelationSourceKind,
    MemoryRelationType, ProposalStatus, ProposalType, ReviewLevel,
};

const MIGRATION_0008: &str = include_str!("../../../migrations/0008_memory_v2_8_governance.sql");

#[cfg(test)]
const UPSERT_PROJECT_IDENTITY_BINDING_SQL: &str = "INSERT INTO project_identity_bindings
                 (id, owner_scope_id, binding_kind, binding_value, canonical_project_key, confirmed_by, confidence, created_at, updated_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
                 ON CONFLICT (id) DO UPDATE SET
                   owner_scope_id = EXCLUDED.owner_scope_id,
                   binding_kind = EXCLUDED.binding_kind,
                   binding_value = EXCLUDED.binding_value,
                   canonical_project_key = EXCLUDED.canonical_project_key,
                   confirmed_by = EXCLUDED.confirmed_by,
                   confidence = EXCLUDED.confidence,
                   updated_at = EXCLUDED.updated_at";
#[cfg(test)]
const LIST_PROJECT_IDENTITY_BINDINGS_SQL: &str =
    "SELECT id, owner_scope_id, binding_kind, binding_value, canonical_project_key, confirmed_by, confidence, created_at, updated_at
             FROM project_identity_bindings
             WHERE owner_scope_id = $1
             ORDER BY updated_at DESC
             LIMIT $2";

const UPSERT_MEMORY_PROPOSAL_SQL: &str = "INSERT INTO memory_proposals
                 (id, scope_id, proposal_type, status, review_level, subject_memory_id, target_memory_ids, reason, evidence, decided_by, decided_at, applied_at, created_at, updated_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
                 ON CONFLICT (id) DO UPDATE SET
                   scope_id = EXCLUDED.scope_id,
                   proposal_type = EXCLUDED.proposal_type,
                   status = EXCLUDED.status,
                   review_level = EXCLUDED.review_level,
                   subject_memory_id = EXCLUDED.subject_memory_id,
                   target_memory_ids = EXCLUDED.target_memory_ids,
                   reason = EXCLUDED.reason,
                   evidence = EXCLUDED.evidence,
                   decided_by = EXCLUDED.decided_by,
                   decided_at = EXCLUDED.decided_at,
                   applied_at = EXCLUDED.applied_at,
                   updated_at = EXCLUDED.updated_at";
const SELECT_MEMORY_PROPOSAL_SQL: &str =
    "SELECT id, scope_id, proposal_type, status, review_level, subject_memory_id, target_memory_ids, reason, evidence, decided_by, decided_at, applied_at, created_at, updated_at
             FROM memory_proposals
             WHERE id = $1";
#[cfg(test)]
const LIST_MEMORY_PROPOSALS_PREFIX_SQL: &str =
    "SELECT id, scope_id, proposal_type, status, review_level, subject_memory_id, target_memory_ids, reason, evidence, decided_by, decided_at, applied_at, created_at, updated_at
             FROM memory_proposals";
const LIST_MEMORY_PROPOSALS_SQL: &str =
    "SELECT id, scope_id, proposal_type, status, review_level, subject_memory_id, target_memory_ids, reason, evidence, decided_by, decided_at, applied_at, created_at, updated_at
             FROM memory_proposals
             WHERE ($1::text IS NULL OR scope_id = $1)
             ORDER BY updated_at DESC
             LIMIT $2";

const INSERT_MEMORY_RELATION_SQL: &str = "INSERT INTO memory_relations
                 (id, scope_id, from_memory_id, to_memory_id, relation_type, confidence, source_kind, source_proposal_id, created_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)";
#[cfg(test)]
const LIST_MEMORY_RELATIONS_PREFIX_SQL: &str =
    "SELECT id, scope_id, from_memory_id, to_memory_id, relation_type, confidence, source_kind, source_proposal_id, created_at
             FROM memory_relations
             WHERE scope_id = $1
               AND (from_memory_id = $2 OR to_memory_id = $2)";
const LIST_MEMORY_RELATIONS_SQL: &str =
    "SELECT id, scope_id, from_memory_id, to_memory_id, relation_type, confidence, source_kind, source_proposal_id, created_at
             FROM memory_relations
             WHERE scope_id = $1
               AND ($2::text IS NULL OR from_memory_id = $2 OR to_memory_id = $2)
             ORDER BY created_at DESC
             LIMIT $3";
const LIST_MEMORY_VERSIONS_SQL: &str =
    "SELECT memory_id, version, title, body, change_kind, actor, reason, source_proposal_id, created_at
             FROM memory_versions
             WHERE memory_id = $1
             ORDER BY version DESC
             LIMIT $2";
const INSERT_MEMORY_VERSION_SNAPSHOT_SQL: &str =
    "INSERT INTO memory_versions (memory_id, version, title, body, change_kind, actor, reason, source_proposal_id)
             SELECT $1, COALESCE(MAX(version), 0) + 1, $2, $3, $4, $5, $6, $7
             FROM memory_versions
             WHERE memory_id = $1
             RETURNING version";
const UPDATE_MEMORY_VERSION_METADATA_SQL: &str = "UPDATE memory_versions
             SET change_kind = $3,
                 actor = $4,
                 reason = $5,
                 source_proposal_id = $6
             WHERE memory_id = $1
               AND version = $2";

const UPSERT_DISTILLATION_PROFILE_SQL: &str = "INSERT INTO distillation_profiles
                 (id, scope_id, profile_level, status, name, prompt_text, rules_json, focus_topics, prefer_memory_kinds, created_by, created_at, updated_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
                 ON CONFLICT (id) DO UPDATE SET
                   scope_id = EXCLUDED.scope_id,
                   profile_level = EXCLUDED.profile_level,
                   status = EXCLUDED.status,
                   name = EXCLUDED.name,
                   prompt_text = EXCLUDED.prompt_text,
                   rules_json = EXCLUDED.rules_json,
                   focus_topics = EXCLUDED.focus_topics,
                   prefer_memory_kinds = EXCLUDED.prefer_memory_kinds,
                   updated_at = EXCLUDED.updated_at";
const SELECT_DISTILLATION_PROFILE_SQL: &str =
    "SELECT id, scope_id, profile_level, status, name, prompt_text, rules_json, focus_topics, prefer_memory_kinds, created_by, created_at, updated_at
             FROM distillation_profiles
             WHERE id = $1";
const LIST_DISTILLATION_PROFILES_SQL: &str =
    "SELECT id, scope_id, profile_level, status, name, prompt_text, rules_json, focus_topics, prefer_memory_kinds, created_by, created_at, updated_at
             FROM distillation_profiles
             WHERE ($1::text IS NULL OR scope_id = $1)
             ORDER BY updated_at DESC
             LIMIT $2";
const INSERT_DISTILLATION_RUN_SQL: &str = "INSERT INTO distillation_runs
                 (id, profile_id, scope_id, input_hash, preview, output_json, created_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7)";

pub(crate) fn migration_0008_sql() -> &'static str {
    MIGRATION_0008
}

#[cfg(test)]
pub(crate) fn upsert_project_identity_binding_sql() -> &'static str {
    UPSERT_PROJECT_IDENTITY_BINDING_SQL
}

#[cfg(test)]
pub(crate) fn list_project_identity_bindings_sql() -> &'static str {
    LIST_PROJECT_IDENTITY_BINDINGS_SQL
}

pub(crate) fn upsert_memory_proposal_sql() -> &'static str {
    UPSERT_MEMORY_PROPOSAL_SQL
}

pub(crate) fn select_memory_proposal_sql() -> &'static str {
    SELECT_MEMORY_PROPOSAL_SQL
}

#[cfg(test)]
pub(crate) fn list_memory_proposals_prefix_sql() -> &'static str {
    LIST_MEMORY_PROPOSALS_PREFIX_SQL
}

pub(crate) fn list_memory_proposals_sql() -> &'static str {
    LIST_MEMORY_PROPOSALS_SQL
}

pub(crate) fn insert_memory_relation_sql() -> &'static str {
    INSERT_MEMORY_RELATION_SQL
}

#[cfg(test)]
pub(crate) fn list_memory_relations_prefix_sql() -> &'static str {
    LIST_MEMORY_RELATIONS_PREFIX_SQL
}

pub(crate) fn list_memory_relations_sql() -> &'static str {
    LIST_MEMORY_RELATIONS_SQL
}

pub(crate) fn list_memory_versions_sql() -> &'static str {
    LIST_MEMORY_VERSIONS_SQL
}

pub(crate) fn insert_memory_version_snapshot_sql() -> &'static str {
    INSERT_MEMORY_VERSION_SNAPSHOT_SQL
}

pub(crate) fn update_memory_version_metadata_sql() -> &'static str {
    UPDATE_MEMORY_VERSION_METADATA_SQL
}

pub(crate) fn upsert_distillation_profile_sql() -> &'static str {
    UPSERT_DISTILLATION_PROFILE_SQL
}

pub(crate) fn select_distillation_profile_sql() -> &'static str {
    SELECT_DISTILLATION_PROFILE_SQL
}

pub(crate) fn list_distillation_profiles_sql() -> &'static str {
    LIST_DISTILLATION_PROFILES_SQL
}

pub(crate) fn insert_distillation_run_sql() -> &'static str {
    INSERT_DISTILLATION_RUN_SQL
}

#[cfg(test)]
pub(crate) fn project_binding_kind_to_str(kind: ProjectBindingKind) -> &'static str {
    kind.as_str()
}

#[cfg(test)]
pub(crate) fn binding_confirmed_by_to_str(confirmed_by: BindingConfirmedBy) -> &'static str {
    confirmed_by.as_str()
}

pub(crate) fn proposal_type_to_str(kind: ProposalType) -> &'static str {
    kind.as_str()
}

pub(crate) fn proposal_status_to_str(status: ProposalStatus) -> &'static str {
    status.as_str()
}

pub(crate) fn review_level_to_str(level: ReviewLevel) -> &'static str {
    level.as_str()
}

pub(crate) fn memory_relation_type_to_str(kind: MemoryRelationType) -> &'static str {
    kind.as_str()
}

pub(crate) fn memory_relation_source_kind_to_str(
    source_kind: MemoryRelationSourceKind,
) -> &'static str {
    source_kind.as_str()
}

pub(crate) fn distillation_profile_level_to_str(level: DistillationProfileLevel) -> &'static str {
    match level {
        DistillationProfileLevel::UserGlobal => "user_global",
        DistillationProfileLevel::Project => "project",
    }
}

pub(crate) fn distillation_profile_status_to_str(
    status: DistillationProfileStatus,
) -> &'static str {
    match status {
        DistillationProfileStatus::Active => "active",
        DistillationProfileStatus::Archived => "archived",
    }
}
