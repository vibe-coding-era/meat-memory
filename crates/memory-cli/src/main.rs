use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use memory_config::AppConfig;
use memory_core::{ServiceInfo, log_startup, startup_banner};
use memory_domain::{ArtifactKind, MemoryKind, ScopeId, Sensitivity, Visibility};
use memory_http::{ApiFeatureFlags, ApiMetadata, HttpAppState, build_router};
use memory_kernel::{Kernel, RememberTextRequest, SearchContextRequest};
use serde_json::json;
use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
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
    Serve(ServeArgs),
    Remember(RememberArgs),
    Search(SearchArgs),
}

#[derive(Debug, Args)]
struct ServeArgs {
    #[arg(long)]
    bind: Option<String>,
}

#[derive(Debug, Args)]
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

#[derive(Debug, Args)]
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

    match cli.command {
        Command::Doctor => {
            println!("Run ./scripts/verify.sh for the full machine check.");
            Ok(())
        }
        Command::PrintPlan => {
            println!("Execution plan lives in tasks/tasklist.md");
            Ok(())
        }
        Command::Serve(args) => serve_command(args).await,
        Command::Remember(args) => remember_command(args).await,
        Command::Search(args) => search_command(args).await,
    }
}

async fn serve_command(args: ServeArgs) -> Result<()> {
    let (config, kernel, service_info) = bootstrap_runtime().await?;
    log_startup(&service_info);
    println!("{}", startup_banner(&service_info));

    let metadata = api_metadata(&config, &service_info);
    let bind = args.bind.unwrap_or(config.server.bind);
    let app = build_router(HttpAppState::new(
        service_info.default_scope.clone(),
        metadata,
        Arc::new(kernel),
    ));

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    println!("Serving Meat Memory on {bind}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn remember_command(args: RememberArgs) -> Result<()> {
    let (_, kernel, service_info) = bootstrap_runtime().await?;
    let body = load_body(args.body, args.file)?;
    let mut request =
        RememberTextRequest::new(scope_id_or_default(args.scope_id, &service_info), body);
    request.title = args.title;
    request.artifact_kind = parse_artifact_kind(&args.artifact_kind)?;
    request.memory_kind = args
        .memory_kind
        .as_deref()
        .map(parse_memory_kind)
        .transpose()?;
    request.source_refs = args.source_refs;
    request.visibility = parse_visibility(&args.visibility)?;
    request.sensitivity = parse_sensitivity(&args.sensitivity)?;

    let result = kernel.remember_text(request).await?;

    if args.json {
        print_json(json!({
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
        }))?;
    } else {
        println!(
            "Remembered {} in scope {} (state={}, pg={}, markdown={})",
            result.memory.id.as_str(),
            result.memory.scope_id.as_str(),
            result.memory.state.as_str(),
            result.wrote_pg,
            result.wrote_markdown
        );
        println!("Title: {}", result.memory.title);
    }

    Ok(())
}

async fn search_command(args: SearchArgs) -> Result<()> {
    let (_, kernel, service_info) = bootstrap_runtime().await?;
    let scope_id = scope_id_or_default(args.scope_id, &service_info);
    let mut request = SearchContextRequest::new(scope_id.clone(), args.query);
    request.limit = args.limit;

    let bundle = kernel.search_context(request).await?;

    if args.json {
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

        print_json(json!({
            "query": bundle.query,
            "scope_id": bundle.scope_id.as_str(),
            "memory_count": bundle.memories.len(),
            "entity_count": bundle.entities.len(),
            "relation_count": bundle.relations.len(),
            "memories": memories
        }))?;
    } else if bundle.memories.is_empty() {
        println!(
            "No memories matched query '{}' in scope {}.",
            bundle.query,
            bundle.scope_id.as_str()
        );
    } else {
        println!(
            "Found {} memories in scope {} for query '{}':",
            bundle.memories.len(),
            bundle.scope_id.as_str(),
            bundle.query
        );
        for memory in &bundle.memories {
            println!("- {} [{}]", memory.title, memory.id.as_str());
        }
    }

    Ok(())
}

async fn bootstrap_runtime() -> Result<(AppConfig, Kernel, ServiceInfo)> {
    let config = AppConfig::load()?;
    memory_observability::init(&config.logging.level, &config.logging.format)?;
    let kernel = build_kernel(&config).await?;
    let service_info = ServiceInfo::default();
    Ok((config, kernel, service_info))
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

fn scope_id_or_default(scope_id: Option<String>, service_info: &ServiceInfo) -> ScopeId {
    scope_id
        .map(ScopeId::from_string)
        .unwrap_or_else(|| service_info.default_scope.clone())
}

fn load_body(body: Option<String>, file: Option<PathBuf>) -> Result<String> {
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
    io::stdin().read_to_string(&mut buffer)?;
    if buffer.trim().is_empty() {
        bail!("remember requires --body, --file, or non-empty stdin");
    }

    Ok(buffer)
}

fn print_json(value: serde_json::Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
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
        load_body, parse_artifact_kind, parse_memory_kind, parse_sensitivity, parse_visibility,
    };
    use memory_domain::{ArtifactKind, MemoryKind, Sensitivity, Visibility};
    use std::path::PathBuf;

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
    fn parses_cli_enums() {
        assert_eq!(
            parse_artifact_kind("message").unwrap(),
            ArtifactKind::Message
        );
        assert_eq!(parse_memory_kind("decision").unwrap(), MemoryKind::Decision);
        assert_eq!(parse_visibility("team").unwrap(), Visibility::Team);
        assert_eq!(
            parse_sensitivity("restricted").unwrap(),
            Sensitivity::Restricted
        );
    }
}
