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
