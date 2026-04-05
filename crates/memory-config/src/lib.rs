use anyhow::{Context, Result};
use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub markdown: MarkdownConfig,
    pub postgres: PostgresConfig,
    pub assets: AssetsConfig,
    pub sync: SyncConfig,
    pub features: FeaturesConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
    pub shutdown_grace_period_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MarkdownConfig {
    pub root: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PostgresConfig {
    pub app_name: String,
    pub database_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetsConfig {
    pub root: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SyncConfig {
    pub mode: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeaturesConfig {
    pub enable_pg: bool,
    pub enable_markdown: bool,
    pub enable_http: bool,
    pub enable_mcp: bool,
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
        toml::from_str(&raw)
            .with_context(|| format!("failed to parse config file {}", path.display()))
    }
}
