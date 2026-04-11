use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use memory_config::AppConfig;
use memory_core::{ServiceInfo, log_startup, startup_banner};
use memory_domain::{ArtifactKind, ContextBundle, MemoryKind, ScopeId, Sensitivity, Visibility};
use memory_http::{ApiFeatureFlags, ApiMetadata, HttpAppState, build_router};
use memory_kernel::{
    Kernel, RememberImageRequest, RememberImageResult, RememberTextRequest, RememberTextResult,
    SearchContextRequest,
};
use memory_mcp::{McpServer, TOOL_SPECS};
use memory_models::CapabilityRoute;
use memory_store_pg::PgStore;
use serde_json::json;
use std::{
    env, fs,
    future::Future,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

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
struct TuiArgs {
    #[command(subcommand)]
    command: TuiCommand,
}

#[derive(Debug, Clone, Subcommand)]
enum TuiCommand {
    Init(TuiInitArgs),
}

#[derive(Debug, Clone, Subcommand)]
enum SkillsCommand {
    Export(SkillsExportArgs),
}

#[derive(Debug, Clone, Subcommand)]
enum McpCommand {
    Info(InspectArgs),
}

#[derive(Debug, Clone, Args)]
struct InspectArgs {
    #[arg(long)]
    json: bool,
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
        Command::Tui(args) => tui_command(args).await,
        Command::Serve(args) => serve_command(args).await,
        Command::Remember(args) => remember_command(args).await,
        Command::RememberImage(args) => remember_image_command(args).await,
        Command::Search(args) => search_command(args).await,
    }
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
            apply_tui_init_overrides(&mut config, &init);
            let check = config_check_report(&config, &path, init.check_database).await?;
            let wrote_config = write_tui_config_if_requested(&config, &init)?;
            if init.json {
                print_json(tui_init_json(
                    &config,
                    &path,
                    wrote_config.as_deref(),
                    &check,
                ))?;
            } else {
                for line in tui_init_lines(&config, &path, wrote_config.as_deref(), &check) {
                    println!("{line}");
                }
            }
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
            if inspect.json {
                print_json(mcp_info_json(&config, &path))?;
            } else {
                for line in mcp_info_lines(&config, &path) {
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
    let request = build_remember_request(&args, &service_info, body)?;

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
    let request = build_remember_image_request(&args, &service_info, media_type, bytes)?;

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
    let request = build_search_request(&args, &service_info);
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
    env::var("MEAT_MEMORY_CONFIG").unwrap_or_else(|_| "config/default.toml".to_string())
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

fn mcp_info_json(config: &AppConfig, path: &str) -> serde_json::Value {
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
        "tools": tools,
    })
}

fn mcp_info_lines(config: &AppConfig, path: &str) -> Vec<String> {
    let mut lines = vec![
        "Meat Memory MCP info".to_string(),
        format!("Config: {path}"),
        format!("Enabled: {}", config.features.enable_mcp),
        format!("HTTP enabled: {}", config.features.enable_http),
        format!("Tools URL: http://{}/mcp/tools", config.server.bind),
        format!("Call URL: http://{}/mcp/tools/call", config.server.bind),
        "Tools:".to_string(),
    ];
    lines.extend(
        TOOL_SPECS
            .iter()
            .map(|tool| format!("- {}: {}", tool.name, tool.description)),
    );
    lines
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

fn tui_init_json(
    config: &AppConfig,
    path: &str,
    wrote_config: Option<&str>,
    check: &ConfigCheckReport,
) -> serde_json::Value {
    json!({
        "title": "Meat Memory setup TUI",
        "mode": if wrote_config.is_some() { "generated_config" } else { "preview" },
        "config_path": path,
        "wrote_config": wrote_config,
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
) -> Vec<String> {
    let mut lines = vec![
        "┌──────────────────────────────────────────────┐".to_string(),
        "│ Meat Memory Setup TUI                        │".to_string(),
        "├──────────────────────────────────────────────┤".to_string(),
        format!("│ Config: {path}"),
        format!("│ LLM locale: {}", config.models.default_locale),
        format!("│ Reasoning: {}", config.models.routing.reasoning.primary),
        format!("│ Extraction: {}", config.models.routing.extraction.primary),
        format!("│ Vision: {}", config.models.routing.vision.primary),
        format!("│ Embedding: {}", config.models.routing.embedding.primary),
        format!(
            "│ Stores: pg={}, markdown={}",
            config.features.enable_pg, config.features.enable_markdown
        ),
        format!("│ Markdown: {}", config.markdown.root),
        format!("│ Assets: {}", config.assets.root),
        format!("│ MCP: {}", config.features.enable_mcp),
        format!("│ Check: {}", if check.ok { "ok" } else { "warning" }),
        format!("│ Database: {}", check.database.message),
    ];
    if let Some(path) = wrote_config {
        lines.push(format!("│ Wrote config: {path}"));
    }
    if !check.warnings.is_empty() {
        lines.push("│ Warnings:".to_string());
        lines.extend(
            check
                .warnings
                .iter()
                .map(|warning| format!("│ - {warning}")),
        );
    }
    lines.extend([
        "├──────────────────────────────────────────────┤".to_string(),
        "│ Next: memory-cli config check                │".to_string(),
        "│ Next: memory-cli mcp info                    │".to_string(),
        "│ Next: memory-cli serve                       │".to_string(),
        "└──────────────────────────────────────────────┘".to_string(),
    ]);
    lines
}

fn scope_id_or_default(scope_id: Option<String>, service_info: &ServiceInfo) -> ScopeId {
    scope_id
        .map(ScopeId::from_string)
        .unwrap_or_else(|| service_info.default_scope.clone())
}

fn static_command_message(command: &Command) -> Option<&'static str> {
    match command {
        Command::Doctor => Some("Run ./scripts/verify.sh for the full machine check."),
        Command::PrintPlan => Some("Execution plan lives in tasks/tasklist.md"),
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

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use super::{
        api_metadata, bootstrap_loaded_config, build_cli_router, build_kernel,
        build_remember_image_request, build_remember_request, build_search_request,
        config_check_report, detect_image_media_type, export_skill_bundle, load_body,
        load_body_from_reader, parse_artifact_kind, parse_memory_kind, parse_sensitivity,
        parse_visibility, remember_command_with_runtime, remember_image_command_with_runtime,
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
        Artifact, ArtifactKind, ContextBundle, Memory, MemoryKind, MemoryState, ScopeId,
        Sensitivity, Visibility,
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
            Some("Run ./scripts/verify.sh for the full machine check.")
        );
        assert_eq!(
            static_command_message(&super::Command::PrintPlan),
            Some("Execution plan lives in tasks/tasklist.md")
        );
        assert_eq!(
            static_command_message(&super::Command::Search(super::SearchArgs {
                query: "q".to_string(),
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
        let report = config_check_report(&config, "config/default.toml", false)
            .await
            .unwrap();
        let wrote = write_tui_config_if_requested(&config, &args).unwrap();
        let rendered = fs::read_to_string(&output).unwrap();
        let payload = tui_init_json(&config, "config/default.toml", wrote.as_deref(), &report);
        let lines = tui_init_lines(&config, "config/default.toml", wrote.as_deref(), &report);

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
    async fn remember_command_runtime_supports_json_and_text_paths() {
        let tempdir = tempdir().unwrap();
        let (config, kernel_json, service_info) = build_local_runtime(tempdir.path(), false).await;
        remember_command_with_runtime(
            super::RememberArgs {
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
