use anyhow::Result;
use memory_domain::{
    AccessKey, AccessKeyId, AccessKeyStatus, AccessKeyUsageStats, AgentContext, AgentContextId,
    Artifact, ArtifactId, ArtifactKind, DocumentConflictState, DocumentSyncState, KeyScopeKind,
    KeySourceKind, KeyUsageBreakdown, Memory, MemoryId, MemoryKind, MemoryScores, MemorySource,
    MemoryState, ProjectDocument, ProjectDocumentId, Scope, ScopeId, ScopeType, Sensitivity,
    SourceId, SourceStatus, SourceSyncMode, StorageMode, Visibility,
};
use sqlx::{Executor, PgPool, Postgres, QueryBuilder, Row};
use time::OffsetDateTime;

const MIGRATION_0001: &str = include_str!("../../../migrations/0001_init_scopes.sql");
const MIGRATION_0002: &str = include_str!("../../../migrations/0002_init_content.sql");
const MIGRATION_0003: &str = include_str!("../../../migrations/0003_scope_governance.sql");
const MIGRATION_0004: &str = include_str!("../../../migrations/0004_memory_v2_metadata.sql");
const MIGRATION_0005: &str = include_str!("../../../migrations/0005_access_keys.sql");
const MIGRATION_0006: &str =
    include_str!("../../../migrations/0006_memory_v2_4_layers_sources.sql");
const MIGRATION_0007: &str = include_str!("../../../migrations/0007_memory_v2_7_lifecycle.sql");
const DEFAULT_SCHEMA: &str = "public";
const SEED_SCOPE_SQL: &str = "INSERT INTO scopes (id, parent_scope_id, scope_type, name, path, owner_principal_id, inherit_policy, default_visibility, sync_policy)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                     ON CONFLICT (id) DO NOTHING";
const INSERT_ARTIFACT_SQL: &str = "INSERT INTO artifacts
                     (id, scope_id, artifact_kind, mime_type, language_code, content_text, content_hash, labels, visibility, sensitivity, created_at, updated_at)
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)";
const INSERT_MEMORY_SQL: &str = "INSERT INTO memories
                 (id, scope_id, owner_scope_id, published_from_scope_id, memory_kind, state, title, body, language_code, source_refs, confidence, importance, stability, freshness, visibility, sensitivity, evidence_count, created_at, updated_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)";
const INSERT_MEMORY_VERSION_SQL: &str =
    "INSERT INTO memory_versions (memory_id, version, title, body)
                 VALUES ($1, $2, $3, $4)";
const UPSERT_MEMORY_SQL: &str = "INSERT INTO memories
                 (id, scope_id, owner_scope_id, published_from_scope_id, memory_kind, state, title, body, language_code, source_refs, confidence, importance, stability, freshness, visibility, sensitivity, evidence_count, created_at, updated_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)
                 ON CONFLICT (id) DO UPDATE SET
                   scope_id = EXCLUDED.scope_id,
                   owner_scope_id = EXCLUDED.owner_scope_id,
                   published_from_scope_id = EXCLUDED.published_from_scope_id,
                   memory_kind = EXCLUDED.memory_kind,
                   state = EXCLUDED.state,
                   title = EXCLUDED.title,
                   body = EXCLUDED.body,
                   language_code = EXCLUDED.language_code,
                   source_refs = EXCLUDED.source_refs,
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
const SELECT_MEMORY_SQL: &str = "SELECT id, scope_id, owner_scope_id, published_from_scope_id, memory_kind, state, title, body, language_code, source_refs, confidence, importance, stability, freshness, visibility, sensitivity, evidence_count, created_at, updated_at
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
const LIST_MEMORY_PREFIX_SQL: &str = "SELECT id, scope_id, owner_scope_id, published_from_scope_id, memory_kind, state, title, body, language_code, source_refs, confidence, importance, stability, freshness, visibility, sensitivity, evidence_count, created_at, updated_at
             FROM memories";
const SEARCH_MEMORY_PREFIX_SQL: &str = "SELECT id, scope_id, owner_scope_id, published_from_scope_id, memory_kind, state, title, body, language_code, source_refs, confidence, importance, stability, freshness, visibility, sensitivity, evidence_count, created_at, updated_at
             FROM memories
             WHERE scope_id = ";
const INSERT_ACCESS_KEY_SQL: &str = "INSERT INTO access_keys
                 (id, key_hash, display_name, source_id, source_kind, owner_principal_id, owner_scope_id, scope_kind, storage_mode, is_fully_isolated, isolation_group_id, status, created_at, last_used_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
                 ON CONFLICT (id) DO UPDATE SET
                   key_hash = EXCLUDED.key_hash,
                   display_name = EXCLUDED.display_name,
                   source_id = EXCLUDED.source_id,
                   source_kind = EXCLUDED.source_kind,
                   owner_principal_id = EXCLUDED.owner_principal_id,
                   owner_scope_id = EXCLUDED.owner_scope_id,
                   scope_kind = EXCLUDED.scope_kind,
                   storage_mode = EXCLUDED.storage_mode,
                   is_fully_isolated = EXCLUDED.is_fully_isolated,
                   isolation_group_id = EXCLUDED.isolation_group_id,
                   status = EXCLUDED.status,
                   last_used_at = EXCLUDED.last_used_at";
const SELECT_ACCESS_KEY_BY_HASH_SQL: &str = "SELECT id, key_hash, display_name, source_id, source_kind, owner_principal_id, owner_scope_id, scope_kind, storage_mode, is_fully_isolated, isolation_group_id, status, created_at, last_used_at
             FROM access_keys
             WHERE key_hash = $1";
const SELECT_ACCESS_KEY_BY_ID_SQL: &str = "SELECT id, key_hash, display_name, source_id, source_kind, owner_principal_id, owner_scope_id, scope_kind, storage_mode, is_fully_isolated, isolation_group_id, status, created_at, last_used_at
             FROM access_keys
             WHERE id = $1";
const LIST_ACCESS_KEYS_SQL: &str = "SELECT id, key_hash, display_name, source_id, source_kind, owner_principal_id, owner_scope_id, scope_kind, storage_mode, is_fully_isolated, isolation_group_id, status, created_at, last_used_at
             FROM access_keys
             ORDER BY created_at DESC
             LIMIT $1";
const UPDATE_ACCESS_KEY_LAST_USED_SQL: &str =
    "UPDATE access_keys SET last_used_at = NOW() WHERE id = $1";
const UPDATE_ACCESS_KEY_STATUS_SQL: &str = "UPDATE access_keys SET status = $2 WHERE id = $1";
const INSERT_KEY_USAGE_EVENT_SQL: &str = "INSERT INTO key_usage_events
                 (id, key_id, source_kind, operation, scope_id, storage_mode, success, latency_ms, error_code)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)";
const INSERT_MEMORY_KEY_LINK_SQL: &str = "INSERT INTO memory_key_links
                 (memory_id, key_id, isolation_group_id)
                 VALUES ($1,$2,$3)
                 ON CONFLICT (memory_id, key_id) DO NOTHING";
const UPSERT_MEMORY_EMBEDDING_SQL: &str = "INSERT INTO memory_embeddings
                 (memory_id, key_id, isolation_group_id, embedding_model_alias, embedding, created_at, updated_at)
                 VALUES ($1,$2,$3,$4,$5::vector,NOW(),NOW())
                 ON CONFLICT (memory_id) DO UPDATE SET
                   key_id = EXCLUDED.key_id,
                   isolation_group_id = EXCLUDED.isolation_group_id,
                   embedding_model_alias = EXCLUDED.embedding_model_alias,
                   embedding = EXCLUDED.embedding,
                   updated_at = NOW()";
const SEARCH_MEMORY_BY_EMBEDDING_SQL: &str = "SELECT memories.id, memories.scope_id, memories.owner_scope_id, memories.published_from_scope_id, memories.memory_kind, memories.state, memories.title, memories.body, memories.language_code, memories.source_refs, memories.confidence, memories.importance, memories.stability, memories.freshness, memories.visibility, memories.sensitivity, memories.evidence_count, memories.created_at, memories.updated_at
             FROM memories
             JOIN memory_embeddings ON memory_embeddings.memory_id = memories.id
             WHERE memories.scope_id = ";
const UPSERT_MEMORY_SOURCE_SQL: &str = "INSERT INTO memory_sources
                 (id, source_kind, display_name, owner_principal_id, owner_scope_id, source_uri, sync_mode, local_root, status, created_at, updated_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
                 ON CONFLICT (id) DO UPDATE SET
                   source_kind = EXCLUDED.source_kind,
                   display_name = EXCLUDED.display_name,
                   owner_principal_id = EXCLUDED.owner_principal_id,
                   owner_scope_id = EXCLUDED.owner_scope_id,
                   source_uri = EXCLUDED.source_uri,
                   sync_mode = EXCLUDED.sync_mode,
                   local_root = EXCLUDED.local_root,
                   status = EXCLUDED.status,
                   updated_at = EXCLUDED.updated_at";
const SELECT_MEMORY_SOURCE_SQL: &str = "SELECT id, source_kind, display_name, owner_principal_id, owner_scope_id, source_uri, sync_mode, local_root, status, created_at, updated_at
             FROM memory_sources WHERE id = $1";
const LIST_MEMORY_SOURCES_SQL: &str = "SELECT id, source_kind, display_name, owner_principal_id, owner_scope_id, source_uri, sync_mode, local_root, status, created_at, updated_at
             FROM memory_sources
             WHERE owner_scope_id = $1
             ORDER BY updated_at DESC
             LIMIT $2";
const LIST_ACCESS_KEYS_FOR_SOURCE_SQL: &str = "SELECT id, key_hash, display_name, source_id, source_kind, owner_principal_id, owner_scope_id, scope_kind, storage_mode, is_fully_isolated, isolation_group_id, status, created_at, last_used_at
             FROM access_keys
             WHERE source_id = $1
             ORDER BY created_at DESC
             LIMIT $2";
const UPSERT_AGENT_CONTEXT_SQL: &str = "INSERT INTO agent_contexts
                 (id, source_id, key_id, scope_id, session_id, task_id, layer, title, body, labels, expires_at, created_at, updated_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
                 ON CONFLICT (id) DO UPDATE SET
                   source_id = EXCLUDED.source_id,
                   key_id = EXCLUDED.key_id,
                   scope_id = EXCLUDED.scope_id,
                   session_id = EXCLUDED.session_id,
                   task_id = EXCLUDED.task_id,
                   layer = EXCLUDED.layer,
                   title = EXCLUDED.title,
                   body = EXCLUDED.body,
                   labels = EXCLUDED.labels,
                   expires_at = EXCLUDED.expires_at,
                   updated_at = EXCLUDED.updated_at";
const LIST_AGENT_CONTEXTS_SQL: &str = "SELECT id, source_id, key_id, scope_id, session_id, task_id, layer, title, body, labels, expires_at, created_at, updated_at
             FROM agent_contexts
             WHERE scope_id = $1
               AND session_id = $2
               AND ($3::text IS NULL OR task_id = $3)
               AND (expires_at IS NULL OR expires_at > NOW())
             ORDER BY updated_at DESC
             LIMIT $4";
const SELECT_AGENT_CONTEXT_SQL: &str = "SELECT id, source_id, key_id, scope_id, session_id, task_id, layer, title, body, labels, expires_at, created_at, updated_at
             FROM agent_contexts
             WHERE id = $1
               AND (expires_at IS NULL OR expires_at > NOW())";
const DELETE_AGENT_CONTEXT_SQL: &str = "DELETE FROM agent_contexts WHERE id = $1";
const UPSERT_PROJECT_DOCUMENT_SQL: &str = "INSERT INTO project_documents
                 (id, source_id, scope_id, local_path, canonical_uri, title, content_hash, last_seen_mtime, sync_state, conflict_state, artifact_id, memory_id, created_at, updated_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
                 ON CONFLICT (source_id, canonical_uri) DO UPDATE SET
                   scope_id = EXCLUDED.scope_id,
                   local_path = EXCLUDED.local_path,
                   title = EXCLUDED.title,
                   content_hash = EXCLUDED.content_hash,
                   last_seen_mtime = EXCLUDED.last_seen_mtime,
                   sync_state = EXCLUDED.sync_state,
                   conflict_state = EXCLUDED.conflict_state,
                   artifact_id = EXCLUDED.artifact_id,
                   memory_id = EXCLUDED.memory_id,
                   updated_at = EXCLUDED.updated_at";
const SELECT_PROJECT_DOCUMENT_SQL: &str = "SELECT id, source_id, scope_id, local_path, canonical_uri, title, content_hash, last_seen_mtime, sync_state, conflict_state, artifact_id, memory_id, created_at, updated_at
             FROM project_documents
             WHERE source_id = $1 AND canonical_uri = $2";
const LIST_PROJECT_DOCUMENTS_FOR_SOURCE_SQL: &str = "SELECT id, source_id, scope_id, local_path, canonical_uri, title, content_hash, last_seen_mtime, sync_state, conflict_state, artifact_id, memory_id, created_at, updated_at
             FROM project_documents
             WHERE source_id = $1
             ORDER BY updated_at DESC
             LIMIT $2";
const LIST_PROJECT_DOCUMENT_CONFLICTS_SQL: &str = "SELECT id, source_id, scope_id, local_path, canonical_uri, title, content_hash, last_seen_mtime, sync_state, conflict_state, artifact_id, memory_id, created_at, updated_at
             FROM project_documents
             WHERE source_id = $1
               AND (sync_state = 'conflicted' OR conflict_state <> 'none')
             ORDER BY updated_at DESC
             LIMIT $2";
const INSERT_LIFECYCLE_AUDIT_EVENT_SQL: &str = "INSERT INTO memory_lifecycle_audit_events
                 (id, scope_id, memory_id, action, actor, before_status, after_status, reason, created_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)";
const LIST_LIFECYCLE_AUDIT_EVENTS_PREFIX_SQL: &str =
    "SELECT id, scope_id, memory_id, action, actor, before_status, after_status, reason, created_at
             FROM memory_lifecycle_audit_events";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleAuditEventRecord {
    pub id: String,
    pub scope_id: String,
    pub memory_id: String,
    pub action: String,
    pub actor: String,
    pub before_status: Option<String>,
    pub after_status: Option<String>,
    pub reason: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
struct MemoryRecord {
    id: String,
    scope_id: String,
    owner_scope_id: String,
    published_from_scope_id: Option<String>,
    memory_kind: String,
    state: String,
    title: String,
    body: String,
    language_code: Option<String>,
    source_refs: Vec<String>,
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

#[derive(Debug, Clone)]
struct AccessKeyRecord {
    id: String,
    key_hash: String,
    display_name: String,
    source_id: Option<String>,
    source_kind: String,
    owner_principal_id: String,
    owner_scope_id: String,
    scope_kind: String,
    storage_mode: String,
    is_fully_isolated: bool,
    isolation_group_id: String,
    status: String,
    created_at: OffsetDateTime,
    last_used_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone)]
struct MemorySourceRecord {
    id: String,
    source_kind: String,
    display_name: String,
    owner_principal_id: String,
    owner_scope_id: String,
    source_uri: Option<String>,
    sync_mode: String,
    local_root: Option<String>,
    status: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
struct AgentContextRecord {
    id: String,
    source_id: Option<String>,
    key_id: Option<String>,
    scope_id: String,
    session_id: String,
    task_id: Option<String>,
    layer: String,
    title: String,
    body: String,
    labels: Vec<String>,
    expires_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
struct ProjectDocumentRecord {
    id: String,
    source_id: String,
    scope_id: String,
    local_path: Option<String>,
    canonical_uri: String,
    title: String,
    content_hash: String,
    last_seen_mtime: Option<OffsetDateTime>,
    sync_state: String,
    conflict_state: String,
    artifact_id: Option<String>,
    memory_id: Option<String>,
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
        let mut tx = self.pool.begin().await?;
        tx.execute(sqlx::query(
            "SELECT pg_advisory_xact_lock(hashtext('meat_memory_schema_migrations'))",
        ))
        .await?;
        tx.execute(sqlx::raw_sql(MIGRATION_0001)).await?;
        tx.execute(sqlx::raw_sql(MIGRATION_0002)).await?;
        tx.execute(sqlx::raw_sql(MIGRATION_0003)).await?;
        tx.execute(sqlx::raw_sql(MIGRATION_0004)).await?;
        tx.execute(sqlx::raw_sql(MIGRATION_0005)).await?;
        tx.execute(sqlx::raw_sql(MIGRATION_0006)).await?;
        tx.execute(sqlx::raw_sql(MIGRATION_0007)).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn seed_scope(
        &self,
        scope_id: &ScopeId,
        scope_name: &str,
        scope_path: &str,
    ) -> Result<()> {
        let scope = Scope::new_with_id(
            scope_id.clone(),
            ScopeType::Project,
            scope_name,
            scope_path,
            None,
        )?;
        self.seed_scope_definition(&scope).await
    }

    pub async fn seed_scope_definition(&self, scope: &Scope) -> Result<()> {
        self.pool
            .execute(
                sqlx::query(seed_scope_sql())
                    .bind(scope.id.as_str())
                    .bind(scope.parent_scope_id.as_ref().map(ScopeId::as_str))
                    .bind(scope_type_to_str(scope.scope_type))
                    .bind(&scope.name)
                    .bind(&scope.path)
                    .bind(scope.owner_principal_id.as_deref())
                    .bind(scope.inherit_policy.as_str())
                    .bind(visibility_to_str(scope.default_visibility))
                    .bind(scope.sync_policy.as_str()),
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
                    .bind(sqlx::types::Json(&artifact.labels))
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
                .bind(memory.owner_scope_id.as_str())
                .bind(memory.published_from_scope_id.as_ref().map(ScopeId::as_str))
                .bind(memory_kind_to_str(memory.kind))
                .bind(memory_state_to_str(memory.state))
                .bind(&memory.title)
                .bind(&memory.body)
                .bind(memory.language_code.as_deref())
                .bind(sqlx::types::Json(&memory.source_refs))
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
                .bind(memory.owner_scope_id.as_str())
                .bind(memory.published_from_scope_id.as_ref().map(ScopeId::as_str))
                .bind(memory_kind_to_str(memory.kind))
                .bind(memory_state_to_str(memory.state))
                .bind(&memory.title)
                .bind(&memory.body)
                .bind(memory.language_code.as_deref())
                .bind(sqlx::types::Json(&memory.source_refs))
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

        let mut builder = build_search_query(scope_id.as_str(), &terms, limit, None);

        let rows = builder.build().fetch_all(&self.pool).await?;

        rows.into_iter().map(row_to_memory).collect()
    }

    pub async fn search_by_keyword_for_isolation_group(
        &self,
        scope_id: &ScopeId,
        keyword: &str,
        isolation_group_id: &str,
        limit: i64,
    ) -> Result<Vec<Memory>> {
        let terms = search_terms(keyword);
        if terms.is_empty() {
            return Ok(Vec::new());
        }

        let mut builder =
            build_search_query(scope_id.as_str(), &terms, limit, Some(isolation_group_id));
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.into_iter().map(row_to_memory).collect()
    }

    pub async fn list_memories(
        &self,
        scope_id: Option<&ScopeId>,
        limit: i64,
    ) -> Result<Vec<Memory>> {
        let mut builder = build_list_query(scope_id.map(ScopeId::as_str), limit);
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.into_iter().map(row_to_memory).collect()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_lifecycle_audit_event(
        &self,
        scope_id: &ScopeId,
        memory_id: &MemoryId,
        action: &str,
        actor: &str,
        before_status: Option<&str>,
        after_status: Option<&str>,
        reason: Option<&str>,
        created_at: OffsetDateTime,
    ) -> Result<LifecycleAuditEventRecord> {
        let event = LifecycleAuditEventRecord {
            id: format!("aud_{}", ulid::Ulid::new()),
            scope_id: scope_id.as_str().to_string(),
            memory_id: memory_id.as_str().to_string(),
            action: action.to_string(),
            actor: actor.to_string(),
            before_status: before_status.map(ToString::to_string),
            after_status: after_status.map(ToString::to_string),
            reason: reason.map(ToString::to_string),
            created_at,
        };

        self.pool
            .execute(
                sqlx::query(INSERT_LIFECYCLE_AUDIT_EVENT_SQL)
                    .bind(&event.id)
                    .bind(&event.scope_id)
                    .bind(&event.memory_id)
                    .bind(&event.action)
                    .bind(&event.actor)
                    .bind(event.before_status.as_deref())
                    .bind(event.after_status.as_deref())
                    .bind(event.reason.as_deref())
                    .bind(event.created_at),
            )
            .await?;

        Ok(event)
    }

    pub async fn list_lifecycle_audit_events(
        &self,
        scope_id: Option<&ScopeId>,
        memory_id: Option<&MemoryId>,
        limit: i64,
    ) -> Result<Vec<LifecycleAuditEventRecord>> {
        let mut builder = QueryBuilder::<Postgres>::new(LIST_LIFECYCLE_AUDIT_EVENTS_PREFIX_SQL);
        let mut has_condition = false;
        if scope_id.is_some() || memory_id.is_some() {
            builder.push(" WHERE ");
            if let Some(scope_id) = scope_id {
                builder.push("scope_id = ");
                builder.push_bind(scope_id.as_str());
                has_condition = true;
            }
            if let Some(memory_id) = memory_id {
                if has_condition {
                    builder.push(" AND ");
                }
                builder.push("memory_id = ");
                builder.push_bind(memory_id.as_str());
            }
        }
        builder.push(" ORDER BY created_at DESC LIMIT ");
        builder.push_bind(limit.clamp(1, 500));

        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.into_iter().map(row_to_lifecycle_audit_event).collect()
    }

    pub async fn upsert_access_key(&self, access_key: &AccessKey) -> Result<AccessKeyId> {
        self.seed_scope(
            &access_key.owner_scope_id,
            access_key.owner_scope_id.as_str(),
            &format!("default/scopes/{}", access_key.owner_scope_id.as_str()),
        )
        .await?;
        self.pool
            .execute(
                sqlx::query(insert_access_key_sql())
                    .bind(access_key.id.as_str())
                    .bind(&access_key.key_hash)
                    .bind(&access_key.display_name)
                    .bind(access_key.source_id.as_ref().map(SourceId::as_str))
                    .bind(access_key.source_kind.as_str())
                    .bind(&access_key.owner_principal_id)
                    .bind(access_key.owner_scope_id.as_str())
                    .bind(access_key.scope_kind.as_str())
                    .bind(access_key.storage_mode.as_str())
                    .bind(access_key.is_fully_isolated)
                    .bind(&access_key.isolation_group_id)
                    .bind(access_key.status.as_str())
                    .bind(access_key.created_at)
                    .bind(access_key.last_used_at),
            )
            .await?;
        Ok(access_key.id.clone())
    }

    pub async fn get_access_key_by_hash(&self, key_hash: &str) -> Result<Option<AccessKey>> {
        let row = sqlx::query(select_access_key_by_hash_sql())
            .bind(key_hash)
            .fetch_optional(&self.pool)
            .await?;

        row.map(row_to_access_key).transpose()
    }

    pub async fn get_access_key_by_id(&self, key_id: &AccessKeyId) -> Result<Option<AccessKey>> {
        let row = sqlx::query(select_access_key_by_id_sql())
            .bind(key_id.as_str())
            .fetch_optional(&self.pool)
            .await?;

        row.map(row_to_access_key).transpose()
    }

    pub async fn list_access_keys(&self, limit: i64) -> Result<Vec<AccessKey>> {
        let rows = sqlx::query(list_access_keys_sql())
            .bind(limit.clamp(1, 500))
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(row_to_access_key).collect()
    }

    pub async fn list_access_keys_for_source(
        &self,
        source_id: &SourceId,
        limit: i64,
    ) -> Result<Vec<AccessKey>> {
        let rows = sqlx::query(list_access_keys_for_source_sql())
            .bind(source_id.as_str())
            .bind(limit.clamp(1, 500))
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(row_to_access_key).collect()
    }

    pub async fn touch_access_key(&self, key_id: &AccessKeyId) -> Result<()> {
        self.pool
            .execute(sqlx::query(update_access_key_last_used_sql()).bind(key_id.as_str()))
            .await?;
        Ok(())
    }

    pub async fn update_access_key_status(
        &self,
        key_id: &AccessKeyId,
        status: AccessKeyStatus,
    ) -> Result<()> {
        self.pool
            .execute(
                sqlx::query(update_access_key_status_sql())
                    .bind(key_id.as_str())
                    .bind(status.as_str()),
            )
            .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn record_key_usage(
        &self,
        key_id: Option<&AccessKeyId>,
        source_kind: KeySourceKind,
        operation: &str,
        scope_id: Option<&ScopeId>,
        storage_mode: StorageMode,
        success: bool,
        latency_ms: u64,
        error_code: Option<&str>,
    ) -> Result<()> {
        self.pool
            .execute(
                sqlx::query(insert_key_usage_event_sql())
                    .bind(format!("evt_{}", ulid::Ulid::new()))
                    .bind(key_id.map(AccessKeyId::as_str))
                    .bind(source_kind.as_str())
                    .bind(operation)
                    .bind(scope_id.map(ScopeId::as_str))
                    .bind(storage_mode.as_str())
                    .bind(success)
                    .bind(i64::try_from(latency_ms).unwrap_or(i64::MAX))
                    .bind(error_code),
            )
            .await?;
        Ok(())
    }

    pub async fn access_key_usage_stats(
        &self,
        key_id: &AccessKeyId,
    ) -> Result<Option<AccessKeyUsageStats>> {
        let Some(access_key) = self.get_access_key_by_id(key_id).await? else {
            return Ok(None);
        };

        let rows = sqlx::query(
            "SELECT source_kind, storage_mode, success, latency_ms \
             FROM key_usage_events WHERE key_id = $1 ORDER BY created_at DESC LIMIT 5000",
        )
        .bind(key_id.as_str())
        .fetch_all(&self.pool)
        .await?;

        let mut total_operations = 0_u64;
        let mut successful_operations = 0_u64;
        let mut failed_operations = 0_u64;
        let mut latency_values = Vec::with_capacity(rows.len());
        let mut by_source = std::collections::BTreeMap::<String, u64>::new();
        let mut by_storage = std::collections::BTreeMap::<String, u64>::new();

        for row in rows {
            total_operations += 1;
            let source_kind = row.try_get::<String, _>("source_kind")?;
            let storage_mode = row.try_get::<String, _>("storage_mode")?;
            let success = row.try_get::<bool, _>("success")?;
            let latency_ms = row.try_get::<i64, _>("latency_ms")?;
            if success {
                successful_operations += 1;
            } else {
                failed_operations += 1;
            }
            latency_values.push(u64::try_from(latency_ms.max(0)).unwrap_or(0));
            *by_source.entry(source_kind).or_insert(0) += 1;
            *by_storage.entry(storage_mode).or_insert(0) += 1;
        }

        latency_values.sort_unstable();
        let avg_latency_ms = if latency_values.is_empty() {
            0.0
        } else {
            latency_values.iter().sum::<u64>() as f64 / latency_values.len() as f64
        };
        let p95_latency_ms = if latency_values.is_empty() {
            0
        } else {
            let index = ((latency_values.len() - 1) as f64 * 0.95).round() as usize;
            latency_values[index]
        };

        Ok(Some(AccessKeyUsageStats {
            key_id: access_key.id,
            total_operations,
            successful_operations,
            failed_operations,
            avg_latency_ms,
            p95_latency_ms,
            last_used_at: access_key.last_used_at,
            by_source: by_source
                .into_iter()
                .map(|(label, count)| KeyUsageBreakdown { label, count })
                .collect(),
            by_storage_mode: by_storage
                .into_iter()
                .map(|(label, count)| KeyUsageBreakdown { label, count })
                .collect(),
        }))
    }

    pub async fn link_memory_key(
        &self,
        memory_id: &MemoryId,
        key_id: &AccessKeyId,
        isolation_group_id: &str,
    ) -> Result<()> {
        self.pool
            .execute(
                sqlx::query(insert_memory_key_link_sql())
                    .bind(memory_id.as_str())
                    .bind(key_id.as_str())
                    .bind(isolation_group_id),
            )
            .await?;
        Ok(())
    }

    pub async fn upsert_memory_embedding(
        &self,
        memory_id: &MemoryId,
        key_id: Option<&AccessKeyId>,
        isolation_group_id: &str,
        embedding_model_alias: &str,
        vector: &[f32],
    ) -> Result<()> {
        self.pool
            .execute(
                sqlx::query(upsert_memory_embedding_sql())
                    .bind(memory_id.as_str())
                    .bind(key_id.map(AccessKeyId::as_str))
                    .bind(isolation_group_id)
                    .bind(embedding_model_alias)
                    .bind(vector_literal(vector)),
            )
            .await?;
        Ok(())
    }

    pub async fn search_by_embedding(
        &self,
        scope_id: &ScopeId,
        vector: &[f32],
        isolation_group_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Memory>> {
        let vector = vector_literal(vector);
        let mut builder =
            build_embedding_search_query(scope_id.as_str(), &vector, isolation_group_id, limit);
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.into_iter().map(row_to_memory).collect()
    }

    pub async fn upsert_memory_source(&self, source: &MemorySource) -> Result<SourceId> {
        self.seed_scope(
            &source.owner_scope_id,
            source.owner_scope_id.as_str(),
            &format!("default/scopes/{}", source.owner_scope_id.as_str()),
        )
        .await?;
        self.pool
            .execute(
                sqlx::query(upsert_memory_source_sql())
                    .bind(source.id.as_str())
                    .bind(&source.source_kind)
                    .bind(&source.display_name)
                    .bind(&source.owner_principal_id)
                    .bind(source.owner_scope_id.as_str())
                    .bind(source.source_uri.as_deref())
                    .bind(source.sync_mode.as_str())
                    .bind(source.local_root.as_deref())
                    .bind(source.status.as_str())
                    .bind(source.created_at)
                    .bind(source.updated_at),
            )
            .await?;
        Ok(source.id.clone())
    }

    pub async fn get_memory_source(&self, source_id: &SourceId) -> Result<Option<MemorySource>> {
        let row = sqlx::query(select_memory_source_sql())
            .bind(source_id.as_str())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_memory_source).transpose()
    }

    pub async fn list_memory_sources(
        &self,
        owner_scope_id: &ScopeId,
        limit: i64,
    ) -> Result<Vec<MemorySource>> {
        let rows = sqlx::query(list_memory_sources_sql())
            .bind(owner_scope_id.as_str())
            .bind(limit.clamp(1, 500))
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(row_to_memory_source).collect()
    }

    pub async fn upsert_agent_context(&self, context: &AgentContext) -> Result<AgentContextId> {
        self.pool
            .execute(
                sqlx::query(upsert_agent_context_sql())
                    .bind(context.id.as_str())
                    .bind(context.source_id.as_ref().map(SourceId::as_str))
                    .bind(context.key_id.as_ref().map(AccessKeyId::as_str))
                    .bind(context.scope_id.as_str())
                    .bind(&context.session_id)
                    .bind(context.task_id.as_deref())
                    .bind(context.layer.as_str())
                    .bind(&context.title)
                    .bind(&context.body)
                    .bind(sqlx::types::Json(&context.labels))
                    .bind(context.expires_at)
                    .bind(context.created_at)
                    .bind(context.updated_at),
            )
            .await?;
        Ok(context.id.clone())
    }

    pub async fn list_agent_contexts(
        &self,
        scope_id: &ScopeId,
        session_id: &str,
        task_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<AgentContext>> {
        let rows = sqlx::query(list_agent_contexts_sql())
            .bind(scope_id.as_str())
            .bind(session_id)
            .bind(task_id)
            .bind(limit.clamp(1, 500))
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(row_to_agent_context).collect()
    }

    pub async fn get_agent_context(
        &self,
        context_id: &AgentContextId,
    ) -> Result<Option<AgentContext>> {
        let row = sqlx::query(select_agent_context_sql())
            .bind(context_id.as_str())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_agent_context).transpose()
    }

    pub async fn delete_agent_context(&self, context_id: &AgentContextId) -> Result<()> {
        self.pool
            .execute(sqlx::query(delete_agent_context_sql()).bind(context_id.as_str()))
            .await?;
        Ok(())
    }

    pub async fn upsert_project_document(
        &self,
        document: &ProjectDocument,
    ) -> Result<ProjectDocumentId> {
        self.pool
            .execute(
                sqlx::query(upsert_project_document_sql())
                    .bind(document.id.as_str())
                    .bind(document.source_id.as_str())
                    .bind(document.scope_id.as_str())
                    .bind(document.local_path.as_deref())
                    .bind(&document.canonical_uri)
                    .bind(&document.title)
                    .bind(&document.content_hash)
                    .bind(document.last_seen_mtime)
                    .bind(document.sync_state.as_str())
                    .bind(document.conflict_state.as_str())
                    .bind(document.artifact_id.as_ref().map(ArtifactId::as_str))
                    .bind(document.memory_id.as_ref().map(MemoryId::as_str))
                    .bind(document.created_at)
                    .bind(document.updated_at),
            )
            .await?;
        Ok(document.id.clone())
    }

    pub async fn get_project_document(
        &self,
        source_id: &SourceId,
        canonical_uri: &str,
    ) -> Result<Option<ProjectDocument>> {
        let row = sqlx::query(select_project_document_sql())
            .bind(source_id.as_str())
            .bind(canonical_uri)
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_project_document).transpose()
    }

    pub async fn list_project_documents_for_source(
        &self,
        source_id: &SourceId,
        limit: i64,
    ) -> Result<Vec<ProjectDocument>> {
        let rows = sqlx::query(list_project_documents_for_source_sql())
            .bind(source_id.as_str())
            .bind(limit.clamp(1, 500))
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(row_to_project_document).collect()
    }

    pub async fn list_project_document_conflicts(
        &self,
        source_id: &SourceId,
        limit: i64,
    ) -> Result<Vec<ProjectDocument>> {
        let rows = sqlx::query(list_project_document_conflicts_sql())
            .bind(source_id.as_str())
            .bind(limit.clamp(1, 500))
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(row_to_project_document).collect()
    }
}

fn vector_literal(vector: &[f32]) -> String {
    let values = vector
        .iter()
        .map(|value| {
            if value.is_finite() {
                value.to_string()
            } else {
                "0".to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
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
    isolation_group_id: Option<&'a str>,
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

    if let Some(isolation_group_id) = isolation_group_id {
        builder.push(" AND EXISTS (SELECT 1 FROM memory_key_links mkl WHERE mkl.memory_id = memories.id AND mkl.isolation_group_id = ");
        builder.push_bind(isolation_group_id);
        builder.push(")");
    }

    builder.push(" ORDER BY updated_at DESC LIMIT ");
    builder.push_bind(limit);
    builder
}

fn build_list_query(scope_id: Option<&str>, limit: i64) -> QueryBuilder<'_, Postgres> {
    let mut builder = QueryBuilder::<Postgres>::new(LIST_MEMORY_PREFIX_SQL);
    if let Some(scope_id) = scope_id {
        builder.push(" WHERE scope_id = ");
        builder.push_bind(scope_id);
    }
    builder.push(" ORDER BY updated_at DESC LIMIT ");
    builder.push_bind(limit);
    builder
}

fn build_embedding_search_query<'a>(
    scope_id: &'a str,
    vector: &'a str,
    isolation_group_id: Option<&'a str>,
    limit: i64,
) -> QueryBuilder<'a, Postgres> {
    let mut builder = QueryBuilder::<Postgres>::new(SEARCH_MEMORY_BY_EMBEDDING_SQL);
    builder.push_bind(scope_id);
    if let Some(isolation_group_id) = isolation_group_id {
        builder.push(" AND memory_embeddings.isolation_group_id = ");
        builder.push_bind(isolation_group_id);
    }
    builder.push(" ORDER BY memory_embeddings.embedding <-> ");
    builder.push_bind(vector);
    builder.push("::vector LIMIT ");
    builder.push_bind(limit);
    builder
}

fn row_to_memory(row: sqlx::postgres::PgRow) -> Result<Memory> {
    let source_refs = row
        .try_get::<sqlx::types::Json<Vec<String>>, _>("source_refs")?
        .0;

    memory_from_record(MemoryRecord {
        id: row.try_get::<String, _>("id")?,
        scope_id: row.try_get::<String, _>("scope_id")?,
        owner_scope_id: row.try_get::<String, _>("owner_scope_id")?,
        published_from_scope_id: row.try_get::<Option<String>, _>("published_from_scope_id")?,
        memory_kind: row.try_get("memory_kind")?,
        state: row.try_get("state")?,
        title: row.try_get("title")?,
        body: row.try_get("body")?,
        language_code: row.try_get::<Option<String>, _>("language_code")?,
        source_refs,
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

fn row_to_access_key(row: sqlx::postgres::PgRow) -> Result<AccessKey> {
    access_key_from_record(AccessKeyRecord {
        id: row.try_get::<String, _>("id")?,
        key_hash: row.try_get("key_hash")?,
        display_name: row.try_get("display_name")?,
        source_id: row.try_get::<Option<String>, _>("source_id")?,
        source_kind: row.try_get("source_kind")?,
        owner_principal_id: row.try_get("owner_principal_id")?,
        owner_scope_id: row.try_get("owner_scope_id")?,
        scope_kind: row.try_get("scope_kind")?,
        storage_mode: row.try_get("storage_mode")?,
        is_fully_isolated: row.try_get("is_fully_isolated")?,
        isolation_group_id: row.try_get("isolation_group_id")?,
        status: row.try_get("status")?,
        created_at: row.try_get("created_at")?,
        last_used_at: row.try_get("last_used_at")?,
    })
}

fn row_to_memory_source(row: sqlx::postgres::PgRow) -> Result<MemorySource> {
    memory_source_from_record(MemorySourceRecord {
        id: row.try_get::<String, _>("id")?,
        source_kind: row.try_get("source_kind")?,
        display_name: row.try_get("display_name")?,
        owner_principal_id: row.try_get("owner_principal_id")?,
        owner_scope_id: row.try_get("owner_scope_id")?,
        source_uri: row.try_get("source_uri")?,
        sync_mode: row.try_get("sync_mode")?,
        local_root: row.try_get("local_root")?,
        status: row.try_get("status")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn row_to_lifecycle_audit_event(row: sqlx::postgres::PgRow) -> Result<LifecycleAuditEventRecord> {
    Ok(LifecycleAuditEventRecord {
        id: row.try_get("id")?,
        scope_id: row.try_get("scope_id")?,
        memory_id: row.try_get("memory_id")?,
        action: row.try_get("action")?,
        actor: row.try_get("actor")?,
        before_status: row.try_get("before_status")?,
        after_status: row.try_get("after_status")?,
        reason: row.try_get("reason")?,
        created_at: row.try_get("created_at")?,
    })
}

fn row_to_agent_context(row: sqlx::postgres::PgRow) -> Result<AgentContext> {
    let labels = row
        .try_get::<sqlx::types::Json<Vec<String>>, _>("labels")?
        .0;
    agent_context_from_record(AgentContextRecord {
        id: row.try_get::<String, _>("id")?,
        source_id: row.try_get("source_id")?,
        key_id: row.try_get("key_id")?,
        scope_id: row.try_get("scope_id")?,
        session_id: row.try_get("session_id")?,
        task_id: row.try_get("task_id")?,
        layer: row.try_get("layer")?,
        title: row.try_get("title")?,
        body: row.try_get("body")?,
        labels,
        expires_at: row.try_get("expires_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn row_to_project_document(row: sqlx::postgres::PgRow) -> Result<ProjectDocument> {
    project_document_from_record(ProjectDocumentRecord {
        id: row.try_get::<String, _>("id")?,
        source_id: row.try_get("source_id")?,
        scope_id: row.try_get("scope_id")?,
        local_path: row.try_get("local_path")?,
        canonical_uri: row.try_get("canonical_uri")?,
        title: row.try_get("title")?,
        content_hash: row.try_get("content_hash")?,
        last_seen_mtime: row.try_get("last_seen_mtime")?,
        sync_state: row.try_get("sync_state")?,
        conflict_state: row.try_get("conflict_state")?,
        artifact_id: row.try_get("artifact_id")?,
        memory_id: row.try_get("memory_id")?,
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
        owner_scope_id: ScopeId::from_string(record.owner_scope_id),
        published_from_scope_id: record.published_from_scope_id.map(ScopeId::from_string),
        kind: parse_memory_kind(&record.memory_kind)?,
        state: parse_memory_state(&record.state)?,
        title: record.title,
        body: record.body,
        language_code: record.language_code,
        source_refs: record.source_refs,
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

fn access_key_from_record(record: AccessKeyRecord) -> Result<AccessKey> {
    Ok(AccessKey {
        id: AccessKeyId::from_string(record.id),
        key_hash: record.key_hash,
        display_name: record.display_name,
        source_id: record.source_id.map(SourceId::from_string),
        source_kind: KeySourceKind::parse(&record.source_kind)?,
        owner_principal_id: record.owner_principal_id,
        owner_scope_id: ScopeId::from_string(record.owner_scope_id),
        scope_kind: KeyScopeKind::parse(&record.scope_kind)?,
        storage_mode: StorageMode::parse(&record.storage_mode)?,
        is_fully_isolated: record.is_fully_isolated,
        isolation_group_id: record.isolation_group_id,
        status: AccessKeyStatus::parse(&record.status)?,
        created_at: record.created_at,
        last_used_at: record.last_used_at,
    })
}

fn memory_source_from_record(record: MemorySourceRecord) -> Result<MemorySource> {
    Ok(MemorySource {
        id: SourceId::from_string(record.id),
        source_kind: record.source_kind,
        display_name: record.display_name,
        owner_principal_id: record.owner_principal_id,
        owner_scope_id: ScopeId::from_string(record.owner_scope_id),
        source_uri: record.source_uri,
        sync_mode: SourceSyncMode::parse(&record.sync_mode)?,
        local_root: record.local_root,
        status: SourceStatus::parse(&record.status)?,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

fn agent_context_from_record(record: AgentContextRecord) -> Result<AgentContext> {
    Ok(AgentContext {
        id: AgentContextId::from_string(record.id),
        source_id: record.source_id.map(SourceId::from_string),
        key_id: record.key_id.map(AccessKeyId::from_string),
        scope_id: ScopeId::from_string(record.scope_id),
        session_id: record.session_id,
        task_id: record.task_id,
        layer: memory_domain::MemoryLayer::parse(&record.layer)?,
        title: record.title,
        body: record.body,
        labels: record.labels,
        expires_at: record.expires_at,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

fn project_document_from_record(record: ProjectDocumentRecord) -> Result<ProjectDocument> {
    Ok(ProjectDocument {
        id: ProjectDocumentId::from_string(record.id),
        source_id: SourceId::from_string(record.source_id),
        scope_id: ScopeId::from_string(record.scope_id),
        local_path: record.local_path,
        canonical_uri: record.canonical_uri,
        title: record.title,
        content_hash: record.content_hash,
        last_seen_mtime: record.last_seen_mtime,
        sync_state: DocumentSyncState::parse(&record.sync_state)?,
        conflict_state: DocumentConflictState::parse(&record.conflict_state)?,
        artifact_id: record.artifact_id.map(ArtifactId::from_string),
        memory_id: record.memory_id.map(MemoryId::from_string),
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

fn insert_access_key_sql() -> &'static str {
    INSERT_ACCESS_KEY_SQL
}

fn select_access_key_by_hash_sql() -> &'static str {
    SELECT_ACCESS_KEY_BY_HASH_SQL
}

fn select_access_key_by_id_sql() -> &'static str {
    SELECT_ACCESS_KEY_BY_ID_SQL
}

fn list_access_keys_sql() -> &'static str {
    LIST_ACCESS_KEYS_SQL
}

fn list_access_keys_for_source_sql() -> &'static str {
    LIST_ACCESS_KEYS_FOR_SOURCE_SQL
}

fn update_access_key_last_used_sql() -> &'static str {
    UPDATE_ACCESS_KEY_LAST_USED_SQL
}

fn update_access_key_status_sql() -> &'static str {
    UPDATE_ACCESS_KEY_STATUS_SQL
}

fn insert_key_usage_event_sql() -> &'static str {
    INSERT_KEY_USAGE_EVENT_SQL
}

fn insert_memory_key_link_sql() -> &'static str {
    INSERT_MEMORY_KEY_LINK_SQL
}

fn upsert_memory_embedding_sql() -> &'static str {
    UPSERT_MEMORY_EMBEDDING_SQL
}

fn upsert_memory_source_sql() -> &'static str {
    UPSERT_MEMORY_SOURCE_SQL
}

fn select_memory_source_sql() -> &'static str {
    SELECT_MEMORY_SOURCE_SQL
}

fn list_memory_sources_sql() -> &'static str {
    LIST_MEMORY_SOURCES_SQL
}

fn upsert_agent_context_sql() -> &'static str {
    UPSERT_AGENT_CONTEXT_SQL
}

fn list_agent_contexts_sql() -> &'static str {
    LIST_AGENT_CONTEXTS_SQL
}

fn select_agent_context_sql() -> &'static str {
    SELECT_AGENT_CONTEXT_SQL
}

fn delete_agent_context_sql() -> &'static str {
    DELETE_AGENT_CONTEXT_SQL
}

fn upsert_project_document_sql() -> &'static str {
    UPSERT_PROJECT_DOCUMENT_SQL
}

fn select_project_document_sql() -> &'static str {
    SELECT_PROJECT_DOCUMENT_SQL
}

fn list_project_documents_for_source_sql() -> &'static str {
    LIST_PROJECT_DOCUMENTS_FOR_SOURCE_SQL
}

fn list_project_document_conflicts_sql() -> &'static str {
    LIST_PROJECT_DOCUMENT_CONFLICTS_SQL
}

#[cfg(test)]
fn list_memory_prefix_sql() -> &'static str {
    LIST_MEMORY_PREFIX_SQL
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
        MemoryState::Forgotten => "forgotten",
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

fn scope_type_to_str(scope_type: ScopeType) -> &'static str {
    scope_type.as_str()
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
        "forgotten" => MemoryState::Forgotten,
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
#[path = "lib_tests.rs"]
mod lib_tests;
