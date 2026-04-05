use anyhow::{Result, anyhow, bail};
use async_trait::async_trait;
use memory_assets::{FileSystemAssetStore, PutAssetRequest, StorageClass, StoredAsset};
use memory_core::MemoryService;
use memory_domain::{
    Artifact, ArtifactKind, ContextBundle, Entity, Memory, MemoryId, MemoryKind, Relation, ScopeId,
    Sensitivity, Visibility,
};
use memory_extract::{
    ExtractionEnvelope, distill_candidate_memory, extract_entities, extract_relations,
    should_extract,
};
use memory_index::{SearchQuery, normalize_query};
use memory_observability::{
    operation_span, record_search_failure, record_search_success, record_write_failure,
    record_write_success,
};
use memory_policy::{PolicyDecision, WritePolicyInput, evaluate_write_policy};
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
        }
    }
}

#[derive(Debug, Clone)]
pub struct RememberImageResult {
    pub asset: StoredAsset,
    pub artifact: Artifact,
    pub memory: Memory,
    pub wrote_pg: bool,
    pub wrote_markdown: bool,
}

#[derive(Debug, Clone)]
pub struct PublishMemoryResult {
    pub memory: Memory,
    pub wrote_pg: bool,
    pub wrote_markdown: bool,
}

#[derive(Debug, Clone)]
pub struct SearchContextRequest {
    pub scope_id: ScopeId,
    pub query: String,
    pub limit: usize,
}

impl SearchContextRequest {
    pub fn new(scope_id: ScopeId, query: impl Into<String>) -> Self {
        Self {
            scope_id,
            query: query.into(),
            limit: 10,
        }
    }
}

pub struct KernelBuilder {
    pg_store: Option<PgStore>,
    markdown_store: Option<MarkdownStore>,
    asset_store: Option<FileSystemAssetStore>,
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

    pub fn build(self) -> Result<Kernel> {
        if self.pg_store.is_none() && self.markdown_store.is_none() {
            bail!("kernel requires at least one backing store");
        }

        Ok(Kernel {
            pg_store: self.pg_store,
            markdown_store: self.markdown_store,
            asset_store: self.asset_store,
        })
    }
}

pub struct Kernel {
    pg_store: Option<PgStore>,
    markdown_store: Option<MarkdownStore>,
    asset_store: Option<FileSystemAssetStore>,
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

    pub async fn get_memory(
        &self,
        scope_id: ScopeId,
        memory_id: MemoryId,
    ) -> Result<Option<Memory>> {
        match &self.pg_store {
            Some(pg_store) => pg_store.get_memory(&scope_id, &memory_id).await,
            None => Ok(None),
        }
    }

    pub async fn remember_text(&self, request: RememberTextRequest) -> Result<RememberTextResult> {
        let started_at = Instant::now();
        let scope_id = request.scope_id.as_str().to_string();
        let result = async {
            let artifact = self.build_artifact(&request)?;
            self.remember_artifact(artifact, request.title.as_deref(), request.memory_kind)
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

        result
    }

    pub async fn remember_image(
        &self,
        request: RememberImageRequest,
    ) -> Result<RememberImageResult> {
        let started_at = Instant::now();
        let scope_id = request.scope_id.as_str().to_string();
        let result = async {
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
            } = request;

            let asset = asset_store.store_bytes(PutAssetRequest {
                bytes,
                media_type: media_type.clone(),
                storage_class: StorageClass::Raw,
                extension: file_extension,
                width: None,
                height: None,
                duration_ms: None,
                page_count: None,
                codec: None,
            })?;
            let title_override = title
                .filter(|value| !value.trim().is_empty())
                .or_else(|| Some(format!("Image asset {}", &asset.reference.sha256[..12])));
            let artifact = build_image_artifact(
                scope_id,
                media_type,
                body,
                source_refs,
                visibility,
                sensitivity,
                &asset,
            )?;
            let remembered = self
                .remember_artifact(artifact, title_override.as_deref(), memory_kind)
                .await?;

            Ok(RememberImageResult {
                asset,
                artifact: remembered.artifact,
                memory: remembered.memory,
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

        result
    }

    async fn remember_artifact(
        &self,
        artifact: Artifact,
        title_override: Option<&str>,
        memory_kind: Option<MemoryKind>,
    ) -> Result<RememberTextResult> {
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

        if let Some(pg_store) = &self.pg_store {
            self.seed_scope_if_needed(pg_store, &artifact.scope_id)
                .await?;
            let artifact_id = pg_store.insert_artifact(&artifact).await?;
            let memory_id = pg_store.insert_memory(&memory).await?;
            pg_store
                .link_evidence(&memory_id, &artifact_id, None)
                .await?;
            wrote_pg = true;
        }

        memory.evidence_count = 1;
        if let Some(markdown_store) = &self.markdown_store {
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
        let result = async {
            let limit = request.limit.clamp(1, 50);
            let normalized_query = normalize_query(&SearchQuery::new(&request.query, limit));
            let memories = match &self.pg_store {
                Some(pg_store) => {
                    pg_store
                        .search_by_keyword(&request.scope_id, &normalized_query, limit as i64)
                        .await?
                }
                None => Vec::new(),
            };
            let (entities, relations) = build_context_graph(&request.scope_id, &memories);

            Ok(ContextBundle {
                query: normalized_query,
                scope_id: request.scope_id,
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

    fn build_artifact(&self, request: &RememberTextRequest) -> Result<Artifact> {
        let mut artifact = Artifact::new(
            request.scope_id.clone(),
            request.artifact_kind,
            request.body.clone(),
            request.source_refs.clone(),
        )?;
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

    async fn seed_scope_if_needed(&self, pg_store: &PgStore, scope_id: &ScopeId) -> Result<()> {
        let path = match &self.markdown_store {
            Some(markdown_store) => {
                format!("{}/scopes/{}", markdown_store.tenant(), scope_id.as_str())
            }
            None => format!("default/scopes/{}", scope_id.as_str()),
        };

        pg_store
            .seed_scope(scope_id, scope_id.as_str(), &path)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl MemoryService for Kernel {
    async fn remember(&self, artifact: Artifact) -> Result<Memory> {
        Ok(self.remember_artifact(artifact, None, None).await?.memory)
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

fn build_image_artifact(
    scope_id: ScopeId,
    media_type: String,
    body: Option<String>,
    mut source_refs: Vec<String>,
    visibility: Visibility,
    sensitivity: Sensitivity,
    asset: &StoredAsset,
) -> Result<Artifact> {
    source_refs.push(asset.reference.uri());

    let body = body
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let content_text = match body {
        Some(note) => format!(
            "image asset: {}\nmedia_type: {}\nuser_note: {}",
            asset.reference.uri(),
            media_type,
            note
        ),
        None => format!(
            "image asset: {}\nmedia_type: {}\nimage recorded for future memory retrieval.",
            asset.reference.uri(),
            media_type
        ),
    };

    let mut artifact = Artifact::new(scope_id, ArtifactKind::Image, content_text, source_refs)?;
    artifact.mime_type = Some(media_type);
    artifact.visibility = visibility;
    artifact.sensitivity = sensitivity;
    Ok(artifact)
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

#[cfg(test)]
mod tests {
    use super::{
        Kernel, RememberImageRequest, RememberTextRequest, SearchContextRequest,
        build_context_graph,
    };
    use memory_domain::{ArtifactKind, Memory, MemoryKind, RelationType, ScopeId};
    use tempfile::tempdir;

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
    async fn remember_image_stores_asset_and_projection() {
        let tempdir = tempdir().unwrap();
        let kernel = Kernel::builder()
            .with_markdown_root(tempdir.path().join("markdown"))
            .unwrap()
            .with_asset_root(tempdir.path().join("assets"))
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
        assert!(result.memory.body.contains("image asset:"));
        assert!(projection_path.exists());
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
    }
}
