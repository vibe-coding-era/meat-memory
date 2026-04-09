use async_trait::async_trait;
use image::{ColorType, DynamicImage, GenericImageView, ImageFormat};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
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

    fn runtime_unavailability_reason(&self, deployment: DeploymentTarget) -> Option<String> {
        if !self.enabled {
            return Some("provider is disabled".to_string());
        }

        if deployment == DeploymentTarget::Cloud {
            let missing_key = self
                .api_key_env
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .filter(|name| {
                    env::var(name)
                        .map(|value| value.trim().is_empty())
                        .unwrap_or(true)
                });

            if let Some(env_name) = missing_key {
                return Some(format!("missing environment variable {env_name}"));
            }
        }

        None
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct UnavailableModel {
    pub alias: String,
    pub display_name: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ModelSwitchNotice {
    pub capability: ModelCapability,
    pub from_model_alias: String,
    pub from_model_display_name: String,
    pub to_model_alias: String,
    pub to_model_display_name: String,
    pub reason: String,
    pub message: String,
}

impl ModelSwitchNotice {
    fn new(
        capability: ModelCapability,
        from: &ModelDescriptor,
        to: &ModelDescriptor,
        reason: impl Into<String>,
    ) -> Self {
        let reason = reason.into();
        Self {
            capability,
            from_model_alias: from.alias.clone(),
            from_model_display_name: from.display_name.clone(),
            to_model_alias: to.alias.clone(),
            to_model_display_name: to.display_name.clone(),
            message: format!(
                "{} LLM 不可用，已经切换到{}",
                from.display_name, to.display_name
            ),
            reason,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRuntimeRoute<'a> {
    pub primary: &'a ModelDescriptor,
    pub selected: &'a ModelDescriptor,
    pub fallbacks: Vec<&'a ModelDescriptor>,
    pub unavailable: Vec<UnavailableModel>,
    pub notice: Option<ModelSwitchNotice>,
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

    pub fn resolve_available(
        &self,
        capability: ModelCapability,
    ) -> Result<ResolvedRuntimeRoute<'_>, RegistryError> {
        let route = self.resolve(capability)?;
        let mut unavailable = Vec::new();

        for model in route.chain() {
            if let Some(reason) = self.model_runtime_unavailability_reason(model) {
                unavailable.push(UnavailableModel {
                    alias: model.alias.clone(),
                    display_name: model.display_name.clone(),
                    reason,
                });
                continue;
            }

            let notice = if model.alias != route.primary.alias {
                let primary_reason = unavailable
                    .iter()
                    .find(|candidate| candidate.alias == route.primary.alias)
                    .map(|candidate| candidate.reason.clone())
                    .unwrap_or_else(|| "unknown runtime failure".to_string());
                Some(ModelSwitchNotice::new(
                    capability,
                    route.primary,
                    model,
                    primary_reason,
                ))
            } else {
                None
            };

            return Ok(ResolvedRuntimeRoute {
                primary: route.primary,
                selected: model,
                fallbacks: route.fallbacks,
                unavailable,
                notice,
            });
        }

        Err(RegistryError::NoAvailableRouteTarget {
            capability,
            aliases: route
                .chain()
                .into_iter()
                .map(|model| model.alias.clone())
                .collect(),
        })
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

    fn model_runtime_unavailability_reason(&self, model: &ModelDescriptor) -> Option<String> {
        if !model.enabled {
            return Some("model is disabled".to_string());
        }

        self.provider(model.provider)
            .and_then(|provider| provider.runtime_unavailability_reason(model.deployment))
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
        let runtime_route = self
            .registry
            .resolve_available(ModelCapability::Vision)
            .ok();
        let selected_model = runtime_route
            .as_ref()
            .map(|resolved| resolved.selected)
            .unwrap_or(route.primary);
        let switch_notice = runtime_route
            .as_ref()
            .and_then(|resolved| resolved.notice.clone());
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
            "provider": selected_model.provider.as_str(),
            "model_alias": selected_model.alias,
            "requested_model_alias": route.primary.alias,
            "fallback_chain": route.fallbacks.iter().map(|model| model.alias.as_str()).collect::<Vec<_>>(),
            "unavailable_models": runtime_route
                .as_ref()
                .map(|resolved| resolved.unavailable.clone())
                .unwrap_or_default(),
            "switch_notice": switch_notice.as_ref().map(|notice| notice.message.as_str()),
            "profile": request.profile,
            "byte_size": request.byte_size,
            "user_note": request.prompt.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        });

        Ok(VisionResponse {
            caption,
            structured,
            model_alias: selected_model.alias.clone(),
            switch_notice,
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
    #[serde(default)]
    pub switch_notice: Option<ModelSwitchNotice>,
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
    #[error("no available route target for capability '{capability}': {aliases:?}")]
    NoAvailableRouteTarget {
        capability: ModelCapability,
        aliases: Vec<String>,
    },
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
        CapabilityRoute, DeploymentTarget, ExtractionRequest, ImageProfile, ModelCapability,
        ModelDescriptor, ModelError, ModelRegistry, Provider, ProviderDescriptor, ReasoningRequest,
        RegistryError, VisionGateway, VisionRequest, brightness_hint, color_mode_label,
        english_brightness_hint, english_orientation_hint, format_for_media_type, inspect_image,
        orientation_hint, profile_from_image,
    };
    use image::{ColorType, DynamicImage, ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};
    use serde_json::{from_value, json};
    use std::collections::{BTreeMap, BTreeSet};
    use std::io::Cursor;

    #[test]
    fn renders_capability_provider_and_deployment_labels() {
        let capabilities = [
            (ModelCapability::Reasoning, "reasoning"),
            (ModelCapability::Extraction, "extraction"),
            (ModelCapability::Vision, "vision"),
            (ModelCapability::Embedding, "embedding"),
        ];
        for (capability, expected) in capabilities {
            assert_eq!(capability.as_str(), expected);
            assert_eq!(capability.to_string(), expected);
        }

        let providers = [
            (Provider::OpenAI, "openai"),
            (Provider::Anthropic, "anthropic"),
            (Provider::Gemini, "gemini"),
            (Provider::Qwen, "qwen"),
            (Provider::Doubao, "doubao"),
            (Provider::MiniMax, "minimax"),
            (Provider::Glm, "glm"),
        ];
        for (provider, expected) in providers {
            assert_eq!(provider.as_str(), expected);
            assert_eq!(provider.to_string(), expected);
        }

        let deployments = [
            (DeploymentTarget::Cloud, "cloud"),
            (DeploymentTarget::Local, "local"),
            (DeploymentTarget::HybridReady, "hybrid_ready"),
        ];
        for (target, expected) in deployments {
            assert_eq!(target.as_str(), expected);
            assert_eq!(target.to_string(), expected);
        }
    }

    #[test]
    fn serde_defaults_apply_to_descriptors_and_requests() {
        let provider: ProviderDescriptor = from_value(json!({
            "provider": "openai",
            "display_name": "ChatGPT"
        }))
        .expect("provider descriptor should deserialize");
        assert!(provider.enabled);

        let model: ModelDescriptor = from_value(json!({
            "alias": "chatgpt_reasoning",
            "provider": "openai",
            "remote_model_id": "gpt-5-mini",
            "display_name": "ChatGPT Reasoning",
            "capabilities": ["reasoning", "extraction"],
            "deployment": "cloud"
        }))
        .expect("model descriptor should deserialize");
        assert_eq!(model.locale, "zh-CN");
        assert!(model.enabled);

        let reasoning: ReasoningRequest = from_value(json!({
            "prompt": "Summarize this"
        }))
        .expect("reasoning request should deserialize");
        let extraction: ExtractionRequest = from_value(json!({
            "content": "Extract entities"
        }))
        .expect("extraction request should deserialize");
        let vision: VisionRequest = from_value(json!({
            "asset_uri": "asset://demo.png",
            "media_type": "image/png"
        }))
        .expect("vision request should deserialize");

        assert_eq!(reasoning.locale, "zh-CN");
        assert_eq!(extraction.locale, "zh-CN");
        assert_eq!(vision.locale, "zh-CN");
    }

    #[test]
    fn descriptor_validation_rejects_empty_fields() {
        let provider_error = ProviderDescriptor {
            provider: Provider::OpenAI,
            display_name: "  ".to_string(),
            base_url: None,
            api_key_env: None,
            enabled: true,
        }
        .validate()
        .expect_err("empty provider name should fail");
        assert_eq!(
            provider_error,
            RegistryError::EmptyProviderDisplayName(Provider::OpenAI)
        );

        let empty_alias = build_model("  ", Provider::OpenAI, [ModelCapability::Reasoning]);
        assert_eq!(
            empty_alias.validate().expect_err("empty alias should fail"),
            RegistryError::EmptyModelAlias
        );

        let mut empty_id = build_model(
            "chatgpt_reasoning",
            Provider::OpenAI,
            [ModelCapability::Reasoning],
        );
        empty_id.remote_model_id = " ".to_string();
        assert_eq!(
            empty_id
                .validate()
                .expect_err("empty remote model id should fail"),
            RegistryError::EmptyModelId("chatgpt_reasoning".to_string())
        );

        let mut empty_display = build_model(
            "chatgpt_reasoning",
            Provider::OpenAI,
            [ModelCapability::Reasoning],
        );
        empty_display.display_name = "".to_string();
        assert_eq!(
            empty_display
                .validate()
                .expect_err("empty display name should fail"),
            RegistryError::EmptyModelDisplayName("chatgpt_reasoning".to_string())
        );

        let mut empty_capabilities = build_model(
            "chatgpt_reasoning",
            Provider::OpenAI,
            [ModelCapability::Reasoning],
        );
        empty_capabilities.capabilities.clear();
        assert_eq!(
            empty_capabilities
                .validate()
                .expect_err("empty capability set should fail"),
            RegistryError::EmptyCapabilitySet("chatgpt_reasoning".to_string())
        );

        let mut empty_locale = build_model(
            "chatgpt_reasoning",
            Provider::OpenAI,
            [ModelCapability::Reasoning],
        );
        empty_locale.locale = " ".to_string();
        assert_eq!(
            empty_locale
                .validate()
                .expect_err("empty locale should fail"),
            RegistryError::EmptyModelLocale("chatgpt_reasoning".to_string())
        );
    }

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
    fn registry_lookup_helpers_and_resolved_chain_work() {
        let registry = demo_registry().expect("registry should build");
        let route = registry
            .resolve(ModelCapability::Reasoning)
            .expect("reasoning route should resolve");
        let chain = route.chain();

        assert_eq!(registry.route_count(), 4);
        assert_eq!(
            registry
                .provider(Provider::OpenAI)
                .expect("provider should exist")
                .display_name,
            "ChatGPT"
        );
        assert_eq!(
            registry
                .model("chatgpt_vision")
                .expect("vision model should exist")
                .remote_model_id,
            "gpt-4.1-mini"
        );
        assert_eq!(registry.models_for_provider(Provider::OpenAI).len(), 3);
        assert_eq!(
            registry
                .resolve_primary(ModelCapability::Embedding)
                .expect("embedding primary should resolve")
                .alias,
            "chatgpt_embedding"
        );
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].alias, "chatgpt_reasoning");
        assert_eq!(chain[1].alias, "claude_reasoning");
    }

    #[test]
    fn resolve_available_skips_unavailable_primary_and_returns_switch_notice() {
        let registry = ModelRegistry::build(
            vec![
                ProviderDescriptor {
                    provider: Provider::OpenAI,
                    display_name: "ChatGPT".to_string(),
                    base_url: Some("https://api.openai.com/v1".to_string()),
                    api_key_env: Some("MEAT_MEMORY_TEST_PRIMARY_MISSING".to_string()),
                    enabled: true,
                },
                ProviderDescriptor {
                    provider: Provider::Anthropic,
                    display_name: "Claude".to_string(),
                    base_url: Some("https://api.anthropic.com".to_string()),
                    api_key_env: None,
                    enabled: true,
                },
            ],
            vec![
                ModelDescriptor {
                    display_name: "ChatGPT Reasoning".to_string(),
                    ..build_model(
                        "chatgpt_reasoning",
                        Provider::OpenAI,
                        [ModelCapability::Reasoning, ModelCapability::Extraction],
                    )
                },
                ModelDescriptor {
                    display_name: "Claude Reasoning".to_string(),
                    ..build_model(
                        "claude_reasoning",
                        Provider::Anthropic,
                        [ModelCapability::Reasoning, ModelCapability::Extraction],
                    )
                },
                ModelDescriptor {
                    display_name: "Claude Vision".to_string(),
                    ..build_model(
                        "claude_vision",
                        Provider::Anthropic,
                        [ModelCapability::Vision],
                    )
                },
                ModelDescriptor {
                    display_name: "Claude Embedding".to_string(),
                    ..build_model(
                        "claude_embedding",
                        Provider::Anthropic,
                        [ModelCapability::Embedding],
                    )
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
                        fallbacks: vec!["claude_reasoning".to_string()],
                    },
                ),
                (
                    ModelCapability::Vision,
                    CapabilityRoute {
                        primary: "claude_vision".to_string(),
                        fallbacks: Vec::new(),
                    },
                ),
                (
                    ModelCapability::Embedding,
                    CapabilityRoute {
                        primary: "claude_embedding".to_string(),
                        fallbacks: Vec::new(),
                    },
                ),
            ],
        )
        .expect("registry should build");

        let resolved = registry
            .resolve_available(ModelCapability::Reasoning)
            .expect("runtime route should resolve");

        assert_eq!(resolved.primary.alias, "chatgpt_reasoning");
        assert_eq!(resolved.selected.alias, "claude_reasoning");
        assert_eq!(resolved.unavailable.len(), 1);
        assert_eq!(resolved.unavailable[0].alias, "chatgpt_reasoning");
        assert_eq!(
            resolved.unavailable[0].reason,
            "missing environment variable MEAT_MEMORY_TEST_PRIMARY_MISSING"
        );
        assert_eq!(
            resolved
                .notice
                .as_ref()
                .expect("switch notice should exist")
                .message,
            "ChatGPT Reasoning LLM 不可用，已经切换到Claude Reasoning"
        );
    }

    #[test]
    fn registry_rejects_duplicate_missing_and_disabled_configs() {
        let duplicate_provider = ModelRegistry::build(
            vec![
                build_provider(Provider::OpenAI, "ChatGPT", true),
                build_provider(Provider::OpenAI, "ChatGPT Again", true),
            ],
            vec![],
            [],
        )
        .expect_err("duplicate provider should fail");
        assert_eq!(
            duplicate_provider,
            RegistryError::DuplicateProvider(Provider::OpenAI)
        );

        let missing_provider = ModelRegistry::build(
            vec![build_provider(Provider::Anthropic, "Claude", true)],
            vec![build_model(
                "chatgpt_reasoning",
                Provider::OpenAI,
                [ModelCapability::Reasoning, ModelCapability::Extraction],
            )],
            [],
        )
        .expect_err("missing provider should fail");
        assert_eq!(
            missing_provider,
            RegistryError::MissingProviderForModel {
                alias: "chatgpt_reasoning".to_string(),
                provider: Provider::OpenAI,
            }
        );

        let disabled_provider = ModelRegistry::build(
            vec![build_provider(Provider::OpenAI, "ChatGPT", false)],
            vec![build_model(
                "chatgpt_reasoning",
                Provider::OpenAI,
                [ModelCapability::Reasoning, ModelCapability::Extraction],
            )],
            [],
        )
        .expect_err("disabled provider should fail");
        assert_eq!(
            disabled_provider,
            RegistryError::DisabledProviderForModel {
                alias: "chatgpt_reasoning".to_string(),
                provider: Provider::OpenAI,
            }
        );

        let duplicate_model = ModelRegistry::build(
            vec![build_provider(Provider::OpenAI, "ChatGPT", true)],
            vec![
                build_model(
                    "chatgpt_reasoning",
                    Provider::OpenAI,
                    [ModelCapability::Reasoning, ModelCapability::Extraction],
                ),
                build_model(
                    "chatgpt_reasoning",
                    Provider::OpenAI,
                    [ModelCapability::Vision],
                ),
            ],
            [],
        )
        .expect_err("duplicate model should fail");
        assert_eq!(
            duplicate_model,
            RegistryError::DuplicateModelAlias("chatgpt_reasoning".to_string())
        );
    }

    #[test]
    fn route_validation_rejects_invalid_targets() {
        let models = BTreeMap::from([
            (
                "vision_enabled".to_string(),
                build_model(
                    "vision_enabled",
                    Provider::OpenAI,
                    [ModelCapability::Vision],
                ),
            ),
            (
                "vision_disabled".to_string(),
                ModelDescriptor {
                    enabled: false,
                    ..build_model(
                        "vision_disabled",
                        Provider::OpenAI,
                        [ModelCapability::Vision],
                    )
                },
            ),
        ]);

        let empty_primary = ModelRegistry::validate_route(
            ModelCapability::Vision,
            &CapabilityRoute {
                primary: " ".to_string(),
                fallbacks: Vec::new(),
            },
            &models,
        )
        .expect_err("empty primary should fail");
        assert_eq!(
            empty_primary,
            RegistryError::EmptyRoutePrimary(ModelCapability::Vision)
        );

        let unknown_model = ModelRegistry::validate_route(
            ModelCapability::Vision,
            &CapabilityRoute {
                primary: "missing".to_string(),
                fallbacks: Vec::new(),
            },
            &models,
        )
        .expect_err("unknown model should fail");
        assert_eq!(
            unknown_model,
            RegistryError::UnknownRouteModel {
                capability: ModelCapability::Vision,
                alias: "missing".to_string(),
            }
        );

        let duplicate_target = ModelRegistry::validate_route(
            ModelCapability::Vision,
            &CapabilityRoute {
                primary: "vision_enabled".to_string(),
                fallbacks: vec!["vision_enabled".to_string()],
            },
            &models,
        )
        .expect_err("duplicate target should fail");
        assert_eq!(
            duplicate_target,
            RegistryError::DuplicateRouteTarget {
                capability: ModelCapability::Vision,
                alias: "vision_enabled".to_string(),
            }
        );

        let disabled_model = ModelRegistry::validate_route(
            ModelCapability::Vision,
            &CapabilityRoute {
                primary: "vision_disabled".to_string(),
                fallbacks: Vec::new(),
            },
            &models,
        )
        .expect_err("disabled model should fail");
        assert_eq!(
            disabled_model,
            RegistryError::DisabledRouteModel {
                capability: ModelCapability::Vision,
                alias: "vision_disabled".to_string(),
            }
        );
    }

    #[test]
    fn build_rejects_duplicate_and_missing_routes() {
        let duplicate_route = ModelRegistry::build(
            vec![build_provider(Provider::OpenAI, "ChatGPT", true)],
            vec![
                build_model(
                    "chatgpt_reasoning",
                    Provider::OpenAI,
                    [ModelCapability::Reasoning, ModelCapability::Extraction],
                ),
                build_model(
                    "chatgpt_vision",
                    Provider::OpenAI,
                    [ModelCapability::Vision],
                ),
                build_model(
                    "chatgpt_embedding",
                    Provider::OpenAI,
                    [ModelCapability::Embedding],
                ),
            ],
            [
                (
                    ModelCapability::Reasoning,
                    CapabilityRoute {
                        primary: "chatgpt_reasoning".to_string(),
                        fallbacks: Vec::new(),
                    },
                ),
                (
                    ModelCapability::Reasoning,
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
        .expect_err("duplicate route should fail");
        assert_eq!(
            duplicate_route,
            RegistryError::DuplicateRouteConfig(ModelCapability::Reasoning)
        );

        let missing_route = ModelRegistry::build(
            vec![build_provider(Provider::OpenAI, "ChatGPT", true)],
            vec![
                build_model(
                    "chatgpt_reasoning",
                    Provider::OpenAI,
                    [ModelCapability::Reasoning, ModelCapability::Extraction],
                ),
                build_model(
                    "chatgpt_vision",
                    Provider::OpenAI,
                    [ModelCapability::Vision],
                ),
                build_model(
                    "chatgpt_embedding",
                    Provider::OpenAI,
                    [ModelCapability::Embedding],
                ),
            ],
            [
                (
                    ModelCapability::Reasoning,
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
        .expect_err("missing extraction route should fail");
        assert_eq!(
            missing_route,
            RegistryError::MissingRoute(ModelCapability::Extraction)
        );
    }

    #[test]
    fn resolve_reports_missing_routes_and_unknown_models() {
        let empty_registry = ModelRegistry::default();
        assert_eq!(
            empty_registry
                .resolve(ModelCapability::Vision)
                .expect_err("missing route should fail"),
            RegistryError::MissingRoute(ModelCapability::Vision)
        );
        assert_eq!(
            empty_registry
                .resolve_primary(ModelCapability::Embedding)
                .expect_err("missing primary should fail"),
            RegistryError::MissingRoute(ModelCapability::Embedding)
        );

        let registry = ModelRegistry {
            providers: BTreeMap::new(),
            models: BTreeMap::new(),
            routes: BTreeMap::from([(
                ModelCapability::Vision,
                CapabilityRoute {
                    primary: "missing_primary".to_string(),
                    fallbacks: vec!["missing_fallback".to_string()],
                },
            )]),
        };
        assert_eq!(
            registry
                .resolve(ModelCapability::Vision)
                .expect_err("unknown primary should fail"),
            RegistryError::UnknownRouteModel {
                capability: ModelCapability::Vision,
                alias: "missing_primary".to_string(),
            }
        );

        let registry = ModelRegistry {
            providers: BTreeMap::new(),
            models: BTreeMap::from([(
                "vision_primary".to_string(),
                build_model(
                    "vision_primary",
                    Provider::Gemini,
                    [ModelCapability::Vision],
                ),
            )]),
            routes: BTreeMap::from([(
                ModelCapability::Vision,
                CapabilityRoute {
                    primary: "vision_primary".to_string(),
                    fallbacks: vec!["missing_fallback".to_string()],
                },
            )]),
        };
        assert_eq!(
            registry
                .resolve(ModelCapability::Vision)
                .expect_err("unknown fallback should fail"),
            RegistryError::UnknownRouteModel {
                capability: ModelCapability::Vision,
                alias: "missing_fallback".to_string(),
            }
        );
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
    fn image_helper_functions_cover_supported_formats_and_hints() {
        assert_eq!(
            format_for_media_type("image/png").unwrap(),
            ImageFormat::Png
        );
        assert_eq!(
            format_for_media_type("image/jpeg").unwrap(),
            ImageFormat::Jpeg
        );
        assert_eq!(
            format_for_media_type("image/gif").unwrap(),
            ImageFormat::Gif
        );
        assert_eq!(
            format_for_media_type("image/webp").unwrap(),
            ImageFormat::WebP
        );
        assert!(matches!(
            format_for_media_type("image/svg+xml"),
            Err(ModelError::UnsupportedImageMediaType(media_type)) if media_type == "image/svg+xml"
        ));

        assert_eq!(color_mode_label(ColorType::L8), "grayscale");
        assert_eq!(color_mode_label(ColorType::Rgb8), "rgb");
        assert_eq!(color_mode_label(ColorType::Rgba8), "rgba");

        assert_eq!(orientation_hint(1920, 1080), "横向");
        assert_eq!(orientation_hint(1080, 1920), "纵向");
        assert_eq!(orientation_hint(512, 512), "近方形");
        assert_eq!(english_orientation_hint(1920, 1080), "landscape");
        assert_eq!(english_orientation_hint(1080, 1920), "portrait");
        assert_eq!(english_orientation_hint(512, 512), "square");

        assert_eq!(brightness_hint(32), "偏暗");
        assert_eq!(brightness_hint(120), "亮度中等");
        assert_eq!(brightness_hint(220), "偏亮");
        assert_eq!(english_brightness_hint(32), "dark");
        assert_eq!(english_brightness_hint(120), "balanced");
        assert_eq!(english_brightness_hint(220), "bright");
    }

    #[test]
    fn profile_from_image_detects_rgb_and_rgba_modes() {
        let rgba =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 1, Rgba([240, 240, 240, 128])));
        let rgb = DynamicImage::ImageRgb8(RgbImage::from_pixel(1, 2, Rgb([20, 40, 60])));

        let rgba_profile = profile_from_image(&rgba);
        let rgb_profile = profile_from_image(&rgb);

        assert_eq!(
            rgba_profile,
            ImageProfile {
                width: 2,
                height: 1,
                has_alpha: true,
                average_luma: 240,
                color_mode: "rgba".to_string(),
            }
        );
        assert_eq!(rgb_profile.width, 1);
        assert_eq!(rgb_profile.height, 2);
        assert!(!rgb_profile.has_alpha);
        assert_eq!(rgb_profile.color_mode, "rgb");
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

    #[test]
    fn inspect_image_rejects_empty_unsupported_and_invalid_payloads() {
        assert!(matches!(
            inspect_image(&[], "image/png"),
            Err(ModelError::EmptyInput)
        ));
        assert!(matches!(
            inspect_image(b"not-an-image", "image/svg+xml"),
            Err(ModelError::UnsupportedImageMediaType(media_type)) if media_type == "image/svg+xml"
        ));
        assert!(matches!(
            inspect_image(b"not-an-image", "image/png"),
            Err(ModelError::ImageDecode(_))
        ));
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

    #[tokio::test]
    async fn vision_gateway_switches_to_fallback_model_and_exposes_notice() {
        let registry = ModelRegistry::build(
            vec![
                ProviderDescriptor {
                    provider: Provider::Gemini,
                    display_name: "Gemini".to_string(),
                    base_url: Some("https://generativelanguage.googleapis.com".to_string()),
                    api_key_env: Some("MEAT_MEMORY_TEST_GEMINI_MISSING".to_string()),
                    enabled: true,
                },
                ProviderDescriptor {
                    provider: Provider::Anthropic,
                    display_name: "Claude".to_string(),
                    base_url: Some("https://api.anthropic.com".to_string()),
                    api_key_env: None,
                    enabled: true,
                },
            ],
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
                    alias: "claude_reasoning".to_string(),
                    provider: Provider::Anthropic,
                    remote_model_id: "claude-sonnet-4-5".to_string(),
                    display_name: "Claude Reasoning".to_string(),
                    capabilities: BTreeSet::from([
                        ModelCapability::Reasoning,
                        ModelCapability::Extraction,
                    ]),
                    deployment: DeploymentTarget::Cloud,
                    locale: "zh-CN".to_string(),
                    priority: 90,
                    enabled: true,
                },
                ModelDescriptor {
                    alias: "claude_vision".to_string(),
                    provider: Provider::Anthropic,
                    remote_model_id: "claude-sonnet-4-5".to_string(),
                    display_name: "Claude Vision".to_string(),
                    capabilities: BTreeSet::from([ModelCapability::Vision]),
                    deployment: DeploymentTarget::Cloud,
                    locale: "zh-CN".to_string(),
                    priority: 90,
                    enabled: true,
                },
                ModelDescriptor {
                    alias: "claude_embedding".to_string(),
                    provider: Provider::Anthropic,
                    remote_model_id: "text-embedding-3-large".to_string(),
                    display_name: "Claude Embedding".to_string(),
                    capabilities: BTreeSet::from([ModelCapability::Embedding]),
                    deployment: DeploymentTarget::Cloud,
                    locale: "zh-CN".to_string(),
                    priority: 90,
                    enabled: true,
                },
            ],
            [
                (
                    ModelCapability::Reasoning,
                    CapabilityRoute {
                        primary: "gemini_reasoning".to_string(),
                        fallbacks: vec!["claude_reasoning".to_string()],
                    },
                ),
                (
                    ModelCapability::Extraction,
                    CapabilityRoute {
                        primary: "gemini_reasoning".to_string(),
                        fallbacks: vec!["claude_reasoning".to_string()],
                    },
                ),
                (
                    ModelCapability::Vision,
                    CapabilityRoute {
                        primary: "gemini_vision".to_string(),
                        fallbacks: vec!["claude_vision".to_string()],
                    },
                ),
                (
                    ModelCapability::Embedding,
                    CapabilityRoute {
                        primary: "claude_embedding".to_string(),
                        fallbacks: Vec::new(),
                    },
                ),
            ],
        )
        .expect("registry should build");
        let gateway = VisionGateway::new(registry, "zh-CN").expect("gateway should build");

        let response = gateway
            .analyze(VisionRequest {
                asset_uri: "asset://raw/sha256/demo.png".to_string(),
                media_type: "image/png".to_string(),
                profile: Some(ImageProfile {
                    width: 640,
                    height: 480,
                    has_alpha: false,
                    average_luma: 128,
                    color_mode: "rgb".to_string(),
                }),
                byte_size: Some(2048),
                prompt: Some("控制台截图".to_string()),
                locale: "zh-CN".to_string(),
            })
            .await
            .expect("vision analysis should succeed");

        assert_eq!(response.model_alias, "claude_vision");
        assert_eq!(
            response
                .switch_notice
                .as_ref()
                .expect("switch notice should exist")
                .message,
            "Gemini Vision LLM 不可用，已经切换到Claude Vision"
        );
        assert_eq!(response.structured["provider"], "anthropic");
        assert_eq!(response.structured["model_alias"], "claude_vision");
        assert_eq!(
            response.structured["requested_model_alias"],
            "gemini_vision"
        );
        assert_eq!(
            response.structured["switch_notice"],
            "Gemini Vision LLM 不可用，已经切换到Claude Vision"
        );
    }

    #[tokio::test]
    async fn vision_gateway_covers_default_locale_and_english_caption_paths() {
        let gateway = VisionGateway::new(demo_registry().expect("registry should build"), "zh-CN")
            .expect("gateway should build");
        let zh_profile = inspect_image(&tiny_jpeg_bytes([220, 220, 220]), "image/jpeg")
            .expect("jpeg should decode");
        let en_profile = profile_from_image(&DynamicImage::ImageRgb8(RgbImage::from_pixel(
            1,
            2,
            Rgb([20, 40, 60]),
        )));

        let zh_response = gateway
            .analyze(VisionRequest {
                asset_uri: "asset://raw/cover.jpg".to_string(),
                media_type: "image/jpeg".to_string(),
                profile: Some(zh_profile),
                byte_size: None,
                prompt: None,
                locale: " ".to_string(),
            })
            .await
            .expect("default locale branch should succeed");
        assert!(zh_response.caption.contains("已生成可检索视觉摘要"));
        assert!(zh_response.caption.contains("整体偏亮"));

        let en_response = gateway
            .analyze(VisionRequest {
                asset_uri: "asset://raw/portrait.jpg".to_string(),
                media_type: "image/jpeg".to_string(),
                profile: Some(en_profile),
                byte_size: Some(42),
                prompt: Some("landing page".to_string()),
                locale: "en-US".to_string(),
            })
            .await
            .expect("english branch should succeed");
        assert!(en_response.caption.contains("Detected a image/jpeg image"));
        assert!(en_response.caption.contains("portrait composition"));
        assert!(en_response.caption.contains("User note: landing page"));

        let raw_response = gateway
            .analyze(VisionRequest {
                asset_uri: "asset://raw/no-profile.png".to_string(),
                media_type: "image/png".to_string(),
                profile: None,
                byte_size: Some(11),
                prompt: None,
                locale: "en-US".to_string(),
            })
            .await
            .expect("raw asset branch should succeed");
        assert!(raw_response.caption.contains("recorded the raw asset"));
    }

    #[tokio::test]
    async fn vision_gateway_validates_registry_and_request_inputs() {
        assert!(matches!(
            VisionGateway::new(ModelRegistry::default(), "zh-CN"),
            Err(ModelError::Registry(RegistryError::MissingRoute(
                ModelCapability::Vision
            )))
        ));
        assert!(matches!(
            VisionGateway::new(demo_registry().expect("registry should build"), " "),
            Err(ModelError::EmptyInput)
        ));

        let gateway = VisionGateway::new(demo_registry().expect("registry should build"), "zh-CN")
            .expect("gateway should build");
        assert!(matches!(
            gateway
                .analyze(VisionRequest {
                    asset_uri: " ".to_string(),
                    media_type: "image/png".to_string(),
                    profile: None,
                    byte_size: None,
                    prompt: None,
                    locale: "zh-CN".to_string(),
                })
                .await,
            Err(ModelError::EmptyInput)
        ));
        assert!(matches!(
            gateway
                .analyze(VisionRequest {
                    asset_uri: "asset://raw/empty.png".to_string(),
                    media_type: " ".to_string(),
                    profile: None,
                    byte_size: None,
                    prompt: None,
                    locale: "zh-CN".to_string(),
                })
                .await,
            Err(ModelError::EmptyInput)
        ));
    }

    fn tiny_png_bytes(pixel: [u8; 4]) -> Vec<u8> {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, Rgba(pixel)));
        let mut buffer = Cursor::new(Vec::new());
        image
            .write_to(&mut buffer, ImageFormat::Png)
            .expect("png should encode");
        buffer.into_inner()
    }

    fn tiny_jpeg_bytes(pixel: [u8; 3]) -> Vec<u8> {
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(1, 1, Rgb(pixel)));
        let mut buffer = Cursor::new(Vec::new());
        image
            .write_to(&mut buffer, ImageFormat::Jpeg)
            .expect("jpeg should encode");
        buffer.into_inner()
    }

    fn build_provider(provider: Provider, display_name: &str, enabled: bool) -> ProviderDescriptor {
        ProviderDescriptor {
            provider,
            display_name: display_name.to_string(),
            base_url: Some(format!("https://{}.example.com", provider.as_str())),
            api_key_env: Some(format!("{}_API_KEY", provider.as_str().to_uppercase())),
            enabled,
        }
    }

    fn build_model(
        alias: &str,
        provider: Provider,
        capabilities: impl IntoIterator<Item = ModelCapability>,
    ) -> ModelDescriptor {
        ModelDescriptor {
            alias: alias.to_string(),
            provider,
            remote_model_id: format!("{alias}_remote"),
            display_name: alias.replace('_', " "),
            capabilities: capabilities.into_iter().collect(),
            deployment: DeploymentTarget::Cloud,
            locale: "zh-CN".to_string(),
            priority: 100,
            enabled: true,
        }
    }

    fn demo_registry() -> Result<ModelRegistry, RegistryError> {
        ModelRegistry::build(
            vec![
                build_provider(Provider::OpenAI, "ChatGPT", true),
                build_provider(Provider::Anthropic, "Claude", true),
            ],
            vec![
                ModelDescriptor {
                    remote_model_id: "gpt-5-mini".to_string(),
                    display_name: "ChatGPT Reasoning".to_string(),
                    ..build_model(
                        "chatgpt_reasoning",
                        Provider::OpenAI,
                        [ModelCapability::Reasoning, ModelCapability::Extraction],
                    )
                },
                ModelDescriptor {
                    remote_model_id: "claude-sonnet-4-5".to_string(),
                    display_name: "Claude Reasoning".to_string(),
                    locale: "en-US".to_string(),
                    priority: 90,
                    ..build_model(
                        "claude_reasoning",
                        Provider::Anthropic,
                        [ModelCapability::Reasoning],
                    )
                },
                ModelDescriptor {
                    remote_model_id: "text-embedding-3-large".to_string(),
                    display_name: "ChatGPT Embedding".to_string(),
                    ..build_model(
                        "chatgpt_embedding",
                        Provider::OpenAI,
                        [ModelCapability::Embedding],
                    )
                },
                ModelDescriptor {
                    remote_model_id: "gpt-4.1-mini".to_string(),
                    display_name: "ChatGPT Vision".to_string(),
                    ..build_model(
                        "chatgpt_vision",
                        Provider::OpenAI,
                        [ModelCapability::Vision],
                    )
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
    }
}
