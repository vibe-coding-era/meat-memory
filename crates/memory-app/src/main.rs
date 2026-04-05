use anyhow::Result;
use memory_config::AppConfig;
use memory_core::{ServiceInfo, log_startup, startup_banner};
use memory_http::{ApiFeatureFlags, ApiMetadata, HttpAppState, build_router};
use memory_kernel::Kernel;
use std::{net::SocketAddr, sync::Arc};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::load()?;
    memory_observability::init(&config.logging.level, &config.logging.format)?;

    let service_info = ServiceInfo::default();
    log_startup(&service_info);
    println!("{}", startup_banner(&service_info));

    let kernel = Arc::new(build_kernel(&config).await?);
    let metadata = ApiMetadata {
        service: service_info.name.to_string(),
        version: service_info.version.to_string(),
        default_scope: service_info.default_scope.as_str().to_string(),
        features: ApiFeatureFlags {
            pg: config.features.enable_pg,
            markdown: config.features.enable_markdown,
            http: config.features.enable_http,
            mcp: config.features.enable_mcp,
        },
    };

    let bind = config.server.bind.parse::<SocketAddr>()?;
    let app = build_router(HttpAppState::new(
        service_info.default_scope.clone(),
        metadata,
        kernel,
    ));
    info!(bind = %bind, "starting http server");

    let listener = tokio::net::TcpListener::bind(bind).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn build_kernel(config: &AppConfig) -> Result<Kernel> {
    let mut builder = Kernel::builder();
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

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
