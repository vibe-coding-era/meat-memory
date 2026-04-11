use anyhow::{Context, Result};
use memory_models::{
    CapabilityRoute, ModelCapability, ModelDescriptor, ModelRegistry, ProviderDescriptor,
};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub markdown: MarkdownConfig,
    pub postgres: PostgresConfig,
    pub assets: AssetsConfig,
    pub models: ModelsConfig,
    pub sync: SyncConfig,
    pub features: FeaturesConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub bind: String,
    pub shutdown_grace_period_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarkdownConfig {
    pub root: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PostgresConfig {
    pub app_name: String,
    pub database_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetsConfig {
    pub root: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelsConfig {
    #[serde(default = "default_locale")]
    pub default_locale: String,
    #[serde(default)]
    pub providers: Vec<ProviderDescriptor>,
    #[serde(default)]
    pub catalog: Vec<ModelDescriptor>,
    pub routing: ModelRoutingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelRoutingConfig {
    pub reasoning: CapabilityRoute,
    pub extraction: CapabilityRoute,
    pub vision: CapabilityRoute,
    pub embedding: CapabilityRoute,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyncConfig {
    pub mode: String,
    #[serde(default = "default_sync_node_id")]
    pub node_id: String,
    #[serde(default = "default_sync_state_path")]
    pub state_path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FeaturesConfig {
    pub enable_pg: bool,
    pub enable_markdown: bool,
    pub enable_http: bool,
    pub enable_mcp: bool,
}

fn default_locale() -> String {
    "zh-CN".to_string()
}

fn default_sync_node_id() -> String {
    "node-local".to_string()
}

fn default_sync_state_path() -> String {
    "./storage/sync/state.json".to_string()
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let path =
            env::var("MEAT_MEMORY_CONFIG").unwrap_or_else(|_| "config/default.toml".to_string());
        Self::from_file(PathBuf::from(path))
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file {}", path.display()))?;
        Self::from_toml_str(&raw)
            .with_context(|| format!("failed to parse config file {}", path.display()))
    }

    pub fn from_toml_str(raw: &str) -> Result<Self> {
        toml::from_str(raw).context("failed to deserialize app config")
    }

    pub fn to_toml_string_pretty(&self) -> Result<String> {
        toml::to_string_pretty(self).context("failed to serialize app config")
    }

    pub fn model_registry(&self) -> Result<ModelRegistry> {
        self.models.build_registry()
    }
}

impl ModelsConfig {
    pub fn build_registry(&self) -> Result<ModelRegistry> {
        ModelRegistry::build(
            self.providers.clone(),
            self.catalog.clone(),
            self.routing.clone().into_routes(),
        )
        .context("failed to build model registry from config")
    }
}

impl ModelRoutingConfig {
    pub fn into_routes(self) -> [(ModelCapability, CapabilityRoute); 4] {
        [
            (ModelCapability::Reasoning, self.reasoning),
            (ModelCapability::Extraction, self.extraction),
            (ModelCapability::Vision, self.vision),
            (ModelCapability::Embedding, self.embedding),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::AppConfig;
    use memory_models::{ModelCapability, Provider};
    use std::{
        env, fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn loads_models_and_builds_registry() {
        let config = AppConfig::from_toml_str(&sample_toml(true)).expect("config should parse");

        let registry = config.model_registry().expect("registry should build");
        let reasoning = registry
            .resolve_primary(ModelCapability::Reasoning)
            .expect("reasoning route should resolve");

        assert_eq!(config.models.default_locale, "zh-CN");
        assert_eq!(registry.provider_count(), 2);
        assert_eq!(registry.model_count(), 4);
        assert_eq!(reasoning.provider, Provider::OpenAI);
        assert_eq!(reasoning.alias, "chatgpt_reasoning");
    }

    #[test]
    fn applies_default_locale_and_preserves_route_order() {
        let config = AppConfig::from_toml_str(&sample_toml(false)).expect("config should parse");
        let routes = config.models.routing.clone().into_routes();

        assert_eq!(config.models.default_locale, "zh-CN");
        assert_eq!(routes[0].0, ModelCapability::Reasoning);
        assert_eq!(routes[1].0, ModelCapability::Extraction);
        assert_eq!(routes[2].0, ModelCapability::Vision);
        assert_eq!(routes[3].0, ModelCapability::Embedding);
        assert_eq!(routes[0].1.primary, "chatgpt_reasoning");
        assert_eq!(routes[3].1.primary, "chatgpt_embedding");
    }

    #[test]
    fn serializes_config_back_to_toml() {
        let config = AppConfig::from_toml_str(&sample_toml(true)).expect("config should parse");
        let rendered = config
            .to_toml_string_pretty()
            .expect("config should serialize");

        assert!(rendered.contains("[server]"));
        assert!(rendered.contains("bind = \"127.0.0.1:8080\""));
        assert!(rendered.contains("[models.routing.reasoning]"));
        assert!(rendered.contains("primary = \"chatgpt_reasoning\""));
    }

    #[test]
    fn load_reads_environment_override_and_from_file_roundtrips() {
        let path = temp_config_path("load");
        fs::write(&path, sample_toml(true)).expect("config file should be written");

        let from_file = AppConfig::from_file(&path).expect("config should load from file");

        unsafe {
            env::set_var("MEAT_MEMORY_CONFIG", &path);
        }
        let loaded = AppConfig::load().expect("config should load from env path");
        unsafe {
            env::remove_var("MEAT_MEMORY_CONFIG");
        }

        assert_eq!(from_file.server.bind, "127.0.0.1:8080");
        assert_eq!(loaded.assets.root, "./storage/assets");
        assert_eq!(loaded.sync.mode, "dual_write");
        assert_eq!(loaded.sync.node_id, "node-main");
        assert_eq!(loaded.sync.state_path, "./storage/sync/main.json");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn from_file_surfaces_read_and_parse_errors() {
        let missing = temp_config_path("missing");
        let read_error = AppConfig::from_file(&missing).expect_err("missing file should fail");
        assert!(
            read_error
                .to_string()
                .contains("failed to read config file")
        );

        let invalid = temp_config_path("invalid");
        fs::write(&invalid, "not = [valid").expect("invalid config file should be written");
        let parse_error = AppConfig::from_file(&invalid).expect_err("invalid toml should fail");
        assert!(
            parse_error
                .to_string()
                .contains("failed to parse config file")
        );

        let deserialize_error =
            AppConfig::from_toml_str("not = [valid").expect_err("invalid toml should fail");
        assert!(
            deserialize_error
                .to_string()
                .contains("failed to deserialize app config")
        );

        let _ = fs::remove_file(invalid);
    }

    fn sample_toml(include_default_locale: bool) -> String {
        let default_locale = if include_default_locale {
            "default_locale = \"zh-CN\"\n"
        } else {
            ""
        };

        format!(
            r#"
[server]
bind = "127.0.0.1:8080"
shutdown_grace_period_secs = 10

[logging]
level = "info"
format = "pretty"

[markdown]
root = "./docs"

[postgres]
app_name = "meat-memory"
database_url = "postgres://postgres:postgres@127.0.0.1:5433/meat_memory_dev"

[assets]
root = "./storage/assets"

[sync]
mode = "dual_write"
node_id = "node-main"
state_path = "./storage/sync/main.json"

[features]
enable_pg = true
enable_markdown = true
enable_http = true
enable_mcp = true

[models]
{default_locale}

[[models.providers]]
provider = "openai"
display_name = "ChatGPT"
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"
enabled = true

[[models.providers]]
provider = "anthropic"
display_name = "Claude"
base_url = "https://api.anthropic.com"
api_key_env = "ANTHROPIC_API_KEY"
enabled = true

[[models.catalog]]
alias = "chatgpt_reasoning"
provider = "openai"
remote_model_id = "gpt-5-mini"
display_name = "ChatGPT Reasoning"
capabilities = ["reasoning", "extraction"]
deployment = "cloud"
locale = "zh-CN"
priority = 100
enabled = true

[[models.catalog]]
alias = "claude_reasoning"
provider = "anthropic"
remote_model_id = "claude-sonnet-4-5"
display_name = "Claude Reasoning"
capabilities = ["reasoning", "extraction"]
deployment = "cloud"
locale = "zh-CN"
priority = 90
enabled = true

[[models.catalog]]
alias = "chatgpt_vision"
provider = "openai"
remote_model_id = "gpt-4.1-mini"
display_name = "ChatGPT Vision"
capabilities = ["vision"]
deployment = "cloud"
locale = "zh-CN"
priority = 80
enabled = true

[[models.catalog]]
alias = "chatgpt_embedding"
provider = "openai"
remote_model_id = "text-embedding-3-large"
display_name = "ChatGPT Embedding"
capabilities = ["embedding"]
deployment = "cloud"
locale = "zh-CN"
priority = 100
enabled = true

[models.routing.reasoning]
primary = "chatgpt_reasoning"
fallbacks = ["claude_reasoning"]

[models.routing.extraction]
primary = "chatgpt_reasoning"
fallbacks = ["claude_reasoning"]

[models.routing.vision]
primary = "chatgpt_vision"
fallbacks = []

[models.routing.embedding]
primary = "chatgpt_embedding"
fallbacks = []
"#
        )
    }

    fn temp_config_path(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be valid")
            .as_nanos();
        env::temp_dir().join(format!("meat-memory-{label}-{unique}.toml"))
    }
}
