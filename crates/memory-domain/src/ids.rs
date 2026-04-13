use serde::{Deserialize, Serialize};
use ulid::Ulid;

macro_rules! id_type {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new() -> Self {
                Self(format!("{}_{}", $prefix, Ulid::new()))
            }

            pub fn from_string(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

id_type!(ArtifactId, "art");
id_type!(EpisodeId, "epi");
id_type!(MemoryId, "mem");
id_type!(EntityId, "ent");
id_type!(RelationId, "rel");
id_type!(EvidenceId, "evd");
id_type!(ScopeId, "scp");
id_type!(AccessKeyId, "key");
id_type!(SourceId, "src");
id_type!(AgentContextId, "ctx");
id_type!(ProjectDocumentId, "doc");

#[cfg(test)]
mod tests {
    use super::{
        AccessKeyId, AgentContextId, ArtifactId, EntityId, EpisodeId, EvidenceId, MemoryId,
        ProjectDocumentId, RelationId, ScopeId, SourceId,
    };

    #[test]
    fn generated_ids_use_expected_prefixes() {
        let cases = [
            ArtifactId::new().as_str().to_string(),
            EpisodeId::new().as_str().to_string(),
            MemoryId::new().as_str().to_string(),
            EntityId::new().as_str().to_string(),
            RelationId::new().as_str().to_string(),
            EvidenceId::new().as_str().to_string(),
            ScopeId::new().as_str().to_string(),
            AccessKeyId::new().as_str().to_string(),
            SourceId::new().as_str().to_string(),
            AgentContextId::new().as_str().to_string(),
            ProjectDocumentId::new().as_str().to_string(),
        ];

        assert!(cases[0].starts_with("art_"));
        assert!(cases[1].starts_with("epi_"));
        assert!(cases[2].starts_with("mem_"));
        assert!(cases[3].starts_with("ent_"));
        assert!(cases[4].starts_with("rel_"));
        assert!(cases[5].starts_with("evd_"));
        assert!(cases[6].starts_with("scp_"));
        assert!(cases[7].starts_with("key_"));
        assert!(cases[8].starts_with("src_"));
        assert!(cases[9].starts_with("ctx_"));
        assert!(cases[10].starts_with("doc_"));
    }

    #[test]
    fn from_string_and_default_roundtrip_values() {
        let artifact = ArtifactId::from_string("art_custom");
        let episode = EpisodeId::from_string("epi_custom");
        let memory = MemoryId::from_string("mem_custom");
        let entity = EntityId::from_string("ent_custom");
        let relation = RelationId::from_string("rel_custom");
        let evidence = EvidenceId::from_string("evd_custom");
        let scope = ScopeId::from_string("scp_custom");
        let access_key = AccessKeyId::from_string("key_custom");
        let source = SourceId::from_string("src_custom");
        let agent_context = AgentContextId::from_string("ctx_custom");
        let project_document = ProjectDocumentId::from_string("doc_custom");

        assert_eq!(artifact.as_str(), "art_custom");
        assert_eq!(episode.as_str(), "epi_custom");
        assert_eq!(memory.as_str(), "mem_custom");
        assert_eq!(entity.as_str(), "ent_custom");
        assert_eq!(relation.as_str(), "rel_custom");
        assert_eq!(evidence.as_str(), "evd_custom");
        assert_eq!(scope.as_str(), "scp_custom");
        assert_eq!(access_key.as_str(), "key_custom");
        assert_eq!(source.as_str(), "src_custom");
        assert_eq!(agent_context.as_str(), "ctx_custom");
        assert_eq!(project_document.as_str(), "doc_custom");

        assert!(ArtifactId::default().as_str().starts_with("art_"));
        assert!(EpisodeId::default().as_str().starts_with("epi_"));
        assert!(MemoryId::default().as_str().starts_with("mem_"));
        assert!(EntityId::default().as_str().starts_with("ent_"));
        assert!(RelationId::default().as_str().starts_with("rel_"));
        assert!(EvidenceId::default().as_str().starts_with("evd_"));
        assert!(ScopeId::default().as_str().starts_with("scp_"));
        assert!(AccessKeyId::default().as_str().starts_with("key_"));
        assert!(SourceId::default().as_str().starts_with("src_"));
        assert!(AgentContextId::default().as_str().starts_with("ctx_"));
        assert!(ProjectDocumentId::default().as_str().starts_with("doc_"));
    }
}
