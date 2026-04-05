use anyhow::Result;
use memory_domain::{
    Artifact, ArtifactId, ArtifactKind, Memory, MemoryId, MemoryKind, MemoryScores, MemoryState,
    ScopeId, Sensitivity, Visibility,
};
use sqlx::{Executor, PgPool, Postgres, QueryBuilder, Row};

const MIGRATION_0001: &str = include_str!("../../../migrations/0001_init_scopes.sql");
const MIGRATION_0002: &str = include_str!("../../../migrations/0002_init_content.sql");
const DEFAULT_SCHEMA: &str = "public";

pub struct PgStore {
    pool: PgPool,
}

impl PgStore {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub fn default_schema() -> &'static str {
        DEFAULT_SCHEMA
    }

    pub async fn migrate(&self) -> Result<()> {
        self.pool.execute(sqlx::raw_sql(MIGRATION_0001)).await?;
        self.pool.execute(sqlx::raw_sql(MIGRATION_0002)).await?;
        Ok(())
    }

    pub async fn seed_scope(
        &self,
        scope_id: &ScopeId,
        scope_name: &str,
        scope_path: &str,
    ) -> Result<()> {
        self.pool
            .execute(
                sqlx::query(
                    "INSERT INTO scopes (id, scope_type, name, path, default_visibility)
                     VALUES ($1, $2, $3, $4, $5)
                     ON CONFLICT (id) DO NOTHING",
                )
                .bind(scope_id.as_str())
                .bind("project")
                .bind(scope_name)
                .bind(scope_path)
                .bind("private"),
            )
            .await?;

        Ok(())
    }

    pub async fn insert_artifact(&self, artifact: &Artifact) -> Result<ArtifactId> {
        self.pool
            .execute(
                sqlx::query(
                    "INSERT INTO artifacts
                     (id, scope_id, artifact_kind, mime_type, language_code, content_text, content_hash, visibility, sensitivity, created_at, updated_at)
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
                )
                .bind(artifact.id.as_str())
                .bind(artifact.scope_id.as_str())
                .bind(artifact_kind_to_str(artifact.kind))
                .bind(artifact.mime_type.as_deref())
                .bind(artifact.language_code.as_deref())
                .bind(&artifact.content_text)
                .bind(&artifact.content_hash)
                .bind(visibility_to_str(artifact.visibility))
                .bind(sensitivity_to_str(artifact.sensitivity))
                .bind(artifact.created_at)
                .bind(artifact.updated_at),
            )
            .await?;

        Ok(artifact.id.clone())
    }

    pub async fn insert_memory(&self, memory: &Memory) -> Result<MemoryId> {
        let mut tx = self.pool.begin().await?;

        tx.execute(
            sqlx::query(
                "INSERT INTO memories
                 (id, scope_id, memory_kind, state, title, body, confidence, importance, stability, freshness, visibility, sensitivity, evidence_count, created_at, updated_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)",
            )
            .bind(memory.id.as_str())
            .bind(memory.scope_id.as_str())
            .bind(memory_kind_to_str(memory.kind))
            .bind(memory_state_to_str(memory.state))
            .bind(&memory.title)
            .bind(&memory.body)
            .bind(memory.scores.confidence)
            .bind(memory.scores.importance)
            .bind(memory.scores.stability)
            .bind(memory.scores.freshness)
            .bind(visibility_to_str(memory.visibility))
            .bind(sensitivity_to_str(memory.sensitivity))
            .bind(memory.evidence_count as i32)
            .bind(memory.created_at)
            .bind(memory.updated_at),
        )
        .await?;

        tx.execute(
            sqlx::query(
                "INSERT INTO memory_versions (memory_id, version, title, body)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(memory.id.as_str())
            .bind(1_i32)
            .bind(&memory.title)
            .bind(&memory.body),
        )
        .await?;

        tx.commit().await?;

        Ok(memory.id.clone())
    }

    pub async fn upsert_memory(&self, memory: &Memory) -> Result<MemoryId> {
        let mut tx = self.pool.begin().await?;

        tx.execute(
            sqlx::query(
                "INSERT INTO memories
                 (id, scope_id, memory_kind, state, title, body, confidence, importance, stability, freshness, visibility, sensitivity, evidence_count, created_at, updated_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
                 ON CONFLICT (id) DO UPDATE SET
                   scope_id = EXCLUDED.scope_id,
                   memory_kind = EXCLUDED.memory_kind,
                   state = EXCLUDED.state,
                   title = EXCLUDED.title,
                   body = EXCLUDED.body,
                   confidence = EXCLUDED.confidence,
                   importance = EXCLUDED.importance,
                   stability = EXCLUDED.stability,
                   freshness = EXCLUDED.freshness,
                   visibility = EXCLUDED.visibility,
                   sensitivity = EXCLUDED.sensitivity,
                   evidence_count = EXCLUDED.evidence_count,
                   updated_at = EXCLUDED.updated_at",
            )
            .bind(memory.id.as_str())
            .bind(memory.scope_id.as_str())
            .bind(memory_kind_to_str(memory.kind))
            .bind(memory_state_to_str(memory.state))
            .bind(&memory.title)
            .bind(&memory.body)
            .bind(memory.scores.confidence)
            .bind(memory.scores.importance)
            .bind(memory.scores.stability)
            .bind(memory.scores.freshness)
            .bind(visibility_to_str(memory.visibility))
            .bind(sensitivity_to_str(memory.sensitivity))
            .bind(memory.evidence_count as i32)
            .bind(memory.created_at)
            .bind(memory.updated_at),
        )
        .await?;

        tx.execute(
            sqlx::query(
                "INSERT INTO memory_versions (memory_id, version, title, body)
                 SELECT $1, COALESCE(MAX(version), 0) + 1, $2, $3
                 FROM memory_versions
                 WHERE memory_id = $1",
            )
            .bind(memory.id.as_str())
            .bind(&memory.title)
            .bind(&memory.body),
        )
        .await?;

        tx.commit().await?;

        Ok(memory.id.clone())
    }

    pub async fn get_memory(
        &self,
        scope_id: &ScopeId,
        memory_id: &MemoryId,
    ) -> Result<Option<Memory>> {
        let row = sqlx::query(
            "SELECT id, scope_id, memory_kind, state, title, body, confidence, importance, stability, freshness, visibility, sensitivity, evidence_count, created_at, updated_at
             FROM memories
             WHERE scope_id = $1
               AND id = $2",
        )
        .bind(scope_id.as_str())
        .bind(memory_id.as_str())
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_memory).transpose()
    }

    pub async fn link_evidence(
        &self,
        memory_id: &MemoryId,
        artifact_id: &ArtifactId,
        quote_text: Option<&str>,
    ) -> Result<()> {
        self.pool
            .execute(
                sqlx::query(
                    "INSERT INTO memory_evidence_links (memory_id, artifact_id, quote_text)
                     VALUES ($1, $2, $3)
                     ON CONFLICT (memory_id, artifact_id) DO NOTHING",
                )
                .bind(memory_id.as_str())
                .bind(artifact_id.as_str())
                .bind(quote_text),
            )
            .await?;

        self.pool
            .execute(
                sqlx::query(
                    "UPDATE memories
                     SET evidence_count = (
                       SELECT COUNT(*)::int FROM memory_evidence_links WHERE memory_id = $1
                     )
                     WHERE id = $1",
                )
                .bind(memory_id.as_str()),
            )
            .await?;

        Ok(())
    }

    pub async fn search_by_keyword(
        &self,
        scope_id: &ScopeId,
        keyword: &str,
        limit: i64,
    ) -> Result<Vec<Memory>> {
        let terms = search_terms(keyword);
        if terms.is_empty() {
            return Ok(Vec::new());
        }

        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT id, scope_id, memory_kind, state, title, body, confidence, importance, stability, freshness, visibility, sensitivity, evidence_count, created_at, updated_at
             FROM memories
             WHERE scope_id = ",
        );
        builder.push_bind(scope_id.as_str());

        for term in &terms {
            let pattern = format!("%{term}%");
            builder.push(" AND (title ILIKE ");
            builder.push_bind(pattern.clone());
            builder.push(" OR body ILIKE ");
            builder.push_bind(pattern);
            builder.push(")");
        }

        builder.push(" ORDER BY updated_at DESC LIMIT ");
        builder.push_bind(limit);

        let rows = builder.build().fetch_all(&self.pool).await?;

        rows.into_iter().map(row_to_memory).collect()
    }
}

fn search_terms(keyword: &str) -> Vec<String> {
    let mut terms = keyword
        .split_whitespace()
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    if terms.is_empty() {
        let fallback = keyword.trim();
        if !fallback.is_empty() {
            terms.push(fallback.to_string());
        }
    }

    terms.sort();
    terms.dedup();
    terms
}

fn row_to_memory(row: sqlx::postgres::PgRow) -> Result<Memory> {
    Ok(Memory {
        id: MemoryId::from_string(row.try_get::<String, _>("id")?),
        scope_id: ScopeId::from_string(row.try_get::<String, _>("scope_id")?),
        kind: parse_memory_kind(row.try_get("memory_kind")?)?,
        state: parse_memory_state(row.try_get("state")?)?,
        title: row.try_get("title")?,
        body: row.try_get("body")?,
        scores: MemoryScores {
            confidence: row.try_get("confidence")?,
            importance: row.try_get("importance")?,
            stability: row.try_get("stability")?,
            freshness: row.try_get("freshness")?,
        },
        visibility: parse_visibility(row.try_get("visibility")?)?,
        sensitivity: parse_sensitivity(row.try_get("sensitivity")?)?,
        evidence_count: row.try_get::<i32, _>("evidence_count")? as usize,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn artifact_kind_to_str(kind: ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::Message => "message",
        ArtifactKind::Document => "document",
        ArtifactKind::CodeDiff => "code_diff",
        ArtifactKind::CodeFileSnapshot => "code_file_snapshot",
        ArtifactKind::TerminalOutput => "terminal_output",
        ArtifactKind::Image => "image",
        ArtifactKind::Audio => "audio",
        ArtifactKind::Video => "video",
        ArtifactKind::ToolResult => "tool_result",
        ArtifactKind::WebPage => "web_page",
    }
}

fn memory_kind_to_str(kind: MemoryKind) -> &'static str {
    match kind {
        MemoryKind::Fact => "fact",
        MemoryKind::Preference => "preference",
        MemoryKind::Decision => "decision",
        MemoryKind::Procedure => "procedure",
        MemoryKind::Constraint => "constraint",
        MemoryKind::Risk => "risk",
        MemoryKind::Summary => "summary",
        MemoryKind::Insight => "insight",
    }
}

fn memory_state_to_str(state: MemoryState) -> &'static str {
    match state {
        MemoryState::Candidate => "candidate",
        MemoryState::Active => "active",
        MemoryState::Deprecated => "deprecated",
        MemoryState::Conflicted => "conflicted",
        MemoryState::Archived => "archived",
        MemoryState::Deleted => "deleted",
    }
}

fn visibility_to_str(visibility: Visibility) -> &'static str {
    match visibility {
        Visibility::Private => "private",
        Visibility::Project => "project",
        Visibility::Team => "team",
        Visibility::Organization => "organization",
    }
}

fn sensitivity_to_str(sensitivity: Sensitivity) -> &'static str {
    match sensitivity {
        Sensitivity::Public => "public",
        Sensitivity::Internal => "internal",
        Sensitivity::Private => "private",
        Sensitivity::Restricted => "restricted",
    }
}

fn parse_memory_kind(value: &str) -> Result<MemoryKind> {
    Ok(match value {
        "fact" => MemoryKind::Fact,
        "preference" => MemoryKind::Preference,
        "decision" => MemoryKind::Decision,
        "procedure" => MemoryKind::Procedure,
        "constraint" => MemoryKind::Constraint,
        "risk" => MemoryKind::Risk,
        "summary" => MemoryKind::Summary,
        "insight" => MemoryKind::Insight,
        _ => anyhow::bail!("unknown memory kind: {value}"),
    })
}

fn parse_memory_state(value: &str) -> Result<MemoryState> {
    Ok(match value {
        "candidate" => MemoryState::Candidate,
        "active" => MemoryState::Active,
        "deprecated" => MemoryState::Deprecated,
        "conflicted" => MemoryState::Conflicted,
        "archived" => MemoryState::Archived,
        "deleted" => MemoryState::Deleted,
        _ => anyhow::bail!("unknown memory state: {value}"),
    })
}

fn parse_visibility(value: &str) -> Result<Visibility> {
    Ok(match value {
        "private" => Visibility::Private,
        "project" => Visibility::Project,
        "team" => Visibility::Team,
        "organization" => Visibility::Organization,
        _ => anyhow::bail!("unknown visibility: {value}"),
    })
}

fn parse_sensitivity(value: &str) -> Result<Sensitivity> {
    Ok(match value {
        "public" => Sensitivity::Public,
        "internal" => Sensitivity::Internal,
        "private" => Sensitivity::Private,
        "restricted" => Sensitivity::Restricted,
        _ => anyhow::bail!("unknown sensitivity: {value}"),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        PgStore, artifact_kind_to_str, memory_kind_to_str, memory_state_to_str, search_terms,
        sensitivity_to_str, visibility_to_str,
    };
    use memory_domain::{ArtifactKind, MemoryKind, MemoryState, Sensitivity, Visibility};

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
    fn exposes_default_schema() {
        assert_eq!(PgStore::default_schema(), "public");
    }

    #[test]
    fn serializes_enums_for_storage() {
        assert_eq!(artifact_kind_to_str(ArtifactKind::Message), "message");
        assert_eq!(memory_kind_to_str(MemoryKind::Decision), "decision");
        assert_eq!(memory_state_to_str(MemoryState::Candidate), "candidate");
        assert_eq!(visibility_to_str(Visibility::Team), "team");
        assert_eq!(sensitivity_to_str(Sensitivity::Restricted), "restricted");
    }
}
