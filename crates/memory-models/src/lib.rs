use async_trait::async_trait;
use image::{ColorType, DynamicImage, GenericImageView, ImageFormat};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
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
    #[error("unsupported image media type '{0}'")]
    UnsupportedImageMediaType(String),
    #[error("failed to decode image payload: {0}")]
    ImageDecode(String),
    #[error("registry validation failed: {0}")]
    Registry(#[from] RegistryError),
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone)]
pub struct VisionGateway {
    registry: ModelRegistry,
    default_locale: String,
}

impl VisionGateway {
    pub fn new(
        registry: ModelRegistry,
        default_locale: impl Into<String>,
    ) -> Result<Self, ModelError> {
        registry.resolve_primary(ModelCapability::Vision)?;
        let default_locale = default_locale.into();
        if default_locale.trim().is_empty() {
            return Err(ModelError::EmptyInput);
        }

        Ok(Self {
            registry,
            default_locale,
        })
    }

    pub async fn analyze(&self, request: VisionRequest) -> Result<VisionResponse, ModelError> {
        if request.asset_uri.trim().is_empty() || request.media_type.trim().is_empty() {
            return Err(ModelError::EmptyInput);
        }

        let route = self.registry.resolve(ModelCapability::Vision)?;
        let locale = if request.locale.trim().is_empty() {
            self.default_locale.clone()
        } else {
            request.locale.clone()
        };
        let caption = render_caption(&request, &locale);
        let structured = json!({
            "asset_uri": request.asset_uri,
            "media_type": request.media_type,
            "locale": locale,
            "provider": route.primary.provider.as_str(),
            "model_alias": route.primary.alias,
            "fallback_chain": route.fallbacks.iter().map(|model| model.alias.as_str()).collect::<Vec<_>>(),
            "profile": request.profile,
            "byte_size": request.byte_size,
            "user_note": request.prompt.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        });

        Ok(VisionResponse {
            caption,
            structured,
            model_alias: route.primary.alias.clone(),
        })
    }
}

pub fn inspect_image(bytes: &[u8], media_type: &str) -> Result<ImageProfile, ModelError> {
    if bytes.is_empty() {
        return Err(ModelError::EmptyInput);
    }

    let format = format_for_media_type(media_type)?;
    let image = image::load_from_memory_with_format(bytes, format)
        .map_err(|error| ModelError::ImageDecode(error.to_string()))?;
    Ok(profile_from_image(&image))
}

fn render_caption(request: &VisionRequest, locale: &str) -> String {
    let note = request
        .prompt
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match request.profile.as_ref() {
        Some(profile) if locale.starts_with("zh") => {
            let alpha_hint = if profile.has_alpha {
                "，包含透明通道"
            } else {
                ""
            };
            match note {
                Some(note) => format!(
                    "检测到一张 {} 图片，尺寸 {}x{}，{}布局，整体{}{}。用户备注：{}",
                    request.media_type,
                    profile.width,
                    profile.height,
                    orientation_hint(profile.width, profile.height),
                    brightness_hint(profile.average_luma),
                    alpha_hint,
                    note
                ),
                None => format!(
                    "检测到一张 {} 图片，尺寸 {}x{}，{}布局，整体{}{}，已生成可检索视觉摘要。",
                    request.media_type,
                    profile.width,
                    profile.height,
                    orientation_hint(profile.width, profile.height),
                    brightness_hint(profile.average_luma),
                    alpha_hint
                ),
            }
        }
        Some(profile) => match note {
            Some(note) => format!(
                "Detected a {} image at {}x{} with {} composition and {} lighting. User note: {}",
                request.media_type,
                profile.width,
                profile.height,
                english_orientation_hint(profile.width, profile.height),
                english_brightness_hint(profile.average_luma),
                note
            ),
            None => format!(
                "Detected a {} image at {}x{} with {} composition and {} lighting.",
                request.media_type,
                profile.width,
                profile.height,
                english_orientation_hint(profile.width, profile.height),
                english_brightness_hint(profile.average_luma)
            ),
        },
        None if locale.starts_with("zh") => match note {
            Some(note) => format!(
                "检测到一张 {} 图片，已记录原始资产并保留后续视觉解析入口。用户备注：{}",
                request.media_type, note
            ),
            None => format!(
                "检测到一张 {} 图片，已记录原始资产并保留后续视觉解析入口。",
                request.media_type
            ),
        },
        None => match note {
            Some(note) => format!(
                "Detected a {} image and recorded the raw asset for later visual analysis. User note: {}",
                request.media_type, note
            ),
            None => format!(
                "Detected a {} image and recorded the raw asset for later visual analysis.",
                request.media_type
            ),
        },
    }
}

fn format_for_media_type(media_type: &str) -> Result<ImageFormat, ModelError> {
    match media_type {
        "image/png" => Ok(ImageFormat::Png),
        "image/jpeg" => Ok(ImageFormat::Jpeg),
        "image/gif" => Ok(ImageFormat::Gif),
        "image/webp" => Ok(ImageFormat::WebP),
        other => Err(ModelError::UnsupportedImageMediaType(other.to_string())),
    }
}

fn profile_from_image(image: &DynamicImage) -> ImageProfile {
    let (width, height) = image.dimensions();
    let rgba = image.to_rgba8();
    let pixel_count = rgba.pixels().len().max(1) as u64;
    let luma_sum = rgba.pixels().fold(0_u64, |accumulator, pixel| {
        let [red, green, blue, _] = pixel.0;
        accumulator
            + ((2126_u64 * red as u64) + (7152_u64 * green as u64) + (722_u64 * blue as u64))
                / 10_000
    });
    let has_alpha = matches!(
        image.color(),
        ColorType::La8
            | ColorType::La16
            | ColorType::Rgba8
            | ColorType::Rgba16
            | ColorType::Rgba32F
    ) || rgba.pixels().any(|pixel| pixel.0[3] < 255);

    ImageProfile {
        width,
        height,
        has_alpha,
        average_luma: (luma_sum / pixel_count) as u8,
        color_mode: color_mode_label(image.color()).to_string(),
    }
}

fn color_mode_label(color_type: ColorType) -> &'static str {
    match color_type {
        ColorType::L8 | ColorType::L16 | ColorType::La8 | ColorType::La16 => "grayscale",
        ColorType::Rgb8 | ColorType::Rgb16 | ColorType::Rgb32F => "rgb",
        ColorType::Rgba8 | ColorType::Rgba16 | ColorType::Rgba32F => "rgba",
        _ => "unknown",
    }
}

fn orientation_hint(width: u32, height: u32) -> &'static str {
    match width.cmp(&height) {
        std::cmp::Ordering::Greater => "横向",
        std::cmp::Ordering::Less => "纵向",
        std::cmp::Ordering::Equal => "近方形",
    }
}

fn english_orientation_hint(width: u32, height: u32) -> &'static str {
    match width.cmp(&height) {
        std::cmp::Ordering::Greater => "landscape",
        std::cmp::Ordering::Less => "portrait",
        std::cmp::Ordering::Equal => "square",
    }
}

fn brightness_hint(average_luma: u8) -> &'static str {
    match average_luma {
        0..=63 => "偏暗",
        64..=179 => "亮度中等",
        _ => "偏亮",
    }
}

fn english_brightness_hint(average_luma: u8) -> &'static str {
    match average_luma {
        0..=63 => "dark",
        64..=179 => "balanced",
        _ => "bright",
    }
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
    pub media_type: String,
    #[serde(default)]
    pub profile: Option<ImageProfile>,
    #[serde(default)]
    pub byte_size: Option<u64>,
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
pub struct ImageProfile {
    pub width: u32,
    pub height: u32,
    pub has_alpha: bool,
    pub average_luma: u8,
    pub color_mode: String,
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
        CapabilityRoute, DeploymentTarget, ImageProfile, ModelCapability, ModelDescriptor,
        ModelRegistry, Provider, ProviderDescriptor, RegistryError, VisionGateway, VisionRequest,
        inspect_image,
    };
    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
    use std::collections::BTreeSet;
    use std::io::Cursor;

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

    #[test]
    fn inspects_png_dimensions_and_profile() {
        let bytes = tiny_png_bytes([240, 240, 240, 255]);
        let profile = inspect_image(&bytes, "image/png").expect("png should decode");

        assert_eq!(
            profile,
            ImageProfile {
                width: 1,
                height: 1,
                has_alpha: true,
                average_luma: 240,
                color_mode: "rgba".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn vision_gateway_generates_structured_caption() {
        let registry = ModelRegistry::build(
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
        .expect("registry should build");
        let gateway = VisionGateway::new(registry, "zh-CN").expect("gateway should build");
        let profile = inspect_image(&tiny_png_bytes([220, 220, 220, 255]), "image/png")
            .expect("png should decode");

        let response = gateway
            .analyze(VisionRequest {
                asset_uri: "asset://raw/sha256/demo.png".to_string(),
                media_type: "image/png".to_string(),
                profile: Some(profile),
                byte_size: Some(68),
                prompt: Some("登录页截图".to_string()),
                locale: "zh-CN".to_string(),
            })
            .await
            .expect("vision analysis should succeed");

        assert_eq!(response.model_alias, "gemini_vision");
        assert!(response.caption.contains("检测到一张 image/png 图片"));
        assert!(response.caption.contains("登录页截图"));
        assert_eq!(response.structured["provider"], "gemini");
    }

    fn tiny_png_bytes(pixel: [u8; 4]) -> Vec<u8> {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, Rgba(pixel)));
        let mut buffer = Cursor::new(Vec::new());
        image
            .write_to(&mut buffer, ImageFormat::Png)
            .expect("png should encode");
        buffer.into_inner()
    }
}
