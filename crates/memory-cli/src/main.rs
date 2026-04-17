use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use memory_config::AppConfig;
use memory_core::{ServiceInfo, log_startup, startup_banner};
use memory_domain::{
    AccessKeyId, AgentContextId, ArtifactKind, ContextBundle, DocumentConflictState,
    DocumentSyncState, KeyScopeKind, KeySourceKind, MemoryKind, MemorySource, RequestContext,
    ScopeId, Sensitivity, SourceId, SourceSyncMode, StorageMode, Visibility,
};
use memory_http::{ApiFeatureFlags, ApiMetadata, HttpAppState, build_router};
use memory_kernel::{
    ApplyProjectDocumentSyncPlanRequest, CreateAccessKeyRequest, ImportProjectDocumentRequest,
    Kernel, ListAgentContextsRequest, ListProjectDocumentsRequest, PromoteAgentContextRequest,
    RememberImageRequest, RememberImageResult, RememberTextRequest, RememberTextResult,
    SearchContextRequest, UpsertAgentContextRequest,
};
use memory_mcp::{McpServer, TOOL_SPECS};
use memory_models::{CapabilityRoute, ModelCapability};
use memory_store_pg::PgStore;
use memory_sync::{LocalProjectDocumentSyncEngine, ProjectDocumentSnapshot};
use serde_json::json;
use std::{
    env, fs,
    future::Future,
    io::{self, BufRead, Read, Write},
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

#[derive(Debug, Clone)]
struct DefaultKeyMaterial {
    key_id: String,
    raw_key: String,
    path: String,
}

#[derive(Debug, Parser)]
#[command(
    name = "memory-cli",
    about = "Local operator entrypoint for Meat Memory"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Doctor,
    PrintPlan,
    Config(ConfigArgs),
    Mcp(McpArgs),
    Skills(SkillsArgs),
    Key(KeyArgs),
    Source(SourceArgs),
    Project(ProjectArgs),
    Context(ContextArgs),
    Docs(DocsArgs),
    Tui(TuiArgs),
    Serve(ServeArgs),
    Remember(RememberArgs),
    RememberImage(RememberImageArgs),
    Search(SearchArgs),
}

#[derive(Debug, Clone, Args)]
struct ConfigArgs {
    #[command(subcommand)]
    command: ConfigCommand,
}

#[derive(Debug, Clone, Subcommand)]
enum ConfigCommand {
    Show(InspectArgs),
    Check(CheckArgs),
}

#[derive(Debug, Clone, Args)]
struct McpArgs {
    #[command(subcommand)]
    command: McpCommand,
}

#[derive(Debug, Clone, Args)]
struct SkillsArgs {
    #[command(subcommand)]
    command: SkillsCommand,
}

#[derive(Debug, Clone, Args)]
struct KeyArgs {
    #[command(subcommand)]
    command: KeyCommand,
}

#[derive(Debug, Clone, Args)]
struct SourceArgs {
    #[command(subcommand)]
    command: SourceCommand,
}

#[derive(Debug, Clone, Args)]
struct ProjectArgs {
    #[command(subcommand)]
    command: ProjectCommand,
}

#[derive(Debug, Clone, Args)]
struct ContextArgs {
    #[command(subcommand)]
    command: ContextCommand,
}

#[derive(Debug, Clone, Args)]
struct DocsArgs {
    #[command(subcommand)]
    command: DocsCommand,
}

#[derive(Debug, Clone, Subcommand)]
enum KeyCommand {
    Create(KeyCreateArgs),
    List(KeyListArgs),
    Rotate(KeyRotateArgs),
    Use(KeyUseArgs),
    Stats(KeyStatsArgs),
}

#[derive(Debug, Clone, Subcommand)]
enum SourceCommand {
    Create(SourceCreateArgs),
    List(SourceListArgs),
    Keys(SourceKeysArgs),
    KeyCreate(SourceKeyCreateArgs),
}

#[derive(Debug, Clone, Subcommand)]
enum ProjectCommand {
    Init(ProjectInitArgs),
}

#[derive(Debug, Clone, Subcommand)]
enum ContextCommand {
    Upsert(ContextUpsertArgs),
    List(ContextListArgs),
    Promote(ContextPromoteArgs),
    Delete(ContextDeleteArgs),
}

#[derive(Debug, Clone, Subcommand)]
enum DocsCommand {
    Import(DocsImportArgs),
    List(DocsListArgs),
    Projection(DocsProjectionArgs),
    Conflicts(DocsConflictsArgs),
    Sync(DocsSyncArgs),
    Status(DocsStatusArgs),
}

#[derive(Debug, Clone, Args)]
struct TuiArgs {
    #[command(subcommand)]
    command: TuiCommand,
}

#[derive(Debug, Clone, Subcommand)]
enum TuiCommand {
    Init(TuiInitArgs),
    KeyCreate(KeyCreateArgs),
    ProjectInit(ProjectInitArgs),
}

#[derive(Debug, Clone, Subcommand)]
enum SkillsCommand {
    Export(SkillsExportArgs),
}

#[derive(Debug, Clone, Args)]
struct KeyCreateArgs {
    #[arg(long)]
    name: String,
    #[arg(long, default_value = "cli")]
    source: String,
    #[arg(long, default_value = "local-user")]
    owner_principal_id: String,
    #[arg(long)]
    owner_scope_id: Option<String>,
    #[arg(long, default_value = "personal")]
    scope_kind: String,
    #[arg(long, default_value = "all")]
    storage: String,
    #[arg(long)]
    isolated: bool,
    #[arg(long)]
    raw_key: Option<String>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct KeyListArgs {
    #[arg(long, default_value_t = 200)]
    limit: usize,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct KeyUseArgs {
    #[arg(long)]
    raw_key: String,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long)]
    force: bool,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct KeyRotateArgs {
    #[arg(long)]
    key_id: String,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct KeyStatsArgs {
    #[arg(long)]
    key_id: Option<String>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct SourceCreateArgs {
    #[arg(long)]
    key: Option<String>,
    #[arg(long)]
    name: String,
    #[arg(long, default_value = "custom")]
    source_kind: String,
    #[arg(long)]
    source_uri: Option<String>,
    #[arg(long, default_value = "read_only")]
    sync_mode: String,
    #[arg(long)]
    local_root: Option<String>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct SourceListArgs {
    #[arg(long)]
    key: Option<String>,
    #[arg(long, default_value_t = 100)]
    limit: usize,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct SourceKeysArgs {
    #[arg(long)]
    key: Option<String>,
    #[arg(long)]
    source_id: String,
    #[arg(long, default_value_t = 100)]
    limit: usize,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct SourceKeyCreateArgs {
    #[arg(long)]
    key: Option<String>,
    #[arg(long)]
    source_id: String,
    #[arg(long)]
    name: String,
    #[arg(long, default_value = "custom")]
    source: String,
    #[arg(long, default_value = "personal")]
    scope_kind: String,
    #[arg(long, default_value = "all")]
    storage: String,
    #[arg(long)]
    isolated: bool,
    #[arg(long)]
    raw_key: Option<String>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct ProjectInitArgs {
    #[arg(long)]
    json: bool,
    #[arg(long)]
    interactive: bool,
    #[arg(long)]
    existing: bool,
    #[arg(long)]
    name: Option<String>,
    #[arg(long, default_value = "local-user")]
    owner_principal_id: String,
    #[arg(long, visible_alias = "scope-id")]
    owner_scope_id: Option<String>,
    #[arg(long, default_value = "team")]
    scope_kind: String,
    #[arg(long, default_value = "all")]
    storage: String,
    #[arg(long, conflicts_with = "shared")]
    isolated: bool,
    #[arg(long, conflicts_with = "isolated")]
    shared: bool,
    #[arg(long, default_value_t = 20)]
    list_limit: usize,
}

#[derive(Debug, Clone, Args)]
struct ContextUpsertArgs {
    #[arg(long)]
    key: Option<String>,
    #[arg(long)]
    scope_id: Option<String>,
    #[arg(long)]
    session_id: String,
    #[arg(long)]
    task_id: Option<String>,
    #[arg(long)]
    title: String,
    #[arg(long, conflicts_with = "file")]
    body: Option<String>,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long, value_delimiter = ',')]
    labels: Vec<String>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct ContextListArgs {
    #[arg(long)]
    key: Option<String>,
    #[arg(long)]
    scope_id: Option<String>,
    #[arg(long)]
    session_id: String,
    #[arg(long)]
    task_id: Option<String>,
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct ContextPromoteArgs {
    #[arg(long)]
    key: Option<String>,
    #[arg(long)]
    context_id: String,
    #[arg(long)]
    memory_kind: Option<String>,
    #[arg(long, default_value = "private")]
    visibility: String,
    #[arg(long, default_value = "internal")]
    sensitivity: String,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct ContextDeleteArgs {
    #[arg(long)]
    key: Option<String>,
    #[arg(long)]
    context_id: String,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct DocsImportArgs {
    #[arg(long)]
    key: Option<String>,
    #[arg(long)]
    source_id: String,
    #[arg(long)]
    scope_id: Option<String>,
    #[arg(long)]
    canonical_uri: String,
    #[arg(long)]
    title: String,
    #[arg(long, conflicts_with = "file")]
    body: Option<String>,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    local_path: Option<String>,
    #[arg(long, default_value = "clean")]
    sync_state: String,
    #[arg(long, default_value = "none")]
    conflict_state: String,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct DocsListArgs {
    #[arg(long)]
    key: Option<String>,
    #[arg(long)]
    source_id: String,
    #[arg(long)]
    query: Option<String>,
    #[arg(long, default_value_t = 50)]
    limit: usize,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct DocsProjectionArgs {
    #[arg(long)]
    key: Option<String>,
    #[arg(long)]
    source_id: String,
    #[arg(long)]
    document_id: String,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct DocsConflictsArgs {
    #[arg(long)]
    key: Option<String>,
    #[arg(long)]
    source_id: String,
    #[arg(long, default_value_t = 50)]
    limit: usize,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct DocsSyncArgs {
    #[arg(long)]
    key: Option<String>,
    #[arg(long)]
    source_id: String,
    #[arg(long)]
    scope_id: Option<String>,
    #[arg(long)]
    local_root: Option<String>,
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct DocsStatusArgs {
    #[arg(long)]
    key: Option<String>,
    #[arg(long)]
    source_id: String,
    #[arg(long)]
    local_root: Option<String>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Subcommand)]
enum McpCommand {
    Info(InspectArgs),
}

#[derive(Debug, Clone, Args)]
struct InspectArgs {
    #[arg(long)]
    json: bool,
    #[arg(long)]
    check_http: bool,
}

#[derive(Debug, Clone, Args)]
struct CheckArgs {
    #[arg(long)]
    json: bool,
    #[arg(long)]
    database: bool,
}

#[derive(Debug, Clone, Args)]
struct SkillsExportArgs {
    #[arg(long, value_enum, default_value_t = SkillExportTarget::All)]
    target: SkillExportTarget,
    #[arg(long)]
    output_dir: Option<PathBuf>,
    #[arg(long)]
    force: bool,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum SkillExportTarget {
    Codex,
    ClaudeCode,
    ExecutionAgent,
    All,
}

#[derive(Debug, Clone, Args)]
struct TuiInitArgs {
    #[arg(long)]
    json: bool,
    #[arg(long)]
    interactive: bool,
    #[arg(long)]
    default_locale: Option<String>,
    #[arg(long)]
    write_config: Option<PathBuf>,
    #[arg(long)]
    force: bool,
    #[arg(long)]
    check_database: bool,
    #[arg(long, conflicts_with = "disable_mcp")]
    enable_mcp: bool,
    #[arg(long, conflicts_with = "enable_mcp")]
    disable_mcp: bool,
    #[arg(long)]
    database_url: Option<String>,
    #[arg(long)]
    markdown_root: Option<String>,
    #[arg(long)]
    assets_root: Option<String>,
    #[arg(long)]
    reasoning_primary: Option<String>,
    #[arg(long)]
    extraction_primary: Option<String>,
    #[arg(long)]
    vision_primary: Option<String>,
    #[arg(long)]
    embedding_primary: Option<String>,
}

#[derive(Debug, Clone, Args)]
struct ServeArgs {
    #[arg(long)]
    bind: Option<String>,
}

#[derive(Debug, Clone, Args)]
struct RememberArgs {
    #[arg(long)]
    key: Option<String>,
    #[arg(long)]
    scope_id: Option<String>,
    #[arg(long)]
    title: Option<String>,
    #[arg(long, conflicts_with = "file")]
    body: Option<String>,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long, default_value = "message")]
    artifact_kind: String,
    #[arg(long)]
    memory_kind: Option<String>,
    #[arg(long, value_delimiter = ',')]
    source_refs: Vec<String>,
    #[arg(long, default_value = "private")]
    visibility: String,
    #[arg(long, default_value = "internal")]
    sensitivity: String,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct RememberImageArgs {
    #[arg(long)]
    key: Option<String>,
    #[arg(long)]
    scope_id: Option<String>,
    #[arg(long)]
    title: Option<String>,
    #[arg(long)]
    body: Option<String>,
    #[arg(long)]
    file: PathBuf,
    #[arg(long)]
    media_type: Option<String>,
    #[arg(long)]
    memory_kind: Option<String>,
    #[arg(long, value_delimiter = ',')]
    source_refs: Vec<String>,
    #[arg(long, default_value = "private")]
    visibility: String,
    #[arg(long, default_value = "internal")]
    sensitivity: String,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct SearchArgs {
    query: String,
    #[arg(long)]
    key: Option<String>,
    #[arg(long)]
    scope_id: Option<String>,
    #[arg(long, default_value_t = 10)]
    limit: usize,
    #[arg(long)]
    json: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(message) = static_command_message(&cli.command) {
        println!("{message}");
        return Ok(());
    }

    match cli.command {
        Command::Doctor | Command::PrintPlan => unreachable!("handled above"),
        Command::Config(args) => config_command(args).await,
        Command::Mcp(args) => mcp_command(args),
        Command::Skills(args) => skills_command(args),
        Command::Key(args) => key_command(args).await,
        Command::Source(args) => source_command(args).await,
        Command::Project(args) => project_command(args).await,
        Command::Context(args) => context_command(args).await,
        Command::Docs(args) => docs_command(args).await,
        Command::Tui(args) => tui_command(args).await,
        Command::Serve(args) => serve_command(args).await,
        Command::Remember(args) => remember_command(args).await,
        Command::RememberImage(args) => remember_image_command(args).await,
        Command::Search(args) => search_command(args).await,
    }
}

async fn key_command(args: KeyArgs) -> Result<()> {
    match args.command {
        KeyCommand::Create(create) => {
            let (_, kernel, service_info) = bootstrap_runtime().await?;
            let result = kernel
                .create_access_key(CreateAccessKeyRequest {
                    raw_key: create.raw_key,
                    display_name: create.name,
                    source_id: None,
                    source_kind: parse_key_source(&create.source)?,
                    owner_principal_id: create.owner_principal_id,
                    owner_scope_id: create
                        .owner_scope_id
                        .map(ScopeId::from_string)
                        .unwrap_or_else(|| service_info.default_scope.clone()),
                    scope_kind: parse_key_scope(&create.scope_kind)?,
                    storage_mode: parse_storage_mode(&create.storage)?,
                    is_fully_isolated: create.isolated,
                })
                .await?;
            let payload = access_key_json(&result.access_key, Some(&result.raw_key));
            if create.json {
                print_json(payload)?;
            } else {
                println!("Created key {}", result.access_key.id.as_str());
                println!("Name: {}", result.access_key.display_name);
                println!("Source: {}", result.access_key.source_kind.as_str());
                println!("Scope: {}", result.access_key.scope_kind.as_str());
                println!("Storage: {}", result.access_key.storage_mode.as_str());
                println!("Raw key: {}", result.raw_key);
            }
        }
        KeyCommand::List(list) => {
            let (_, kernel, _) = bootstrap_runtime().await?;
            let keys = kernel.list_access_keys(list.limit).await?;
            if list.json {
                print_json(json!({
                    "keys": keys.iter().map(|key| access_key_json(key, None)).collect::<Vec<_>>(),
                }))?;
            } else {
                println!("Meat Memory keys");
                for key in keys {
                    println!(
                        "- {} {} source={} scope={} storage={} isolated={}",
                        key.id.as_str(),
                        key.display_name,
                        key.source_kind.as_str(),
                        key.scope_kind.as_str(),
                        key.storage_mode.as_str(),
                        key.is_fully_isolated
                    );
                }
            }
        }
        KeyCommand::Rotate(rotate) => {
            let (_, kernel, _) = bootstrap_runtime().await?;
            let result = kernel
                .rotate_access_key(&AccessKeyId::from_string(rotate.key_id), None)
                .await?;
            let payload = access_key_json(&result.access_key, Some(&result.raw_key));
            if rotate.json {
                print_json(payload)?;
            } else {
                println!("Rotated key {}", result.access_key.id.as_str());
                println!("Raw key: {}", result.raw_key);
            }
        }
        KeyCommand::Use(use_args) => {
            let path = use_args
                .output
                .clone()
                .unwrap_or_else(|| PathBuf::from(".meat-memory-key"));
            if path.exists() && !use_args.force {
                bail!(
                    "{} already exists; pass --force to overwrite",
                    path.display()
                );
            }
            fs::write(&path, format!("MEAT_MEMORY_KEY={}\n", use_args.raw_key))
                .with_context(|| format!("failed to write {}", path.display()))?;
            if use_args.json {
                print_json(json!({
                    "path": path.display().to_string(),
                    "env": "MEAT_MEMORY_KEY",
                }))?;
            } else {
                println!("Wrote MEAT_MEMORY_KEY to {}", path.display());
            }
        }
        KeyCommand::Stats(stats) => {
            if let Some(key_id) = stats.key_id.as_ref() {
                let (_, kernel, _) = bootstrap_runtime().await?;
                let payload = kernel
                    .access_key_usage_stats(&AccessKeyId::from_string(key_id.clone()))
                    .await?;
                if stats.json {
                    print_json(json!({ "stats": payload }))?;
                } else if let Some(stats) = payload {
                    println!("Meat Memory key usage stats");
                    println!("Key: {}", stats.key_id.as_str());
                    println!("Operations: {}", stats.total_operations);
                    println!("Successful: {}", stats.successful_operations);
                    println!("Failed: {}", stats.failed_operations);
                    println!("Avg latency: {:.1} ms", stats.avg_latency_ms);
                    println!("P95 latency: {} ms", stats.p95_latency_ms);
                } else {
                    bail!("access key not found");
                }
            } else {
                let snapshot = memory_observability::metrics_snapshot();
                if stats.json {
                    print_json(json!({ "metrics": snapshot }))?;
                } else {
                    println!("Meat Memory key metrics");
                    println!("Keyed operations: {}", snapshot.key.keyed_operations);
                    println!("Successful: {}", snapshot.key.successful_operations);
                    println!("Failed: {}", snapshot.key.failed_operations);
                    println!("File mode: {}", snapshot.key.file_mode_operations);
                    println!("Vector mode: {}", snapshot.key.vector_mode_operations);
                    println!("All mode: {}", snapshot.key.all_mode_operations);
                }
            }
        }
    }

    Ok(())
}

async fn source_command(args: SourceArgs) -> Result<()> {
    let (_, kernel, _) = bootstrap_runtime().await?;
    match args.command {
        SourceCommand::Create(create) => {
            let context =
                resolve_required_cli_request_context(&kernel, create.key.as_deref()).await?;
            let mut source = MemorySource::new(
                create.source_kind,
                create.name,
                context.principal_id.clone(),
                context.owner_scope_id.clone(),
            )?;
            if let Some(source_uri) = create.source_uri {
                source = source.with_source_uri(source_uri)?;
            }
            if let Some(local_root) = create.local_root {
                source = source.with_local_root(local_root)?;
            }
            source = source.with_sync_mode(parse_source_sync_mode(&create.sync_mode)?);
            let source = kernel.upsert_memory_source(source, Some(&context)).await?;
            if create.json {
                print_json(memory_source_json(&source))?;
            } else {
                println!("Created source {}", source.id.as_str());
                println!("Name: {}", source.display_name);
                println!("Kind: {}", source.source_kind);
            }
        }
        SourceCommand::List(list) => {
            let context =
                resolve_required_cli_request_context(&kernel, list.key.as_deref()).await?;
            let sources = kernel
                .list_memory_sources(context.owner_scope_id.clone(), list.limit, Some(&context))
                .await?;
            if list.json {
                print_json(json!({
                    "sources": sources.iter().map(memory_source_json).collect::<Vec<_>>(),
                }))?;
            } else {
                println!("Meat Memory sources");
                for source in sources {
                    println!(
                        "- {} [{}] {}",
                        source.display_name,
                        source.id.as_str(),
                        source.source_kind
                    );
                }
            }
        }
        SourceCommand::Keys(keys) => {
            let context =
                resolve_required_cli_request_context(&kernel, keys.key.as_deref()).await?;
            let source = get_source_for_context(&kernel, &context, &keys.source_id).await?;
            let access_keys = kernel
                .list_access_keys_for_source(source.id.clone(), keys.limit)
                .await?;
            if keys.json {
                print_json(json!({
                    "source": memory_source_json(&source),
                    "keys": access_keys.iter().map(|key| access_key_json(key, None)).collect::<Vec<_>>(),
                }))?;
            } else {
                println!("Keys for source {}:", source.id.as_str());
                for key in access_keys {
                    println!("- {} [{}]", key.display_name, key.id.as_str());
                }
            }
        }
        SourceCommand::KeyCreate(create) => {
            let context =
                resolve_required_cli_request_context(&kernel, create.key.as_deref()).await?;
            let source = get_source_for_context(&kernel, &context, &create.source_id).await?;
            let result = kernel
                .create_access_key(CreateAccessKeyRequest {
                    raw_key: create.raw_key,
                    display_name: create.name,
                    source_id: Some(source.id),
                    source_kind: parse_key_source(&create.source)?,
                    owner_principal_id: source.owner_principal_id,
                    owner_scope_id: source.owner_scope_id,
                    scope_kind: parse_key_scope(&create.scope_kind)?,
                    storage_mode: parse_storage_mode(&create.storage)?,
                    is_fully_isolated: create.isolated,
                })
                .await?;
            if create.json {
                print_json(access_key_json(&result.access_key, Some(&result.raw_key)))?;
            } else {
                println!("Created source key {}", result.access_key.id.as_str());
                println!("Raw key: {}", result.raw_key);
            }
        }
    }
    Ok(())
}

async fn project_command(args: ProjectArgs) -> Result<()> {
    match args.command {
        ProjectCommand::Init(init) => project_init_command(init, ProjectInitSurface::Cli).await,
    }
}

async fn project_init_command(init: ProjectInitArgs, surface: ProjectInitSurface) -> Result<()> {
    if init.json && init.interactive {
        bail!("project init --interactive cannot be combined with --json");
    }

    let (_, kernel, _) = bootstrap_runtime().await?;
    let request = if init.interactive {
        run_project_init_interactive(&kernel, &init, surface).await?
    } else {
        build_non_interactive_project_init_request(&init)?
    };
    let result = apply_project_init_request(&kernel, &init, request, surface).await?;

    if init.json {
        print_json(project_init_result_json(&result))?;
    } else {
        for line in project_init_result_lines(&result) {
            println!("{line}");
        }
    }

    Ok(())
}

async fn context_command(args: ContextArgs) -> Result<()> {
    let (_, kernel, _) = bootstrap_runtime().await?;
    match args.command {
        ContextCommand::Upsert(upsert) => {
            let context =
                resolve_required_cli_request_context(&kernel, upsert.key.as_deref()).await?;
            let body = load_body(upsert.body.clone(), upsert.file.clone())?;
            let scope_id = upsert
                .scope_id
                .map(ScopeId::from_string)
                .unwrap_or_else(|| context.owner_scope_id.clone());
            let mut request =
                UpsertAgentContextRequest::new(scope_id, upsert.session_id, upsert.title, body);
            request.task_id = upsert.task_id;
            request.labels = upsert.labels;
            request.context = Some(context);
            let agent_context = kernel.upsert_agent_context(request).await?;
            if upsert.json {
                print_json(agent_context_json(&agent_context))?;
            } else {
                println!("Upserted context {}", agent_context.id.as_str());
                println!("Title: {}", agent_context.title);
            }
        }
        ContextCommand::List(list) => {
            let context =
                resolve_required_cli_request_context(&kernel, list.key.as_deref()).await?;
            let scope_id = list
                .scope_id
                .map(ScopeId::from_string)
                .unwrap_or_else(|| context.owner_scope_id.clone());
            let mut request = ListAgentContextsRequest::new(scope_id, list.session_id);
            request.task_id = list.task_id;
            request.limit = list.limit;
            request.context = Some(context);
            let contexts = kernel.list_agent_contexts(request).await?;
            if list.json {
                print_json(json!({
                    "contexts": contexts.iter().map(agent_context_json).collect::<Vec<_>>(),
                }))?;
            } else {
                println!("Agent contexts");
                for item in contexts {
                    println!("- {} [{}]", item.title, item.id.as_str());
                }
            }
        }
        ContextCommand::Promote(promote) => {
            let context =
                resolve_required_cli_request_context(&kernel, promote.key.as_deref()).await?;
            let mut request =
                PromoteAgentContextRequest::new(AgentContextId::from_string(promote.context_id));
            request.memory_kind = promote
                .memory_kind
                .as_deref()
                .map(parse_memory_kind)
                .transpose()?;
            request.visibility = parse_visibility(&promote.visibility)?;
            request.sensitivity = parse_sensitivity(&promote.sensitivity)?;
            request.context = Some(context);
            let result = kernel.promote_agent_context(request).await?;
            if promote.json {
                print_json(remember_result_json(&result))?;
            } else {
                for line in remember_result_lines(&result) {
                    println!("{line}");
                }
            }
        }
        ContextCommand::Delete(delete) => {
            let context =
                resolve_required_cli_request_context(&kernel, delete.key.as_deref()).await?;
            kernel
                .delete_agent_context(
                    AgentContextId::from_string(delete.context_id.clone()),
                    Some(&context),
                )
                .await?;
            if delete.json {
                print_json(json!({"context_id": delete.context_id, "deleted": true}))?;
            } else {
                println!("Deleted context {}", delete.context_id);
            }
        }
    }
    Ok(())
}

async fn docs_command(args: DocsArgs) -> Result<()> {
    let (_, kernel, _) = bootstrap_runtime().await?;
    match args.command {
        DocsCommand::Import(import) => {
            let context =
                resolve_required_cli_request_context(&kernel, import.key.as_deref()).await?;
            let body = load_body(import.body.clone(), import.file.clone())?;
            let source = get_source_for_context(&kernel, &context, &import.source_id).await?;
            let scope_id = import
                .scope_id
                .map(ScopeId::from_string)
                .unwrap_or_else(|| source.owner_scope_id.clone());
            let mut request = ImportProjectDocumentRequest::new(
                source.id,
                scope_id,
                import.canonical_uri,
                import.title,
                body,
            );
            request.local_path = import.local_path;
            request.sync_state = parse_document_sync_state(&import.sync_state)?;
            request.conflict_state = parse_document_conflict_state(&import.conflict_state)?;
            request.context = Some(context);
            let document = kernel.import_project_document(request).await?;
            if import.json {
                print_json(project_document_json(&document))?;
            } else {
                println!("Imported document {}", document.id.as_str());
                println!("Title: {}", document.title);
            }
        }
        DocsCommand::List(list) => {
            let context =
                resolve_required_cli_request_context(&kernel, list.key.as_deref()).await?;
            let source = get_source_for_context(&kernel, &context, &list.source_id).await?;
            let mut request = ListProjectDocumentsRequest::new(source.id);
            request.limit = list.limit;
            request.query = list.query;
            request.context = Some(context);
            let documents = kernel.list_project_documents(request).await?;
            if list.json {
                print_json(json!({
                    "documents": documents.iter().map(project_document_json).collect::<Vec<_>>(),
                }))?;
            } else {
                println!("Project documents");
                for document in documents {
                    println!("- {} [{}]", document.title, document.id.as_str());
                }
            }
        }
        DocsCommand::Projection(projection) => {
            let context =
                resolve_required_cli_request_context(&kernel, projection.key.as_deref()).await?;
            let payload = kernel
                .get_project_document_projection(
                    SourceId::from_string(projection.source_id),
                    memory_domain::ProjectDocumentId::from_string(projection.document_id),
                    Some(&context),
                )
                .await?;
            if projection.json {
                print_json(json!({
                    "document": project_document_json(&payload.document),
                    "projection_path": payload.projection_path.display().to_string(),
                    "markdown": payload.markdown,
                }))?;
            } else {
                println!("Projection path: {}", payload.projection_path.display());
                println!(
                    "Document: {} [{}]",
                    payload.document.title,
                    payload.document.id.as_str()
                );
                println!();
                print!("{}", payload.markdown);
                if !payload.markdown.ends_with('\n') {
                    println!();
                }
            }
        }
        DocsCommand::Conflicts(conflicts) => {
            let context =
                resolve_required_cli_request_context(&kernel, conflicts.key.as_deref()).await?;
            let source = get_source_for_context(&kernel, &context, &conflicts.source_id).await?;
            let documents = kernel
                .list_project_document_conflicts(source.id, conflicts.limit, Some(&context))
                .await?;
            if conflicts.json {
                print_json(json!({
                    "documents": documents.iter().map(project_document_json).collect::<Vec<_>>(),
                }))?;
            } else {
                println!("Project document conflicts");
                for document in documents {
                    println!(
                        "- {} [{}] {}",
                        document.title,
                        document.id.as_str(),
                        document.conflict_state.as_str()
                    );
                }
            }
        }
        DocsCommand::Sync(sync) => {
            let context =
                resolve_required_cli_request_context(&kernel, sync.key.as_deref()).await?;
            let source = get_source_for_context(&kernel, &context, &sync.source_id).await?;
            let scope_id = sync
                .scope_id
                .map(ScopeId::from_string)
                .unwrap_or_else(|| source.owner_scope_id.clone());
            let plan = build_local_docs_sync_plan(
                &kernel,
                &context,
                &source,
                sync.local_root.as_deref(),
                500,
            )
            .await?;
            let planned_count = plan.documents.len();
            let missing_count = plan.missing.len();
            let conflict_count = plan.conflicts.len();
            if sync.dry_run {
                if sync.json {
                    print_json(local_docs_plan_json(true, &plan, &[]))?;
                } else {
                    print_docs_sync_summary(true, planned_count, 0, missing_count, conflict_count);
                }
                return Ok(());
            }
            let result = kernel
                .apply_project_document_sync_plan(ApplyProjectDocumentSyncPlanRequest {
                    source_id: source.id,
                    scope_id,
                    plan,
                    context: Some(context),
                })
                .await?;
            if sync.json {
                print_json(json!({
                    "dry_run": false,
                    "planned_count": planned_count,
                    "imported": result.imported.iter().map(project_document_json).collect::<Vec<_>>(),
                    "missing": result.missing,
                    "conflicts": result.conflicts,
                }))?;
            } else {
                print_docs_sync_summary(
                    false,
                    planned_count,
                    result.imported.len(),
                    result.missing.len(),
                    result.conflicts.len(),
                );
            }
        }
        DocsCommand::Status(status) => {
            let context =
                resolve_required_cli_request_context(&kernel, status.key.as_deref()).await?;
            let source = get_source_for_context(&kernel, &context, &status.source_id).await?;
            let plan = build_local_docs_sync_plan(
                &kernel,
                &context,
                &source,
                status.local_root.as_deref(),
                500,
            )
            .await?;
            if status.json {
                print_json(local_docs_plan_json(true, &plan, &[]))?;
            } else {
                print_docs_sync_summary(
                    true,
                    plan.documents.len(),
                    0,
                    plan.missing.len(),
                    plan.conflicts.len(),
                );
            }
        }
    }
    Ok(())
}

fn skills_command(args: SkillsArgs) -> Result<()> {
    match args.command {
        SkillsCommand::Export(export) => {
            let root = env::current_dir().context("failed to resolve current directory")?;
            let output_dir = export
                .output_dir
                .clone()
                .unwrap_or_else(|| root.join("dist/agent-skills"));
            let exported = export_skill_bundle(&root, export.target, &output_dir, export.force)?;
            if export.json {
                print_json(json!({
                    "target": skill_export_target_label(export.target),
                    "output_dir": output_dir.display().to_string(),
                    "exported": exported,
                }))?;
            } else {
                println!(
                    "Exported {} skill bundle(s) to {}",
                    exported.len(),
                    output_dir.display()
                );
                for item in exported {
                    println!("- {item}");
                }
            }
        }
    }

    Ok(())
}

async fn tui_command(args: TuiArgs) -> Result<()> {
    match args.command {
        TuiCommand::Init(init) => {
            let path = config_path();
            let mut config = AppConfig::from_file(&path)?;
            if init.json && init.interactive {
                bail!("tui init --interactive cannot be combined with --json");
            }
            let init = if init.interactive {
                run_tui_init_interactive(&config, &init)?
            } else {
                init
            };
            apply_tui_init_overrides(&mut config, &init);
            let display_language = if init.interactive {
                Some(wizard_language_from_locale(&config.models.default_locale))
            } else {
                None
            };
            let check = config_check_report(&config, &path, init.check_database).await?;
            let wrote_config = write_tui_config_if_requested(&config, &init)?;
            let default_key = if wrote_config.is_some() {
                ensure_default_key_material(&config).await?
            } else {
                None
            };
            if init.json {
                print_json(tui_init_json(
                    &config,
                    &path,
                    wrote_config.as_deref(),
                    &check,
                    default_key.as_ref(),
                ))?;
            } else {
                for line in tui_init_lines(
                    &config,
                    &path,
                    wrote_config.as_deref(),
                    &check,
                    default_key.as_ref(),
                    display_language,
                ) {
                    println!("{line}");
                }
            }
        }
        TuiCommand::KeyCreate(create) => {
            key_command(KeyArgs {
                command: KeyCommand::Create(create),
            })
            .await?;
        }
        TuiCommand::ProjectInit(init) => {
            project_init_command(init, ProjectInitSurface::Tui).await?;
        }
    }

    Ok(())
}

async fn config_command(args: ConfigArgs) -> Result<()> {
    let path = config_path();
    let config = AppConfig::from_file(&path)?;

    match args.command {
        ConfigCommand::Show(inspect) => {
            if inspect.json {
                print_json(config_summary_json(&config, &path)?)?;
            } else {
                for line in config_summary_lines(&config, &path)? {
                    println!("{line}");
                }
            }
        }
        ConfigCommand::Check(check) => {
            let report = config_check_report(&config, &path, check.database).await?;
            if check.json {
                print_json(config_check_json(&report))?;
            } else {
                for line in config_check_lines(&report) {
                    println!("{line}");
                }
            }
        }
    }

    Ok(())
}

fn mcp_command(args: McpArgs) -> Result<()> {
    match args.command {
        McpCommand::Info(inspect) => {
            let path = config_path();
            let config = AppConfig::from_file(&path)?;
            let http_check = if inspect.check_http {
                Some(check_mcp_http_endpoint(&config))
            } else {
                None
            };
            if inspect.json {
                print_json(mcp_info_json(&config, &path, http_check.as_ref()))?;
            } else {
                for line in mcp_info_lines(&config, &path, http_check.as_ref()) {
                    println!("{line}");
                }
            }
        }
    }

    Ok(())
}

async fn serve_command(args: ServeArgs) -> Result<()> {
    let (config, kernel, service_info) = bootstrap_runtime().await?;
    serve_command_with_runtime(args, config, kernel, service_info).await
}

async fn serve_command_with_runtime(
    args: ServeArgs,
    config: AppConfig,
    kernel: Kernel,
    service_info: ServiceInfo,
) -> Result<()> {
    log_startup(&service_info);
    println!("{}", startup_banner(&service_info));
    let bind = resolve_bind(args.bind, &config);
    let app = build_cli_router(&config, &service_info, Arc::new(kernel));
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    serve_with_listener(listener, &bind, app, shutdown_signal()).await
}

async fn remember_command(args: RememberArgs) -> Result<()> {
    let (_, kernel, service_info) = bootstrap_runtime().await?;
    remember_command_with_runtime(args, kernel, service_info).await
}

async fn remember_command_with_runtime(
    args: RememberArgs,
    kernel: Kernel,
    service_info: ServiceInfo,
) -> Result<()> {
    let body = load_body(args.body.clone(), args.file.clone())?;
    let mut request = build_remember_request(&args, &service_info, body)?;
    request.context = resolve_cli_request_context(&kernel, args.key.as_deref()).await?;

    let result = kernel.remember_text(request).await?;

    if args.json {
        print_json(remember_result_json(&result))?;
    } else {
        for line in remember_result_lines(&result) {
            println!("{line}");
        }
    }

    Ok(())
}

async fn remember_image_command(args: RememberImageArgs) -> Result<()> {
    let (_, kernel, service_info) = bootstrap_runtime().await?;
    remember_image_command_with_runtime(args, kernel, service_info).await
}

async fn remember_image_command_with_runtime(
    args: RememberImageArgs,
    kernel: Kernel,
    service_info: ServiceInfo,
) -> Result<()> {
    let media_type = detect_image_media_type(&args.file, args.media_type.clone())?;
    let bytes = fs::read(&args.file)?;
    let mut request = build_remember_image_request(&args, &service_info, media_type, bytes)?;
    request.context = resolve_cli_request_context(&kernel, args.key.as_deref()).await?;

    let result = kernel.remember_image(request).await?;
    let asset_id = result.asset.reference.asset_id.clone();
    let asset_uri = result.asset.reference.uri();

    if args.json {
        print_json(remember_image_result_json(&result, &asset_id, &asset_uri))?;
    } else {
        for line in remember_image_result_lines(&result, &asset_uri) {
            println!("{line}");
        }
    }

    Ok(())
}

async fn search_command(args: SearchArgs) -> Result<()> {
    let (_, kernel, service_info) = bootstrap_runtime().await?;
    search_command_with_runtime(args, kernel, service_info).await
}

async fn search_command_with_runtime(
    args: SearchArgs,
    kernel: Kernel,
    service_info: ServiceInfo,
) -> Result<()> {
    let mut request = build_search_request(&args, &service_info);
    request.context = resolve_cli_request_context(&kernel, args.key.as_deref()).await?;
    let bundle = kernel.search_context(request).await?;

    if args.json {
        print_json(search_bundle_json(&bundle))?;
    } else {
        for line in search_bundle_lines(&bundle) {
            println!("{line}");
        }
    }

    Ok(())
}

async fn bootstrap_runtime() -> Result<(AppConfig, Kernel, ServiceInfo)> {
    let config = AppConfig::load()?;
    init_runtime_observability(&config)?;
    let (kernel, service_info) = bootstrap_loaded_config(&config).await?;
    Ok((config, kernel, service_info))
}

fn init_runtime_observability(config: &AppConfig) -> Result<()> {
    memory_observability::init(&config.logging.level, &config.logging.format)
}

async fn bootstrap_loaded_config(config: &AppConfig) -> Result<(Kernel, ServiceInfo)> {
    validate_model_registry(config)?;
    let kernel = build_kernel(config).await?;
    let service_info = ServiceInfo::default();
    Ok((kernel, service_info))
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

fn resolve_bind(bind: Option<String>, config: &AppConfig) -> String {
    bind.unwrap_or_else(|| config.server.bind.clone())
}

async fn serve_with_listener<F>(
    listener: tokio::net::TcpListener,
    bind: &str,
    app: axum::Router,
    shutdown: F,
) -> Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    println!("Serving Meat Memory on {bind}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}

fn build_cli_router(
    config: &AppConfig,
    service_info: &ServiceInfo,
    kernel: Arc<Kernel>,
) -> axum::Router {
    let metadata = api_metadata(config, service_info);
    let mut app = build_router(HttpAppState::new(
        service_info.default_scope.clone(),
        metadata,
        Arc::clone(&kernel),
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
            require_key: config.access.require_key,
        },
    }
}

fn validate_model_registry(config: &AppConfig) -> Result<()> {
    let registry = config.model_registry()?;
    tracing::info!(
        providers = registry.provider_count(),
        models = registry.model_count(),
        routes = registry.route_count(),
        default_locale = %config.models.default_locale,
        "validated model registry"
    );
    Ok(())
}

fn config_path() -> String {
    env::var("MEAT_MEMORY_CONFIG").unwrap_or_else(|_| "config/app.toml".to_string())
}

fn config_summary_json(config: &AppConfig, path: &str) -> Result<serde_json::Value> {
    let registry = config.model_registry()?;
    Ok(json!({
        "config_path": path,
        "server": {
            "bind": config.server.bind,
            "shutdown_grace_period_secs": config.server.shutdown_grace_period_secs,
        },
        "logging": {
            "level": config.logging.level,
            "format": config.logging.format,
        },
        "storage": {
            "postgres_enabled": config.features.enable_pg,
            "postgres_app_name": config.postgres.app_name,
            "markdown_enabled": config.features.enable_markdown,
            "markdown_root": config.markdown.root,
            "assets_root": config.assets.root,
        },
        "models": {
            "default_locale": config.models.default_locale,
            "provider_count": registry.provider_count(),
            "model_count": registry.model_count(),
            "route_count": registry.route_count(),
            "reasoning_primary": config.models.routing.reasoning.primary,
            "extraction_primary": config.models.routing.extraction.primary,
            "vision_primary": config.models.routing.vision.primary,
            "embedding_primary": config.models.routing.embedding.primary,
        },
        "sync": {
            "mode": config.sync.mode,
            "node_id": config.sync.node_id,
            "state_path": config.sync.state_path,
        },
        "features": {
            "http": config.features.enable_http,
            "mcp": config.features.enable_mcp,
            "pg": config.features.enable_pg,
            "markdown": config.features.enable_markdown,
        },
    }))
}

fn config_summary_lines(config: &AppConfig, path: &str) -> Result<Vec<String>> {
    let registry = config.model_registry()?;
    Ok(vec![
        "Meat Memory config".to_string(),
        format!("Config: {path}"),
        format!("Server: {}", config.server.bind),
        format!(
            "Stores: pg={}, markdown={}, markdown_root={}, assets_root={}",
            config.features.enable_pg,
            config.features.enable_markdown,
            config.markdown.root,
            config.assets.root
        ),
        format!(
            "Models: locale={}, providers={}, models={}, routes={}",
            config.models.default_locale,
            registry.provider_count(),
            registry.model_count(),
            registry.route_count()
        ),
        format!(
            "Routes: reasoning={}, extraction={}, vision={}, embedding={}",
            config.models.routing.reasoning.primary,
            config.models.routing.extraction.primary,
            config.models.routing.vision.primary,
            config.models.routing.embedding.primary
        ),
        format!(
            "Sync: mode={}, node_id={}, state_path={}",
            config.sync.mode, config.sync.node_id, config.sync.state_path
        ),
        format!(
            "Features: http={}, mcp={}, pg={}, markdown={}",
            config.features.enable_http,
            config.features.enable_mcp,
            config.features.enable_pg,
            config.features.enable_markdown
        ),
    ])
}

#[derive(Debug, Clone)]
struct ConfigCheckReport {
    path: String,
    ok: bool,
    warnings: Vec<String>,
    database: DatabaseCheckReport,
    provider_count: usize,
    model_count: usize,
    route_count: usize,
}

#[derive(Debug, Clone)]
struct DatabaseCheckReport {
    checked: bool,
    ok: Option<bool>,
    message: String,
}

async fn config_check_report(
    config: &AppConfig,
    path: &str,
    check_database: bool,
) -> Result<ConfigCheckReport> {
    let registry = config.model_registry()?;
    let mut warnings = Vec::new();

    if !config.features.enable_pg && !config.features.enable_markdown {
        warnings.push("No persistence store is enabled; enable pg or markdown.".to_string());
    }
    if config.features.enable_mcp && !config.features.enable_http {
        warnings.push(
            "MCP is enabled but HTTP is disabled; /mcp routes will not be served.".to_string(),
        );
    }
    if config.models.providers.is_empty() {
        warnings.push("No model providers configured.".to_string());
    }
    if config.models.catalog.is_empty() {
        warnings.push("No models configured in catalog.".to_string());
    }
    let database = check_database_connectivity(config, check_database).await;
    if database.ok == Some(false) {
        warnings.push(format!("Database check failed: {}", database.message));
    }

    Ok(ConfigCheckReport {
        path: path.to_string(),
        ok: warnings.is_empty(),
        warnings,
        database,
        provider_count: registry.provider_count(),
        model_count: registry.model_count(),
        route_count: registry.route_count(),
    })
}

async fn check_database_connectivity(
    config: &AppConfig,
    check_database: bool,
) -> DatabaseCheckReport {
    if !check_database {
        return DatabaseCheckReport {
            checked: false,
            ok: None,
            message: "skipped; pass --database or --check-database".to_string(),
        };
    }
    if !config.features.enable_pg {
        return DatabaseCheckReport {
            checked: false,
            ok: None,
            message: "skipped; postgres feature is disabled".to_string(),
        };
    }

    match PgStore::connect(&config.postgres.database_url).await {
        Ok(_) => DatabaseCheckReport {
            checked: true,
            ok: Some(true),
            message: "connected".to_string(),
        },
        Err(error) => DatabaseCheckReport {
            checked: true,
            ok: Some(false),
            message: error.to_string(),
        },
    }
}

fn config_check_json(report: &ConfigCheckReport) -> serde_json::Value {
    json!({
        "config_path": report.path,
        "ok": report.ok,
        "warnings": report.warnings,
        "database": {
            "checked": report.database.checked,
            "ok": report.database.ok,
            "message": report.database.message,
        },
        "provider_count": report.provider_count,
        "model_count": report.model_count,
        "route_count": report.route_count,
    })
}

fn config_check_lines(report: &ConfigCheckReport) -> Vec<String> {
    let mut lines = vec![
        format!("Config check: {}", if report.ok { "ok" } else { "warning" }),
        format!("Config: {}", report.path),
        format!(
            "Model registry: providers={}, models={}, routes={}",
            report.provider_count, report.model_count, report.route_count
        ),
        format!("Database: {}", report.database.message),
    ];
    if report.warnings.is_empty() {
        lines.push("Warnings: none".to_string());
    } else {
        lines.push("Warnings:".to_string());
        lines.extend(report.warnings.iter().map(|warning| format!("- {warning}")));
    }
    lines
}

#[derive(Debug, Clone)]
struct HttpEndpointCheck {
    checked: bool,
    ok: bool,
    message: String,
}

fn mcp_info_json(
    config: &AppConfig,
    path: &str,
    http_check: Option<&HttpEndpointCheck>,
) -> serde_json::Value {
    let tools = TOOL_SPECS
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "config_path": path,
        "enabled": config.features.enable_mcp,
        "http_enabled": config.features.enable_http,
        "base_url": format!("http://{}", config.server.bind),
        "tools_url": format!("http://{}/mcp/tools", config.server.bind),
        "call_url": format!("http://{}/mcp/tools/call", config.server.bind),
        "http_check": http_check.map(|check| json!({
            "checked": check.checked,
            "ok": check.ok,
            "message": check.message,
        })),
        "tools": tools,
    })
}

fn mcp_info_lines(
    config: &AppConfig,
    path: &str,
    http_check: Option<&HttpEndpointCheck>,
) -> Vec<String> {
    let mut lines = vec![
        "Meat Memory MCP info".to_string(),
        format!("Config: {path}"),
        format!("Enabled: {}", config.features.enable_mcp),
        format!("HTTP enabled: {}", config.features.enable_http),
        format!("Tools URL: http://{}/mcp/tools", config.server.bind),
        format!("Call URL: http://{}/mcp/tools/call", config.server.bind),
    ];
    if let Some(check) = http_check {
        lines.push(format!("HTTP check: {}", check.message));
    }
    lines.push("Tools:".to_string());
    lines.extend(
        TOOL_SPECS
            .iter()
            .map(|tool| format!("- {}: {}", tool.name, tool.description)),
    );
    lines
}

fn check_mcp_http_endpoint(config: &AppConfig) -> HttpEndpointCheck {
    if !config.features.enable_http {
        return HttpEndpointCheck {
            checked: true,
            ok: false,
            message: "http disabled in config".to_string(),
        };
    }

    match resolve_socket_addr(&config.server.bind)
        .and_then(|addr| tcp_connect_with_timeout(addr, Duration::from_millis(800)))
    {
        Ok(mut stream) => {
            let request = format!(
                "GET /mcp/tools HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
                config.server.bind
            );
            if let Err(error) = stream.write_all(request.as_bytes()) {
                return HttpEndpointCheck {
                    checked: true,
                    ok: false,
                    message: format!("tcp connected but request failed: {error}"),
                };
            }
            let mut response = String::new();
            if let Err(error) = stream.read_to_string(&mut response) {
                return HttpEndpointCheck {
                    checked: true,
                    ok: false,
                    message: format!("tcp connected but response read failed: {error}"),
                };
            }
            let first_line = response.lines().next().unwrap_or_default().to_string();
            let ok = first_line.contains("200");
            HttpEndpointCheck {
                checked: true,
                ok,
                message: if ok {
                    "reachable (HTTP 200)".to_string()
                } else if first_line.is_empty() {
                    "reachable but empty response".to_string()
                } else {
                    format!("reachable but unexpected response: {first_line}")
                },
            }
        }
        Err(error) => HttpEndpointCheck {
            checked: true,
            ok: false,
            message: format!("unreachable: {error}"),
        },
    }
}

fn resolve_socket_addr(bind: &str) -> Result<SocketAddr> {
    bind.to_socket_addrs()
        .context("failed to resolve MCP bind address")?
        .next()
        .context("no socket address resolved for MCP bind address")
}

fn tcp_connect_with_timeout(addr: SocketAddr, timeout: Duration) -> Result<TcpStream> {
    TcpStream::connect_timeout(&addr, timeout)
        .with_context(|| format!("failed to connect to {}", addr))
}

fn skill_export_target_label(target: SkillExportTarget) -> &'static str {
    match target {
        SkillExportTarget::Codex => "codex",
        SkillExportTarget::ClaudeCode => "claude-code",
        SkillExportTarget::ExecutionAgent => "execution-agent",
        SkillExportTarget::All => "all",
    }
}

fn export_skill_bundle(
    root: &Path,
    target: SkillExportTarget,
    output_dir: &Path,
    force: bool,
) -> Result<Vec<String>> {
    let source_dir = root.join("docs/agent-skills");
    if !source_dir.exists() {
        bail!("skill source directory not found: {}", source_dir.display());
    }
    if output_dir.exists() {
        if !force {
            bail!(
                "{} already exists; pass --force to overwrite",
                output_dir.display()
            );
        }
        fs::remove_dir_all(output_dir)
            .with_context(|| format!("failed to remove {}", output_dir.display()))?;
    }
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    let selected = selected_skill_templates(target);
    let mut exported = Vec::new();
    for (source_name, dest_name) in selected {
        let dest_dir = output_dir.join(dest_name);
        fs::create_dir_all(&dest_dir)
            .with_context(|| format!("failed to create {}", dest_dir.display()))?;
        fs::copy(
            source_dir.join(source_name).join("SKILL.md"),
            dest_dir.join("SKILL.md"),
        )
        .with_context(|| format!("failed to export {}", dest_name))?;
        exported.push(format!("{dest_name}/SKILL.md"));
        let source_agents_dir = source_dir.join(source_name).join("agents");
        if source_agents_dir.exists() {
            let dest_agents_dir = dest_dir.join("agents");
            fs::create_dir_all(&dest_agents_dir)
                .with_context(|| format!("failed to create {}", dest_agents_dir.display()))?;
            for entry in fs::read_dir(&source_agents_dir)
                .with_context(|| format!("failed to read {}", source_agents_dir.display()))?
            {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    let file_name = entry.file_name();
                    fs::copy(entry.path(), dest_agents_dir.join(&file_name)).with_context(
                        || {
                            format!(
                                "failed to export {}",
                                dest_agents_dir.join(&file_name).display()
                            )
                        },
                    )?;
                    exported.push(format!(
                        "{dest_name}/agents/{}",
                        file_name.to_string_lossy()
                    ));
                }
            }
        }
        let source_assets_dir = source_dir.join(source_name).join("assets");
        if source_assets_dir.exists() {
            let dest_assets_dir = dest_dir.join("assets");
            fs::create_dir_all(&dest_assets_dir)
                .with_context(|| format!("failed to create {}", dest_assets_dir.display()))?;
            for entry in fs::read_dir(&source_assets_dir)
                .with_context(|| format!("failed to read {}", source_assets_dir.display()))?
            {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    let file_name = entry.file_name();
                    fs::copy(entry.path(), dest_assets_dir.join(&file_name)).with_context(
                        || {
                            format!(
                                "failed to export {}",
                                dest_assets_dir.join(&file_name).display()
                            )
                        },
                    )?;
                    exported.push(format!(
                        "{dest_name}/assets/{}",
                        file_name.to_string_lossy()
                    ));
                }
            }
        }
    }

    let bundle_readme =
        render_skill_bundle_readme(&source_dir, skill_export_target_label(target), &exported);
    fs::write(output_dir.join("README.md"), bundle_readme)
        .with_context(|| format!("failed to write {}", output_dir.join("README.md").display()))?;

    Ok(exported)
}

fn selected_skill_templates(target: SkillExportTarget) -> &'static [(&'static str, &'static str)] {
    match target {
        SkillExportTarget::Codex => &[("codex-meat-memory", "codex-meat-memory")],
        SkillExportTarget::ClaudeCode => &[("claude-code-meat-memory", "claude-code-meat-memory")],
        SkillExportTarget::ExecutionAgent => {
            &[("execution-agent-meat-memory", "execution-agent-meat-memory")]
        }
        SkillExportTarget::All => &[
            ("codex-meat-memory", "codex-meat-memory"),
            ("claude-code-meat-memory", "claude-code-meat-memory"),
            ("execution-agent-meat-memory", "execution-agent-meat-memory"),
        ],
    }
}

fn render_skill_bundle_readme(source_dir: &Path, target: &str, exported: &[String]) -> String {
    format!(
        "# Meat Memory Agent Skills Bundle\n\nThis bundle was exported from:\n\n- source: `{}`\n- target: `{}`\n\nIncluded skills:\n\n{}\n\nSuggested next steps:\n\n1. Run `memory-cli config check`\n2. Run `memory-cli mcp info`\n3. Copy the exported skill folder into the target Agent platform's skill or instruction directory\n4. Configure MCP with:\n   - tools url: `http://127.0.0.1:8080/mcp/tools`\n   - call url: `http://127.0.0.1:8080/mcp/tools/call`\n",
        source_dir.display(),
        target,
        exported
            .iter()
            .map(|item| format!("- `{item}`"))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn apply_tui_init_overrides(config: &mut AppConfig, args: &TuiInitArgs) {
    if let Some(default_locale) = args.default_locale.as_ref() {
        config.models.default_locale = default_locale.clone();
    }
    if args.enable_mcp {
        config.features.enable_mcp = true;
    }
    if args.disable_mcp {
        config.features.enable_mcp = false;
    }
    if let Some(database_url) = args.database_url.as_ref() {
        config.postgres.database_url = database_url.clone();
    }
    if let Some(markdown_root) = args.markdown_root.as_ref() {
        config.markdown.root = markdown_root.clone();
    }
    if let Some(assets_root) = args.assets_root.as_ref() {
        config.assets.root = assets_root.clone();
    }
    if let Some(primary) = args.reasoning_primary.as_ref() {
        set_route_primary(&mut config.models.routing.reasoning, primary);
    }
    if let Some(primary) = args.extraction_primary.as_ref() {
        set_route_primary(&mut config.models.routing.extraction, primary);
    }
    if let Some(primary) = args.vision_primary.as_ref() {
        set_route_primary(&mut config.models.routing.vision, primary);
    }
    if let Some(primary) = args.embedding_primary.as_ref() {
        set_route_primary(&mut config.models.routing.embedding, primary);
    }
}

fn set_route_primary(route: &mut CapabilityRoute, primary: &str) {
    route.primary = primary.to_string();
    route.fallbacks.retain(|fallback| fallback != primary);
}

fn write_tui_config_if_requested(config: &AppConfig, args: &TuiInitArgs) -> Result<Option<String>> {
    let Some(path) = args.write_config.as_ref() else {
        return Ok(None);
    };
    if path.exists() && !args.force {
        bail!(
            "{} already exists; pass --force to overwrite",
            path.display()
        );
    }
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;
    }
    let rendered = config.to_toml_string_pretty()?;
    fs::write(path, rendered)
        .with_context(|| format!("failed to write config file {}", path.display()))?;
    Ok(Some(path.display().to_string()))
}

async fn ensure_default_key_material(config: &AppConfig) -> Result<Option<DefaultKeyMaterial>> {
    if !config.features.enable_pg {
        return Ok(None);
    }

    let path = PathBuf::from(&config.access.key_store_path);
    if path.exists() {
        return Ok(None);
    }

    let (kernel, service_info) = bootstrap_loaded_config(config).await?;
    let result = kernel
        .create_access_key(CreateAccessKeyRequest {
            raw_key: None,
            display_name: config.access.default_key_name.clone(),
            source_id: None,
            source_kind: parse_key_source(&config.access.default_key_source)?,
            owner_principal_id: "local-user".to_string(),
            owner_scope_id: service_info.default_scope,
            scope_kind: parse_key_scope(&config.access.default_key_scope_kind)?,
            storage_mode: parse_storage_mode(&config.access.default_key_storage_mode)?,
            is_fully_isolated: config.access.default_key_isolated,
        })
        .await?;

    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create key directory {}", parent.display()))?;
    }
    fs::write(&path, format!("MEAT_MEMORY_KEY={}\n", result.raw_key))
        .with_context(|| format!("failed to write default key file {}", path.display()))?;

    Ok(Some(DefaultKeyMaterial {
        key_id: result.access_key.id.as_str().to_string(),
        raw_key: result.raw_key,
        path: path.display().to_string(),
    }))
}

fn run_tui_init_interactive(config: &AppConfig, args: &TuiInitArgs) -> Result<TuiInitArgs> {
    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    run_tui_init_interactive_io(config, args, &mut reader, &mut writer)
}

fn run_tui_init_interactive_io<R: BufRead, W: Write>(
    config: &AppConfig,
    args: &TuiInitArgs,
    reader: &mut R,
    writer: &mut W,
) -> Result<TuiInitArgs> {
    let mut updated = args.clone();
    let default_language = wizard_language_from_locale(effective_text(
        args.default_locale.as_deref(),
        &config.models.default_locale,
    ));

    writeln!(
        writer,
        "Meat Memory interactive setup wizard / Meat Memory 交互式安装向导"
    )?;
    writeln!(
        writer,
        "Press Enter to keep the current value. / 直接回车可保留当前值。"
    )?;
    writeln!(writer)?;

    let language_choice = prompt_choice(
        reader,
        writer,
        "Choose language / 选择语言",
        &[
            ("中文", "使用中文提示，并将默认 locale 设为 zh-CN"),
            (
                "English",
                "Use English prompts and set default locale to en-US",
            ),
        ],
        default_language.as_choice_index(),
    )?;
    let language = WizardLanguage::from_choice_index(language_choice);
    updated.default_locale = Some(language.locale().to_string());

    writeln!(writer)?;
    writeln!(
        writer,
        "{}",
        wizard_text(
            language,
            "第 1/5 步：选择安装预设",
            "Step 1/5: choose setup profile"
        )
    )?;
    let profile = prompt_choice(
        reader,
        writer,
        wizard_text(language, "选择安装预设", "Select setup profile"),
        &[
            (
                wizard_text(language, "本地默认", "Local default"),
                wizard_text(
                    language,
                    "保持当前存储和模型路由，适合首次本地试跑",
                    "Keep current storage and routes, best for first local run",
                ),
            ),
            (
                wizard_text(language, "MCP 就绪", "MCP-ready"),
                wizard_text(
                    language,
                    "默认启用 MCP，并保留双存储，适合 Agent 接入",
                    "Enable MCP by default and keep dual storage for agent usage",
                ),
            ),
            (
                wizard_text(language, "Markdown 优先", "Markdown-first"),
                wizard_text(
                    language,
                    "更偏向 Markdown 工作流，默认不启 MCP",
                    "Prefer markdown workflow and leave MCP disabled by default",
                ),
            ),
        ],
        0,
    )?;
    apply_tui_profile_defaults(config, &mut updated, profile);

    writeln!(writer)?;
    writeln!(
        writer,
        "{}",
        wizard_text(
            language,
            "第 2/5 步：存储与功能设置",
            "Step 2/5: storage and feature settings"
        )
    )?;

    let enable_mcp = prompt_bool(
        reader,
        writer,
        wizard_text(language, "启用 MCP", "Enable MCP"),
        effective_mcp_enabled(config, &updated),
    )?;
    if let Some(enable_mcp) = enable_mcp {
        updated.enable_mcp = enable_mcp;
        updated.disable_mcp = !enable_mcp;
    }

    updated.database_url = prompt_text(
        reader,
        writer,
        wizard_text(language, "数据库 URL", "Database URL"),
        effective_text(args.database_url.as_deref(), &config.postgres.database_url),
    )?;
    updated.markdown_root = prompt_text(
        reader,
        writer,
        wizard_text(language, "Markdown 根目录", "Markdown root"),
        effective_text(args.markdown_root.as_deref(), &config.markdown.root),
    )?;
    updated.assets_root = prompt_text(
        reader,
        writer,
        wizard_text(language, "Assets 根目录", "Assets root"),
        effective_text(args.assets_root.as_deref(), &config.assets.root),
    )?;

    writeln!(writer)?;
    writeln!(
        writer,
        "{}",
        wizard_text(language, "第 3/5 步：模型路由", "Step 3/5: model routing")
    )?;
    updated.reasoning_primary = prompt_model_alias(
        reader,
        writer,
        config,
        language,
        wizard_text(language, "主 reasoning 模型", "Reasoning primary"),
        ModelCapability::Reasoning,
        effective_text(
            args.reasoning_primary.as_deref(),
            &config.models.routing.reasoning.primary,
        ),
    )?;
    updated.extraction_primary = prompt_model_alias(
        reader,
        writer,
        config,
        language,
        wizard_text(language, "主 extraction 模型", "Extraction primary"),
        ModelCapability::Extraction,
        effective_text(
            args.extraction_primary.as_deref(),
            &config.models.routing.extraction.primary,
        ),
    )?;
    updated.vision_primary = prompt_model_alias(
        reader,
        writer,
        config,
        language,
        wizard_text(language, "主 vision 模型", "Vision primary"),
        ModelCapability::Vision,
        effective_text(
            args.vision_primary.as_deref(),
            &config.models.routing.vision.primary,
        ),
    )?;
    updated.embedding_primary = prompt_model_alias(
        reader,
        writer,
        config,
        language,
        wizard_text(language, "主 embedding 模型", "Embedding primary"),
        ModelCapability::Embedding,
        effective_text(
            args.embedding_primary.as_deref(),
            &config.models.routing.embedding.primary,
        ),
    )?;

    writeln!(writer)?;
    writeln!(
        writer,
        "{}",
        wizard_text(
            language,
            "第 4/5 步：校验与输出",
            "Step 4/5: validation and output"
        )
    )?;
    updated.check_database = prompt_bool(
        reader,
        writer,
        wizard_text(
            language,
            "立即检查数据库连通性",
            "Check database connectivity now",
        ),
        args.check_database,
    )?
    .unwrap_or(args.check_database);

    updated.write_config = prompt_optional_path(
        reader,
        writer,
        wizard_text(language, "将配置写入文件", "Write config to file"),
        args.write_config.as_deref(),
    )?;
    if updated.write_config.is_some() {
        updated.force = prompt_bool(
            reader,
            writer,
            wizard_text(
                language,
                "如果文件已存在则覆盖",
                "Overwrite target if it exists",
            ),
            args.force,
        )?
        .unwrap_or(args.force);
    }

    writeln!(writer)?;
    writeln!(
        writer,
        "{}",
        wizard_text(language, "第 5/5 步：确认设置", "Step 5/5: confirm setup")
    )?;
    writeln!(
        writer,
        "{}",
        wizard_text(language, "配置摘要：", "Review summary:")
    )?;
    for line in render_tui_interactive_summary(config, &updated, language) {
        writeln!(writer, "- {line}")?;
    }
    let confirmed = prompt_bool(
        reader,
        writer,
        wizard_text(language, "应用这些设置", "Apply this interactive setup"),
        true,
    )?
    .unwrap_or(true);
    if !confirmed {
        bail!(
            "{}",
            wizard_text(
                language,
                "交互式安装已由用户取消",
                "interactive setup cancelled by user"
            )
        );
    }

    Ok(updated)
}

fn apply_tui_profile_defaults(config: &AppConfig, args: &mut TuiInitArgs, profile: usize) {
    match profile {
        1 => {
            args.enable_mcp = true;
            args.disable_mcp = false;
        }
        2 => {
            args.enable_mcp = false;
            args.disable_mcp = true;
            if args.markdown_root.is_none() {
                args.markdown_root = Some(config.markdown.root.clone());
            }
        }
        _ => {}
    }
}

fn render_tui_interactive_summary(
    config: &AppConfig,
    args: &TuiInitArgs,
    language: WizardLanguage,
) -> Vec<String> {
    vec![
        format!(
            "{}: {}",
            wizard_text(language, "语言", "Language"),
            effective_text(
                args.default_locale.as_deref(),
                &config.models.default_locale
            )
        ),
        format!("MCP: {}", effective_mcp_enabled(config, args)),
        format!(
            "{}: {}",
            wizard_text(language, "数据库 URL", "Database URL"),
            effective_text(args.database_url.as_deref(), &config.postgres.database_url)
        ),
        format!(
            "{}: {}",
            wizard_text(language, "Markdown 根目录", "Markdown root"),
            effective_text(args.markdown_root.as_deref(), &config.markdown.root)
        ),
        format!(
            "{}: {}",
            wizard_text(language, "Assets 根目录", "Assets root"),
            effective_text(args.assets_root.as_deref(), &config.assets.root)
        ),
        format!(
            "{}: {}",
            wizard_text(language, "Reasoning", "Reasoning"),
            effective_text(
                args.reasoning_primary.as_deref(),
                &config.models.routing.reasoning.primary
            )
        ),
        format!(
            "{}: {}",
            wizard_text(language, "Extraction", "Extraction"),
            effective_text(
                args.extraction_primary.as_deref(),
                &config.models.routing.extraction.primary
            )
        ),
        format!(
            "{}: {}",
            wizard_text(language, "Vision", "Vision"),
            effective_text(
                args.vision_primary.as_deref(),
                &config.models.routing.vision.primary
            )
        ),
        format!(
            "{}: {}",
            wizard_text(language, "Embedding", "Embedding"),
            effective_text(
                args.embedding_primary.as_deref(),
                &config.models.routing.embedding.primary
            )
        ),
        format!(
            "{}: {}",
            wizard_text(language, "立即检查数据库", "Check database now"),
            args.check_database
        ),
        format!(
            "{}: {}",
            wizard_text(language, "写出配置", "Write config"),
            args.write_config
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "preview only".to_string())
        ),
    ]
}

fn effective_mcp_enabled(config: &AppConfig, args: &TuiInitArgs) -> bool {
    if args.enable_mcp {
        true
    } else if args.disable_mcp {
        false
    } else {
        config.features.enable_mcp
    }
}

fn effective_text<'a>(override_value: Option<&'a str>, current: &'a str) -> &'a str {
    override_value.unwrap_or(current)
}

fn prompt_text<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    label: &str,
    current: &str,
) -> Result<Option<String>> {
    write!(writer, "{label} [{current}]: ")?;
    writer.flush()?;
    let input = read_prompt_line(reader)?;
    if input.is_empty() {
        Ok(None)
    } else {
        Ok(Some(input))
    }
}

fn prompt_optional_path<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    label: &str,
    current: Option<&Path>,
) -> Result<Option<PathBuf>> {
    let current_display = current
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "preview only".to_string());
    write!(writer, "{label} [{current_display}]: ")?;
    writer.flush()?;
    let input = read_prompt_line(reader)?;
    if input.is_empty() {
        Ok(current.map(Path::to_path_buf))
    } else {
        Ok(Some(PathBuf::from(input)))
    }
}

fn prompt_bool<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    label: &str,
    current: bool,
) -> Result<Option<bool>> {
    let current_label = if current { "Y/n" } else { "y/N" };
    write!(writer, "{label} [{current_label}]: ")?;
    writer.flush()?;
    let input = read_prompt_line(reader)?;
    if input.is_empty() {
        return Ok(None);
    }
    match input.to_ascii_lowercase().as_str() {
        "y" | "yes" | "true" | "1" => Ok(Some(true)),
        "n" | "no" | "false" | "0" => Ok(Some(false)),
        other => bail!("unsupported answer for {label}: {other}"),
    }
}

fn prompt_choice<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    label: &str,
    choices: &[(&str, &str)],
    default: usize,
) -> Result<usize> {
    writeln!(writer, "{label}:")?;
    for (index, (title, description)) in choices.iter().enumerate() {
        writeln!(writer, "  {}. {} - {}", index + 1, title, description)?;
    }
    write!(writer, "Choose [{}]: ", default + 1)?;
    writer.flush()?;
    let input = read_prompt_line(reader)?;
    if input.is_empty() {
        return Ok(default);
    }
    let selected = input
        .parse::<usize>()
        .with_context(|| format!("invalid selection for {label}: {input}"))?;
    if selected == 0 || selected > choices.len() {
        bail!("selection out of range for {label}: {selected}");
    }
    Ok(selected - 1)
}

fn prompt_model_alias<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    config: &AppConfig,
    language: WizardLanguage,
    label: &str,
    capability: ModelCapability,
    current: &str,
) -> Result<Option<String>> {
    let choices = available_model_alias_choices(config, capability);
    if !choices.is_empty() {
        writeln!(
            writer,
            "{} {capability} aliases:",
            wizard_text(language, "可用", "Available"),
        )?;
        for (index, choice) in choices.iter().enumerate() {
            writeln!(writer, "  {}. {}", index + 1, choice.summary())?;
        }
        let default_choice = choices
            .iter()
            .position(|choice| choice.alias == current)
            .map(|index| index + 1)
            .unwrap_or(0);
        let default_hint = if default_choice > 0 {
            format!("{default_choice}")
        } else {
            current.to_string()
        };
        write!(
            writer,
            "{} [{}]: ",
            wizard_text(
                language,
                label,
                &format!("{label} (choose a number or keep current)")
            ),
            default_hint
        )?;
        writer.flush()?;
        let input = read_prompt_line(reader)?;
        if input.is_empty() {
            return Ok(None);
        }
        if let Ok(selected) = input.parse::<usize>() {
            if selected == 0 || selected > choices.len() {
                bail!("selection out of range for {label}: {selected}");
            }
            return Ok(Some(choices[selected - 1].alias.clone()));
        }
        return Ok(Some(input));
    }
    prompt_text(reader, writer, label, current)
}

#[derive(Debug, Clone)]
struct ModelAliasChoice {
    alias: String,
    provider: String,
    deployment: String,
    locale: String,
    priority: u16,
}

impl ModelAliasChoice {
    fn summary(&self) -> String {
        format!(
            "{} (provider={}, deployment={}, locale={}, priority={})",
            self.alias, self.provider, self.deployment, self.locale, self.priority
        )
    }
}

fn available_model_alias_choices(
    config: &AppConfig,
    capability: ModelCapability,
) -> Vec<ModelAliasChoice> {
    config
        .models
        .catalog
        .iter()
        .filter(|model| model.enabled && model.supports(capability))
        .map(|model| ModelAliasChoice {
            alias: model.alias.clone(),
            provider: model.provider.to_string(),
            deployment: model.deployment.to_string(),
            locale: model.locale.clone(),
            priority: model.priority,
        })
        .collect()
}

fn read_prompt_line<R: BufRead>(reader: &mut R) -> Result<String> {
    let mut input = String::new();
    let bytes = reader.read_line(&mut input)?;
    if bytes == 0 {
        return Ok(String::new());
    }
    Ok(input.trim().to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WizardLanguage {
    Zh,
    En,
}

impl WizardLanguage {
    fn from_choice_index(index: usize) -> Self {
        match index {
            1 => Self::En,
            _ => Self::Zh,
        }
    }

    fn as_choice_index(self) -> usize {
        match self {
            Self::Zh => 0,
            Self::En => 1,
        }
    }

    fn locale(self) -> &'static str {
        match self {
            Self::Zh => "zh-CN",
            Self::En => "en-US",
        }
    }
}

fn wizard_language_from_locale(locale: &str) -> WizardLanguage {
    if locale.to_ascii_lowercase().starts_with("en") {
        WizardLanguage::En
    } else {
        WizardLanguage::Zh
    }
}

fn wizard_text<'a>(language: WizardLanguage, zh: &'a str, en: &'a str) -> &'a str {
    match language {
        WizardLanguage::Zh => zh,
        WizardLanguage::En => en,
    }
}

fn tui_init_json(
    config: &AppConfig,
    path: &str,
    wrote_config: Option<&str>,
    check: &ConfigCheckReport,
    default_key: Option<&DefaultKeyMaterial>,
) -> serde_json::Value {
    json!({
        "title": "Meat Memory setup TUI",
        "mode": if wrote_config.is_some() { "generated_config" } else { "preview" },
        "config_path": path,
        "wrote_config": wrote_config,
        "default_key": default_key.as_ref().map(|key| json!({
            "key_id": key.key_id,
            "raw_key": key.raw_key,
            "path": key.path,
        })),
        "check": {
            "ok": check.ok,
            "warnings": check.warnings,
            "database": {
                "checked": check.database.checked,
                "ok": check.database.ok,
                "message": check.database.message,
            },
        },
        "llm": {
            "locale": config.models.default_locale,
            "reasoning": config.models.routing.reasoning.primary,
            "extraction": config.models.routing.extraction.primary,
            "vision": config.models.routing.vision.primary,
            "embedding": config.models.routing.embedding.primary,
        },
        "storage": {
            "pg": config.features.enable_pg,
            "markdown": config.features.enable_markdown,
            "markdown_root": config.markdown.root,
            "assets_root": config.assets.root,
        },
        "mcp": {
            "enabled": config.features.enable_mcp,
            "tools_url": format!("http://{}/mcp/tools", config.server.bind),
        },
        "next_steps": [
            "memory-cli config check",
            "memory-cli mcp info",
            "memory-cli serve"
        ],
    })
}

fn tui_init_lines(
    config: &AppConfig,
    path: &str,
    wrote_config: Option<&str>,
    check: &ConfigCheckReport,
    default_key: Option<&DefaultKeyMaterial>,
    language: Option<WizardLanguage>,
) -> Vec<String> {
    let title = match language {
        Some(WizardLanguage::Zh) => "Meat Memory 安装向导",
        _ => "Meat Memory Setup TUI",
    };
    let label = |zh: &'static str, en: &'static str| match language {
        Some(lang) => wizard_text(lang, zh, en),
        None => en,
    };
    let mut lines = vec![
        "┌──────────────────────────────────────────────┐".to_string(),
        format!("│ {title:<44}│"),
        "├──────────────────────────────────────────────┤".to_string(),
        format!("│ {}: {path}", label("配置", "Config")),
        format!(
            "│ {}: {}",
            label("LLM 语言", "LLM locale"),
            config.models.default_locale
        ),
        format!(
            "│ {}: {}",
            label("Reasoning", "Reasoning"),
            config.models.routing.reasoning.primary
        ),
        format!(
            "│ {}: {}",
            label("Extraction", "Extraction"),
            config.models.routing.extraction.primary
        ),
        format!(
            "│ {}: {}",
            label("Vision", "Vision"),
            config.models.routing.vision.primary
        ),
        format!(
            "│ {}: {}",
            label("Embedding", "Embedding"),
            config.models.routing.embedding.primary
        ),
        format!(
            "│ {}: pg={}, markdown={}",
            label("存储", "Stores"),
            config.features.enable_pg,
            config.features.enable_markdown
        ),
        format!(
            "│ {}: {}",
            label("Markdown", "Markdown"),
            config.markdown.root
        ),
        format!("│ {}: {}", label("Assets", "Assets"), config.assets.root),
        format!("│ MCP: {}", config.features.enable_mcp),
        format!(
            "│ {}: {}",
            label("检查", "Check"),
            if check.ok { "ok" } else { "warning" }
        ),
        format!(
            "│ {}: {}",
            label("数据库", "Database"),
            check.database.message
        ),
    ];
    if let Some(path) = wrote_config {
        lines.push(format!("│ {}: {path}", label("已写出配置", "Wrote config")));
    }
    if let Some(default_key) = default_key {
        lines.push(format!(
            "│ {}: {}",
            label("默认 Key", "Default key"),
            default_key.key_id
        ));
        lines.push(format!(
            "│ {}: {}",
            label("Key 文件", "Key file"),
            default_key.path
        ));
    }
    if !check.warnings.is_empty() {
        lines.push(format!("│ {}:", label("告警", "Warnings")));
        lines.extend(
            check
                .warnings
                .iter()
                .map(|warning| format!("│ - {warning}")),
        );
    }
    lines.extend([
        "├──────────────────────────────────────────────┤".to_string(),
        format!("│ {}: memory-cli config check", label("下一步", "Next")),
        format!("│ {}: memory-cli mcp info", label("下一步", "Next")),
        format!("│ {}: memory-cli serve", label("下一步", "Next")),
    ]);
    if let Some(path) = wrote_config {
        lines.push(format!(
            "│ {}: MEAT_MEMORY_CONFIG={} memory-cli config check",
            label("验证新配置", "Verify new config"),
            path
        ));
    }
    lines.push("└──────────────────────────────────────────────┘".to_string());
    lines
}

fn scope_id_or_default(scope_id: Option<String>, service_info: &ServiceInfo) -> ScopeId {
    scope_id
        .map(ScopeId::from_string)
        .unwrap_or_else(|| service_info.default_scope.clone())
}

async fn resolve_cli_request_context(
    kernel: &Kernel,
    raw_key: Option<&str>,
) -> Result<Option<RequestContext>> {
    let raw_key = raw_key
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            env::var("MEAT_MEMORY_KEY")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        });
    let Some(raw_key) = raw_key else {
        return Ok(None);
    };

    kernel
        .resolve_access_key_context(&raw_key)
        .await?
        .with_context(|| format!("access key not found or inactive: {raw_key}"))
        .map(Some)
}

async fn resolve_required_cli_request_context(
    kernel: &Kernel,
    raw_key: Option<&str>,
) -> Result<RequestContext> {
    resolve_cli_request_context(kernel, raw_key)
        .await?
        .context("MEAT_MEMORY_KEY or --key is required")
}

async fn get_source_for_context(
    kernel: &Kernel,
    context: &RequestContext,
    source_id: &str,
) -> Result<MemorySource> {
    let source = kernel
        .get_memory_source(SourceId::from_string(source_id))
        .await?
        .with_context(|| format!("memory source not found: {source_id}"))?;
    if source.owner_scope_id != context.owner_scope_id {
        bail!("source access forbidden for current meat memory key");
    }
    Ok(source)
}

async fn build_local_docs_sync_plan(
    kernel: &Kernel,
    context: &RequestContext,
    source: &MemorySource,
    local_root: Option<&str>,
    limit: usize,
) -> Result<memory_sync::LocalProjectDocumentSyncPlan> {
    let local_root = local_root
        .map(ToOwned::to_owned)
        .or_else(|| source.local_root.clone())
        .context("local_root is required; pass --local-root or set it on the source")?;
    let documents = kernel
        .list_project_documents(ListProjectDocumentsRequest {
            source_id: source.id.clone(),
            limit,
            query: None,
            context: Some(context.clone()),
        })
        .await?;
    let snapshots = documents
        .iter()
        .map(|document| ProjectDocumentSnapshot {
            canonical_uri: document.canonical_uri.clone(),
            content_hash: document.content_hash.clone(),
        })
        .collect::<Vec<_>>();
    LocalProjectDocumentSyncEngine::new(PathBuf::from(local_root))
        .scan(&snapshots)
        .context("failed to scan local project documents")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectInitSurface {
    Cli,
    Tui,
}

impl ProjectInitSurface {
    fn key_source(self) -> KeySourceKind {
        match self {
            Self::Cli => KeySourceKind::Cli,
            Self::Tui => KeySourceKind::Tui,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Tui => "tui",
        }
    }
}

#[derive(Debug, Clone)]
enum ProjectInitRequest {
    New {
        name: String,
        scope_id: ScopeId,
        scope_kind: KeyScopeKind,
        isolated: bool,
    },
    Existing {
        name: Option<String>,
        scope_id: ScopeId,
        key_id: Option<AccessKeyId>,
    },
}

#[derive(Debug, Clone)]
struct ProjectInitResult {
    is_new_project: bool,
    name: Option<String>,
    scope_id: ScopeId,
    key_id: Option<AccessKeyId>,
    raw_key: Option<String>,
    scope_kind: Option<KeyScopeKind>,
    isolated: Option<bool>,
    storage_mode: Option<StorageMode>,
    surface: ProjectInitSurface,
}

async fn run_project_init_interactive(
    kernel: &Kernel,
    args: &ProjectInitArgs,
    surface: ProjectInitSurface,
) -> Result<ProjectInitRequest> {
    let existing_keys = kernel
        .list_access_keys(args.list_limit)
        .await
        .unwrap_or_default();
    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    run_project_init_interactive_io(args, surface, &existing_keys, &mut reader, &mut writer)
}

fn run_project_init_interactive_io<R: BufRead, W: Write>(
    args: &ProjectInitArgs,
    surface: ProjectInitSurface,
    existing_keys: &[memory_domain::AccessKey],
    reader: &mut R,
    writer: &mut W,
) -> Result<ProjectInitRequest> {
    writeln!(writer, "Meat Memory project memory boundary wizard")?;
    writeln!(
        writer,
        "Use numbers to choose; press Enter to keep defaults."
    )?;
    writeln!(writer, "Surface: {}", surface.label())?;
    writeln!(writer)?;

    let project_choice = prompt_choice(
        reader,
        writer,
        "是否为新项目",
        &[
            ("是，新项目", "创建新的 scope 和 key"),
            ("否，已有项目", "选择或输入已有 scope"),
        ],
        0,
    )?;

    if project_choice == 0 {
        let default_name = args
            .name
            .clone()
            .unwrap_or_else(default_project_memory_name);
        let name = prompt_text(reader, writer, "项目名称", &default_name)?.unwrap_or(default_name);
        let sharing_choice = prompt_choice(
            reader,
            writer,
            "是否记忆和其他项目互通",
            &[
                ("互通", "允许同一 owner scope 下的非完全隔离检索"),
                ("不互通", "创建完全隔离 key，控制项目记忆边界"),
            ],
            1,
        )?;
        let scope_kind_choice = prompt_choice(
            reader,
            writer,
            "是团队还是个人记忆",
            &[
                ("团队记忆", "适合项目共享上下文"),
                ("个人记忆", "适合个人偏好和私有上下文"),
            ],
            0,
        )?;
        let scope_id = args
            .owner_scope_id
            .clone()
            .map(ScopeId::from_string)
            .unwrap_or_else(|| generated_project_scope_id(&name));
        let scope_kind = if scope_kind_choice == 0 {
            KeyScopeKind::Team
        } else {
            KeyScopeKind::Personal
        };
        return Ok(ProjectInitRequest::New {
            name,
            scope_id,
            scope_kind,
            isolated: sharing_choice == 1,
        });
    }

    let existing_choice = prompt_choice(
        reader,
        writer,
        "不是新项目，如何选择记忆边界",
        &[
            ("列出现有记忆列表", "从已有 key 的 owner scope 选择"),
            ("手动输入一个", "直接输入已有 scope_id"),
        ],
        0,
    )?;
    if existing_choice == 0 && !existing_keys.is_empty() {
        writeln!(writer, "现有记忆边界:")?;
        let choices = existing_keys
            .iter()
            .map(|key| {
                (
                    format!("{} ({})", key.display_name, key.owner_scope_id.as_str()),
                    format!(
                        "key={}, scope_kind={}, isolated={}",
                        key.id.as_str(),
                        key.scope_kind.as_str(),
                        key.is_fully_isolated
                    ),
                )
            })
            .collect::<Vec<_>>();
        let choice_refs = choices
            .iter()
            .map(|(title, description)| (title.as_str(), description.as_str()))
            .collect::<Vec<_>>();
        let selected = prompt_choice(reader, writer, "选择已有记忆边界", &choice_refs, 0)?;
        let key = &existing_keys[selected];
        return Ok(ProjectInitRequest::Existing {
            name: Some(key.display_name.clone()),
            scope_id: key.owner_scope_id.clone(),
            key_id: Some(key.id.clone()),
        });
    }

    if existing_choice == 0 {
        writeln!(writer, "没有可列出的现有 key，请手动输入 scope_id。")?;
    }
    let scope_id = prompt_text(reader, writer, "已有 scope_id", "scp_existing_project")?
        .context("scope_id is required for existing project")?;
    if scope_id.trim().is_empty() {
        bail!("scope_id is required for existing project");
    }
    Ok(ProjectInitRequest::Existing {
        name: args.name.clone(),
        scope_id: ScopeId::from_string(scope_id),
        key_id: None,
    })
}

fn build_non_interactive_project_init_request(
    args: &ProjectInitArgs,
) -> Result<ProjectInitRequest> {
    if args.existing {
        let scope_id = args
            .owner_scope_id
            .clone()
            .context("--existing requires --scope-id or --owner-scope-id")?;
        return Ok(ProjectInitRequest::Existing {
            name: args.name.clone(),
            scope_id: ScopeId::from_string(scope_id),
            key_id: None,
        });
    }

    let name = args
        .name
        .clone()
        .unwrap_or_else(default_project_memory_name);
    let scope_id = args
        .owner_scope_id
        .clone()
        .map(ScopeId::from_string)
        .unwrap_or_else(|| generated_project_scope_id(&name));
    let isolated = !args.shared;
    Ok(ProjectInitRequest::New {
        name,
        scope_id,
        scope_kind: parse_key_scope(&args.scope_kind)?,
        isolated,
    })
}

async fn apply_project_init_request(
    kernel: &Kernel,
    args: &ProjectInitArgs,
    request: ProjectInitRequest,
    surface: ProjectInitSurface,
) -> Result<ProjectInitResult> {
    match request {
        ProjectInitRequest::New {
            name,
            scope_id,
            scope_kind,
            isolated,
        } => {
            let storage_mode = parse_storage_mode(&args.storage)?;
            let result = kernel
                .create_access_key(CreateAccessKeyRequest {
                    raw_key: None,
                    display_name: format!("{} project memory", name),
                    source_id: None,
                    source_kind: surface.key_source(),
                    owner_principal_id: args.owner_principal_id.clone(),
                    owner_scope_id: scope_id.clone(),
                    scope_kind,
                    storage_mode,
                    is_fully_isolated: isolated,
                })
                .await?;
            Ok(ProjectInitResult {
                is_new_project: true,
                name: Some(name),
                scope_id,
                key_id: Some(result.access_key.id),
                raw_key: Some(result.raw_key),
                scope_kind: Some(scope_kind),
                isolated: Some(isolated),
                storage_mode: Some(storage_mode),
                surface,
            })
        }
        ProjectInitRequest::Existing {
            name,
            scope_id,
            key_id,
        } => Ok(ProjectInitResult {
            is_new_project: false,
            name,
            scope_id,
            key_id,
            raw_key: None,
            scope_kind: None,
            isolated: None,
            storage_mode: None,
            surface,
        }),
    }
}

fn project_init_result_json(result: &ProjectInitResult) -> serde_json::Value {
    json!({
        "is_new_project": result.is_new_project,
        "name": result.name.as_deref(),
        "scope_id": result.scope_id.as_str(),
        "key_id": result.key_id.as_ref().map(|key_id| key_id.as_str()),
        "raw_key": result.raw_key.as_deref(),
        "scope_kind": result.scope_kind.map(KeyScopeKind::as_str),
        "isolated": result.isolated,
        "storage_mode": result.storage_mode.map(StorageMode::as_str),
        "surface": result.surface.label(),
        "next_steps": [
            format!("export MEAT_MEMORY_KEY={}", result.raw_key.as_deref().unwrap_or("<existing-key>")),
            format!("memory-cli search <query> --scope-id {}", result.scope_id.as_str()),
            format!("memory-cli remember --scope-id {} --body <text>", result.scope_id.as_str())
        ],
    })
}

fn project_init_result_lines(result: &ProjectInitResult) -> Vec<String> {
    let mut lines = vec![
        "Meat Memory project boundary ready".to_string(),
        format!("New project: {}", result.is_new_project),
        format!("Scope ID: {}", result.scope_id.as_str()),
        format!("Surface: {}", result.surface.label()),
    ];
    if let Some(name) = result.name.as_ref() {
        lines.push(format!("Name: {name}"));
    }
    if let Some(key_id) = result.key_id.as_ref() {
        lines.push(format!("Key ID: {}", key_id.as_str()));
    }
    if let Some(raw_key) = result.raw_key.as_ref() {
        lines.push(format!("Raw key: {raw_key}"));
        lines.push(format!("Next: export MEAT_MEMORY_KEY={raw_key}"));
    } else {
        lines.push("Raw key: <use the existing key for this scope>".to_string());
    }
    if let Some(scope_kind) = result.scope_kind {
        lines.push(format!("Scope kind: {}", scope_kind.as_str()));
    }
    if let Some(isolated) = result.isolated {
        lines.push(format!("Isolated: {isolated}"));
    }
    lines.push(format!(
        "Use with: memory-cli remember --scope-id {} --body <text>",
        result.scope_id.as_str()
    ));
    lines
}

fn default_project_memory_name() -> String {
    env::current_dir()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "project".to_string())
}

fn generated_project_scope_id(name: &str) -> ScopeId {
    let slug = project_scope_slug(name);
    let suffix = ScopeId::new()
        .as_str()
        .trim_start_matches("scp_")
        .to_ascii_lowercase();
    ScopeId::from_string(format!("scp_{slug}_{suffix}"))
}

fn project_scope_slug(name: &str) -> String {
    let mut slug = String::new();
    let mut last_was_sep = false;
    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_sep = false;
        } else if !last_was_sep && !slug.is_empty() {
            slug.push('_');
            last_was_sep = true;
        }
    }
    while slug.ends_with('_') {
        slug.pop();
    }
    if slug.is_empty() {
        "project".to_string()
    } else {
        slug
    }
}

fn static_command_message(command: &Command) -> Option<&'static str> {
    match command {
        Command::Doctor => Some("Run ./docs/scripts/verify.sh for the full machine check."),
        Command::PrintPlan => Some("Execution plan lives in docs/tasks/tasklist.md"),
        _ => None,
    }
}

fn build_remember_request(
    args: &RememberArgs,
    service_info: &ServiceInfo,
    body: String,
) -> Result<RememberTextRequest> {
    let mut request = RememberTextRequest::new(
        scope_id_or_default(args.scope_id.clone(), service_info),
        body,
    );
    request.title = args.title.clone();
    request.artifact_kind = parse_artifact_kind(&args.artifact_kind)?;
    request.memory_kind = args
        .memory_kind
        .as_deref()
        .map(parse_memory_kind)
        .transpose()?;
    request.source_refs = args.source_refs.clone();
    request.visibility = parse_visibility(&args.visibility)?;
    request.sensitivity = parse_sensitivity(&args.sensitivity)?;
    Ok(request)
}

fn build_remember_image_request(
    args: &RememberImageArgs,
    service_info: &ServiceInfo,
    media_type: String,
    bytes: Vec<u8>,
) -> Result<RememberImageRequest> {
    let mut request = RememberImageRequest::new(
        scope_id_or_default(args.scope_id.clone(), service_info),
        media_type,
        bytes,
    );
    request.title = args.title.clone();
    request.body = args.body.clone();
    request.file_extension = args
        .file
        .extension()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned);
    request.memory_kind = args
        .memory_kind
        .as_deref()
        .map(parse_memory_kind)
        .transpose()?;
    request.source_refs = args.source_refs.clone();
    request.visibility = parse_visibility(&args.visibility)?;
    request.sensitivity = parse_sensitivity(&args.sensitivity)?;
    Ok(request)
}

fn build_search_request(args: &SearchArgs, service_info: &ServiceInfo) -> SearchContextRequest {
    let scope_id = scope_id_or_default(args.scope_id.clone(), service_info);
    let mut request = SearchContextRequest::new(scope_id, args.query.clone());
    request.limit = args.limit;
    request
}

fn remember_result_json(result: &RememberTextResult) -> serde_json::Value {
    json!({
        "artifact_id": result.artifact.id.as_str(),
        "memory_id": result.memory.id.as_str(),
        "scope_id": result.memory.scope_id.as_str(),
        "title": result.memory.title,
        "body": result.memory.body,
        "memory_kind": format!("{:?}", result.memory.kind).to_lowercase(),
        "memory_state": result.memory.state.as_str(),
        "evidence_count": result.memory.evidence_count,
        "wrote_pg": result.wrote_pg,
        "wrote_markdown": result.wrote_markdown
    })
}

fn remember_result_lines(result: &RememberTextResult) -> [String; 2] {
    [
        format!(
            "Remembered {} in scope {} (state={}, pg={}, markdown={})",
            result.memory.id.as_str(),
            result.memory.scope_id.as_str(),
            result.memory.state.as_str(),
            result.wrote_pg,
            result.wrote_markdown
        ),
        format!("Title: {}", result.memory.title),
    ]
}

fn remember_image_result_json(
    result: &RememberImageResult,
    asset_id: &str,
    asset_uri: &str,
) -> serde_json::Value {
    json!({
        "asset_id": asset_id,
        "asset_uri": asset_uri,
        "artifact_id": result.artifact.id.as_str(),
        "memory_id": result.memory.id.as_str(),
        "scope_id": result.memory.scope_id.as_str(),
        "title": result.memory.title,
        "body": result.memory.body,
        "memory_kind": format!("{:?}", result.memory.kind).to_lowercase(),
        "memory_state": result.memory.state.as_str(),
        "evidence_count": result.memory.evidence_count,
        "vision_caption": result.vision.as_ref().map(|vision| vision.caption.as_str()),
        "vision_model_alias": result.vision.as_ref().map(|vision| vision.model_alias.as_str()),
        "llm_notice": result
            .vision
            .as_ref()
            .and_then(|vision| vision.switch_notice.as_ref())
            .map(|notice| notice.message.as_str()),
        "wrote_pg": result.wrote_pg,
        "wrote_markdown": result.wrote_markdown
    })
}

fn remember_image_result_lines(result: &RememberImageResult, asset_uri: &str) -> Vec<String> {
    let mut lines = vec![
        format!(
            "Remembered image {} in scope {} (state={}, pg={}, markdown={})",
            result.memory.id.as_str(),
            result.memory.scope_id.as_str(),
            result.memory.state.as_str(),
            result.wrote_pg,
            result.wrote_markdown
        ),
        format!("Asset: {asset_uri}"),
        format!("Title: {}", result.memory.title),
    ];
    if let Some(vision) = &result.vision {
        lines.push(format!("Vision: {}", vision.caption));
        if let Some(notice) = vision.switch_notice.as_ref() {
            lines.push(format!("LLM Notice: {}", notice.message));
        }
    }
    lines
}

fn search_bundle_json(bundle: &ContextBundle) -> serde_json::Value {
    let memories = bundle
        .memories
        .iter()
        .map(|memory| {
            json!({
                "memory_id": memory.id.as_str(),
                "title": memory.title,
                "body": memory.body,
                "memory_kind": format!("{:?}", memory.kind).to_lowercase(),
                "memory_state": memory.state.as_str(),
                "evidence_count": memory.evidence_count
            })
        })
        .collect::<Vec<_>>();

    json!({
        "query": bundle.query,
        "scope_id": bundle.scope_id.as_str(),
        "memory_count": bundle.memories.len(),
        "entity_count": bundle.entities.len(),
        "relation_count": bundle.relations.len(),
        "memories": memories
    })
}

fn search_bundle_lines(bundle: &ContextBundle) -> Vec<String> {
    if bundle.memories.is_empty() {
        return vec![format!(
            "No memories matched query '{}' in scope {}.",
            bundle.query,
            bundle.scope_id.as_str()
        )];
    }

    let mut lines = vec![format!(
        "Found {} memories in scope {} for query '{}':",
        bundle.memories.len(),
        bundle.scope_id.as_str(),
        bundle.query
    )];
    lines.extend(
        bundle
            .memories
            .iter()
            .map(|memory| format!("- {} [{}]", memory.title, memory.id.as_str())),
    );
    lines
}

fn load_body(body: Option<String>, file: Option<PathBuf>) -> Result<String> {
    let mut stdin = io::stdin();
    load_body_from_reader(body, file, &mut stdin)
}

fn load_body_from_reader<R: Read>(
    body: Option<String>,
    file: Option<PathBuf>,
    reader: &mut R,
) -> Result<String> {
    if let Some(body) = body {
        if body.trim().is_empty() {
            bail!("--body must not be empty");
        }
        return Ok(body);
    }

    if let Some(path) = file {
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read body file {}", path.display()))?;
        if raw.trim().is_empty() {
            bail!("--file content must not be empty");
        }
        return Ok(raw);
    }

    let mut buffer = String::new();
    reader.read_to_string(&mut buffer)?;
    if buffer.trim().is_empty() {
        bail!("remember requires --body, --file, or non-empty stdin");
    }

    Ok(buffer)
}

fn detect_image_media_type(path: &Path, provided: Option<String>) -> Result<String> {
    if let Some(media_type) = provided {
        if media_type.trim().is_empty() {
            bail!("--media-type must not be empty");
        }
        return Ok(media_type);
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());
    match extension.as_deref() {
        Some("png") => Ok("image/png".to_string()),
        Some("jpg") | Some("jpeg") => Ok("image/jpeg".to_string()),
        Some("webp") => Ok("image/webp".to_string()),
        Some("gif") => Ok("image/gif".to_string()),
        _ => bail!("unable to infer image media type; pass --media-type explicitly"),
    }
}

fn print_json(value: serde_json::Value) -> Result<()> {
    println!("{}", render_json(value)?);
    Ok(())
}

fn render_json(value: serde_json::Value) -> Result<String> {
    Ok(serde_json::to_string_pretty(&value)?)
}

fn parse_artifact_kind(raw: &str) -> Result<ArtifactKind> {
    Ok(match raw {
        "message" => ArtifactKind::Message,
        "document" => ArtifactKind::Document,
        "code_diff" => ArtifactKind::CodeDiff,
        "code_file_snapshot" => ArtifactKind::CodeFileSnapshot,
        "terminal_output" => ArtifactKind::TerminalOutput,
        "image" => ArtifactKind::Image,
        "audio" => ArtifactKind::Audio,
        "video" => ArtifactKind::Video,
        "tool_result" => ArtifactKind::ToolResult,
        "web_page" => ArtifactKind::WebPage,
        other => bail!("unsupported artifact_kind: {other}"),
    })
}

fn parse_memory_kind(raw: &str) -> Result<MemoryKind> {
    Ok(match raw {
        "fact" => MemoryKind::Fact,
        "preference" => MemoryKind::Preference,
        "decision" => MemoryKind::Decision,
        "procedure" => MemoryKind::Procedure,
        "constraint" => MemoryKind::Constraint,
        "risk" => MemoryKind::Risk,
        "summary" => MemoryKind::Summary,
        "insight" => MemoryKind::Insight,
        other => bail!("unsupported memory_kind: {other}"),
    })
}

fn parse_visibility(raw: &str) -> Result<Visibility> {
    Ok(match raw {
        "private" => Visibility::Private,
        "project" => Visibility::Project,
        "team" => Visibility::Team,
        "organization" => Visibility::Organization,
        other => bail!("unsupported visibility: {other}"),
    })
}

fn parse_sensitivity(raw: &str) -> Result<Sensitivity> {
    Ok(match raw {
        "public" => Sensitivity::Public,
        "internal" => Sensitivity::Internal,
        "private" => Sensitivity::Private,
        "restricted" => Sensitivity::Restricted,
        other => bail!("unsupported sensitivity: {other}"),
    })
}

fn parse_key_source(raw: &str) -> Result<KeySourceKind> {
    KeySourceKind::parse(raw).map_err(Into::into)
}

fn parse_key_scope(raw: &str) -> Result<KeyScopeKind> {
    KeyScopeKind::parse(raw).map_err(Into::into)
}

fn parse_storage_mode(raw: &str) -> Result<StorageMode> {
    StorageMode::parse(raw).map_err(Into::into)
}

fn parse_source_sync_mode(raw: &str) -> Result<SourceSyncMode> {
    SourceSyncMode::parse(raw).map_err(Into::into)
}

fn parse_document_sync_state(raw: &str) -> Result<DocumentSyncState> {
    DocumentSyncState::parse(raw).map_err(Into::into)
}

fn parse_document_conflict_state(raw: &str) -> Result<DocumentConflictState> {
    DocumentConflictState::parse(raw).map_err(Into::into)
}

fn access_key_json(
    access_key: &memory_domain::AccessKey,
    raw_key: Option<&str>,
) -> serde_json::Value {
    json!({
        "key_id": access_key.id.as_str(),
        "raw_key": raw_key,
        "name": access_key.display_name,
        "source_id": access_key.source_id.as_ref().map(|source_id| source_id.as_str()),
        "source": access_key.source_kind.as_str(),
        "owner_principal_id": access_key.owner_principal_id,
        "owner_scope_id": access_key.owner_scope_id.as_str(),
        "scope_kind": access_key.scope_kind.as_str(),
        "storage_mode": access_key.storage_mode.as_str(),
        "isolated": access_key.is_fully_isolated,
        "isolation_group_id": access_key.isolation_group_id,
        "status": access_key.status.as_str(),
    })
}

fn memory_source_json(source: &MemorySource) -> serde_json::Value {
    json!({
        "source_id": source.id.as_str(),
        "source_kind": source.source_kind,
        "name": source.display_name,
        "owner_principal_id": source.owner_principal_id,
        "owner_scope_id": source.owner_scope_id.as_str(),
        "source_uri": source.source_uri,
        "sync_mode": source.sync_mode.as_str(),
        "local_root": source.local_root,
        "status": source.status.as_str(),
    })
}

fn agent_context_json(agent_context: &memory_domain::AgentContext) -> serde_json::Value {
    json!({
        "context_id": agent_context.id.as_str(),
        "source_id": agent_context.source_id.as_ref().map(|source_id| source_id.as_str()),
        "key_id": agent_context.key_id.as_ref().map(|key_id| key_id.as_str()),
        "scope_id": agent_context.scope_id.as_str(),
        "session_id": agent_context.session_id,
        "task_id": agent_context.task_id,
        "layer": agent_context.layer.as_str(),
        "title": agent_context.title,
        "body": agent_context.body,
        "labels": agent_context.labels,
    })
}

fn project_document_json(document: &memory_domain::ProjectDocument) -> serde_json::Value {
    json!({
        "document_id": document.id.as_str(),
        "source_id": document.source_id.as_str(),
        "scope_id": document.scope_id.as_str(),
        "local_path": document.local_path,
        "canonical_uri": document.canonical_uri,
        "title": document.title,
        "content_hash": document.content_hash,
        "sync_state": document.sync_state.as_str(),
        "conflict_state": document.conflict_state.as_str(),
        "artifact_id": document.artifact_id.as_ref().map(|artifact_id| artifact_id.as_str()),
        "memory_id": document.memory_id.as_ref().map(|memory_id| memory_id.as_str()),
    })
}

fn local_docs_plan_json(
    dry_run: bool,
    plan: &memory_sync::LocalProjectDocumentSyncPlan,
    imported: &[memory_domain::ProjectDocument],
) -> serde_json::Value {
    json!({
        "dry_run": dry_run,
        "root": plan.root.display().to_string(),
        "planned_documents": plan.documents.iter().map(|document| json!({
            "canonical_uri": document.canonical_uri,
            "local_path": document.local_path.display().to_string(),
            "title": document.title,
            "content_hash": document.content_hash,
            "sync_state": document.sync_state.as_str(),
        })).collect::<Vec<_>>(),
        "imported": imported.iter().map(project_document_json).collect::<Vec<_>>(),
        "missing": plan.missing,
        "conflicts": plan.conflicts,
    })
}

fn print_docs_sync_summary(
    dry_run: bool,
    planned_count: usize,
    imported_count: usize,
    missing_count: usize,
    conflict_count: usize,
) {
    println!(
        "Project document sync {}",
        if dry_run { "plan" } else { "completed" }
    );
    println!("Planned documents: {planned_count}");
    println!("Imported documents: {imported_count}");
    println!("Missing documents: {missing_count}");
    println!("Conflicts: {conflict_count}");
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use super::{
        api_metadata, bootstrap_loaded_config, build_cli_router, build_kernel,
        build_non_interactive_project_init_request, build_remember_image_request,
        build_remember_request, build_search_request, config_check_report, detect_image_media_type,
        ensure_default_key_material, export_skill_bundle, generated_project_scope_id, load_body,
        load_body_from_reader, parse_artifact_kind, parse_memory_kind, parse_sensitivity,
        parse_visibility, project_init_result_json, project_init_result_lines, project_scope_slug,
        remember_command_with_runtime, remember_image_command_with_runtime,
        remember_image_result_json, remember_image_result_lines, remember_result_json,
        remember_result_lines, render_json, resolve_bind, scope_id_or_default, search_bundle_json,
        search_bundle_lines, search_command_with_runtime, serve_command_with_runtime,
        static_command_message, tui_init_json, tui_init_lines, validate_model_registry,
        write_tui_config_if_requested,
    };
    use axum::{body::Body, http::Request};
    use memory_assets::{AssetMetadata, AssetRef, StorageClass, StoredAsset};
    use memory_config::AppConfig;
    use memory_core::ServiceInfo;
    use memory_domain::{
        AccessKey, AccessKeyId, Artifact, ArtifactKind, ContextBundle, KeyScopeKind, KeySourceKind,
        Memory, MemoryKind, MemoryState, ScopeId, Sensitivity, StorageMode, Visibility,
    };
    use memory_kernel::{RememberImageResult, RememberTextResult};
    use memory_models::VisionResponse;
    use serde_json::json;
    use std::{
        fs,
        io::Cursor,
        path::{Path, PathBuf},
        sync::Arc,
    };
    use tempfile::tempdir;
    use time::macros::datetime;
    use tower::ServiceExt;

    fn sample_memory() -> Memory {
        let mut memory = Memory::new(
            ScopeId::from_string("scp_cli"),
            MemoryKind::Decision,
            "保留单测优先",
            "先把 unit 补齐",
        )
        .unwrap();
        memory.id = memory_domain::MemoryId::from_string("mem_cli");
        memory.state = MemoryState::Active;
        memory.created_at = datetime!(2025-01-02 03:04:05 UTC);
        memory.updated_at = datetime!(2025-01-03 04:05:06 UTC);
        memory
    }

    fn sample_artifact() -> Artifact {
        let mut artifact = Artifact::new(
            ScopeId::from_string("scp_cli"),
            ArtifactKind::Message,
            "先把 unit 补齐",
            vec!["session://1".to_string()],
        )
        .unwrap();
        artifact.id = memory_domain::ArtifactId::from_string("art_cli");
        artifact
    }

    fn sample_text_result() -> RememberTextResult {
        RememberTextResult {
            artifact: sample_artifact(),
            memory: sample_memory(),
            wrote_pg: true,
            wrote_markdown: true,
        }
    }

    fn sample_image_result(with_vision: bool) -> RememberImageResult {
        sample_image_result_with_notice(with_vision, None)
    }

    fn sample_image_result_with_notice(
        with_vision: bool,
        notice: Option<&str>,
    ) -> RememberImageResult {
        let asset_ref = AssetRef::new(
            "asset_cli",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "image/png",
            StorageClass::Raw,
            "raw/sha256/01/23/sample.png",
        )
        .unwrap();
        let stored_asset = StoredAsset {
            reference: asset_ref,
            metadata: AssetMetadata {
                asset_id: "asset_cli".to_string(),
                sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .to_string(),
                size_bytes: 128,
                mime_type: "image/png".to_string(),
                storage_class: StorageClass::Raw,
                width: Some(800),
                height: Some(600),
                duration_ms: None,
                page_count: None,
                codec: None,
            },
            absolute_path: PathBuf::from("/tmp/raw/sha256/01/23/sample.png"),
        };

        RememberImageResult {
            asset: stored_asset,
            artifact: sample_artifact(),
            memory: sample_memory(),
            vision: with_vision.then(|| VisionResponse {
                caption: "登录页截图".to_string(),
                structured: json!({"scene": "login"}),
                model_alias: "mock-vision".to_string(),
                switch_notice: notice.map(|message| memory_models::ModelSwitchNotice {
                    capability: memory_models::ModelCapability::Vision,
                    from_model_alias: "gemini_vision".to_string(),
                    from_model_display_name: "Gemini Vision".to_string(),
                    to_model_alias: "mock-vision".to_string(),
                    to_model_display_name: "Mock Vision".to_string(),
                    reason: "provider unavailable".to_string(),
                    message: message.to_string(),
                }),
            }),
            wrote_pg: false,
            wrote_markdown: true,
        }
    }

    fn sample_context_bundle(memories: Vec<Memory>) -> ContextBundle {
        ContextBundle {
            query: "测试覆盖率".to_string(),
            scope_id: ScopeId::from_string("scp_cli"),
            memories,
            entities: Vec::new(),
            relations: Vec::new(),
            generated_at: datetime!(2025-01-02 03:04:05 UTC),
        }
    }

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

    async fn build_local_runtime(
        root: &Path,
        enable_mcp: bool,
    ) -> (AppConfig, memory_kernel::Kernel, ServiceInfo) {
        let markdown_root = root.join("markdown");
        let asset_root = root.join("assets");
        fs::create_dir_all(&markdown_root).unwrap();
        fs::create_dir_all(&asset_root).unwrap();

        let config = sample_config(
            &markdown_root.display().to_string(),
            &asset_root.display().to_string(),
            enable_mcp,
        );
        let (kernel, service_info) = bootstrap_loaded_config(&config).await.unwrap();
        (config, kernel, service_info)
    }

    #[test]
    fn load_body_prefers_inline_text() {
        let body = load_body(Some("hello memory".to_string()), None).unwrap();
        assert_eq!(body, "hello memory");
    }

    #[test]
    fn load_body_rejects_empty_inline_text() {
        assert!(load_body(Some("   ".to_string()), None).is_err());
    }

    #[test]
    fn load_body_requires_input_source() {
        assert!(load_body(None, Some(PathBuf::from("/definitely/missing.txt"))).is_err());
    }

    #[test]
    fn load_body_reads_file_and_rejects_empty_file() {
        let dir = tempdir().unwrap();
        let body_path = dir.path().join("body.txt");
        fs::write(&body_path, "来自文件的内容\n").unwrap();
        assert_eq!(
            load_body(None, Some(body_path)).unwrap(),
            "来自文件的内容\n"
        );

        let empty_path = dir.path().join("empty.txt");
        fs::write(&empty_path, "   ").unwrap();
        assert!(load_body(None, Some(empty_path)).is_err());
    }

    #[test]
    fn load_body_reads_from_supplied_reader() {
        let mut stdin = Cursor::new("来自标准输入\n");
        let body = load_body_from_reader(None, None, &mut stdin).unwrap();
        assert_eq!(body, "来自标准输入\n");

        let mut empty = Cursor::new("   ");
        assert!(load_body_from_reader(None, None, &mut empty).is_err());
    }

    #[test]
    fn parses_all_cli_enum_variants() {
        let artifact_cases = [
            ("message", ArtifactKind::Message),
            ("document", ArtifactKind::Document),
            ("code_diff", ArtifactKind::CodeDiff),
            ("code_file_snapshot", ArtifactKind::CodeFileSnapshot),
            ("terminal_output", ArtifactKind::TerminalOutput),
            ("image", ArtifactKind::Image),
            ("audio", ArtifactKind::Audio),
            ("video", ArtifactKind::Video),
            ("tool_result", ArtifactKind::ToolResult),
            ("web_page", ArtifactKind::WebPage),
        ];
        for (raw, expected) in artifact_cases {
            assert_eq!(parse_artifact_kind(raw).unwrap(), expected);
        }

        let memory_cases = [
            ("fact", MemoryKind::Fact),
            ("preference", MemoryKind::Preference),
            ("decision", MemoryKind::Decision),
            ("procedure", MemoryKind::Procedure),
            ("constraint", MemoryKind::Constraint),
            ("risk", MemoryKind::Risk),
            ("summary", MemoryKind::Summary),
            ("insight", MemoryKind::Insight),
        ];
        for (raw, expected) in memory_cases {
            assert_eq!(parse_memory_kind(raw).unwrap(), expected);
        }

        let visibility_cases = [
            ("private", Visibility::Private),
            ("project", Visibility::Project),
            ("team", Visibility::Team),
            ("organization", Visibility::Organization),
        ];
        for (raw, expected) in visibility_cases {
            assert_eq!(parse_visibility(raw).unwrap(), expected);
        }

        let sensitivity_cases = [
            ("public", Sensitivity::Public),
            ("internal", Sensitivity::Internal),
            ("private", Sensitivity::Private),
            ("restricted", Sensitivity::Restricted),
        ];
        for (raw, expected) in sensitivity_cases {
            assert_eq!(parse_sensitivity(raw).unwrap(), expected);
        }
    }

    #[test]
    fn rejects_unknown_cli_enum_variants() {
        assert!(parse_artifact_kind("unknown").is_err());
        assert!(parse_memory_kind("unknown").is_err());
        assert!(parse_visibility("unknown").is_err());
        assert!(parse_sensitivity("unknown").is_err());
    }

    #[test]
    fn infers_and_validates_image_media_type() {
        assert_eq!(
            detect_image_media_type(&PathBuf::from("capture.png"), None).unwrap(),
            "image/png"
        );
        assert_eq!(
            detect_image_media_type(&PathBuf::from("capture.jpg"), None).unwrap(),
            "image/jpeg"
        );
        assert_eq!(
            detect_image_media_type(&PathBuf::from("capture.jpeg"), None).unwrap(),
            "image/jpeg"
        );
        assert_eq!(
            detect_image_media_type(&PathBuf::from("capture.webp"), None).unwrap(),
            "image/webp"
        );
        assert_eq!(
            detect_image_media_type(&PathBuf::from("capture.gif"), None).unwrap(),
            "image/gif"
        );
        assert_eq!(
            detect_image_media_type(
                &PathBuf::from("capture.bin"),
                Some("image/custom".to_string())
            )
            .unwrap(),
            "image/custom"
        );
        assert!(detect_image_media_type(&PathBuf::from("capture.bin"), None).is_err());
        assert!(
            detect_image_media_type(&PathBuf::from("capture.png"), Some("   ".to_string()))
                .is_err()
        );
    }

    #[test]
    fn resolves_bind_and_renders_json_output() {
        let config = sample_config("/tmp/md", "/tmp/assets", false);
        let rendered = render_json(json!({"ok": true, "count": 2})).unwrap();

        assert_eq!(resolve_bind(None, &config), "127.0.0.1:18080");
        assert_eq!(
            resolve_bind(Some("127.0.0.1:19090".to_string()), &config),
            "127.0.0.1:19090"
        );
        assert!(rendered.contains("\"ok\": true"));
        assert!(rendered.contains("\"count\": 2"));
    }

    #[test]
    fn scope_id_defaults_to_service_info_when_missing() {
        let service_info = ServiceInfo::default();

        assert_eq!(
            scope_id_or_default(Some("scp_override".to_string()), &service_info).as_str(),
            "scp_override"
        );
        assert_eq!(
            scope_id_or_default(None, &service_info).as_str(),
            service_info.default_scope.as_str()
        );
    }

    #[test]
    fn exposes_static_messages_for_non_runtime_commands() {
        assert_eq!(
            static_command_message(&super::Command::Doctor),
            Some("Run ./docs/scripts/verify.sh for the full machine check.")
        );
        assert_eq!(
            static_command_message(&super::Command::PrintPlan),
            Some("Execution plan lives in docs/tasks/tasklist.md")
        );
        assert_eq!(
            static_command_message(&super::Command::Search(super::SearchArgs {
                query: "q".to_string(),
                key: None,
                scope_id: None,
                limit: 10,
                json: false,
            })),
            None
        );
    }

    #[test]
    fn renders_remember_outputs_for_json_and_text() {
        let result = sample_text_result();
        let payload = remember_result_json(&result);
        let lines = remember_result_lines(&result);

        assert_eq!(payload["artifact_id"], "art_cli");
        assert_eq!(payload["memory_id"], "mem_cli");
        assert_eq!(payload["memory_kind"], "decision");
        assert_eq!(payload["wrote_pg"], true);
        assert!(lines[0].contains("Remembered mem_cli in scope scp_cli"));
        assert_eq!(lines[1], "Title: 保留单测优先");
    }

    #[test]
    fn renders_image_outputs_with_and_without_vision() {
        let asset_uri = "asset://raw/sha256/01/23/sample.png";
        let with_vision = sample_image_result_with_notice(
            true,
            Some("Gemini Vision LLM 不可用，已经切换到Mock Vision"),
        );
        let without_vision = sample_image_result(false);

        let payload = remember_image_result_json(&with_vision, "asset_cli", asset_uri);
        let lines_with_vision = remember_image_result_lines(&with_vision, asset_uri);
        let lines_without_vision = remember_image_result_lines(&without_vision, asset_uri);

        assert_eq!(payload["asset_id"], "asset_cli");
        assert_eq!(payload["asset_uri"], asset_uri);
        assert_eq!(payload["vision_caption"], "登录页截图");
        assert_eq!(payload["vision_model_alias"], "mock-vision");
        assert_eq!(
            payload["llm_notice"],
            "Gemini Vision LLM 不可用，已经切换到Mock Vision"
        );
        assert!(
            lines_with_vision
                .iter()
                .any(|line| line == "Vision: 登录页截图")
        );
        assert!(lines_with_vision.iter().any(|line| {
            line == "LLM Notice: Gemini Vision LLM 不可用，已经切换到Mock Vision"
        }));
        assert!(
            !lines_without_vision
                .iter()
                .any(|line| line.starts_with("Vision:"))
        );
    }

    #[test]
    fn renders_search_outputs_for_empty_and_non_empty_results() {
        let empty_bundle = sample_context_bundle(Vec::new());
        let filled_bundle = sample_context_bundle(vec![sample_memory()]);

        let empty_lines = search_bundle_lines(&empty_bundle);
        let filled_lines = search_bundle_lines(&filled_bundle);
        let payload = search_bundle_json(&filled_bundle);

        assert_eq!(
            empty_lines,
            vec!["No memories matched query '测试覆盖率' in scope scp_cli.".to_string()]
        );
        assert!(filled_lines[0].contains("Found 1 memories in scope scp_cli"));
        assert!(filled_lines[1].contains("- 保留单测优先 [mem_cli]"));
        assert_eq!(payload["memory_count"], 1);
        assert_eq!(payload["entity_count"], 0);
        assert_eq!(payload["relation_count"], 0);
        assert_eq!(payload["memories"][0]["memory_id"], "mem_cli");
    }

    #[test]
    fn build_remember_request_maps_fields_and_defaults() {
        let service_info = ServiceInfo::default();
        let request = build_remember_request(
            &super::RememberArgs {
                key: None,
                scope_id: Some("scp_override".to_string()),
                title: Some("记忆标题".to_string()),
                body: None,
                file: None,
                artifact_kind: "document".to_string(),
                memory_kind: Some("summary".to_string()),
                source_refs: vec!["doc://1".to_string()],
                visibility: "team".to_string(),
                sensitivity: "restricted".to_string(),
                json: true,
            },
            &service_info,
            "正文".to_string(),
        )
        .unwrap();

        assert_eq!(request.scope_id.as_str(), "scp_override");
        assert_eq!(request.title.as_deref(), Some("记忆标题"));
        assert_eq!(request.body, "正文");
        assert_eq!(request.artifact_kind, ArtifactKind::Document);
        assert_eq!(request.memory_kind, Some(MemoryKind::Summary));
        assert_eq!(request.source_refs, vec!["doc://1".to_string()]);
        assert_eq!(request.visibility, Visibility::Team);
        assert_eq!(request.sensitivity, Sensitivity::Restricted);
    }

    #[test]
    fn build_remember_image_request_maps_fields_and_file_extension() {
        let service_info = ServiceInfo::default();
        let request = build_remember_image_request(
            &super::RememberImageArgs {
                key: None,
                scope_id: None,
                title: Some("截图".to_string()),
                body: Some("登录页".to_string()),
                file: PathBuf::from("capture.jpeg"),
                media_type: None,
                memory_kind: Some("decision".to_string()),
                source_refs: vec!["image://1".to_string()],
                visibility: "project".to_string(),
                sensitivity: "private".to_string(),
                json: false,
            },
            &service_info,
            "image/jpeg".to_string(),
            vec![1, 2, 3],
        )
        .unwrap();

        assert_eq!(
            request.scope_id.as_str(),
            service_info.default_scope.as_str()
        );
        assert_eq!(request.title.as_deref(), Some("截图"));
        assert_eq!(request.body.as_deref(), Some("登录页"));
        assert_eq!(request.media_type, "image/jpeg");
        assert_eq!(request.bytes, vec![1, 2, 3]);
        assert_eq!(request.file_extension.as_deref(), Some("jpeg"));
        assert_eq!(request.memory_kind, Some(MemoryKind::Decision));
        assert_eq!(request.source_refs, vec!["image://1".to_string()]);
        assert_eq!(request.visibility, Visibility::Project);
        assert_eq!(request.sensitivity, Sensitivity::Private);
    }

    #[test]
    fn build_search_request_uses_scope_override_and_limit() {
        let service_info = ServiceInfo::default();
        let request = build_search_request(
            &super::SearchArgs {
                query: "覆盖率".to_string(),
                key: None,
                scope_id: Some("scp_search".to_string()),
                limit: 25,
                json: true,
            },
            &service_info,
        );

        assert_eq!(request.scope_id.as_str(), "scp_search");
        assert_eq!(request.query, "覆盖率");
        assert_eq!(request.limit, 25);
    }

    #[test]
    fn api_metadata_and_model_registry_validation_work_for_cli() {
        let config = sample_config("/tmp/md", "/tmp/assets", true);
        let metadata = api_metadata(&config, &ServiceInfo::default());

        assert_eq!(metadata.service, "meat-memory");
        assert!(metadata.features.markdown);
        assert!(metadata.features.http);
        assert!(metadata.features.mcp);
        assert!(!metadata.features.pg);
        validate_model_registry(&config).unwrap();
    }

    #[tokio::test]
    async fn tui_init_overrides_generate_valid_config_file() {
        let tempdir = tempdir().unwrap();
        let output = tempdir.path().join("generated.toml");
        let mut config = sample_config("/tmp/md", "/tmp/assets", false);
        let args = super::TuiInitArgs {
            json: false,
            interactive: false,
            default_locale: None,
            write_config: Some(output.clone()),
            force: false,
            check_database: false,
            enable_mcp: true,
            disable_mcp: false,
            database_url: Some("postgres://postgres:postgres@127.0.0.1:5433/custom".to_string()),
            markdown_root: Some("./docs/default".to_string()),
            assets_root: Some("./storage/assets".to_string()),
            reasoning_primary: Some("chatgpt_reasoning".to_string()),
            extraction_primary: Some("chatgpt_reasoning".to_string()),
            vision_primary: None,
            embedding_primary: None,
        };

        super::apply_tui_init_overrides(&mut config, &args);
        let report = config_check_report(&config, "config/app.toml", false)
            .await
            .unwrap();
        let wrote = write_tui_config_if_requested(&config, &args).unwrap();
        let rendered = fs::read_to_string(&output).unwrap();
        let payload = tui_init_json(&config, "config/app.toml", wrote.as_deref(), &report, None);
        let lines = tui_init_lines(
            &config,
            "config/app.toml",
            wrote.as_deref(),
            &report,
            None,
            None,
        );

        assert!(report.ok);
        assert_eq!(wrote.as_deref(), Some(output.to_str().unwrap()));
        assert!(rendered.contains("enable_mcp = true"));
        assert!(
            rendered
                .contains("database_url = \"postgres://postgres:postgres@127.0.0.1:5433/custom\"")
        );
        assert_eq!(payload["mode"], "generated_config");
        assert!(lines.iter().any(|line| line.contains("Wrote config")));
    }

    #[test]
    fn interactive_tui_init_collects_user_overrides() {
        let config = sample_config("/tmp/md", "/tmp/assets", false);
        let args = super::TuiInitArgs {
            json: false,
            interactive: true,
            default_locale: None,
            write_config: None,
            force: false,
            check_database: false,
            enable_mcp: false,
            disable_mcp: false,
            database_url: None,
            markdown_root: None,
            assets_root: None,
            reasoning_primary: None,
            extraction_primary: None,
            vision_primary: None,
            embedding_primary: None,
        };
        let input = b"2\n2\ny\npostgres://postgres:postgres@127.0.0.1:5433/interactive\n./docs/interactive\n./storage/interactive-assets\n1\n1\n1\n1\ny\nconfig/local.interactive.toml\ny\ny\n";
        let mut reader = Cursor::new(input.as_slice());
        let mut output = Vec::new();

        let updated =
            super::run_tui_init_interactive_io(&config, &args, &mut reader, &mut output).unwrap();

        assert!(updated.enable_mcp);
        assert!(!updated.disable_mcp);
        assert_eq!(updated.default_locale.as_deref(), Some("en-US"));
        assert_eq!(
            updated.database_url.as_deref(),
            Some("postgres://postgres:postgres@127.0.0.1:5433/interactive")
        );
        assert_eq!(updated.markdown_root.as_deref(), Some("./docs/interactive"));
        assert_eq!(
            updated.assets_root.as_deref(),
            Some("./storage/interactive-assets")
        );
        assert_eq!(
            updated.reasoning_primary.as_deref(),
            Some("chatgpt_reasoning")
        );
        assert_eq!(
            updated.extraction_primary.as_deref(),
            Some("chatgpt_reasoning")
        );
        assert_eq!(updated.vision_primary.as_deref(), Some("chatgpt_vision"));
        assert_eq!(
            updated.embedding_primary.as_deref(),
            Some("chatgpt_embedding")
        );
        assert!(updated.check_database);
        assert_eq!(
            updated.write_config.as_deref(),
            Some(Path::new("config/local.interactive.toml"))
        );
        assert!(updated.force);
        let rendered = String::from_utf8(output).unwrap();
        assert!(rendered.contains("Choose language / 选择语言"));
        assert!(rendered.contains("Step 1/5: choose setup profile"));
        assert!(rendered.contains("MCP-ready"));
        assert!(rendered.contains("Available reasoning aliases"));
        assert!(rendered.contains("1. chatgpt_reasoning"));
        assert!(rendered.contains("Review summary"));
        assert!(rendered.contains("Write config to file"));
    }

    #[test]
    fn interactive_tui_init_can_be_cancelled() {
        let config = sample_config("/tmp/md", "/tmp/assets", false);
        let args = super::TuiInitArgs {
            json: false,
            interactive: true,
            default_locale: None,
            write_config: None,
            force: false,
            check_database: false,
            enable_mcp: false,
            disable_mcp: false,
            database_url: None,
            markdown_root: None,
            assets_root: None,
            reasoning_primary: None,
            extraction_primary: None,
            vision_primary: None,
            embedding_primary: None,
        };
        let input = b"\n\n\n\n\n\n\n\n\n\n\n\nn\n";
        let mut reader = Cursor::new(input.as_slice());
        let mut output = Vec::new();

        let error = super::run_tui_init_interactive_io(&config, &args, &mut reader, &mut output)
            .expect_err("interactive flow should support cancel");

        assert!(error.to_string().contains("取消") || error.to_string().contains("cancelled"));
    }

    fn sample_project_init_args() -> super::ProjectInitArgs {
        super::ProjectInitArgs {
            json: false,
            interactive: true,
            existing: false,
            name: Some("Meat Memory".to_string()),
            owner_principal_id: "rou".to_string(),
            owner_scope_id: None,
            scope_kind: "team".to_string(),
            storage: "all".to_string(),
            isolated: false,
            shared: false,
            list_limit: 20,
        }
    }

    #[test]
    fn project_scope_slug_keeps_boundaries_ascii_and_stable() {
        assert_eq!(project_scope_slug("Meat Memory V2.6"), "meat_memory_v2_6");
        assert_eq!(project_scope_slug("  项目  "), "project");
        assert!(
            generated_project_scope_id("Meat Memory")
                .as_str()
                .starts_with("scp_meat_memory_")
        );
    }

    #[test]
    fn interactive_project_init_collects_new_project_boundary() {
        let args = sample_project_init_args();
        let mut reader = Cursor::new(b"1\nAlpha Project\n2\n1\n".as_slice());
        let mut output = Vec::new();

        let request = super::run_project_init_interactive_io(
            &args,
            super::ProjectInitSurface::Tui,
            &[],
            &mut reader,
            &mut output,
        )
        .unwrap();

        match request {
            super::ProjectInitRequest::New {
                name,
                scope_id,
                scope_kind,
                isolated,
            } => {
                assert_eq!(name, "Alpha Project");
                assert!(scope_id.as_str().starts_with("scp_alpha_project_"));
                assert_eq!(scope_kind, KeyScopeKind::Team);
                assert!(isolated);
            }
            other => panic!("expected new project request, got {other:?}"),
        }
        let rendered = String::from_utf8(output).unwrap();
        assert!(rendered.contains("是否为新项目"));
        assert!(rendered.contains("是否记忆和其他项目互通"));
        assert!(rendered.contains("是团队还是个人记忆"));
    }

    #[test]
    fn interactive_project_init_can_select_existing_boundary() {
        let args = sample_project_init_args();
        let existing_key = AccessKey::new(
            "mmk_existing",
            "Existing Project",
            KeySourceKind::Cli,
            "rou",
            ScopeId::from_string("scp_existing"),
            KeyScopeKind::Personal,
            StorageMode::All,
            false,
        )
        .unwrap();
        let expected_key_id = existing_key.id.clone();
        let mut reader = Cursor::new(b"2\n1\n1\n".as_slice());
        let mut output = Vec::new();

        let request = super::run_project_init_interactive_io(
            &args,
            super::ProjectInitSurface::Cli,
            &[existing_key],
            &mut reader,
            &mut output,
        )
        .unwrap();

        match request {
            super::ProjectInitRequest::Existing {
                name,
                scope_id,
                key_id,
            } => {
                assert_eq!(name.as_deref(), Some("Existing Project"));
                assert_eq!(scope_id.as_str(), "scp_existing");
                assert_eq!(key_id, Some(expected_key_id));
            }
            other => panic!("expected existing project request, got {other:?}"),
        }
        let rendered = String::from_utf8(output).unwrap();
        assert!(rendered.contains("现有记忆边界"));
        assert!(rendered.contains("选择已有记忆边界"));
    }

    #[test]
    fn non_interactive_project_init_supports_shared_personal_and_existing() {
        let mut args = sample_project_init_args();
        args.interactive = false;
        args.name = Some("Solo Project".to_string());
        args.scope_kind = "personal".to_string();
        args.shared = true;
        let request = build_non_interactive_project_init_request(&args).unwrap();

        match request {
            super::ProjectInitRequest::New {
                name,
                scope_id,
                scope_kind,
                isolated,
            } => {
                assert_eq!(name, "Solo Project");
                assert!(scope_id.as_str().starts_with("scp_solo_project_"));
                assert_eq!(scope_kind, KeyScopeKind::Personal);
                assert!(!isolated);
            }
            other => panic!("expected new project request, got {other:?}"),
        }

        args.existing = true;
        args.owner_scope_id = Some("scp_existing_manual".to_string());
        let request = build_non_interactive_project_init_request(&args).unwrap();
        match request {
            super::ProjectInitRequest::Existing {
                name,
                scope_id,
                key_id,
            } => {
                assert_eq!(name.as_deref(), Some("Solo Project"));
                assert_eq!(scope_id.as_str(), "scp_existing_manual");
                assert_eq!(key_id, None);
            }
            other => panic!("expected existing project request, got {other:?}"),
        }

        args.owner_scope_id = None;
        let error = build_non_interactive_project_init_request(&args)
            .expect_err("existing non-interactive mode should require a scope");
        assert!(error.to_string().contains("--existing requires"));
    }

    #[test]
    fn project_init_outputs_include_next_scope_and_key_steps() {
        let result = super::ProjectInitResult {
            is_new_project: true,
            name: Some("Alpha".to_string()),
            scope_id: ScopeId::from_string("scp_alpha"),
            key_id: Some(AccessKeyId::from_string("key_alpha")),
            raw_key: Some("mmk_alpha".to_string()),
            scope_kind: Some(KeyScopeKind::Team),
            isolated: Some(true),
            storage_mode: Some(StorageMode::All),
            surface: super::ProjectInitSurface::Tui,
        };

        let payload = project_init_result_json(&result);
        let lines = project_init_result_lines(&result);

        assert_eq!(payload["scope_id"], "scp_alpha");
        assert_eq!(payload["raw_key"], "mmk_alpha");
        assert_eq!(payload["scope_kind"], "team");
        assert_eq!(payload["surface"], "tui");
        assert!(lines.iter().any(|line| line == "Scope ID: scp_alpha"));
        assert!(
            lines
                .iter()
                .any(|line| line == "Next: export MEAT_MEMORY_KEY=mmk_alpha")
        );
    }

    #[test]
    fn mcp_http_check_reports_unreachable_endpoint() {
        let mut config = sample_config("/tmp/md", "/tmp/assets", true);
        config.server.bind = "127.0.0.1:9".to_string();

        let check = super::check_mcp_http_endpoint(&config);

        assert!(check.checked);
        assert!(!check.ok);
        assert!(check.message.contains("unreachable"));
    }

    #[test]
    fn export_skill_bundle_writes_expected_files() {
        let tempdir = tempdir().unwrap();
        let output = tempdir.path().join("skills");
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();

        let exported = export_skill_bundle(&root, super::SkillExportTarget::All, &output, true)
            .expect("skill export should succeed");

        assert_eq!(exported.len(), 9);
        assert!(output.join("README.md").exists());
        assert!(output.join("codex-meat-memory/SKILL.md").exists());
        assert!(output.join("codex-meat-memory/agents/openai.yaml").exists());
        assert!(output.join("codex-meat-memory/assets/icon.svg").exists());
        assert!(output.join("claude-code-meat-memory/SKILL.md").exists());
        assert!(
            output
                .join("claude-code-meat-memory/agents/openai.yaml")
                .exists()
        );
        assert!(
            output
                .join("claude-code-meat-memory/assets/icon.svg")
                .exists()
        );
        assert!(output.join("execution-agent-meat-memory/SKILL.md").exists());
        assert!(
            output
                .join("execution-agent-meat-memory/agents/openai.yaml")
                .exists()
        );
        assert!(
            output
                .join("execution-agent-meat-memory/assets/icon.svg")
                .exists()
        );
    }

    #[test]
    fn export_skill_bundle_requires_force_for_existing_directory() {
        let tempdir = tempdir().unwrap();
        let output = tempdir.path().join("skills");
        fs::create_dir_all(&output).unwrap();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();

        let error = export_skill_bundle(&root, super::SkillExportTarget::Codex, &output, false)
            .expect_err("existing output dir should require force");
        assert!(error.to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn build_kernel_and_cli_router_cover_local_runtime_paths() {
        let tempdir = tempdir().unwrap();
        let markdown_root = tempdir.path().join("markdown");
        let asset_root = tempdir.path().join("assets");
        fs::create_dir_all(&markdown_root).unwrap();
        fs::create_dir_all(&asset_root).unwrap();

        let config_with_mcp = sample_config(
            &markdown_root.display().to_string(),
            &asset_root.display().to_string(),
            true,
        );
        let kernel = build_kernel(&config_with_mcp).await.unwrap();
        assert!(!kernel.has_postgres());
        assert!(kernel.has_markdown());
        assert!(kernel.has_asset_store());
        assert!(kernel.has_vision_gateway());

        let router = build_cli_router(&config_with_mcp, &ServiceInfo::default(), Arc::new(kernel));
        let mcp = router
            .oneshot(Request::get("/mcp/tools").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(mcp.status(), axum::http::StatusCode::OK);

        let config_without_mcp = sample_config(
            &markdown_root.display().to_string(),
            &asset_root.display().to_string(),
            false,
        );
        let router = build_cli_router(
            &config_without_mcp,
            &ServiceInfo::default(),
            Arc::new(build_kernel(&config_without_mcp).await.unwrap()),
        );
        let healthz = router
            .clone()
            .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let missing = router
            .oneshot(Request::get("/mcp/tools").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(healthz.status(), axum::http::StatusCode::OK);
        assert_eq!(missing.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn bootstrap_loaded_config_builds_kernel_and_service_info() {
        let tempdir = tempdir().unwrap();
        let (_config, kernel, service_info) = build_local_runtime(tempdir.path(), false).await;

        assert!(!kernel.has_postgres());
        assert!(kernel.has_markdown());
        assert!(kernel.has_asset_store());
        assert!(kernel.has_vision_gateway());
        assert_eq!(service_info.name, "meat-memory");
        assert_eq!(service_info.default_scope.as_str(), "scp_default_local");
    }

    #[tokio::test]
    async fn ensure_default_key_material_creates_key_file_for_pg_config() {
        let tempdir = tempdir().unwrap();
        let markdown_root = tempdir.path().join("markdown");
        let asset_root = tempdir.path().join("assets");
        fs::create_dir_all(&markdown_root).unwrap();
        fs::create_dir_all(&asset_root).unwrap();

        let mut config = sample_config(
            &markdown_root.display().to_string(),
            &asset_root.display().to_string(),
            false,
        );
        config.features.enable_pg = true;
        config.access.key_store_path = tempdir
            .path()
            .join("keys/default.env")
            .display()
            .to_string();

        let material = ensure_default_key_material(&config)
            .await
            .unwrap()
            .expect("default key should be created");
        assert!(material.key_id.starts_with("key_"));
        assert!(material.raw_key.starts_with("mmk_"));

        let key_file = fs::read_to_string(&config.access.key_store_path).unwrap();
        assert!(key_file.contains("MEAT_MEMORY_KEY="));

        let second = ensure_default_key_material(&config).await.unwrap();
        assert!(second.is_none());
    }

    #[tokio::test]
    async fn remember_command_runtime_supports_json_and_text_paths() {
        let tempdir = tempdir().unwrap();
        let (config, kernel_json, service_info) = build_local_runtime(tempdir.path(), false).await;
        remember_command_with_runtime(
            super::RememberArgs {
                key: None,
                scope_id: Some("scp_runtime_json".to_string()),
                title: Some("JSON memory".to_string()),
                body: Some("runtime json body".to_string()),
                file: None,
                artifact_kind: "message".to_string(),
                memory_kind: Some("decision".to_string()),
                source_refs: vec!["runtime://json".to_string()],
                visibility: "private".to_string(),
                sensitivity: "internal".to_string(),
                json: true,
            },
            kernel_json,
            service_info.clone(),
        )
        .await
        .unwrap();

        let (_, kernel_text, _) = build_local_runtime(tempdir.path(), false).await;
        let body_path = tempdir.path().join("remember-body.txt");
        fs::write(&body_path, "runtime file body").unwrap();
        remember_command_with_runtime(
            super::RememberArgs {
                key: None,
                scope_id: Some("scp_runtime_text".to_string()),
                title: Some("Text memory".to_string()),
                body: None,
                file: Some(body_path),
                artifact_kind: "document".to_string(),
                memory_kind: None,
                source_refs: Vec::new(),
                visibility: "team".to_string(),
                sensitivity: "private".to_string(),
                json: false,
            },
            kernel_text,
            service_info,
        )
        .await
        .unwrap();

        assert!(
            Path::new(&config.markdown.root)
                .join("default/scopes/scp_runtime_json/MEMORY.md")
                .exists()
        );
        assert!(
            Path::new(&config.markdown.root)
                .join("default/scopes/scp_runtime_text/MEMORY.md")
                .exists()
        );
    }

    #[tokio::test]
    async fn remember_image_command_runtime_supports_json_and_text_paths() {
        let tempdir = tempdir().unwrap();
        let (config, kernel_json, service_info) = build_local_runtime(tempdir.path(), false).await;
        let png_path = tempdir.path().join("capture.png");
        let gif_path = tempdir.path().join("capture.gif");
        fs::write(&png_path, [137_u8, 80, 78, 71, 13, 10, 26, 10]).unwrap();
        fs::write(&gif_path, b"GIF89a").unwrap();

        remember_image_command_with_runtime(
            super::RememberImageArgs {
                key: None,
                scope_id: Some("scp_image_json".to_string()),
                title: Some("Runtime image".to_string()),
                body: Some("runtime image body".to_string()),
                file: png_path.clone(),
                media_type: None,
                memory_kind: Some("summary".to_string()),
                source_refs: vec!["runtime://image".to_string()],
                visibility: "project".to_string(),
                sensitivity: "internal".to_string(),
                json: true,
            },
            kernel_json,
            service_info.clone(),
        )
        .await
        .unwrap();

        let (_, kernel_text, _) = build_local_runtime(tempdir.path(), false).await;
        remember_image_command_with_runtime(
            super::RememberImageArgs {
                key: None,
                scope_id: Some("scp_image_text".to_string()),
                title: Some("Runtime gif".to_string()),
                body: None,
                file: gif_path,
                media_type: Some("image/gif".to_string()),
                memory_kind: None,
                source_refs: Vec::new(),
                visibility: "private".to_string(),
                sensitivity: "private".to_string(),
                json: false,
            },
            kernel_text,
            service_info,
        )
        .await
        .unwrap();

        assert!(Path::new(&config.assets.root).join("raw").exists());
        assert!(
            Path::new(&config.markdown.root)
                .join("default/scopes/scp_image_json/MEMORY.md")
                .exists()
        );
        assert!(
            Path::new(&config.markdown.root)
                .join("default/scopes/scp_image_text/MEMORY.md")
                .exists()
        );
    }

    #[tokio::test]
    async fn search_command_runtime_supports_json_and_text_paths() {
        let tempdir = tempdir().unwrap();
        let (_, kernel_json, service_info) = build_local_runtime(tempdir.path(), false).await;
        search_command_with_runtime(
            super::SearchArgs {
                query: "runtime search".to_string(),
                key: None,
                scope_id: Some("scp_search_json".to_string()),
                limit: 7,
                json: true,
            },
            kernel_json,
            service_info.clone(),
        )
        .await
        .unwrap();

        let (_, kernel_text, _) = build_local_runtime(tempdir.path(), false).await;
        search_command_with_runtime(
            super::SearchArgs {
                query: "runtime search".to_string(),
                key: None,
                scope_id: None,
                limit: 5,
                json: false,
            },
            kernel_text,
            service_info,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn serve_helper_surfaces_bind_errors() {
        let tempdir = tempdir().unwrap();
        let (config, kernel, service_info) = build_local_runtime(tempdir.path(), false).await;
        let error = serve_command_with_runtime(
            super::ServeArgs {
                bind: Some("not-a-valid-bind".to_string()),
            },
            config,
            kernel,
            service_info,
        )
        .await
        .expect_err("invalid bind should fail");
        assert!(!error.to_string().trim().is_empty());
    }
}
