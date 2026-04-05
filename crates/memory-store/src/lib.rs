use anyhow::Result;
use async_trait::async_trait;
use memory_domain::{Artifact, ArtifactId, ContextBundle, Memory, MemoryId, ScopeId};

#[async_trait]
pub trait ArtifactRepository: Send + Sync {
    async fn put(&self, artifact: Artifact) -> Result<Artifact>;
    async fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>>;
}

#[async_trait]
pub trait MemoryRepository: Send + Sync {
    async fn put(&self, memory: Memory) -> Result<Memory>;
    async fn get(&self, id: &MemoryId) -> Result<Option<Memory>>;
    async fn list_by_scope(&self, scope_id: &ScopeId) -> Result<Vec<Memory>>;
}

#[async_trait]
pub trait ContextRepository: Send + Sync {
    async fn fetch_context(&self, query: &str, scope_id: &ScopeId) -> Result<ContextBundle>;
}
