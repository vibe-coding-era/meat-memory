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
    #[serde(default)]
    pub access: AccessConfig,
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
pub struct AccessConfig {
    #[serde(default)]
    pub require_key: bool,
    #[serde(default = "default_key_store_path")]
    pub key_store_path: String,
    #[serde(default = "default_default_key_name")]
    pub default_key_name: String,
    #[serde(default = "default_default_key_source")]
    pub default_key_source: String,
    #[serde(default = "default_default_key_scope_kind")]
    pub default_key_scope_kind: String,
    #[serde(default = "default_default_key_storage_mode")]
    pub default_key_storage_mode: String,
    #[serde(default)]
    pub default_key_isolated: bool,
}

impl Default for AccessConfig {
    fn default() -> Self {
        Self {
            require_key: false,
            key_store_path: default_key_store_path(),
            default_key_name: default_default_key_name(),
            default_key_source: default_default_key_source(),
            default_key_scope_kind: default_default_key_scope_kind(),
            default_key_storage_mode: default_default_key_storage_mode(),
            default_key_isolated: false,
        }
    }
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

fn default_key_store_path() -> String {
    "./storage/keys/default-key.toml".to_string()
}

fn default_default_key_name() -> String {
    "default".to_string()
}

fn default_default_key_source() -> String {
    "tui".to_string()
}

fn default_default_key_scope_kind() -> String {
    "personal".to_string()
}

fn default_default_key_storage_mode() -> String {
    "all".to_string()
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let path = env::var("MEAT_MEMORY_CONFIG").unwrap_or_else(|_| "config/app.toml".to_string());
        Self::from_file(PathBuf::from(path))
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file {}", path.display()))?;
        let mut config = Self::from_toml_str(&raw)
            .with_context(|| format!("failed to parse config file {}", path.display()))?;
        config.apply_env_overrides();
        Ok(config)
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

    fn apply_env_overrides(&mut self) {
        set_if_env_present("MEAT_MEMORY_SERVER_BIND", &mut self.server.bind);
        set_if_env_present("MEAT_MEMORY_LOG_LEVEL", &mut self.logging.level);
        set_if_env_present("MEAT_MEMORY_LOG_FORMAT", &mut self.logging.format);
        set_if_env_present("MEAT_MEMORY_MARKDOWN_ROOT", &mut self.markdown.root);
        set_if_env_present("MEAT_MEMORY_DATABASE_URL", &mut self.postgres.database_url);
        set_if_env_present("MEAT_MEMORY_ASSETS_ROOT", &mut self.assets.root);
        set_if_env_present(
            "MEAT_MEMORY_KEY_STORE_PATH",
            &mut self.access.key_store_path,
        );
        set_if_env_present(
            "MEAT_MEMORY_DEFAULT_KEY_NAME",
            &mut self.access.default_key_name,
        );
        set_if_env_present(
            "MEAT_MEMORY_DEFAULT_KEY_SOURCE",
            &mut self.access.default_key_source,
        );
        set_if_env_present(
            "MEAT_MEMORY_DEFAULT_KEY_SCOPE_KIND",
            &mut self.access.default_key_scope_kind,
        );
        set_if_env_present(
            "MEAT_MEMORY_DEFAULT_KEY_STORAGE_MODE",
            &mut self.access.default_key_storage_mode,
        );
        set_if_env_present("MEAT_MEMORY_SYNC_MODE", &mut self.sync.mode);
        set_if_env_present("MEAT_MEMORY_SYNC_NODE_ID", &mut self.sync.node_id);
        set_if_env_present("MEAT_MEMORY_SYNC_STATE_PATH", &mut self.sync.state_path);
        set_if_env_present(
            "MEAT_MEMORY_DEFAULT_LOCALE",
            &mut self.models.default_locale,
        );
        set_bool_if_env_present("MEAT_MEMORY_ENABLE_PG", &mut self.features.enable_pg);
        set_bool_if_env_present(
            "MEAT_MEMORY_ENABLE_MARKDOWN",
            &mut self.features.enable_markdown,
        );
        set_bool_if_env_present("MEAT_MEMORY_ENABLE_HTTP", &mut self.features.enable_http);
        set_bool_if_env_present("MEAT_MEMORY_ENABLE_MCP", &mut self.features.enable_mcp);
        set_bool_if_env_present("MEAT_MEMORY_REQUIRE_KEY", &mut self.access.require_key);
        set_bool_if_env_present(
            "MEAT_MEMORY_DEFAULT_KEY_ISOLATED",
            &mut self.access.default_key_isolated,
        );
    }
}

fn set_if_env_present(name: &str, target: &mut String) {
    if let Ok(value) = env::var(name) {
        if !value.trim().is_empty() {
            *target = value;
        }
    }
}

fn set_bool_if_env_present(name: &str, target: &mut bool) {
    if let Ok(value) = env::var(name) {
        if let Some(parsed) = parse_env_bool(&value) {
            *target = parsed;
        }
    }
}

fn parse_env_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" | "on" => Some(true),
        "0" | "false" | "no" | "n" | "off" => Some(false),
        _ => None,
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
        sync::Mutex,
        time::{SystemTime, UNIX_EPOCH},
    };

    static ENV_LOCK: Mutex<()> = Mutex::new(());

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
        let _guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
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
        assert_eq!(loaded.access.default_key_storage_mode, "all");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn from_file_applies_environment_overrides() {
        let _guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let path = temp_config_path("env-overrides");
        fs::write(&path, sample_toml(true)).expect("config file should be written");

        unsafe {
            env::set_var("MEAT_MEMORY_SERVER_BIND", "0.0.0.0:9090");
            env::set_var("MEAT_MEMORY_DATABASE_URL", "postgres://override/db");
            env::set_var("MEAT_MEMORY_MARKDOWN_ROOT", "/data/markdown");
            env::set_var("MEAT_MEMORY_ASSETS_ROOT", "/data/assets");
            env::set_var("MEAT_MEMORY_ENABLE_MCP", "true");
            env::set_var("MEAT_MEMORY_DEFAULT_LOCALE", "en-US");
        }
        let config = AppConfig::from_file(&path).expect("config should load with env overrides");
        unsafe {
            env::remove_var("MEAT_MEMORY_SERVER_BIND");
            env::remove_var("MEAT_MEMORY_DATABASE_URL");
            env::remove_var("MEAT_MEMORY_MARKDOWN_ROOT");
            env::remove_var("MEAT_MEMORY_ASSETS_ROOT");
            env::remove_var("MEAT_MEMORY_ENABLE_MCP");
            env::remove_var("MEAT_MEMORY_DEFAULT_LOCALE");
        }

        assert_eq!(config.server.bind, "0.0.0.0:9090");
        assert_eq!(config.postgres.database_url, "postgres://override/db");
        assert_eq!(config.markdown.root, "/data/markdown");
        assert_eq!(config.assets.root, "/data/assets");
        assert!(config.features.enable_mcp);
        assert_eq!(config.models.default_locale, "en-US");

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

[access]
require_key = false
key_store_path = "./storage/keys/default-key.toml"
default_key_name = "default"
default_key_source = "tui"
default_key_scope_kind = "personal"
default_key_storage_mode = "all"
default_key_isolated = false

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
