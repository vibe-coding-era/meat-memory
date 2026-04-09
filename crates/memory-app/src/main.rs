use anyhow::Result;
use memory_config::AppConfig;
use memory_core::{ServiceInfo, log_startup, startup_banner};
use memory_http::{ApiFeatureFlags, ApiMetadata, HttpAppState, build_router};
use memory_kernel::Kernel;
use memory_mcp::McpServer;
use std::{net::SocketAddr, sync::Arc};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::load()?;
    memory_observability::init(&config.logging.level, &config.logging.format)?;
    validate_model_registry(&config)?;

    let service_info = ServiceInfo::default();
    log_startup(&service_info);
    println!("{}", startup_banner(&service_info));

    let kernel = Arc::new(build_kernel(&config).await?);
    let bind = config.server.bind.parse::<SocketAddr>()?;
    let app = build_app_router(&config, &service_info, kernel);
    info!(bind = %bind, "starting http server");

    let listener = tokio::net::TcpListener::bind(bind).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn build_kernel(config: &AppConfig) -> Result<Kernel> {
    let mut builder = Kernel::builder()
        .with_asset_root(&config.assets.root)?
        .with_model_registry(
            config.model_registry()?,
            config.models.default_locale.clone(),
        )?;
    if config.features.enable_pg {
        builder = builder
            .with_postgres_url(&config.postgres.database_url)
            .await?;
    }
    if config.features.enable_markdown {
        builder = builder.with_markdown_root(&config.markdown.root)?;
    }
    builder.build()
}

fn api_metadata(config: &AppConfig, service_info: &ServiceInfo) -> ApiMetadata {
    ApiMetadata {
        service: service_info.name.to_string(),
        version: service_info.version.to_string(),
        default_scope: service_info.default_scope.as_str().to_string(),
        features: ApiFeatureFlags {
            pg: config.features.enable_pg,
            markdown: config.features.enable_markdown,
            http: config.features.enable_http,
            mcp: config.features.enable_mcp,
        },
    }
}

fn build_app_router(
    config: &AppConfig,
    service_info: &ServiceInfo,
    kernel: Arc<Kernel>,
) -> axum::Router {
    let metadata = api_metadata(config, service_info);
    let mut app = build_router(HttpAppState::new(
        service_info.default_scope.clone(),
        metadata,
        kernel.clone(),
    ));
    if config.features.enable_mcp {
        let mcp = McpServer::new(
            service_info.default_scope.clone(),
            service_info.name,
            service_info.version,
            kernel,
        );
        app = app.merge(memory_mcp::build_router(mcp));
    }
    app
}

fn validate_model_registry(config: &AppConfig) -> Result<()> {
    let registry = config.model_registry()?;
    info!(
        providers = registry.provider_count(),
        models = registry.model_count(),
        routes = registry.route_count(),
        default_locale = %config.models.default_locale,
        "validated model registry"
    );
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use super::{api_metadata, build_app_router, build_kernel, validate_model_registry};
    use axum::{body::Body, http::Request};
    use memory_config::AppConfig;
    use memory_core::ServiceInfo;
    use std::{fs, sync::Arc};
    use tempfile::tempdir;
    use tower::ServiceExt;

    fn sample_config(markdown_root: &str, asset_root: &str, enable_mcp: bool) -> AppConfig {
        AppConfig::from_toml_str(&format!(
            r#"
[server]
bind = "127.0.0.1:18080"
shutdown_grace_period_secs = 10

[logging]
level = "info"
format = "pretty"

[markdown]
root = "{markdown_root}"

[postgres]
app_name = "meat-memory"
database_url = "postgres://postgres:postgres@127.0.0.1:5433/meat_memory_dev"

[assets]
root = "{asset_root}"

[sync]
mode = "local_only"

[features]
enable_pg = false
enable_markdown = true
enable_http = true
enable_mcp = {enable_mcp}

[models]
default_locale = "zh-CN"

[[models.providers]]
provider = "openai"
display_name = "ChatGPT"
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"
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
priority = 70
enabled = true

[models.routing.reasoning]
primary = "chatgpt_reasoning"
fallbacks = []

[models.routing.extraction]
primary = "chatgpt_reasoning"
fallbacks = []

[models.routing.vision]
primary = "chatgpt_vision"
fallbacks = []

[models.routing.embedding]
primary = "chatgpt_embedding"
fallbacks = []
"#
        ))
        .expect("config should parse")
    }

    #[test]
    fn api_metadata_reflects_feature_flags() {
        let config = sample_config("/tmp/md", "/tmp/assets", true);
        let metadata = api_metadata(&config, &ServiceInfo::default());

        assert_eq!(metadata.service, "meat-memory");
        assert!(metadata.features.markdown);
        assert!(metadata.features.http);
        assert!(metadata.features.mcp);
        assert!(!metadata.features.pg);
    }

    #[test]
    fn validate_model_registry_accepts_valid_config() {
        let config = sample_config("/tmp/md", "/tmp/assets", false);

        validate_model_registry(&config).expect("registry should validate");
    }

    #[tokio::test]
    async fn build_kernel_enables_local_backends_without_postgres() {
        let tempdir = tempdir().expect("tempdir should build");
        let markdown_root = tempdir.path().join("markdown");
        let asset_root = tempdir.path().join("assets");
        fs::create_dir_all(&markdown_root).expect("markdown dir should exist");
        fs::create_dir_all(&asset_root).expect("asset dir should exist");
        let config = sample_config(
            &markdown_root.display().to_string(),
            &asset_root.display().to_string(),
            false,
        );

        let kernel = build_kernel(&config).await.expect("kernel should build");

        assert!(!kernel.has_postgres());
        assert!(kernel.has_markdown());
        assert!(kernel.has_asset_store());
        assert!(kernel.has_vision_gateway());
    }

    #[tokio::test]
    async fn build_app_router_merges_mcp_routes_only_when_enabled() {
        let tempdir = tempdir().expect("tempdir should build");
        let markdown_root = tempdir.path().join("markdown");
        let asset_root = tempdir.path().join("assets");
        fs::create_dir_all(&markdown_root).expect("markdown dir should exist");
        fs::create_dir_all(&asset_root).expect("asset dir should exist");

        let config_with_mcp = sample_config(
            &markdown_root.display().to_string(),
            &asset_root.display().to_string(),
            true,
        );
        let kernel_with_mcp = Arc::new(
            build_kernel(&config_with_mcp)
                .await
                .expect("kernel should build"),
        );
        let app_with_mcp =
            build_app_router(&config_with_mcp, &ServiceInfo::default(), kernel_with_mcp);
        let mcp_response = app_with_mcp
            .oneshot(
                Request::get("/mcp/tools")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router should respond");
        assert_eq!(mcp_response.status(), axum::http::StatusCode::OK);

        let config_without_mcp = sample_config(
            &markdown_root.display().to_string(),
            &asset_root.display().to_string(),
            false,
        );
        let kernel_without_mcp = Arc::new(
            build_kernel(&config_without_mcp)
                .await
                .expect("kernel should build"),
        );
        let app_without_mcp = build_app_router(
            &config_without_mcp,
            &ServiceInfo::default(),
            kernel_without_mcp,
        );
        let healthz = app_without_mcp
            .clone()
            .oneshot(
                Request::get("/healthz")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router should respond");
        let mcp_missing = app_without_mcp
            .oneshot(
                Request::get("/mcp/tools")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router should respond");

        assert_eq!(healthz.status(), axum::http::StatusCode::OK);
        assert_eq!(mcp_missing.status(), axum::http::StatusCode::NOT_FOUND);
    }
}
