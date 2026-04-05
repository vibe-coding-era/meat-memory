use anyhow::Result;
use memory_config::AppConfig;
use memory_worker::{JobKind, default_poll_interval_secs, queue_name};
use tokio::time::{Duration, sleep};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::load()?;
    memory_observability::init(&config.logging.level, &config.logging.format)?;
    let registry = config.model_registry()?;

    info!(
        sync_mode = %config.sync.mode,
        database_url = %config.postgres.database_url,
        providers = registry.provider_count(),
        models = registry.model_count(),
        routes = registry.route_count(),
        default_locale = %config.models.default_locale,
        "memory worker started"
    );

    loop {
        info!(
            projection_queue = queue_name(JobKind::BuildProjection),
            sync_queue = queue_name(JobKind::SyncBatch),
            "worker heartbeat"
        );
        sleep(Duration::from_secs(default_poll_interval_secs())).await;
    }
}
