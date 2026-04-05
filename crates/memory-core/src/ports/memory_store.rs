use anyhow::Result;
use async_trait::async_trait;
use memory_domain::{Artifact, ArtifactId, ContextBundle, Memory, MemoryId, ScopeId};

#[async_trait]
pub trait MemoryStorePort: Send + Sync {
    async fn write_artifact(&self, artifact: Artifact) -> Result<Artifact>;
    async fn get_artifact(&self, artifact_id: &ArtifactId) -> Result<Option<Artifact>>;
    async fn write_memory(&self, memory: Memory) -> Result<Memory>;
    async fn get_memory(&self, memory_id: &MemoryId) -> Result<Option<Memory>>;
    async fn list_memories_by_scope(&self, scope_id: &ScopeId) -> Result<Vec<Memory>>;
    async fn fetch_context(&self, query: &str, scope_id: &ScopeId) -> Result<ContextBundle>;
}
