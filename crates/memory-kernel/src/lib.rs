use anyhow::{Result, anyhow, bail};
use async_trait::async_trait;
use memory_assets::{FileSystemAssetStore, PutAssetRequest, StorageClass, StoredAsset};
use memory_core::MemoryService;
use memory_domain::{
    AccessKey, AccessKeyId, AccessKeyStatus, AccessKeyUsageStats, Artifact, ArtifactKind,
    ContextBundle, Entity, KeyScopeKind, KeySourceKind, Memory, MemoryId, MemoryKind, Relation,
    RequestContext, Scope, ScopeId, ScopeType, Sensitivity, StorageMode, Visibility,
    hash_access_key,
};
use memory_extract::{
    ExtractionEnvelope, detect_language_code, distill_candidate_memory, extract_entities,
    extract_relations, should_extract,
};
use memory_index::{SearchQuery, normalize_query};
use memory_models::{
    EmbeddingGateway, EmbeddingRequest, ImageProfile, ModelRegistry, VisionGateway, VisionRequest,
    VisionResponse, inspect_image,
};
use memory_observability::{
    operation_span, record_key_operation, record_search_failure, record_search_success,
    record_write_failure, record_write_success,
};
use memory_policy::{
    PolicyDecision, PublishPolicyInput, WritePolicyInput, evaluate_publish_policy,
    evaluate_write_policy, redact_for_shared_scope,
};
use memory_store_md::MarkdownStore;
use memory_store_pg::PgStore;
use std::path::PathBuf;
use std::time::Instant;
use tracing::{Instrument, info, warn};

#[derive(Debug, Clone)]
pub struct RememberTextRequest {
    pub scope_id: ScopeId,
    pub title: Option<String>,
    pub body: String,
    pub artifact_kind: ArtifactKind,
    pub memory_kind: Option<MemoryKind>,
    pub source_refs: Vec<String>,
    pub visibility: Visibility,
    pub sensitivity: Sensitivity,
    pub context: Option<RequestContext>,
}

impl RememberTextRequest {
    pub fn new(scope_id: ScopeId, body: impl Into<String>) -> Self {
        Self {
            scope_id,
            title: None,
            body: body.into(),
            artifact_kind: ArtifactKind::Message,
            memory_kind: None,
            source_refs: Vec::new(),
            visibility: Visibility::Private,
            sensitivity: Sensitivity::Internal,
            context: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RememberTextResult {
    pub artifact: Artifact,
    pub memory: Memory,
    pub wrote_pg: bool,
    pub wrote_markdown: bool,
}

#[derive(Debug, Clone)]
pub struct RememberImageRequest {
    pub scope_id: ScopeId,
    pub title: Option<String>,
    pub body: Option<String>,
    pub bytes: Vec<u8>,
    pub media_type: String,
    pub file_extension: Option<String>,
    pub memory_kind: Option<MemoryKind>,
    pub source_refs: Vec<String>,
    pub visibility: Visibility,
    pub sensitivity: Sensitivity,
    pub context: Option<RequestContext>,
}

impl RememberImageRequest {
    pub fn new(scope_id: ScopeId, media_type: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            scope_id,
            title: None,
            body: None,
            bytes,
            media_type: media_type.into(),
            file_extension: None,
            memory_kind: None,
            source_refs: Vec::new(),
            visibility: Visibility::Private,
            sensitivity: Sensitivity::Internal,
            context: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RememberImageResult {
    pub asset: StoredAsset,
    pub artifact: Artifact,
    pub memory: Memory,
    pub vision: Option<VisionResponse>,
    pub wrote_pg: bool,
    pub wrote_markdown: bool,
}

struct ImageArtifactInput {
    scope_id: ScopeId,
    media_type: String,
    body: Option<String>,
    source_refs: Vec<String>,
    visibility: Visibility,
    sensitivity: Sensitivity,
    image_profile: Option<ImageProfile>,
    vision: Option<VisionResponse>,
}

#[derive(Debug, Clone)]
pub struct PublishMemoryResult {
    pub memory: Memory,
    pub wrote_pg: bool,
    pub wrote_markdown: bool,
}

#[derive(Debug, Clone)]
pub struct PromoteMemoryRequest {
    pub source_scope_type: ScopeType,
    pub target_scope_id: ScopeId,
    pub target_scope_type: ScopeType,
    pub target_visibility: Visibility,
}

#[derive(Debug, Clone)]
pub struct SearchContextRequest {
    pub scope_id: ScopeId,
    pub query: String,
    pub limit: usize,
    pub context: Option<RequestContext>,
}

#[derive(Debug, Clone)]
pub struct CreateAccessKeyRequest {
    pub raw_key: Option<String>,
    pub display_name: String,
    pub source_kind: KeySourceKind,
    pub owner_principal_id: String,
    pub owner_scope_id: ScopeId,
    pub scope_kind: KeyScopeKind,
    pub storage_mode: StorageMode,
    pub is_fully_isolated: bool,
}

#[derive(Debug, Clone)]
pub struct CreateAccessKeyResult {
    pub access_key: AccessKey,
    pub raw_key: String,
}

#[derive(Debug, Clone)]
pub struct UpdateAccessKeyRequest {
    pub key_id: AccessKeyId,
    pub display_name: Option<String>,
    pub storage_mode: Option<StorageMode>,
    pub status: Option<AccessKeyStatus>,
    pub is_fully_isolated: Option<bool>,
}

impl SearchContextRequest {
    pub fn new(scope_id: ScopeId, query: impl Into<String>) -> Self {
        Self {
            scope_id,
            query: query.into(),
            limit: 10,
            context: None,
        }
    }
}

pub struct KernelBuilder {
    pg_store: Option<PgStore>,
    markdown_store: Option<MarkdownStore>,
    asset_store: Option<FileSystemAssetStore>,
    vision_gateway: Option<VisionGateway>,
    embedding_gateway: Option<EmbeddingGateway>,
}

impl Default for KernelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl KernelBuilder {
    pub fn new() -> Self {
        Self {
            pg_store: None,
            markdown_store: None,
            asset_store: None,
            vision_gateway: None,
            embedding_gateway: None,
        }
    }

    pub async fn with_postgres_url(mut self, database_url: &str) -> Result<Self> {
        let store = PgStore::connect(database_url).await?;
        store.migrate().await?;
        self.pg_store = Some(store);
        Ok(self)
    }

    pub fn with_markdown_root(mut self, root: impl Into<PathBuf>) -> Result<Self> {
        self.markdown_store = Some(MarkdownStore::new(root)?);
        Ok(self)
    }

    pub fn with_markdown_tenant(
        mut self,
        root: impl Into<PathBuf>,
        tenant: impl Into<String>,
    ) -> Result<Self> {
        self.markdown_store = Some(MarkdownStore::with_tenant(root, tenant)?);
        Ok(self)
    }

    pub fn with_asset_root(mut self, root: impl Into<PathBuf>) -> Result<Self> {
        self.asset_store = Some(FileSystemAssetStore::new(root)?);
        Ok(self)
    }

    pub fn with_model_registry(
        mut self,
        registry: ModelRegistry,
        default_locale: impl Into<String>,
    ) -> Result<Self> {
        self.vision_gateway = Some(VisionGateway::new(registry.clone(), default_locale)?);
        self.embedding_gateway = Some(EmbeddingGateway::new(registry)?);
        Ok(self)
    }

    pub fn build(self) -> Result<Kernel> {
        if self.pg_store.is_none() && self.markdown_store.is_none() {
            bail!("kernel requires at least one backing store");
        }

        Ok(Kernel {
            pg_store: self.pg_store,
            markdown_store: self.markdown_store,
            asset_store: self.asset_store,
            vision_gateway: self.vision_gateway,
            embedding_gateway: self.embedding_gateway,
        })
    }
}

pub struct Kernel {
    pg_store: Option<PgStore>,
    markdown_store: Option<MarkdownStore>,
    asset_store: Option<FileSystemAssetStore>,
    vision_gateway: Option<VisionGateway>,
    embedding_gateway: Option<EmbeddingGateway>,
}

impl Kernel {
    pub fn builder() -> KernelBuilder {
        KernelBuilder::new()
    }

    pub fn has_postgres(&self) -> bool {
        self.pg_store.is_some()
    }

    pub fn has_markdown(&self) -> bool {
        self.markdown_store.is_some()
    }

    pub fn has_asset_store(&self) -> bool {
        self.asset_store.is_some()
    }

    pub fn has_vision_gateway(&self) -> bool {
        self.vision_gateway.is_some()
    }

    pub fn has_embedding_gateway(&self) -> bool {
        self.embedding_gateway.is_some()
    }

    pub async fn get_memory(
        &self,
        scope_id: ScopeId,
        memory_id: MemoryId,
    ) -> Result<Option<Memory>> {
        match &self.pg_store {
            Some(pg_store) => pg_store.get_memory(&scope_id, &memory_id).await,
            None => match &self.markdown_store {
                Some(markdown_store) => markdown_store
                    .read_memory_markdown(&scope_id, &memory_id)?
                    .map(|parsed| parsed.into_memory())
                    .transpose(),
                None => Ok(None),
            },
        }
    }

    pub async fn browse_memories(
        &self,
        scope_id: Option<ScopeId>,
        limit: usize,
    ) -> Result<Vec<Memory>> {
        let limit = limit.clamp(1, 500);
        if let Some(pg_store) = &self.pg_store {
            return pg_store
                .list_memories(scope_id.as_ref(), limit as i64)
                .await;
        }

        let Some(markdown_store) = &self.markdown_store else {
            return Ok(Vec::new());
        };

        let mut memories = match scope_id.as_ref() {
            Some(scope_id) => markdown_store.list_memories_by_scope(scope_id)?,
            None => markdown_store.list_all_memories()?,
        };
        memories.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        memories.truncate(limit);
        Ok(memories)
    }

    pub async fn create_access_key(
        &self,
        request: CreateAccessKeyRequest,
    ) -> Result<CreateAccessKeyResult> {
        let raw_key = request
            .raw_key
            .unwrap_or_else(|| format!("mmk_{}", ulid::Ulid::new()));
        let access_key = AccessKey::new(
            &raw_key,
            request.display_name,
            request.source_kind,
            request.owner_principal_id,
            request.owner_scope_id,
            request.scope_kind,
            request.storage_mode,
            request.is_fully_isolated,
        )?;
        let pg_store = self
            .pg_store
            .as_ref()
            .ok_or_else(|| anyhow!("postgres store is required for key management"))?;
        pg_store.upsert_access_key(&access_key).await?;
        Ok(CreateAccessKeyResult {
            access_key,
            raw_key,
        })
    }

    pub async fn list_access_keys(&self, limit: usize) -> Result<Vec<AccessKey>> {
        let pg_store = self
            .pg_store
            .as_ref()
            .ok_or_else(|| anyhow!("postgres store is required for key management"))?;
        pg_store.list_access_keys(limit as i64).await
    }

    pub async fn update_access_key(&self, request: UpdateAccessKeyRequest) -> Result<AccessKey> {
        let pg_store = self
            .pg_store
            .as_ref()
            .ok_or_else(|| anyhow!("postgres store is required for key management"))?;
        let mut access_key = pg_store
            .get_access_key_by_id(&request.key_id)
            .await?
            .ok_or_else(|| anyhow!("access key not found"))?;
        if let Some(display_name) = request.display_name {
            access_key.display_name = display_name.trim().to_string();
        }
        if let Some(storage_mode) = request.storage_mode {
            access_key.storage_mode = storage_mode;
        }
        if let Some(is_fully_isolated) = request.is_fully_isolated {
            access_key.is_fully_isolated = is_fully_isolated;
        }
        if let Some(status) = request.status {
            access_key.status = status;
        }
        pg_store.upsert_access_key(&access_key).await?;
        Ok(access_key)
    }

    pub async fn rotate_access_key(
        &self,
        key_id: &AccessKeyId,
        raw_key: Option<String>,
    ) -> Result<CreateAccessKeyResult> {
        let pg_store = self
            .pg_store
            .as_ref()
            .ok_or_else(|| anyhow!("postgres store is required for key management"))?;
        let existing = pg_store
            .get_access_key_by_id(key_id)
            .await?
            .ok_or_else(|| anyhow!("access key not found"))?;
        pg_store
            .update_access_key_status(key_id, AccessKeyStatus::Revoked)
            .await?;
        let raw_key = raw_key.unwrap_or_else(|| format!("mmk_{}", ulid::Ulid::new()));
        let access_key = AccessKey::new(
            &raw_key,
            existing.display_name,
            existing.source_kind,
            existing.owner_principal_id,
            existing.owner_scope_id,
            existing.scope_kind,
            existing.storage_mode,
            existing.is_fully_isolated,
        )?;
        pg_store.upsert_access_key(&access_key).await?;
        Ok(CreateAccessKeyResult {
            access_key,
            raw_key,
        })
    }

    pub async fn access_key_usage_stats(
        &self,
        key_id: &AccessKeyId,
    ) -> Result<Option<AccessKeyUsageStats>> {
        let pg_store = self
            .pg_store
            .as_ref()
            .ok_or_else(|| anyhow!("postgres store is required for key management"))?;
        pg_store.access_key_usage_stats(key_id).await
    }

    pub async fn resolve_access_key_context(
        &self,
        raw_key: &str,
    ) -> Result<Option<RequestContext>> {
        let Some(pg_store) = &self.pg_store else {
            return Ok(None);
        };
        let key_hash = hash_access_key(raw_key)?;
        let Some(access_key) = pg_store.get_access_key_by_hash(&key_hash).await? else {
            return Ok(None);
        };
        if !matches!(access_key.status, memory_domain::AccessKeyStatus::Active) {
            return Ok(None);
        }
        pg_store.touch_access_key(&access_key.id).await?;
        Ok(Some(access_key.to_context()))
    }

    pub async fn remember_text(&self, request: RememberTextRequest) -> Result<RememberTextResult> {
        let started_at = Instant::now();
        let scope_id = request.scope_id.as_str().to_string();
        let result: Result<RememberTextResult> = async {
            let artifact = self.build_artifact(&request)?;
            self.remember_artifact(
                artifact,
                request.title.as_deref(),
                request.memory_kind,
                request.context.as_ref(),
            )
            .await
        }
        .instrument(operation_span(
            "kernel",
            "remember_text",
            Some(&scope_id),
            Some("remember_text"),
        ))
        .await;

        match &result {
            Ok(payload) => record_write_success(
                payload.wrote_pg,
                payload.wrote_markdown,
                started_at.elapsed(),
            ),
            Err(_) => record_write_failure(started_at.elapsed()),
        }
        if let Some(context) = request.context.as_ref() {
            record_key_operation(context.storage_mode.as_str(), result.is_ok());
            if let Some(pg_store) = &self.pg_store {
                let error_code = match result.as_ref() {
                    Ok(_) => None,
                    Err(error) => Some(error.to_string()),
                };
                let _ = pg_store
                    .record_key_usage(
                        Some(&context.key_id),
                        context.source_kind,
                        "remember_text",
                        Some(&request.scope_id),
                        context.storage_mode,
                        result.is_ok(),
                        started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                        error_code.as_deref(),
                    )
                    .await;
            }
        }

        result
    }

    pub async fn remember_image(
        &self,
        request: RememberImageRequest,
    ) -> Result<RememberImageResult> {
        let started_at = Instant::now();
        let scope_id = request.scope_id.as_str().to_string();
        let request_context = request.context.clone();
        let result: Result<RememberImageResult> = async {
            let asset_store = self
                .asset_store
                .as_ref()
                .ok_or_else(|| anyhow!("asset store is not configured"))?;
            let RememberImageRequest {
                scope_id,
                title,
                body,
                bytes,
                media_type,
                file_extension,
                memory_kind,
                source_refs,
                visibility,
                sensitivity,
                context,
            } = request;
            self.ensure_key_can_access_scope(context.as_ref(), &scope_id)?;
            let image_profile = inspect_image(&bytes, &media_type).ok();
            let byte_size = bytes.len() as u64;

            let asset = asset_store.store_bytes(PutAssetRequest {
                bytes,
                media_type: media_type.clone(),
                storage_class: StorageClass::Raw,
                extension: file_extension,
                width: image_profile.as_ref().map(|profile| profile.width),
                height: image_profile.as_ref().map(|profile| profile.height),
                duration_ms: None,
                page_count: None,
                codec: None,
            })?;
            let vision = match &self.vision_gateway {
                Some(vision_gateway) => Some(
                    vision_gateway
                        .analyze(VisionRequest {
                            asset_uri: asset.reference.uri(),
                            media_type: media_type.clone(),
                            profile: image_profile.clone(),
                            byte_size: Some(byte_size),
                            prompt: body.clone(),
                            locale: String::new(),
                        })
                        .await?,
                ),
                None => None,
            };
            if let Some(notice) = vision
                .as_ref()
                .and_then(|response| response.switch_notice.as_ref())
            {
                tracing::warn!(
                    from_model = notice.from_model_alias.as_str(),
                    to_model = notice.to_model_alias.as_str(),
                    capability = notice.capability.as_str(),
                    reason = notice.reason.as_str(),
                    message = notice.message.as_str(),
                    "vision llm failover triggered"
                );
            }
            let title_override = title
                .filter(|value| !value.trim().is_empty())
                .or_else(|| vision.as_ref().map(|response| response.caption.clone()))
                .or_else(|| Some(format!("Image asset {}", &asset.reference.sha256[..12])));
            let artifact = build_image_artifact(
                ImageArtifactInput {
                    scope_id,
                    media_type,
                    body,
                    source_refs,
                    visibility,
                    sensitivity,
                    image_profile: image_profile.clone(),
                    vision: vision.clone(),
                },
                &asset,
            )?;
            let remembered = self
                .remember_artifact(
                    artifact,
                    title_override.as_deref(),
                    memory_kind,
                    context.as_ref(),
                )
                .await?;

            Ok(RememberImageResult {
                asset,
                artifact: remembered.artifact,
                memory: remembered.memory,
                vision,
                wrote_pg: remembered.wrote_pg,
                wrote_markdown: remembered.wrote_markdown,
            })
        }
        .instrument(operation_span(
            "kernel",
            "remember_image",
            Some(&scope_id),
            Some("remember_image"),
        ))
        .await;

        match &result {
            Ok(payload) => record_write_success(
                payload.wrote_pg,
                payload.wrote_markdown,
                started_at.elapsed(),
            ),
            Err(_) => record_write_failure(started_at.elapsed()),
        }
        if let Some(context) = request_context.as_ref() {
            record_key_operation(context.storage_mode.as_str(), result.is_ok());
            if let Some(pg_store) = &self.pg_store {
                let error_code = match result.as_ref() {
                    Ok(_) => None,
                    Err(error) => Some(error.to_string()),
                };
                let scope_id_ref = ScopeId::from_string(scope_id.clone());
                let _ = pg_store
                    .record_key_usage(
                        Some(&context.key_id),
                        context.source_kind,
                        "remember_image",
                        Some(&scope_id_ref),
                        context.storage_mode,
                        result.is_ok(),
                        started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                        error_code.as_deref(),
                    )
                    .await;
            }
        }

        result
    }

    async fn remember_artifact(
        &self,
        artifact: Artifact,
        title_override: Option<&str>,
        memory_kind: Option<MemoryKind>,
        context: Option<&RequestContext>,
    ) -> Result<RememberTextResult> {
        self.ensure_key_can_access_scope(context, &artifact.scope_id)?;
        self.evaluate_policy(artifact.visibility, artifact.sensitivity)?;

        let envelope = ExtractionEnvelope::new(
            artifact_kind_label(artifact.kind),
            artifact.content_text.clone(),
        );
        if !should_extract(&envelope) {
            bail!("artifact text is empty after normalization");
        }

        let mut memory = distill_candidate_memory(&artifact, title_override, memory_kind)?;
        memory.visibility = artifact.visibility;
        memory.sensitivity = artifact.sensitivity;
        memory.activate()?;

        let mut wrote_pg = false;
        let mut wrote_markdown = false;

        let write_pg = self.should_write_pg(context);
        let write_markdown = self.should_write_markdown(context);

        if write_pg {
            let pg_store = self
                .pg_store
                .as_ref()
                .ok_or_else(|| anyhow!("postgres store is required by storage mode"))?;
            self.seed_scope_if_needed(pg_store, &artifact.scope_id)
                .await?;
            let artifact_id = pg_store.insert_artifact(&artifact).await?;
            let memory_id = pg_store.insert_memory(&memory).await?;
            pg_store
                .link_evidence(&memory_id, &artifact_id, None)
                .await?;
            if let Some(context) = context {
                pg_store
                    .link_memory_key(&memory_id, &context.key_id, &context.isolation_group_id)
                    .await?;
            }
            if self.should_embed(context) {
                self.embed_memory(pg_store, &memory, context).await?;
            }
            wrote_pg = true;
        }

        memory.evidence_count = 1;
        if write_markdown {
            let markdown_store = self
                .markdown_store
                .as_ref()
                .ok_or_else(|| anyhow!("markdown store is required by storage mode"))?;
            markdown_store.write_memory_markdown(&memory)?;
            wrote_markdown = true;
        }

        info!(
            scope_id = artifact.scope_id.as_str(),
            artifact_id = artifact.id.as_str(),
            memory_id = memory.id.as_str(),
            wrote_pg,
            wrote_markdown,
            "remember_text completed"
        );

        Ok(RememberTextResult {
            artifact,
            memory,
            wrote_pg,
            wrote_markdown,
        })
    }

    pub async fn search_context(&self, request: SearchContextRequest) -> Result<ContextBundle> {
        let started_at = Instant::now();
        let scope_id = request.scope_id.as_str().to_string();
        let result: Result<ContextBundle> = async {
            let limit = request.limit.clamp(1, 50);
            self.ensure_key_can_access_scope(request.context.as_ref(), &request.scope_id)?;
            let normalized_query = normalize_query(&SearchQuery::new(&request.query, limit));
            let memories = match &self.pg_store {
                Some(pg_store) if self.should_query_pg(request.context.as_ref()) => {
                    let mut keyword_memories = if let Some(context) = request
                        .context
                        .as_ref()
                        .filter(|context| context.is_fully_isolated)
                    {
                        pg_store
                            .search_by_keyword_for_isolation_group(
                                &request.scope_id,
                                &normalized_query,
                                &context.isolation_group_id,
                                limit as i64,
                            )
                            .await?
                    } else {
                        pg_store
                            .search_by_keyword(&request.scope_id, &normalized_query, limit as i64)
                            .await?
                    };
                    if self.should_embed(request.context.as_ref()) {
                        let vector_memories = self
                            .search_embedding_memories(
                                pg_store,
                                &request.scope_id,
                                &normalized_query,
                                request.context.as_ref(),
                                limit,
                            )
                            .await?;
                        merge_memories(&mut keyword_memories, vector_memories, limit);
                    }
                    self.expand_graph_memories(
                        pg_store,
                        &request.scope_id,
                        &normalized_query,
                        request.context.as_ref(),
                        &mut keyword_memories,
                        limit,
                    )
                    .await?;
                    rerank_memories(&mut keyword_memories, &normalized_query);
                    keyword_memories.truncate(limit);
                    keyword_memories
                }
                None => {
                    let mut memories =
                        self.search_markdown_memories(&request.scope_id, &normalized_query, limit)?;
                    self.expand_markdown_graph_memories(
                        &request.scope_id,
                        &normalized_query,
                        &mut memories,
                        limit,
                    )?;
                    rerank_memories(&mut memories, &normalized_query);
                    memories.truncate(limit);
                    memories
                }
                _ => {
                    let mut memories =
                        self.search_markdown_memories(&request.scope_id, &normalized_query, limit)?;
                    self.expand_markdown_graph_memories(
                        &request.scope_id,
                        &normalized_query,
                        &mut memories,
                        limit,
                    )?;
                    rerank_memories(&mut memories, &normalized_query);
                    memories.truncate(limit);
                    memories
                }
            };
            let (entities, relations) = build_context_graph(&request.scope_id, &memories);

            Ok(ContextBundle {
                query: normalized_query,
                scope_id: request.scope_id.clone(),
                memories,
                entities,
                relations,
                generated_at: time::OffsetDateTime::now_utc(),
            })
        }
        .instrument(operation_span(
            "kernel",
            "search_context",
            Some(&scope_id),
            Some("search_context"),
        ))
        .await;

        match &result {
            Ok(bundle) => {
                record_search_success(bundle.memories.len(), started_at.elapsed());
                info!(
                    scope_id = bundle.scope_id.as_str(),
                    query = bundle.query,
                    memory_count = bundle.memories.len(),
                    entity_count = bundle.entities.len(),
                    relation_count = bundle.relations.len(),
                    latency_ms = started_at.elapsed().as_millis() as u64,
                    "search_context completed"
                );
            }
            Err(_) => record_search_failure(started_at.elapsed()),
        }
        if let Some(context) = request.context.as_ref() {
            record_key_operation(context.storage_mode.as_str(), result.is_ok());
            if let Some(pg_store) = &self.pg_store {
                let error_code = match result.as_ref() {
                    Ok(_) => None,
                    Err(error) => Some(error.to_string()),
                };
                let _ = pg_store
                    .record_key_usage(
                        Some(&context.key_id),
                        context.source_kind,
                        "search_context",
                        Some(&request.scope_id),
                        context.storage_mode,
                        result.is_ok(),
                        started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                        error_code.as_deref(),
                    )
                    .await;
            }
        }

        result
    }

    pub async fn publish_memory(
        &self,
        mut memory: Memory,
        target_visibility: Visibility,
    ) -> Result<PublishMemoryResult> {
        let started_at = Instant::now();
        let scope_id = memory.scope_id.as_str().to_string();
        let result = async {
            let previous_visibility = memory.visibility;
            if visibility_rank(target_visibility) < visibility_rank(previous_visibility) {
                bail!("publish target must be broader than current visibility");
            }

            memory.visibility = target_visibility;
            memory.updated_at = time::OffsetDateTime::now_utc();
            self.evaluate_policy(memory.visibility, memory.sensitivity)?;

            let mut wrote_pg = false;
            let mut wrote_markdown = false;
            if let Some(pg_store) = &self.pg_store {
                self.seed_scope_if_needed(pg_store, &memory.scope_id)
                    .await?;
                pg_store.upsert_memory(&memory).await?;
                wrote_pg = true;
            }
            if let Some(markdown_store) = &self.markdown_store {
                markdown_store.write_memory_markdown(&memory)?;
                wrote_markdown = true;
            }

            info!(
                scope_id = memory.scope_id.as_str(),
                memory_id = memory.id.as_str(),
                from_visibility = ?previous_visibility,
                to_visibility = ?target_visibility,
                wrote_pg,
                wrote_markdown,
                latency_ms = started_at.elapsed().as_millis() as u64,
                "publish_memory completed"
            );

            Ok(PublishMemoryResult {
                memory,
                wrote_pg,
                wrote_markdown,
            })
        }
        .instrument(operation_span(
            "kernel",
            "publish_memory",
            Some(&scope_id),
            Some("publish_memory"),
        ))
        .await;

        match &result {
            Ok(payload) => record_write_success(
                payload.wrote_pg,
                payload.wrote_markdown,
                started_at.elapsed(),
            ),
            Err(_) => record_write_failure(started_at.elapsed()),
        }

        result
    }

    pub async fn publish_memory_by_id(
        &self,
        scope_id: ScopeId,
        memory_id: MemoryId,
        target_visibility: Visibility,
    ) -> Result<PublishMemoryResult> {
        let Some(memory) = self.get_memory(scope_id, memory_id.clone()).await? else {
            bail!("memory not found: {}", memory_id.as_str());
        };

        self.publish_memory(memory, target_visibility).await
    }

    pub async fn promote_memory(
        &self,
        memory: Memory,
        request: PromoteMemoryRequest,
    ) -> Result<PublishMemoryResult> {
        let started_at = Instant::now();
        let source_scope_id = memory.scope_id.as_str().to_string();
        let result = async {
            let decision = evaluate_publish_policy(PublishPolicyInput {
                source_scope_type: request.source_scope_type,
                target_scope_type: request.target_scope_type,
                target_visibility: request.target_visibility,
                sensitivity: memory.sensitivity,
            });
            if matches!(decision, PolicyDecision::Deny) {
                bail!("publish denied by scope policy");
            }

            let mut promoted =
                memory.publish_into(request.target_scope_id.clone(), request.target_visibility);
            promoted.id = MemoryId::new();
            promoted.body = redact_for_shared_scope(&promoted.body, promoted.sensitivity);
            promoted.state = if matches!(decision, PolicyDecision::Review) {
                memory_domain::MemoryState::Candidate
            } else {
                memory_domain::MemoryState::Active
            };

            let scope = build_seed_scope(&request.target_scope_id, request.target_scope_type)?;
            let mut wrote_pg = false;
            let mut wrote_markdown = false;
            if let Some(pg_store) = &self.pg_store {
                pg_store.seed_scope_definition(&scope).await?;
                pg_store.insert_memory(&promoted).await?;
                wrote_pg = true;
            }
            if let Some(markdown_store) = &self.markdown_store {
                markdown_store.write_memory_markdown(&promoted)?;
                wrote_markdown = true;
            }

            info!(
                source_scope_id,
                target_scope_id = promoted.scope_id.as_str(),
                memory_id = promoted.id.as_str(),
                reviewed = matches!(decision, PolicyDecision::Review),
                wrote_pg,
                wrote_markdown,
                latency_ms = started_at.elapsed().as_millis() as u64,
                "promote_memory completed"
            );

            Ok(PublishMemoryResult {
                memory: promoted,
                wrote_pg,
                wrote_markdown,
            })
        }
        .instrument(operation_span(
            "kernel",
            "promote_memory",
            Some(&source_scope_id),
            Some("promote_memory"),
        ))
        .await;

        match &result {
            Ok(payload) => record_write_success(
                payload.wrote_pg,
                payload.wrote_markdown,
                started_at.elapsed(),
            ),
            Err(_) => record_write_failure(started_at.elapsed()),
        }

        result
    }

    pub async fn promote_memory_by_id(
        &self,
        scope_id: ScopeId,
        memory_id: MemoryId,
        request: PromoteMemoryRequest,
    ) -> Result<PublishMemoryResult> {
        let Some(memory) = self.get_memory(scope_id, memory_id.clone()).await? else {
            bail!("memory not found: {}", memory_id.as_str());
        };

        self.promote_memory(memory, request).await
    }

    fn build_artifact(&self, request: &RememberTextRequest) -> Result<Artifact> {
        let mut artifact = Artifact::new(
            request.scope_id.clone(),
            request.artifact_kind,
            request.body.clone(),
            request.source_refs.clone(),
        )?;
        artifact.language_code = detect_language_code(&artifact.content_text);
        artifact.visibility = request.visibility;
        artifact.sensitivity = request.sensitivity;
        Ok(artifact)
    }

    fn evaluate_policy(&self, visibility: Visibility, sensitivity: Sensitivity) -> Result<()> {
        match evaluate_write_policy(WritePolicyInput {
            visibility,
            sensitivity,
        }) {
            PolicyDecision::Allow => Ok(()),
            PolicyDecision::Review => {
                warn!(
                    ?visibility,
                    ?sensitivity,
                    "write requires review, continuing in local mode"
                );
                Ok(())
            }
            PolicyDecision::Deny => Err(anyhow!("write denied by policy")),
        }
    }

    fn should_write_pg(&self, context: Option<&RequestContext>) -> bool {
        match context.map(|context| context.storage_mode) {
            Some(StorageMode::File) => false,
            Some(StorageMode::Vector | StorageMode::All) | None => self.pg_store.is_some(),
        }
    }

    fn should_write_markdown(&self, context: Option<&RequestContext>) -> bool {
        match context.map(|context| context.storage_mode) {
            Some(StorageMode::Vector) => false,
            Some(StorageMode::File | StorageMode::All) | None => self.markdown_store.is_some(),
        }
    }

    fn should_query_pg(&self, context: Option<&RequestContext>) -> bool {
        !matches!(
            context.map(|context| context.storage_mode),
            Some(StorageMode::File)
        )
    }

    fn should_embed(&self, context: Option<&RequestContext>) -> bool {
        self.embedding_gateway.is_some()
            && self.pg_store.is_some()
            && !matches!(
                context.map(|context| context.storage_mode),
                Some(StorageMode::File)
            )
    }

    async fn embed_memory(
        &self,
        pg_store: &PgStore,
        memory: &Memory,
        context: Option<&RequestContext>,
    ) -> Result<()> {
        let Some(embedding_gateway) = &self.embedding_gateway else {
            return Ok(());
        };
        let response = embedding_gateway
            .embed(EmbeddingRequest {
                inputs: vec![format!("{}\n{}", memory.title, memory.body)],
            })
            .await?;
        let Some(vector) = response.vectors.first() else {
            return Ok(());
        };
        pg_store
            .upsert_memory_embedding(
                &memory.id,
                context.map(|context| &context.key_id),
                context
                    .map(|context| context.isolation_group_id.as_str())
                    .unwrap_or("default"),
                &response.model_alias,
                vector,
            )
            .await
    }

    async fn search_embedding_memories(
        &self,
        pg_store: &PgStore,
        scope_id: &ScopeId,
        query: &str,
        context: Option<&RequestContext>,
        limit: usize,
    ) -> Result<Vec<Memory>> {
        let Some(embedding_gateway) = &self.embedding_gateway else {
            return Ok(Vec::new());
        };
        let response = embedding_gateway
            .embed(EmbeddingRequest {
                inputs: vec![query.to_string()],
            })
            .await?;
        let Some(vector) = response.vectors.first() else {
            return Ok(Vec::new());
        };
        let isolation_group_id = context
            .filter(|context| context.is_fully_isolated)
            .map(|context| context.isolation_group_id.as_str());
        pg_store
            .search_by_embedding(scope_id, vector, isolation_group_id, limit as i64)
            .await
    }

    fn search_markdown_memories(
        &self,
        scope_id: &ScopeId,
        normalized_query: &str,
        limit: usize,
    ) -> Result<Vec<Memory>> {
        let Some(markdown_store) = &self.markdown_store else {
            return Ok(Vec::new());
        };
        let terms = normalized_query
            .split_whitespace()
            .filter(|term| !term.is_empty())
            .collect::<Vec<_>>();
        let mut memories = markdown_store
            .list_memories_by_scope(scope_id)?
            .into_iter()
            .filter(|memory| {
                let haystack = format!("{}\n{}", memory.title, memory.body).to_lowercase();
                if terms.is_empty() {
                    haystack.contains(normalized_query)
                } else {
                    terms.iter().all(|term| haystack.contains(term))
                }
            })
            .collect::<Vec<_>>();
        memories.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        memories.truncate(limit);
        Ok(memories)
    }

    async fn expand_graph_memories(
        &self,
        pg_store: &PgStore,
        scope_id: &ScopeId,
        normalized_query: &str,
        context: Option<&RequestContext>,
        memories: &mut Vec<Memory>,
        limit: usize,
    ) -> Result<()> {
        let expansion_terms = graph_expansion_terms(scope_id, memories, normalized_query);
        for term in expansion_terms {
            if memories.len() >= limit {
                break;
            }
            let expansion_results =
                if let Some(context) = context.filter(|context| context.is_fully_isolated) {
                    pg_store
                        .search_by_keyword_for_isolation_group(
                            scope_id,
                            &term,
                            &context.isolation_group_id,
                            limit as i64,
                        )
                        .await?
                } else {
                    pg_store
                        .search_by_keyword(scope_id, &term, limit as i64)
                        .await?
                };
            merge_memories(memories, expansion_results, limit);
        }
        Ok(())
    }

    fn expand_markdown_graph_memories(
        &self,
        scope_id: &ScopeId,
        normalized_query: &str,
        memories: &mut Vec<Memory>,
        limit: usize,
    ) -> Result<()> {
        let expansion_terms = graph_expansion_terms(scope_id, memories, normalized_query);
        for term in expansion_terms {
            if memories.len() >= limit {
                break;
            }
            let expansion_results = self.search_markdown_memories(scope_id, &term, limit)?;
            merge_memories(memories, expansion_results, limit);
        }
        Ok(())
    }

    fn ensure_key_can_access_scope(
        &self,
        context: Option<&RequestContext>,
        scope_id: &ScopeId,
    ) -> Result<()> {
        let Some(context) = context else {
            return Ok(());
        };

        if matches!(context.scope_kind, memory_domain::KeyScopeKind::Team)
            && looks_like_personal_scope(scope_id)
        {
            bail!("team key cannot access personal memory scope");
        }

        Ok(())
    }

    async fn seed_scope_if_needed(&self, pg_store: &PgStore, scope_id: &ScopeId) -> Result<()> {
        let path = match &self.markdown_store {
            Some(markdown_store) => {
                format!("{}/scopes/{}", markdown_store.tenant(), scope_id.as_str())
            }
            None => format!("default/scopes/{}", scope_id.as_str()),
        };
        let scope = Scope::new_with_id(
            scope_id.clone(),
            ScopeType::Project,
            scope_id.as_str(),
            path,
            None,
        )?;

        pg_store.seed_scope_definition(&scope).await?;
        Ok(())
    }
}

#[async_trait]
impl MemoryService for Kernel {
    async fn remember(&self, artifact: Artifact) -> Result<Memory> {
        Ok(self
            .remember_artifact(artifact, None, None, None)
            .await?
            .memory)
    }

    async fn fetch_context(&self, query: &str, scope_id: ScopeId) -> Result<ContextBundle> {
        self.search_context(SearchContextRequest::new(scope_id, query))
            .await
    }
}

fn artifact_kind_label(kind: ArtifactKind) -> &'static str {
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

fn build_image_artifact(mut input: ImageArtifactInput, asset: &StoredAsset) -> Result<Artifact> {
    input.source_refs.push(asset.reference.uri());

    let body = input
        .body
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let mut lines = vec![
        format!("image asset: {}", asset.reference.uri()),
        format!("media_type: {}", input.media_type),
        format!("asset_sha256: {}", asset.reference.sha256),
    ];
    if let Some(profile) = input.image_profile.as_ref() {
        lines.push(format!(
            "image_dimensions: {}x{}",
            profile.width, profile.height
        ));
        lines.push(format!("image_color_mode: {}", profile.color_mode));
        lines.push(format!("image_average_luma: {}", profile.average_luma));
    }
    if let Some(vision) = input.vision.as_ref() {
        lines.push(format!("vision_model: {}", vision.model_alias));
        lines.push(format!("vision_caption: {}", vision.caption));
    }
    if let Some(note) = body {
        lines.push(format!("user_note: {}", note));
    } else {
        lines.push("image recorded for future memory retrieval.".to_string());
    }
    let content_text = lines.join("\n");

    let mut artifact = Artifact::new(
        input.scope_id,
        ArtifactKind::Image,
        content_text,
        input.source_refs,
    )?;
    artifact.mime_type = Some(input.media_type);
    artifact.language_code = detect_language_code(&artifact.content_text);
    artifact.visibility = input.visibility;
    artifact.sensitivity = input.sensitivity;
    Ok(artifact)
}

fn build_seed_scope(scope_id: &ScopeId, scope_type: ScopeType) -> Result<Scope> {
    Scope::new_with_id(
        scope_id.clone(),
        scope_type,
        scope_id.as_str(),
        format!("default/scopes/{}", scope_id.as_str()),
        None,
    )
    .map_err(Into::into)
}

fn build_context_graph(scope_id: &ScopeId, memories: &[Memory]) -> (Vec<Entity>, Vec<Relation>) {
    let mut entity_map = std::collections::HashMap::<String, Entity>::new();
    let mut relation_map = std::collections::HashMap::<String, Relation>::new();

    for memory in memories {
        let source_text = format!("{}\n{}", memory.title, memory.body);
        let entity_candidates = extract_entities(scope_id, &source_text);
        for candidate in &entity_candidates {
            entity_map
                .entry(candidate.entity.normalized_key.clone())
                .or_insert_with(|| candidate.entity.clone());
        }

        for candidate in extract_relations(scope_id, &source_text, &entity_candidates) {
            let relation_type = relation_type_label(candidate.relation.relation_type);
            let key = format!(
                "{}:{}:{}",
                relation_type,
                candidate.relation.subject_entity_id.as_str(),
                candidate.relation.object_entity_id.as_str()
            );
            relation_map
                .entry(key)
                .or_insert_with(|| candidate.relation.clone());
        }
    }

    (
        entity_map.into_values().collect::<Vec<_>>(),
        relation_map.into_values().collect::<Vec<_>>(),
    )
}

fn relation_type_label(relation_type: memory_domain::RelationType) -> &'static str {
    match relation_type {
        memory_domain::RelationType::MemberOf => "member_of",
        memory_domain::RelationType::BelongsTo => "belongs_to",
        memory_domain::RelationType::Owns => "owns",
        memory_domain::RelationType::DependsOn => "depends_on",
        memory_domain::RelationType::Uses => "uses",
        memory_domain::RelationType::Implements => "implements",
        memory_domain::RelationType::References => "references",
        memory_domain::RelationType::DerivedFrom => "derived_from",
        memory_domain::RelationType::Documents => "documents",
    }
}

fn visibility_rank(visibility: Visibility) -> usize {
    match visibility {
        Visibility::Private => 0,
        Visibility::Project => 1,
        Visibility::Team => 2,
        Visibility::Organization => 3,
    }
}

fn looks_like_personal_scope(scope_id: &ScopeId) -> bool {
    let scope = scope_id.as_str().to_ascii_lowercase();
    scope.contains("_user_")
        || scope.starts_with("scp_user")
        || scope.contains("/user/")
        || scope.contains("personal")
}

fn merge_memories(memories: &mut Vec<Memory>, candidates: Vec<Memory>, limit: usize) {
    let mut seen = memories
        .iter()
        .map(|memory| memory.id.as_str().to_string())
        .collect::<std::collections::HashSet<_>>();
    for candidate in candidates {
        if seen.insert(candidate.id.as_str().to_string()) {
            memories.push(candidate);
        }
        if memories.len() >= limit {
            break;
        }
    }
}

fn graph_expansion_terms(
    scope_id: &ScopeId,
    memories: &[Memory],
    normalized_query: &str,
) -> Vec<String> {
    let query_terms = normalized_query
        .split_whitespace()
        .filter(|term| !term.is_empty())
        .map(|term| term.to_ascii_lowercase())
        .collect::<std::collections::HashSet<_>>();
    let (entities, _) = build_context_graph(scope_id, memories);
    let mut terms = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();

    for entity in entities {
        let label = entity.canonical_name.trim().to_ascii_lowercase();
        if label.len() < 4 || query_terms.contains(&label) {
            continue;
        }
        if seen.insert(label.clone()) {
            terms.push(label);
        }
        if terms.len() >= 4 {
            break;
        }
    }

    terms
}

fn rerank_memories(memories: &mut [Memory], normalized_query: &str) {
    let query_terms = normalized_query
        .split_whitespace()
        .filter(|term| !term.is_empty())
        .map(|term| term.to_ascii_lowercase())
        .collect::<Vec<_>>();
    memories.sort_by(|left, right| {
        let right_score = memory_rank_score(right, &query_terms);
        let left_score = memory_rank_score(left, &query_terms);
        right_score
            .cmp(&left_score)
            .then_with(|| right.updated_at.cmp(&left.updated_at))
    });
}

fn memory_rank_score(memory: &Memory, query_terms: &[String]) -> i64 {
    let title = memory.title.to_ascii_lowercase();
    let body = memory.body.to_ascii_lowercase();
    let mut score = 0_i64;
    for term in query_terms {
        if title.contains(term) {
            score += 6;
        }
        if body.contains(term) {
            score += 3;
        }
    }
    score += i64::from(memory.evidence_count.min(8) as i32);
    score
}

#[cfg(test)]
mod tests {
    use super::{
        ImageArtifactInput, Kernel, RememberImageRequest, RememberTextRequest,
        SearchContextRequest, artifact_kind_label, build_context_graph, build_image_artifact,
        graph_expansion_terms, relation_type_label, rerank_memories, visibility_rank,
    };
    use memory_assets::{AssetMetadata, AssetRef, StorageClass, StoredAsset};
    use memory_core::MemoryService;
    use memory_domain::{
        AccessKeyId, Artifact, KeyScopeKind, KeySourceKind, MemoryId, RequestContext, Sensitivity,
        StorageMode, Visibility,
    };
    use memory_domain::{ArtifactKind, Memory, MemoryKind, RelationType, ScopeId};
    use memory_models::{
        CapabilityRoute, DeploymentTarget, ImageProfile, ModelCapability, ModelDescriptor,
        ModelRegistry, Provider, ProviderDescriptor, VisionResponse,
    };
    use serde_json::json;
    use std::collections::BTreeSet;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn sample_stored_asset() -> StoredAsset {
        let reference = AssetRef::new(
            "asset_kernel",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "image/png",
            StorageClass::Raw,
            "raw/sha256/01/23/sample.png",
        )
        .unwrap();

        StoredAsset {
            reference,
            metadata: AssetMetadata {
                asset_id: "asset_kernel".to_string(),
                sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .to_string(),
                size_bytes: 128,
                mime_type: "image/png".to_string(),
                storage_class: StorageClass::Raw,
                width: Some(640),
                height: Some(480),
                duration_ms: None,
                page_count: None,
                codec: None,
            },
            absolute_path: PathBuf::from("/tmp/raw/sha256/01/23/sample.png"),
        }
    }

    #[tokio::test]
    async fn remember_text_writes_markdown_projection() {
        let tempdir = tempdir().unwrap();
        let kernel = Kernel::builder()
            .with_markdown_root(tempdir.path())
            .unwrap()
            .build()
            .unwrap();
        let scope_id = ScopeId::from_string("scp_kernel_md");

        let mut request = RememberTextRequest::new(scope_id.clone(), "The default branch is main.");
        request.title = Some("Default branch".to_string());

        let result = kernel.remember_text(request).await.unwrap();
        let projection_path = tempdir
            .path()
            .join("default")
            .join("scopes")
            .join(scope_id.as_str())
            .join("MEMORY.md");

        assert!(result.wrote_markdown);
        assert_eq!(result.memory.evidence_count, 1);
        assert!(projection_path.exists());
    }

    #[tokio::test]
    async fn kernel_builder_and_service_trait_cover_local_defaults() {
        let tempdir = tempdir().unwrap();
        let scope_id = ScopeId::from_string("scp_kernel_trait");

        assert!(super::KernelBuilder::default().build().is_err());

        let kernel = Kernel::builder()
            .with_markdown_tenant(tempdir.path(), "tenant_x")
            .unwrap()
            .build()
            .unwrap();

        assert!(!kernel.has_postgres());
        assert!(kernel.has_markdown());
        assert!(!kernel.has_asset_store());
        assert!(!kernel.has_vision_gateway());
        assert!(
            kernel
                .get_memory(scope_id.clone(), MemoryId::from_string("mem_missing"))
                .await
                .unwrap()
                .is_none()
        );

        let remembered = <Kernel as MemoryService>::remember(
            &kernel,
            Artifact::new(
                scope_id.clone(),
                ArtifactKind::Document,
                "Decision log for Meat Memory",
                vec![],
            )
            .unwrap(),
        )
        .await
        .unwrap();
        let bundle =
            <Kernel as MemoryService>::fetch_context(&kernel, "Decision", scope_id.clone())
                .await
                .unwrap();
        let publish_error = kernel
            .publish_memory_by_id(
                scope_id.clone(),
                MemoryId::from_string("mem_missing"),
                Visibility::Team,
            )
            .await
            .expect_err("missing memory should fail");

        assert_eq!(remembered.evidence_count, 1);
        assert_eq!(bundle.query, "decision");
        assert_eq!(bundle.memories.len(), 1);
        assert!(publish_error.to_string().contains("memory not found"));
    }

    #[tokio::test]
    async fn search_without_postgres_returns_empty_bundle() {
        let tempdir = tempdir().unwrap();
        let kernel = Kernel::builder()
            .with_markdown_root(tempdir.path())
            .unwrap()
            .build()
            .unwrap();

        let bundle = kernel
            .search_context(SearchContextRequest::new(
                ScopeId::from_string("scp_kernel_search"),
                "postgres",
            ))
            .await
            .unwrap();

        assert!(bundle.memories.is_empty());
        assert_eq!(bundle.query, "postgres");
    }

    #[tokio::test]
    async fn file_storage_mode_writes_and_searches_markdown_only() {
        let tempdir = tempdir().unwrap();
        let kernel = Kernel::builder()
            .with_markdown_root(tempdir.path())
            .unwrap()
            .build()
            .unwrap();
        let scope_id = ScopeId::from_string("scp_user_file_mode");
        let context = RequestContext {
            key_id: AccessKeyId::from_string("key_file"),
            source_kind: KeySourceKind::Cli,
            principal_id: "alice".to_string(),
            owner_scope_id: scope_id.clone(),
            scope_kind: KeyScopeKind::Personal,
            storage_mode: StorageMode::File,
            is_fully_isolated: false,
            isolation_group_id: "personal:alice".to_string(),
        };
        let mut request = RememberTextRequest::new(
            scope_id.clone(),
            "File mode memory remains searchable in markdown.",
        );
        request.title = Some("File mode".to_string());
        request.context = Some(context.clone());

        let remembered = kernel.remember_text(request).await.unwrap();
        assert!(!remembered.wrote_pg);
        assert!(remembered.wrote_markdown);

        let mut search = SearchContextRequest::new(scope_id, "markdown");
        search.context = Some(context);
        let bundle = kernel.search_context(search).await.unwrap();
        assert_eq!(bundle.memories.len(), 1);
    }

    #[tokio::test]
    async fn team_key_cannot_access_personal_scope() {
        let tempdir = tempdir().unwrap();
        let kernel = Kernel::builder()
            .with_markdown_root(tempdir.path())
            .unwrap()
            .build()
            .unwrap();
        let mut request = RememberTextRequest::new(
            ScopeId::from_string("scp_user_private_alice"),
            "personal note",
        );
        request.context = Some(RequestContext {
            key_id: AccessKeyId::from_string("key_team"),
            source_kind: KeySourceKind::Mcp,
            principal_id: "team-bot".to_string(),
            owner_scope_id: ScopeId::from_string("scp_team_platform"),
            scope_kind: KeyScopeKind::Team,
            storage_mode: StorageMode::All,
            is_fully_isolated: false,
            isolation_group_id: "team:scp_team_platform".to_string(),
        });

        let error = kernel
            .remember_text(request)
            .await
            .expect_err("team key should not write user scope");
        assert!(
            error
                .to_string()
                .contains("team key cannot access personal")
        );
    }

    #[tokio::test]
    async fn remember_and_publish_cover_error_and_policy_paths() {
        let tempdir = tempdir().unwrap();
        let kernel = Kernel::builder()
            .with_markdown_root(tempdir.path())
            .unwrap()
            .build()
            .unwrap();
        let scope_id = ScopeId::from_string("scp_kernel_policy");

        let remember_error = kernel
            .remember_text(RememberTextRequest::new(scope_id.clone(), "   "))
            .await
            .expect_err("blank body should fail");
        assert!(remember_error.to_string().contains("artifact.content_text"));

        let mut review_memory = Memory::new(
            scope_id.clone(),
            MemoryKind::Decision,
            "Review gate",
            "restricted but team-visible",
        )
        .unwrap();
        review_memory.activate().unwrap();
        review_memory.sensitivity = Sensitivity::Restricted;
        let reviewed = kernel
            .publish_memory(review_memory, Visibility::Team)
            .await
            .unwrap();
        assert_eq!(reviewed.memory.visibility, Visibility::Team);

        let mut denied_memory = Memory::new(
            scope_id.clone(),
            MemoryKind::Decision,
            "Denied gate",
            "restricted and org-visible",
        )
        .unwrap();
        denied_memory.activate().unwrap();
        denied_memory.sensitivity = Sensitivity::Restricted;
        let denied = kernel
            .publish_memory(denied_memory, Visibility::Organization)
            .await
            .expect_err("org+restricted should be denied");
        assert!(denied.to_string().contains("write denied by policy"));

        let mut narrower_memory = Memory::new(
            scope_id,
            MemoryKind::Decision,
            "Narrow publish",
            "cannot narrow visibility",
        )
        .unwrap();
        narrower_memory.activate().unwrap();
        narrower_memory.visibility = Visibility::Team;
        let narrower = kernel
            .publish_memory(narrower_memory, Visibility::Project)
            .await
            .expect_err("narrower publish should fail");
        assert!(
            narrower
                .to_string()
                .contains("broader than current visibility")
        );
    }

    #[tokio::test]
    async fn publish_memory_updates_markdown_projection() {
        let tempdir = tempdir().unwrap();
        let kernel = Kernel::builder()
            .with_markdown_root(tempdir.path())
            .unwrap()
            .build()
            .unwrap();
        let scope_id = ScopeId::from_string("scp_kernel_publish");

        let result = kernel
            .remember_text(RememberTextRequest::new(
                scope_id.clone(),
                "Project Meat Memory uses Service Gateway.",
            ))
            .await
            .unwrap();

        let published = kernel
            .publish_memory(result.memory, memory_domain::Visibility::Team)
            .await
            .unwrap();
        let projection_path = tempdir
            .path()
            .join("default")
            .join("scopes")
            .join(scope_id.as_str())
            .join("MEMORY.md");
        let raw = std::fs::read_to_string(projection_path).unwrap();

        assert!(published.wrote_markdown);
        assert!(raw.contains("visibility: team"));
    }

    #[tokio::test]
    async fn remember_image_covers_missing_asset_store_and_no_vision_fallback() {
        let tempdir = tempdir().unwrap();
        let scope_id = ScopeId::from_string("scp_kernel_image_fallback");

        let markdown_only = Kernel::builder()
            .with_markdown_root(tempdir.path().join("markdown-only"))
            .unwrap()
            .build()
            .unwrap();
        let missing_asset = markdown_only
            .remember_image(RememberImageRequest::new(
                scope_id.clone(),
                "image/png",
                vec![1, 2, 3],
            ))
            .await
            .expect_err("missing asset store should fail");
        assert!(
            missing_asset
                .to_string()
                .contains("asset store is not configured")
        );

        let kernel = Kernel::builder()
            .with_markdown_root(tempdir.path().join("markdown"))
            .unwrap()
            .with_asset_root(tempdir.path().join("assets"))
            .unwrap()
            .build()
            .unwrap();
        let result = kernel
            .remember_image(RememberImageRequest::new(
                scope_id.clone(),
                "image/png",
                vec![1, 2, 3],
            ))
            .await
            .unwrap();

        assert!(result.vision.is_none());
        assert!(result.memory.title.starts_with("Image asset "));
        assert!(
            result
                .memory
                .body
                .contains("image recorded for future memory retrieval.")
        );
        assert!(!result.memory.body.contains("vision_caption:"));
    }

    #[tokio::test]
    async fn remember_image_stores_asset_and_projection() {
        let tempdir = tempdir().unwrap();
        let kernel = Kernel::builder()
            .with_markdown_root(tempdir.path().join("markdown"))
            .unwrap()
            .with_asset_root(tempdir.path().join("assets"))
            .unwrap()
            .with_model_registry(test_model_registry(), "zh-CN")
            .unwrap()
            .build()
            .unwrap();
        let scope_id = ScopeId::from_string("scp_kernel_image");
        let mut request = RememberImageRequest::new(
            scope_id.clone(),
            "image/png",
            vec![137, 80, 78, 71, 13, 10, 26, 10],
        );
        request.body = Some("UI reference screenshot".to_string());

        let result = kernel.remember_image(request).await.unwrap();
        let projection_path = tempdir
            .path()
            .join("markdown")
            .join("default")
            .join("scopes")
            .join(scope_id.as_str())
            .join("MEMORY.md");

        assert!(result.asset.absolute_path.exists());
        assert_eq!(result.artifact.kind, ArtifactKind::Image);
        assert_eq!(result.artifact.mime_type.as_deref(), Some("image/png"));
        assert!(result.vision.is_some());
        assert!(result.memory.body.contains("image asset:"));
        assert!(result.memory.body.contains("vision_caption:"));
        assert!(projection_path.exists());
    }

    #[tokio::test]
    async fn memory_service_rejects_empty_artifact_after_normalization() {
        let tempdir = tempdir().unwrap();
        let kernel = Kernel::builder()
            .with_markdown_root(tempdir.path())
            .unwrap()
            .build()
            .unwrap();
        let scope_id = ScopeId::from_string("scp_kernel_empty_artifact");
        let mut artifact =
            Artifact::new(scope_id, ArtifactKind::Message, "placeholder", vec![]).unwrap();
        artifact.content_text = "   ".to_string();

        let error = <Kernel as MemoryService>::remember(&kernel, artifact)
            .await
            .expect_err("normalized empty artifact should fail");
        assert!(
            error
                .to_string()
                .contains("artifact text is empty after normalization")
        );
    }

    #[tokio::test]
    async fn build_context_graph_extracts_entities_and_relations_from_memories() {
        let scope_id = ScopeId::from_string("scp_kernel_graph");
        let mut memory = Memory::new(
            scope_id.clone(),
            MemoryKind::Decision,
            "Gateway relation",
            "Project Meat Memory uses Service Gateway for context routing.",
        )
        .unwrap();
        memory.activate().unwrap();

        let (entities, relations) = build_context_graph(&scope_id, &[memory]);

        assert!(entities.len() >= 2);
        assert!(
            entities
                .iter()
                .any(|entity| entity.normalized_key == "projectmeatmemory")
        );
        assert!(
            entities
                .iter()
                .any(|entity| entity.normalized_key == "servicegateway")
        );
        assert!(
            relations
                .iter()
                .any(|relation| relation.relation_type == RelationType::Uses)
        );

        let empty = build_context_graph(&scope_id, &[]);
        assert!(empty.0.is_empty());
        assert!(empty.1.is_empty());
    }

    #[test]
    fn graph_expansion_terms_and_rerank_cover_related_memories() {
        let scope_id = ScopeId::from_string("scp_kernel_graph_rank");
        let mut seed = Memory::new(
            scope_id.clone(),
            MemoryKind::Decision,
            "Gateway relation",
            "Project Meat Memory uses Service Gateway for context routing.",
        )
        .unwrap();
        seed.activate().unwrap();
        let mut neighbor = Memory::new(
            scope_id.clone(),
            MemoryKind::Procedure,
            "Gateway deployment",
            "Service Gateway deployment playbook for release windows.",
        )
        .unwrap();
        neighbor.activate().unwrap();
        let terms = graph_expansion_terms(
            &scope_id,
            &[seed.clone(), neighbor.clone()],
            "project meat memory",
        );
        assert!(!terms.is_empty());
        assert!(terms.iter().all(|term| !term.trim().is_empty()));

        let mut memories = vec![neighbor, seed];
        rerank_memories(&mut memories, "project meat memory");
        assert_eq!(memories[0].title, "Gateway relation");
    }

    #[test]
    fn helper_functions_cover_labels_ranks_and_image_artifact_branches() {
        let asset = sample_stored_asset();
        let scope_id = ScopeId::from_string("scp_kernel_helpers");

        for (kind, label) in [
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
        ] {
            assert_eq!(artifact_kind_label(kind), label);
        }

        for (relation_type, label) in [
            (RelationType::MemberOf, "member_of"),
            (RelationType::BelongsTo, "belongs_to"),
            (RelationType::Owns, "owns"),
            (RelationType::DependsOn, "depends_on"),
            (RelationType::Uses, "uses"),
            (RelationType::Implements, "implements"),
            (RelationType::References, "references"),
            (RelationType::DerivedFrom, "derived_from"),
            (RelationType::Documents, "documents"),
        ] {
            assert_eq!(relation_type_label(relation_type), label);
        }

        assert_eq!(visibility_rank(Visibility::Private), 0);
        assert_eq!(visibility_rank(Visibility::Project), 1);
        assert_eq!(visibility_rank(Visibility::Team), 2);
        assert_eq!(visibility_rank(Visibility::Organization), 3);

        let rich_artifact = build_image_artifact(
            ImageArtifactInput {
                scope_id: scope_id.clone(),
                media_type: "image/png".to_string(),
                body: Some("  user supplied note  ".to_string()),
                source_refs: vec!["image://1".to_string()],
                visibility: Visibility::Project,
                sensitivity: Sensitivity::Private,
                image_profile: Some(ImageProfile {
                    width: 640,
                    height: 480,
                    has_alpha: true,
                    average_luma: 127,
                    color_mode: "rgba".to_string(),
                }),
                vision: Some(VisionResponse {
                    caption: "登录页截图".to_string(),
                    structured: json!({"scene": "login"}),
                    model_alias: "mock-vision".to_string(),
                    switch_notice: None,
                }),
            },
            &asset,
        )
        .unwrap();
        let minimal_artifact = build_image_artifact(
            ImageArtifactInput {
                scope_id,
                media_type: "image/png".to_string(),
                body: None,
                source_refs: Vec::new(),
                visibility: Visibility::Private,
                sensitivity: Sensitivity::Internal,
                image_profile: None,
                vision: None,
            },
            &asset,
        )
        .unwrap();

        assert!(
            rich_artifact
                .content_text
                .contains("image_dimensions: 640x480")
        );
        assert!(
            rich_artifact
                .content_text
                .contains("image_color_mode: rgba")
        );
        assert!(
            rich_artifact
                .content_text
                .contains("vision_model: mock-vision")
        );
        assert!(
            rich_artifact
                .content_text
                .contains("user_note: user supplied note")
        );
        assert_eq!(rich_artifact.mime_type.as_deref(), Some("image/png"));
        assert!(
            minimal_artifact
                .content_text
                .contains("image recorded for future memory retrieval.")
        );
        assert!(
            minimal_artifact
                .source_refs
                .iter()
                .any(|source| source.starts_with("asset://raw/sha256/"))
        );
    }

    fn test_model_registry() -> ModelRegistry {
        ModelRegistry::build(
            vec![ProviderDescriptor {
                provider: Provider::Gemini,
                display_name: "Gemini".to_string(),
                base_url: Some("https://generativelanguage.googleapis.com".to_string()),
                api_key_env: Some("GEMINI_API_KEY".to_string()),
                enabled: true,
            }],
            vec![
                ModelDescriptor {
                    alias: "gemini_reasoning".to_string(),
                    provider: Provider::Gemini,
                    remote_model_id: "gemini-2.5-flash".to_string(),
                    display_name: "Gemini Reasoning".to_string(),
                    capabilities: BTreeSet::from([
                        ModelCapability::Reasoning,
                        ModelCapability::Extraction,
                    ]),
                    deployment: DeploymentTarget::Cloud,
                    locale: "zh-CN".to_string(),
                    priority: 100,
                    enabled: true,
                },
                ModelDescriptor {
                    alias: "gemini_vision".to_string(),
                    provider: Provider::Gemini,
                    remote_model_id: "gemini-2.5-flash".to_string(),
                    display_name: "Gemini Vision".to_string(),
                    capabilities: BTreeSet::from([ModelCapability::Vision]),
                    deployment: DeploymentTarget::Cloud,
                    locale: "zh-CN".to_string(),
                    priority: 100,
                    enabled: true,
                },
                ModelDescriptor {
                    alias: "gemini_embedding".to_string(),
                    provider: Provider::Gemini,
                    remote_model_id: "text-embedding-004".to_string(),
                    display_name: "Gemini Embedding".to_string(),
                    capabilities: BTreeSet::from([ModelCapability::Embedding]),
                    deployment: DeploymentTarget::Cloud,
                    locale: "zh-CN".to_string(),
                    priority: 100,
                    enabled: true,
                },
            ],
            [
                (
                    ModelCapability::Reasoning,
                    CapabilityRoute {
                        primary: "gemini_reasoning".to_string(),
                        fallbacks: Vec::new(),
                    },
                ),
                (
                    ModelCapability::Extraction,
                    CapabilityRoute {
                        primary: "gemini_reasoning".to_string(),
                        fallbacks: Vec::new(),
                    },
                ),
                (
                    ModelCapability::Vision,
                    CapabilityRoute {
                        primary: "gemini_vision".to_string(),
                        fallbacks: Vec::new(),
                    },
                ),
                (
                    ModelCapability::Embedding,
                    CapabilityRoute {
                        primary: "gemini_embedding".to_string(),
                        fallbacks: Vec::new(),
                    },
                ),
            ],
        )
        .expect("test registry should build")
    }
}
