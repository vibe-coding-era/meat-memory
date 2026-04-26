use anyhow::{Result, bail};
use memory_domain::{
    DistillationProfile, DistillationProfileId, DistillationProfileLevel,
    DistillationProfileStatus, DistillationRun, DistillationRunId, MemoryId, MemoryProposal,
    MemoryRelation, MemoryRelationId, MemoryRelationSourceKind, MemoryRelationType, ProposalId,
    ProposalStatus, ProposalType, ReviewLevel, ScopeId,
};
use serde_json::{Value, json};
use sqlx::{Executor, Row};

impl super::PgStore {
    pub async fn upsert_distillation_profile(
        &self,
        profile: &DistillationProfile,
    ) -> Result<DistillationProfileId> {
        if let Some(scope_id) = profile.scope_id.as_ref() {
            self.seed_scope(
                scope_id,
                scope_id.as_str(),
                &format!("default/scopes/{}", scope_id.as_str()),
            )
            .await?;
        }

        self.pool
            .execute(
                sqlx::query(crate::v28_sql::upsert_distillation_profile_sql())
                    .bind(profile.id.as_str())
                    .bind(profile.scope_id.as_ref().map(ScopeId::as_str))
                    .bind(distillation_profile_level_to_str(profile.profile_level))
                    .bind(distillation_profile_status_to_str(profile.status))
                    .bind(&profile.name)
                    .bind(&profile.prompt_text)
                    .bind(sqlx::types::Json(json!({})))
                    .bind(sqlx::types::Json(profile.focus_topics.clone()))
                    .bind(sqlx::types::Json(
                        profile
                            .prefer_memory_kinds
                            .iter()
                            .map(|kind| super::memory_kind_to_str(*kind).to_string())
                            .collect::<Vec<_>>(),
                    ))
                    .bind(&profile.created_by)
                    .bind(profile.created_at)
                    .bind(profile.updated_at),
            )
            .await?;

        Ok(profile.id.clone())
    }

    pub async fn get_distillation_profile(
        &self,
        profile_id: &DistillationProfileId,
    ) -> Result<Option<DistillationProfile>> {
        let row = sqlx::query(crate::v28_sql::select_distillation_profile_sql())
            .bind(profile_id.as_str())
            .fetch_optional(&self.pool)
            .await?;

        row.map(row_to_distillation_profile).transpose()
    }

    pub async fn list_distillation_profiles(
        &self,
        scope_id: Option<&ScopeId>,
        limit: i64,
    ) -> Result<Vec<DistillationProfile>> {
        let rows = sqlx::query(crate::v28_sql::list_distillation_profiles_sql())
            .bind(scope_id.map(ScopeId::as_str))
            .bind(limit.clamp(1, 500))
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter().map(row_to_distillation_profile).collect()
    }

    pub async fn insert_distillation_run(
        &self,
        run: &DistillationRun,
        output_json: Value,
    ) -> Result<DistillationRunId> {
        self.seed_scope(
            &run.scope_id,
            run.scope_id.as_str(),
            &format!("default/scopes/{}", run.scope_id.as_str()),
        )
        .await?;

        self.pool
            .execute(
                sqlx::query(crate::v28_sql::insert_distillation_run_sql())
                    .bind(run.id.as_str())
                    .bind(run.profile_id.as_ref().map(DistillationProfileId::as_str))
                    .bind(run.scope_id.as_str())
                    .bind(&run.input_hash)
                    .bind(run.preview)
                    .bind(sqlx::types::Json(output_json))
                    .bind(run.created_at),
            )
            .await?;

        Ok(run.id.clone())
    }

    pub async fn upsert_memory_proposal(&self, proposal: &MemoryProposal) -> Result<ProposalId> {
        self.seed_scope(
            &proposal.scope_id,
            proposal.scope_id.as_str(),
            &format!("default/scopes/{}", proposal.scope_id.as_str()),
        )
        .await?;

        self.pool
            .execute(
                sqlx::query(crate::v28_sql::upsert_memory_proposal_sql())
                    .bind(proposal.id.as_str())
                    .bind(proposal.scope_id.as_str())
                    .bind(proposal_type_to_str(proposal.proposal_type))
                    .bind(proposal_status_to_str(proposal.status))
                    .bind(review_level_to_str(proposal.review_level))
                    .bind(proposal.subject_memory_id.as_ref().map(MemoryId::as_str))
                    .bind(sqlx::types::Json(
                        proposal
                            .target_memory_ids
                            .iter()
                            .map(|memory_id| memory_id.as_str().to_string())
                            .collect::<Vec<_>>(),
                    ))
                    .bind(&proposal.reason)
                    .bind(sqlx::types::Json(proposal.evidence.clone()))
                    .bind(proposal.decided_by.as_deref())
                    .bind(proposal.decided_at)
                    .bind(proposal.applied_at)
                    .bind(proposal.created_at)
                    .bind(proposal.updated_at),
            )
            .await?;

        Ok(proposal.id.clone())
    }

    pub async fn get_memory_proposal(
        &self,
        proposal_id: &ProposalId,
    ) -> Result<Option<MemoryProposal>> {
        let row = sqlx::query(crate::v28_sql::select_memory_proposal_sql())
            .bind(proposal_id.as_str())
            .fetch_optional(&self.pool)
            .await?;

        row.map(row_to_memory_proposal).transpose()
    }

    pub async fn list_memory_proposals(
        &self,
        scope_id: Option<&ScopeId>,
        limit: i64,
    ) -> Result<Vec<MemoryProposal>> {
        let rows = sqlx::query(crate::v28_sql::list_memory_proposals_sql())
            .bind(scope_id.map(ScopeId::as_str))
            .bind(limit.clamp(1, 500))
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter().map(row_to_memory_proposal).collect()
    }

    pub async fn insert_memory_relation(
        &self,
        relation: &MemoryRelation,
    ) -> Result<MemoryRelationId> {
        self.seed_scope(
            &relation.scope_id,
            relation.scope_id.as_str(),
            &format!("default/scopes/{}", relation.scope_id.as_str()),
        )
        .await?;

        self.pool
            .execute(
                sqlx::query(crate::v28_sql::insert_memory_relation_sql())
                    .bind(relation.id.as_str())
                    .bind(relation.scope_id.as_str())
                    .bind(relation.from_memory_id.as_str())
                    .bind(relation.to_memory_id.as_str())
                    .bind(memory_relation_type_to_str(relation.relation_type))
                    .bind(relation.confidence)
                    .bind(memory_relation_source_kind_to_str(relation.source_kind))
                    .bind(relation.source_proposal_id.as_ref().map(ProposalId::as_str))
                    .bind(relation.created_at),
            )
            .await?;

        Ok(relation.id.clone())
    }

    pub async fn list_memory_relations(
        &self,
        scope_id: &ScopeId,
        memory_id: Option<&MemoryId>,
        limit: i64,
    ) -> Result<Vec<MemoryRelation>> {
        let rows = sqlx::query(crate::v28_sql::list_memory_relations_sql())
            .bind(scope_id.as_str())
            .bind(memory_id.map(MemoryId::as_str))
            .bind(limit.clamp(1, 500))
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter().map(row_to_memory_relation).collect()
    }

    pub async fn list_memory_versions(
        &self,
        memory_id: &MemoryId,
        limit: i64,
    ) -> Result<Vec<super::MemoryVersionRecord>> {
        let rows = sqlx::query(crate::v28_sql::list_memory_versions_sql())
            .bind(memory_id.as_str())
            .bind(limit.clamp(1, 500))
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter().map(row_to_memory_version).collect()
    }

    pub async fn insert_memory_version_snapshot(
        &self,
        memory_id: &MemoryId,
        title: &str,
        body: &str,
        change_kind: &str,
        actor: &str,
        reason: Option<&str>,
        source_proposal_id: Option<&ProposalId>,
    ) -> Result<i32> {
        let version = sqlx::query_scalar(crate::v28_sql::insert_memory_version_snapshot_sql())
            .bind(memory_id.as_str())
            .bind(title)
            .bind(body)
            .bind(change_kind)
            .bind(actor)
            .bind(reason)
            .bind(source_proposal_id.map(ProposalId::as_str))
            .fetch_one(&self.pool)
            .await?;

        Ok(version)
    }

    pub async fn update_memory_version_metadata(
        &self,
        memory_id: &MemoryId,
        version: i32,
        change_kind: &str,
        actor: &str,
        reason: Option<&str>,
        source_proposal_id: Option<&ProposalId>,
    ) -> Result<()> {
        let result = self
            .pool
            .execute(
                sqlx::query(crate::v28_sql::update_memory_version_metadata_sql())
                    .bind(memory_id.as_str())
                    .bind(version)
                    .bind(change_kind)
                    .bind(actor)
                    .bind(reason)
                    .bind(source_proposal_id.map(ProposalId::as_str)),
            )
            .await?;
        if result.rows_affected() == 0 {
            bail!("memory version not found");
        }

        Ok(())
    }
}

fn row_to_distillation_profile(row: sqlx::postgres::PgRow) -> Result<DistillationProfile> {
    let focus_topics = row
        .try_get::<sqlx::types::Json<Vec<String>>, _>("focus_topics")?
        .0;
    let prefer_memory_kinds = row
        .try_get::<sqlx::types::Json<Vec<String>>, _>("prefer_memory_kinds")?
        .0
        .into_iter()
        .map(|raw| super::parse_memory_kind(&raw))
        .collect::<Result<Vec<_>>>()?;
    let profile_level =
        parse_distillation_profile_level(&row.try_get::<String, _>("profile_level")?)?;
    let status = parse_distillation_profile_status(&row.try_get::<String, _>("status")?)?;

    Ok(DistillationProfile {
        id: DistillationProfileId::from_string(row.try_get::<String, _>("id")?),
        scope_id: row
            .try_get::<Option<String>, _>("scope_id")?
            .map(ScopeId::from_string),
        profile_level,
        status,
        name: row.try_get("name")?,
        prompt_text: row.try_get("prompt_text")?,
        focus_topics,
        prefer_memory_kinds,
        created_by: row.try_get("created_by")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn row_to_memory_proposal(row: sqlx::postgres::PgRow) -> Result<MemoryProposal> {
    Ok(MemoryProposal {
        id: ProposalId::from_string(row.try_get::<String, _>("id")?),
        scope_id: ScopeId::from_string(row.try_get::<String, _>("scope_id")?),
        proposal_type: parse_proposal_type(&row.try_get::<String, _>("proposal_type")?)?,
        status: parse_proposal_status(&row.try_get::<String, _>("status")?)?,
        review_level: parse_review_level(&row.try_get::<String, _>("review_level")?)?,
        subject_memory_id: row
            .try_get::<Option<String>, _>("subject_memory_id")?
            .map(MemoryId::from_string),
        target_memory_ids: row
            .try_get::<sqlx::types::Json<Vec<String>>, _>("target_memory_ids")?
            .0
            .into_iter()
            .map(MemoryId::from_string)
            .collect(),
        reason: row.try_get("reason")?,
        evidence: row
            .try_get::<sqlx::types::Json<Vec<String>>, _>("evidence")?
            .0,
        decided_by: row.try_get("decided_by")?,
        decided_at: row.try_get("decided_at")?,
        applied_at: row.try_get("applied_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn row_to_memory_relation(row: sqlx::postgres::PgRow) -> Result<MemoryRelation> {
    Ok(MemoryRelation {
        id: MemoryRelationId::from_string(row.try_get::<String, _>("id")?),
        scope_id: ScopeId::from_string(row.try_get::<String, _>("scope_id")?),
        from_memory_id: MemoryId::from_string(row.try_get::<String, _>("from_memory_id")?),
        to_memory_id: MemoryId::from_string(row.try_get::<String, _>("to_memory_id")?),
        relation_type: parse_memory_relation_type(&row.try_get::<String, _>("relation_type")?)?,
        confidence: row.try_get("confidence")?,
        source_kind: parse_memory_relation_source_kind(&row.try_get::<String, _>("source_kind")?)?,
        source_proposal_id: row
            .try_get::<Option<String>, _>("source_proposal_id")?
            .map(ProposalId::from_string),
        created_at: row.try_get("created_at")?,
    })
}

fn row_to_memory_version(row: sqlx::postgres::PgRow) -> Result<super::MemoryVersionRecord> {
    Ok(super::MemoryVersionRecord {
        memory_id: MemoryId::from_string(row.try_get::<String, _>("memory_id")?),
        version: row.try_get("version")?,
        title: row.try_get("title")?,
        body: row.try_get("body")?,
        change_kind: row.try_get("change_kind")?,
        actor: row.try_get("actor")?,
        reason: row.try_get("reason")?,
        source_proposal_id: row
            .try_get::<Option<String>, _>("source_proposal_id")?
            .map(ProposalId::from_string),
        created_at: row.try_get("created_at")?,
    })
}

fn parse_distillation_profile_level(value: &str) -> Result<DistillationProfileLevel> {
    match value {
        "user_global" => Ok(DistillationProfileLevel::UserGlobal),
        "project" => Ok(DistillationProfileLevel::Project),
        other => bail!("unknown distillation profile_level: {other}"),
    }
}

fn parse_distillation_profile_status(value: &str) -> Result<DistillationProfileStatus> {
    match value {
        "active" => Ok(DistillationProfileStatus::Active),
        "archived" => Ok(DistillationProfileStatus::Archived),
        other => bail!("unknown distillation profile status: {other}"),
    }
}

fn parse_proposal_type(value: &str) -> Result<ProposalType> {
    match value {
        "merge" => Ok(ProposalType::Merge),
        "new_version" => Ok(ProposalType::NewVersion),
        "supersede" => Ok(ProposalType::Supersede),
        "conflict_mark" => Ok(ProposalType::ConflictMark),
        "archive" => Ok(ProposalType::Archive),
        "forget" => Ok(ProposalType::Forget),
        "restore" => Ok(ProposalType::Restore),
        "hard_delete" => Ok(ProposalType::HardDelete),
        "distill_upsert" => Ok(ProposalType::DistillUpsert),
        other => bail!("unknown proposal_type: {other}"),
    }
}

fn parse_proposal_status(value: &str) -> Result<ProposalStatus> {
    match value {
        "open" => Ok(ProposalStatus::Open),
        "approved" => Ok(ProposalStatus::Approved),
        "rejected" => Ok(ProposalStatus::Rejected),
        "applied" => Ok(ProposalStatus::Applied),
        "canceled" => Ok(ProposalStatus::Canceled),
        "expired" => Ok(ProposalStatus::Expired),
        other => bail!("unknown proposal_status: {other}"),
    }
}

fn parse_review_level(value: &str) -> Result<ReviewLevel> {
    match value {
        "auto" => Ok(ReviewLevel::Auto),
        "suggested" => Ok(ReviewLevel::Suggested),
        "required" => Ok(ReviewLevel::Required),
        "blocked" => Ok(ReviewLevel::Blocked),
        other => bail!("unknown review_level: {other}"),
    }
}

fn parse_memory_relation_type(value: &str) -> Result<MemoryRelationType> {
    match value {
        "supersedes" => Ok(MemoryRelationType::Supersedes),
        "conflicts_with" => Ok(MemoryRelationType::ConflictsWith),
        "derived_from" => Ok(MemoryRelationType::DerivedFrom),
        "merged_into" => Ok(MemoryRelationType::MergedInto),
        "related_to" => Ok(MemoryRelationType::RelatedTo),
        other => bail!("unknown memory_relation_type: {other}"),
    }
}

fn parse_memory_relation_source_kind(value: &str) -> Result<MemoryRelationSourceKind> {
    match value {
        "user" => Ok(MemoryRelationSourceKind::User),
        "agent" => Ok(MemoryRelationSourceKind::Agent),
        "system" => Ok(MemoryRelationSourceKind::System),
        other => bail!("unknown memory_relation_source_kind: {other}"),
    }
}

fn distillation_profile_level_to_str(level: DistillationProfileLevel) -> &'static str {
    crate::v28_sql::distillation_profile_level_to_str(level)
}

fn distillation_profile_status_to_str(status: DistillationProfileStatus) -> &'static str {
    crate::v28_sql::distillation_profile_status_to_str(status)
}

fn proposal_type_to_str(proposal_type: ProposalType) -> &'static str {
    crate::v28_sql::proposal_type_to_str(proposal_type)
}

fn proposal_status_to_str(status: ProposalStatus) -> &'static str {
    crate::v28_sql::proposal_status_to_str(status)
}

fn review_level_to_str(level: ReviewLevel) -> &'static str {
    crate::v28_sql::review_level_to_str(level)
}

fn memory_relation_type_to_str(relation_type: MemoryRelationType) -> &'static str {
    crate::v28_sql::memory_relation_type_to_str(relation_type)
}

fn memory_relation_source_kind_to_str(source_kind: MemoryRelationSourceKind) -> &'static str {
    crate::v28_sql::memory_relation_source_kind_to_str(source_kind)
}

#[cfg(test)]
#[path = "v28_store_tests.rs"]
mod tests;
