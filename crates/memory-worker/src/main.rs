use anyhow::Result;
use memory_config::AppConfig;
use memory_worker::{JobKind, default_poll_interval_secs, queue_name};
use std::future::Future;
use std::time::Duration;
use tracing::info;

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkerRuntimeSnapshot {
    sync_mode: String,
    sync_node_id: String,
    sync_state_path: String,
    poll_interval_secs: u64,
    projection_queue: &'static str,
    embedding_queue: &'static str,
    sync_queue: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkerLoopEvent {
    Shutdown,
    Heartbeat,
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::load()?;
    memory_observability::init(&config.logging.level, &config.logging.format)?;
    validate_model_registry(&config)?;

    let snapshot = runtime_snapshot(&config);
    log_worker_started(&snapshot);
    run_worker_loop(&snapshot).await;

    Ok(())
}

fn runtime_snapshot(config: &AppConfig) -> WorkerRuntimeSnapshot {
    WorkerRuntimeSnapshot {
        sync_mode: config.sync.mode.clone(),
        sync_node_id: config.sync.node_id.clone(),
        sync_state_path: config.sync.state_path.clone(),
        poll_interval_secs: default_poll_interval_secs(),
        projection_queue: queue_name(JobKind::BuildProjection),
        embedding_queue: queue_name(JobKind::EmbedContent),
        sync_queue: queue_name(JobKind::SyncBatch),
    }
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

fn log_worker_started(snapshot: &WorkerRuntimeSnapshot) {
    info!(
        sync_mode = %snapshot.sync_mode,
        sync_node_id = %snapshot.sync_node_id,
        sync_state_path = %snapshot.sync_state_path,
        poll_interval_secs = snapshot.poll_interval_secs,
        projection_queue = snapshot.projection_queue,
        embedding_queue = snapshot.embedding_queue,
        sync_queue = snapshot.sync_queue,
        "memory worker started"
    );
}

async fn run_worker_loop(snapshot: &WorkerRuntimeSnapshot) {
    let poll_interval = Duration::from_secs(snapshot.poll_interval_secs);

    loop {
        let event =
            next_loop_event(tokio::signal::ctrl_c(), tokio::time::sleep(poll_interval)).await;
        if handle_loop_event(snapshot, event) {
            break;
        }
    }
}

async fn next_loop_event<S, T>(shutdown: S, tick: T) -> WorkerLoopEvent
where
    S: Future,
    T: Future,
{
    tokio::select! {
        _ = shutdown => WorkerLoopEvent::Shutdown,
        _ = tick => WorkerLoopEvent::Heartbeat,
    }
}

fn handle_loop_event(snapshot: &WorkerRuntimeSnapshot, event: WorkerLoopEvent) -> bool {
    match event {
        WorkerLoopEvent::Shutdown => {
            info!("memory worker received shutdown signal");
            true
        }
        WorkerLoopEvent::Heartbeat => {
            info!(
                sync_node_id = %snapshot.sync_node_id,
                sync_state_path = %snapshot.sync_state_path,
                projection_queue = snapshot.projection_queue,
                embedding_queue = snapshot.embedding_queue,
                sync_queue = snapshot.sync_queue,
                "memory worker heartbeat"
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        WorkerLoopEvent, handle_loop_event, log_worker_started, next_loop_event, runtime_snapshot,
        validate_model_registry,
    };
    use memory_config::AppConfig;
    use std::future::{pending, ready};

    fn sample_config() -> AppConfig {
        AppConfig::from_toml_str(
            r#"
[server]
bind = "127.0.0.1:18080"
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
mode = "local_only"
node_id = "node-worker-test"
state_path = "./storage/sync/worker-test.json"

[features]
enable_pg = false
enable_markdown = true
enable_http = true
enable_mcp = false

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
"#,
        )
        .expect("config should parse")
    }

    #[test]
    fn runtime_snapshot_uses_expected_queues_and_poll_interval() {
        let snapshot = runtime_snapshot(&sample_config());

        assert_eq!(snapshot.sync_mode, "local_only");
        assert_eq!(snapshot.sync_node_id, "node-worker-test");
        assert_eq!(snapshot.sync_state_path, "./storage/sync/worker-test.json");
        assert_eq!(snapshot.poll_interval_secs, 5);
        assert_eq!(snapshot.projection_queue, "projection");
        assert_eq!(snapshot.embedding_queue, "embedding");
        assert_eq!(snapshot.sync_queue, "sync");
    }

    #[test]
    fn validate_model_registry_accepts_valid_config() {
        validate_model_registry(&sample_config()).expect("registry should validate");
    }

    #[test]
    fn validate_model_registry_rejects_invalid_config() {
        let config = AppConfig::from_toml_str(
            r#"
[server]
bind = "127.0.0.1:18080"
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
mode = "local_only"

[features]
enable_pg = false
enable_markdown = true
enable_http = true
enable_mcp = false

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
primary = "missing_vision"
fallbacks = []

[models.routing.embedding]
primary = "chatgpt_embedding"
fallbacks = []
"#,
        )
        .expect("config should parse");

        assert!(validate_model_registry(&config).is_err());
    }

    #[test]
    fn handle_loop_event_returns_expected_control_flow() {
        let snapshot = runtime_snapshot(&sample_config());
        log_worker_started(&snapshot);

        assert!(!handle_loop_event(&snapshot, WorkerLoopEvent::Heartbeat));
        assert!(handle_loop_event(&snapshot, WorkerLoopEvent::Shutdown));
    }

    #[tokio::test]
    async fn next_loop_event_prefers_ready_shutdown() {
        let event = next_loop_event(ready(()), pending::<()>()).await;
        assert_eq!(event, WorkerLoopEvent::Shutdown);
    }

    #[tokio::test]
    async fn next_loop_event_prefers_ready_heartbeat() {
        let event = next_loop_event(pending::<()>(), ready(())).await;
        assert_eq!(event, WorkerLoopEvent::Heartbeat);
    }
}
