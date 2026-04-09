use anyhow::Result;
use memory_domain::{
    Artifact, ArtifactId, ArtifactKind, Memory, MemoryId, MemoryKind, MemoryScores, MemoryState,
    ScopeId, Sensitivity, Visibility,
};
use sqlx::{Executor, PgPool, Postgres, QueryBuilder, Row};
use time::OffsetDateTime;

const MIGRATION_0001: &str = include_str!("../../../migrations/0001_init_scopes.sql");
const MIGRATION_0002: &str = include_str!("../../../migrations/0002_init_content.sql");
const DEFAULT_SCHEMA: &str = "public";
const SEED_SCOPE_SQL: &str = "INSERT INTO scopes (id, scope_type, name, path, default_visibility)
                     VALUES ($1, $2, $3, $4, $5)
                     ON CONFLICT (id) DO NOTHING";
const INSERT_ARTIFACT_SQL: &str = "INSERT INTO artifacts
                     (id, scope_id, artifact_kind, mime_type, language_code, content_text, content_hash, visibility, sensitivity, created_at, updated_at)
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)";
const INSERT_MEMORY_SQL: &str = "INSERT INTO memories
                 (id, scope_id, memory_kind, state, title, body, confidence, importance, stability, freshness, visibility, sensitivity, evidence_count, created_at, updated_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)";
const INSERT_MEMORY_VERSION_SQL: &str =
    "INSERT INTO memory_versions (memory_id, version, title, body)
                 VALUES ($1, $2, $3, $4)";
const UPSERT_MEMORY_SQL: &str = "INSERT INTO memories
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
                   updated_at = EXCLUDED.updated_at";
const UPSERT_MEMORY_VERSION_SQL: &str =
    "INSERT INTO memory_versions (memory_id, version, title, body)
                 SELECT $1, COALESCE(MAX(version), 0) + 1, $2, $3
                 FROM memory_versions
                 WHERE memory_id = $1";
const SELECT_MEMORY_SQL: &str = "SELECT id, scope_id, memory_kind, state, title, body, confidence, importance, stability, freshness, visibility, sensitivity, evidence_count, created_at, updated_at
             FROM memories
             WHERE scope_id = $1
               AND id = $2";
const INSERT_MEMORY_EVIDENCE_SQL: &str =
    "INSERT INTO memory_evidence_links (memory_id, artifact_id, quote_text)
                     VALUES ($1, $2, $3)
                     ON CONFLICT (memory_id, artifact_id) DO NOTHING";
const UPDATE_MEMORY_EVIDENCE_COUNT_SQL: &str = "UPDATE memories
                     SET evidence_count = (
                       SELECT COUNT(*)::int FROM memory_evidence_links WHERE memory_id = $1
                     )
                     WHERE id = $1";
const SEARCH_MEMORY_PREFIX_SQL: &str = "SELECT id, scope_id, memory_kind, state, title, body, confidence, importance, stability, freshness, visibility, sensitivity, evidence_count, created_at, updated_at
             FROM memories
             WHERE scope_id = ";

#[derive(Debug, Clone)]
struct MemoryRecord {
    id: String,
    scope_id: String,
    memory_kind: String,
    state: String,
    title: String,
    body: String,
    confidence: f32,
    importance: f32,
    stability: f32,
    freshness: f32,
    visibility: String,
    sensitivity: String,
    evidence_count: i32,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

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
                sqlx::query(seed_scope_sql())
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
                sqlx::query(insert_artifact_sql())
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
            sqlx::query(insert_memory_sql())
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
            sqlx::query(insert_memory_version_sql())
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
            sqlx::query(upsert_memory_sql())
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
            sqlx::query(upsert_memory_version_sql())
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
        let row = sqlx::query(select_memory_sql())
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
                sqlx::query(insert_memory_evidence_sql())
                    .bind(memory_id.as_str())
                    .bind(artifact_id.as_str())
                    .bind(quote_text),
            )
            .await?;

        self.pool
            .execute(sqlx::query(update_memory_evidence_count_sql()).bind(memory_id.as_str()))
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

        let mut builder = build_search_query(scope_id.as_str(), &terms, limit);

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

fn build_search_query<'a>(
    scope_id: &'a str,
    terms: &'a [String],
    limit: i64,
) -> QueryBuilder<'a, Postgres> {
    let mut builder = QueryBuilder::<Postgres>::new(search_memory_prefix_sql());
    builder.push_bind(scope_id);

    for term in terms {
        let pattern = format!("%{term}%");
        builder.push(" AND (title ILIKE ");
        builder.push_bind(pattern.clone());
        builder.push(" OR body ILIKE ");
        builder.push_bind(pattern);
        builder.push(")");
    }

    builder.push(" ORDER BY updated_at DESC LIMIT ");
    builder.push_bind(limit);
    builder
}

fn row_to_memory(row: sqlx::postgres::PgRow) -> Result<Memory> {
    memory_from_record(MemoryRecord {
        id: row.try_get::<String, _>("id")?,
        scope_id: row.try_get::<String, _>("scope_id")?,
        memory_kind: row.try_get("memory_kind")?,
        state: row.try_get("state")?,
        title: row.try_get("title")?,
        body: row.try_get("body")?,
        confidence: row.try_get("confidence")?,
        importance: row.try_get("importance")?,
        stability: row.try_get("stability")?,
        freshness: row.try_get("freshness")?,
        visibility: row.try_get("visibility")?,
        sensitivity: row.try_get("sensitivity")?,
        evidence_count: row.try_get::<i32, _>("evidence_count")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn memory_from_record(record: MemoryRecord) -> Result<Memory> {
    let evidence_count = usize::try_from(record.evidence_count)
        .map_err(|_| anyhow::anyhow!("evidence_count must be non-negative"))?;

    Ok(Memory {
        id: MemoryId::from_string(record.id),
        scope_id: ScopeId::from_string(record.scope_id),
        kind: parse_memory_kind(&record.memory_kind)?,
        state: parse_memory_state(&record.state)?,
        title: record.title,
        body: record.body,
        scores: MemoryScores {
            confidence: record.confidence,
            importance: record.importance,
            stability: record.stability,
            freshness: record.freshness,
        },
        visibility: parse_visibility(&record.visibility)?,
        sensitivity: parse_sensitivity(&record.sensitivity)?,
        evidence_count,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

fn seed_scope_sql() -> &'static str {
    SEED_SCOPE_SQL
}

fn insert_artifact_sql() -> &'static str {
    INSERT_ARTIFACT_SQL
}

fn insert_memory_sql() -> &'static str {
    INSERT_MEMORY_SQL
}

fn insert_memory_version_sql() -> &'static str {
    INSERT_MEMORY_VERSION_SQL
}

fn upsert_memory_sql() -> &'static str {
    UPSERT_MEMORY_SQL
}

fn upsert_memory_version_sql() -> &'static str {
    UPSERT_MEMORY_VERSION_SQL
}

fn select_memory_sql() -> &'static str {
    SELECT_MEMORY_SQL
}

fn insert_memory_evidence_sql() -> &'static str {
    INSERT_MEMORY_EVIDENCE_SQL
}

fn update_memory_evidence_count_sql() -> &'static str {
    UPDATE_MEMORY_EVIDENCE_COUNT_SQL
}

fn search_memory_prefix_sql() -> &'static str {
    SEARCH_MEMORY_PREFIX_SQL
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
        MemoryRecord, PgStore, artifact_kind_to_str, build_search_query, insert_artifact_sql,
        insert_memory_evidence_sql, insert_memory_sql, insert_memory_version_sql,
        memory_from_record, memory_kind_to_str, memory_state_to_str, parse_memory_kind,
        parse_memory_state, parse_sensitivity, parse_visibility, search_memory_prefix_sql,
        search_terms, seed_scope_sql, select_memory_sql, sensitivity_to_str,
        update_memory_evidence_count_sql, upsert_memory_sql, upsert_memory_version_sql,
        visibility_to_str,
    };
    use memory_domain::{
        Artifact, ArtifactId, ArtifactKind, Memory, MemoryId, MemoryKind, MemoryState, ScopeId,
        Sensitivity, Visibility,
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
        assert!(search_memory_prefix_sql().contains("WHERE scope_id = "));
    }

    #[test]
    fn build_search_query_generates_expected_postgres_sql() {
        let terms = vec!["gateway".to_string(), "http".to_string()];
        let builder = build_search_query("scp_search", &terms, 25);
        let sql = builder.sql().to_string();

        assert!(sql.starts_with(search_memory_prefix_sql()));
        assert!(sql.contains("WHERE scope_id = $1"));
        assert!(sql.contains("title ILIKE $2"));
        assert!(sql.contains("body ILIKE $3"));
        assert!(sql.contains("title ILIKE $4"));
        assert!(sql.contains("body ILIKE $5"));
        assert!(sql.ends_with("ORDER BY updated_at DESC LIMIT $6"));
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
            memory_kind: "summary".to_string(),
            state: "active".to_string(),
            title: "数据库映射".to_string(),
            body: "映射成功".to_string(),
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
        assert_eq!(memory.kind, MemoryKind::Summary);
        assert_eq!(memory.state, MemoryState::Active);
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
}
