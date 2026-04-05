use anyhow::Result;
use memory_config::AppConfig;
use memory_worker::{JobKind, default_poll_interval_secs, queue_name};
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::load()?;
    memory_observability::init(&config.logging.level, &config.logging.format)?;
    validate_model_registry(&config)?;

    let poll_interval = Duration::from_secs(default_poll_interval_secs());
    info!(
        sync_mode = %config.sync.mode,
        poll_interval_secs = default_poll_interval_secs(),
        projection_queue = queue_name(JobKind::BuildProjection),
        embedding_queue = queue_name(JobKind::EmbedContent),
        sync_queue = queue_name(JobKind::SyncBatch),
        "memory worker started"
    );

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("memory worker received shutdown signal");
                break;
            }
            _ = tokio::time::sleep(poll_interval) => {
                info!(
                    projection_queue = queue_name(JobKind::BuildProjection),
                    embedding_queue = queue_name(JobKind::EmbedContent),
                    sync_queue = queue_name(JobKind::SyncBatch),
                    "memory worker heartbeat"
                );
            }
        }
    }

    Ok(())
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
