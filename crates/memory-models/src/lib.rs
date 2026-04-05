use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapability {
    Reasoning,
    Extraction,
    Vision,
    Embedding,
}

impl ModelCapability {
    pub const ALL: [Self; 4] = [
        Self::Reasoning,
        Self::Extraction,
        Self::Vision,
        Self::Embedding,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reasoning => "reasoning",
            Self::Extraction => "extraction",
            Self::Vision => "vision",
            Self::Embedding => "embedding",
        }
    }
}

impl fmt::Display for ModelCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum Provider {
    #[serde(rename = "openai")]
    OpenAI,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "gemini")]
    Gemini,
    #[serde(rename = "qwen")]
    Qwen,
    #[serde(rename = "doubao")]
    Doubao,
    #[serde(rename = "minimax")]
    MiniMax,
    #[serde(rename = "glm")]
    Glm,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::Qwen => "qwen",
            Self::Doubao => "doubao",
            Self::MiniMax => "minimax",
            Self::Glm => "glm",
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentTarget {
    Cloud,
    Local,
    HybridReady,
}

impl DeploymentTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cloud => "cloud",
            Self::Local => "local",
            Self::HybridReady => "hybrid_ready",
        }
    }
}

impl fmt::Display for DeploymentTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

fn default_enabled() -> bool {
    true
}

fn default_locale() -> String {
    "zh-CN".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ProviderDescriptor {
    pub provider: Provider,
    pub display_name: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

impl ProviderDescriptor {
    pub fn validate(&self) -> Result<(), RegistryError> {
        if self.display_name.trim().is_empty() {
            return Err(RegistryError::EmptyProviderDisplayName(self.provider));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ModelDescriptor {
    pub alias: String,
    pub provider: Provider,
    pub remote_model_id: String,
    pub display_name: String,
    pub capabilities: BTreeSet<ModelCapability>,
    pub deployment: DeploymentTarget,
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default)]
    pub priority: u16,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

impl ModelDescriptor {
    pub fn supports(&self, capability: ModelCapability) -> bool {
        self.capabilities.contains(&capability)
    }

    pub fn validate(&self) -> Result<(), RegistryError> {
        if self.alias.trim().is_empty() {
            return Err(RegistryError::EmptyModelAlias);
        }
        if self.remote_model_id.trim().is_empty() {
            return Err(RegistryError::EmptyModelId(self.alias.clone()));
        }
        if self.display_name.trim().is_empty() {
            return Err(RegistryError::EmptyModelDisplayName(self.alias.clone()));
        }
        if self.capabilities.is_empty() {
            return Err(RegistryError::EmptyCapabilitySet(self.alias.clone()));
        }
        if self.locale.trim().is_empty() {
            return Err(RegistryError::EmptyModelLocale(self.alias.clone()));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct CapabilityRoute {
    pub primary: String,
    #[serde(default)]
    pub fallbacks: Vec<String>,
}

impl CapabilityRoute {
    pub fn aliases(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.primary.as_str()).chain(self.fallbacks.iter().map(String::as_str))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRoute<'a> {
    pub primary: &'a ModelDescriptor,
    pub fallbacks: Vec<&'a ModelDescriptor>,
}

impl<'a> ResolvedRoute<'a> {
    pub fn chain(&self) -> Vec<&'a ModelDescriptor> {
        let mut models = Vec::with_capacity(1 + self.fallbacks.len());
        models.push(self.primary);
        models.extend(self.fallbacks.iter().copied());
        models
    }
}

#[derive(Debug, Clone, Default)]
pub struct ModelRegistry {
    providers: BTreeMap<Provider, ProviderDescriptor>,
    models: BTreeMap<String, ModelDescriptor>,
    routes: BTreeMap<ModelCapability, CapabilityRoute>,
}

impl ModelRegistry {
    pub fn build(
        providers: impl IntoIterator<Item = ProviderDescriptor>,
        models: impl IntoIterator<Item = ModelDescriptor>,
        routes: impl IntoIterator<Item = (ModelCapability, CapabilityRoute)>,
    ) -> Result<Self, RegistryError> {
        let mut provider_index = BTreeMap::new();
        for provider in providers {
            provider.validate()?;
            let provider_key = provider.provider;
            if provider_index.insert(provider_key, provider).is_some() {
                return Err(RegistryError::DuplicateProvider(provider_key));
            }
        }

        let mut model_index = BTreeMap::new();
        for model in models {
            model.validate()?;
            let alias = model.alias.clone();

            let provider = provider_index.get(&model.provider).ok_or_else(|| {
                RegistryError::MissingProviderForModel {
                    alias: alias.clone(),
                    provider: model.provider,
                }
            })?;

            if model.enabled && !provider.enabled {
                return Err(RegistryError::DisabledProviderForModel {
                    alias: alias.clone(),
                    provider: model.provider,
                });
            }

            if model_index.insert(alias.clone(), model).is_some() {
                return Err(RegistryError::DuplicateModelAlias(alias));
            }
        }

        let mut route_index = BTreeMap::new();
        for (capability, route) in routes {
            Self::validate_route(capability, &route, &model_index)?;
            if route_index.insert(capability, route).is_some() {
                return Err(RegistryError::DuplicateRouteConfig(capability));
            }
        }

        for capability in ModelCapability::ALL {
            if !route_index.contains_key(&capability) {
                return Err(RegistryError::MissingRoute(capability));
            }
        }

        Ok(Self {
            providers: provider_index,
            models: model_index,
            routes: route_index,
        })
    }

    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    pub fn model_count(&self) -> usize {
        self.models.len()
    }

    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    pub fn provider(&self, provider: Provider) -> Option<&ProviderDescriptor> {
        self.providers.get(&provider)
    }

    pub fn model(&self, alias: &str) -> Option<&ModelDescriptor> {
        self.models.get(alias)
    }

    pub fn models_for_provider(&self, provider: Provider) -> Vec<&ModelDescriptor> {
        self.models
            .values()
            .filter(|model| model.provider == provider)
            .collect()
    }

    pub fn resolve(&self, capability: ModelCapability) -> Result<ResolvedRoute<'_>, RegistryError> {
        let route = self
            .routes
            .get(&capability)
            .ok_or(RegistryError::MissingRoute(capability))?;
        let primary =
            self.models
                .get(&route.primary)
                .ok_or_else(|| RegistryError::UnknownRouteModel {
                    capability,
                    alias: route.primary.clone(),
                })?;

        let fallbacks = route
            .fallbacks
            .iter()
            .map(|alias| {
                self.models
                    .get(alias)
                    .ok_or_else(|| RegistryError::UnknownRouteModel {
                        capability,
                        alias: alias.clone(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ResolvedRoute { primary, fallbacks })
    }

    pub fn resolve_primary(
        &self,
        capability: ModelCapability,
    ) -> Result<&ModelDescriptor, RegistryError> {
        Ok(self.resolve(capability)?.primary)
    }

    fn validate_route(
        capability: ModelCapability,
        route: &CapabilityRoute,
        models: &BTreeMap<String, ModelDescriptor>,
    ) -> Result<(), RegistryError> {
        if route.primary.trim().is_empty() {
            return Err(RegistryError::EmptyRoutePrimary(capability));
        }

        let mut seen = BTreeSet::new();
        for alias in route.aliases() {
            if !seen.insert(alias.to_string()) {
                return Err(RegistryError::DuplicateRouteTarget {
                    capability,
                    alias: alias.to_string(),
                });
            }

            let model = models
                .get(alias)
                .ok_or_else(|| RegistryError::UnknownRouteModel {
                    capability,
                    alias: alias.to_string(),
                })?;

            if !model.enabled {
                return Err(RegistryError::DisabledRouteModel {
                    capability,
                    alias: alias.to_string(),
                });
            }

            if !model.supports(capability) {
                return Err(RegistryError::UnsupportedRouteCapability {
                    capability,
                    alias: alias.to_string(),
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("request payload must not be empty")]
    EmptyInput,
    #[error("model returned no output")]
    EmptyOutput,
    #[error("registry validation failed: {0}")]
    Registry(#[from] RegistryError),
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ReasoningRequest {
    pub prompt: String,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default = "default_locale")]
    pub locale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ReasoningResponse {
    pub content: String,
    pub model_alias: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ExtractionRequest {
    pub content: String,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default = "default_locale")]
    pub locale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ExtractionResponse {
    pub summary: String,
    #[serde(default)]
    pub structured: Value,
    pub model_alias: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct VisionRequest {
    pub asset_uri: String,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default = "default_locale")]
    pub locale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct VisionResponse {
    pub caption: String,
    #[serde(default)]
    pub structured: Value,
    pub model_alias: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct EmbeddingRequest {
    pub inputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub vectors: Vec<Vec<f32>>,
    pub dimension: usize,
    pub model_alias: String,
}

pub trait CapabilityModel {
    fn descriptor(&self) -> &ModelDescriptor;
}

#[async_trait]
pub trait ReasoningModel: CapabilityModel + Send + Sync {
    async fn reason(&self, request: ReasoningRequest) -> Result<ReasoningResponse, ModelError>;
}

#[async_trait]
pub trait ExtractionModel: CapabilityModel + Send + Sync {
    async fn extract(&self, request: ExtractionRequest) -> Result<ExtractionResponse, ModelError>;
}

#[async_trait]
pub trait VisionModel: CapabilityModel + Send + Sync {
    async fn analyze(&self, request: VisionRequest) -> Result<VisionResponse, ModelError>;
}

#[async_trait]
pub trait EmbeddingModel: CapabilityModel + Send + Sync {
    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, ModelError>;
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RegistryError {
    #[error("provider '{0}' has an empty display name")]
    EmptyProviderDisplayName(Provider),
    #[error("duplicate provider '{0}'")]
    DuplicateProvider(Provider),
    #[error("model alias must not be empty")]
    EmptyModelAlias,
    #[error("model '{0}' has an empty remote_model_id")]
    EmptyModelId(String),
    #[error("model '{0}' has an empty display name")]
    EmptyModelDisplayName(String),
    #[error("model '{0}' has no capabilities configured")]
    EmptyCapabilitySet(String),
    #[error("model '{0}' has an empty locale")]
    EmptyModelLocale(String),
    #[error("model '{alias}' references missing provider '{provider}'")]
    MissingProviderForModel { alias: String, provider: Provider },
    #[error("model '{alias}' references disabled provider '{provider}'")]
    DisabledProviderForModel { alias: String, provider: Provider },
    #[error("duplicate model alias '{0}'")]
    DuplicateModelAlias(String),
    #[error("route for capability '{0}' is missing")]
    MissingRoute(ModelCapability),
    #[error("route for capability '{0}' is defined more than once")]
    DuplicateRouteConfig(ModelCapability),
    #[error("route for capability '{0}' has an empty primary alias")]
    EmptyRoutePrimary(ModelCapability),
    #[error("route for capability '{capability}' references unknown model '{alias}'")]
    UnknownRouteModel {
        capability: ModelCapability,
        alias: String,
    },
    #[error("route for capability '{capability}' references duplicate model '{alias}'")]
    DuplicateRouteTarget {
        capability: ModelCapability,
        alias: String,
    },
    #[error("route for capability '{capability}' targets disabled model '{alias}'")]
    DisabledRouteModel {
        capability: ModelCapability,
        alias: String,
    },
    #[error(
        "route for capability '{capability}' targets model '{alias}' without required capability"
    )]
    UnsupportedRouteCapability {
        capability: ModelCapability,
        alias: String,
    },
}

#[cfg(test)]
mod tests {
    use super::{
        CapabilityRoute, DeploymentTarget, ModelCapability, ModelDescriptor, ModelRegistry,
        Provider, ProviderDescriptor, RegistryError,
    };
    use std::collections::BTreeSet;

    #[test]
    fn renders_provider_name() {
        assert_eq!(Provider::Qwen.as_str(), "qwen");
    }

    #[test]
    fn builds_registry_and_resolves_fallback_chain() {
        let registry = ModelRegistry::build(
            vec![
                ProviderDescriptor {
                    provider: Provider::OpenAI,
                    display_name: "ChatGPT".to_string(),
                    base_url: Some("https://api.openai.com/v1".to_string()),
                    api_key_env: Some("OPENAI_API_KEY".to_string()),
                    enabled: true,
                },
                ProviderDescriptor {
                    provider: Provider::Anthropic,
                    display_name: "Claude".to_string(),
                    base_url: Some("https://api.anthropic.com".to_string()),
                    api_key_env: Some("ANTHROPIC_API_KEY".to_string()),
                    enabled: true,
                },
            ],
            vec![
                ModelDescriptor {
                    alias: "chatgpt_reasoning".to_string(),
                    provider: Provider::OpenAI,
                    remote_model_id: "gpt-5-mini".to_string(),
                    display_name: "ChatGPT Reasoning".to_string(),
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
                    alias: "claude_reasoning".to_string(),
                    provider: Provider::Anthropic,
                    remote_model_id: "claude-sonnet-4-5".to_string(),
                    display_name: "Claude Reasoning".to_string(),
                    capabilities: BTreeSet::from([ModelCapability::Reasoning]),
                    deployment: DeploymentTarget::Cloud,
                    locale: "zh-CN".to_string(),
                    priority: 90,
                    enabled: true,
                },
                ModelDescriptor {
                    alias: "chatgpt_embedding".to_string(),
                    provider: Provider::OpenAI,
                    remote_model_id: "text-embedding-3-large".to_string(),
                    display_name: "ChatGPT Embedding".to_string(),
                    capabilities: BTreeSet::from([ModelCapability::Embedding]),
                    deployment: DeploymentTarget::Cloud,
                    locale: "zh-CN".to_string(),
                    priority: 100,
                    enabled: true,
                },
                ModelDescriptor {
                    alias: "chatgpt_vision".to_string(),
                    provider: Provider::OpenAI,
                    remote_model_id: "gpt-4.1-mini".to_string(),
                    display_name: "ChatGPT Vision".to_string(),
                    capabilities: BTreeSet::from([ModelCapability::Vision]),
                    deployment: DeploymentTarget::Cloud,
                    locale: "zh-CN".to_string(),
                    priority: 80,
                    enabled: true,
                },
            ],
            [
                (
                    ModelCapability::Reasoning,
                    CapabilityRoute {
                        primary: "chatgpt_reasoning".to_string(),
                        fallbacks: vec!["claude_reasoning".to_string()],
                    },
                ),
                (
                    ModelCapability::Extraction,
                    CapabilityRoute {
                        primary: "chatgpt_reasoning".to_string(),
                        fallbacks: Vec::new(),
                    },
                ),
                (
                    ModelCapability::Vision,
                    CapabilityRoute {
                        primary: "chatgpt_vision".to_string(),
                        fallbacks: Vec::new(),
                    },
                ),
                (
                    ModelCapability::Embedding,
                    CapabilityRoute {
                        primary: "chatgpt_embedding".to_string(),
                        fallbacks: Vec::new(),
                    },
                ),
            ],
        )
        .expect("registry should build");

        let route = registry
            .resolve(ModelCapability::Reasoning)
            .expect("reasoning route should resolve");

        assert_eq!(registry.provider_count(), 2);
        assert_eq!(registry.model_count(), 4);
        assert_eq!(route.primary.alias, "chatgpt_reasoning");
        assert_eq!(route.fallbacks.len(), 1);
        assert_eq!(route.fallbacks[0].alias, "claude_reasoning");
    }

    #[test]
    fn rejects_route_to_model_without_capability() {
        let error = ModelRegistry::build(
            vec![ProviderDescriptor {
                provider: Provider::Gemini,
                display_name: "Gemini".to_string(),
                base_url: Some("https://generativelanguage.googleapis.com".to_string()),
                api_key_env: Some("GEMINI_API_KEY".to_string()),
                enabled: true,
            }],
            vec![
                ModelDescriptor {
                    alias: "gemini_embed".to_string(),
                    provider: Provider::Gemini,
                    remote_model_id: "text-embedding-004".to_string(),
                    display_name: "Gemini Embedding".to_string(),
                    capabilities: BTreeSet::from([ModelCapability::Embedding]),
                    deployment: DeploymentTarget::Cloud,
                    locale: "zh-CN".to_string(),
                    priority: 100,
                    enabled: true,
                },
                ModelDescriptor {
                    alias: "gemini_reasoning".to_string(),
                    provider: Provider::Gemini,
                    remote_model_id: "gemini-2.5-flash".to_string(),
                    display_name: "Gemini Reasoning".to_string(),
                    capabilities: BTreeSet::from([
                        ModelCapability::Reasoning,
                        ModelCapability::Extraction,
                        ModelCapability::Vision,
                    ]),
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
                        primary: "gemini_reasoning".to_string(),
                        fallbacks: Vec::new(),
                    },
                ),
                (
                    ModelCapability::Embedding,
                    CapabilityRoute {
                        primary: "gemini_reasoning".to_string(),
                        fallbacks: Vec::new(),
                    },
                ),
            ],
        )
        .expect_err("embedding route should fail");

        assert_eq!(
            error,
            RegistryError::UnsupportedRouteCapability {
                capability: ModelCapability::Embedding,
                alias: "gemini_reasoning".to_string(),
            }
        );
    }
}
