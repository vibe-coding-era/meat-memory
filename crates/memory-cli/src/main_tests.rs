use super::{
    Cli, api_metadata, bootstrap_loaded_config, bootstrap_runtime, build_cli_router, build_kernel,
    build_non_interactive_project_init_request, build_remember_image_request,
    build_remember_request, build_search_request, check_mcp_http_endpoint, config_check_json,
    config_check_lines, config_check_report, config_command, config_summary_json,
    config_summary_lines, context_command, detect_image_media_type, docs_command,
    ensure_default_key_material, export_skill_bundle, generated_project_scope_id, key_command,
    lifecycle_command, lifecycle_inspect_json, lifecycle_status_json, load_body,
    load_body_from_reader, mcp_command, mcp_info_json, mcp_info_lines, parse_artifact_kind,
    parse_document_conflict_state, parse_document_sync_state, parse_key_scope, parse_key_source,
    parse_memory_kind, parse_record_status, parse_sensitivity, parse_source_sync_mode,
    parse_storage_mode, parse_visibility, project_command, project_init_result_json,
    project_init_result_lines, project_scope_slug, remember_command, remember_command_with_runtime,
    remember_image_command_with_runtime, remember_image_result_json, remember_image_result_lines,
    remember_result_json, remember_result_lines, render_json, resolve_bind, run_with_cli,
    scope_id_or_default, search_bundle_json, search_bundle_lines, search_command,
    search_command_with_runtime, serve_command_with_runtime, skills_command, source_command,
    static_command_message, tui_command, tui_init_json, tui_init_lines, validate_model_registry,
    write_tui_config_if_requested,
};
use axum::{body::Body, http::Request};
use clap::Parser;
use memory_assets::{AssetMetadata, AssetRef, StorageClass, StoredAsset};
use memory_config::AppConfig;
use memory_core::ServiceInfo;
use memory_domain::{
    AccessKey, AccessKeyId, Artifact, ArtifactKind, ContextBundle, KeyScopeKind, KeySourceKind,
    Memory, MemoryKind, MemoryRecordStatus, MemoryState, ScopeId, Sensitivity, SourceSyncMode,
    StorageMode, Visibility,
};
use memory_domain::{DocumentConflictState, DocumentSyncState};
use memory_kernel::{
    AuditLogService, ChangeMemoryLifecycleStatusResult, InspectMemoryLifecycleResult,
    LifecycleNormalizer, RecallExplainer, RememberImageResult, RememberTextResult,
};
use memory_mcp::TOOL_NAMES;
use memory_models::VisionResponse;
use serde_json::json;
use std::{
    env, fs,
    io::Cursor,
    net::{SocketAddr, TcpListener as StdTcpListener, TcpStream as StdTcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    thread,
    time::Duration,
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

fn sample_image_result_with_notice(with_vision: bool, notice: Option<&str>) -> RememberImageResult {
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
            sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
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

fn write_runtime_config(root: &Path, enable_pg: bool, enable_mcp: bool) -> PathBuf {
    let markdown_root = root.join("markdown");
    let asset_root = root.join("assets");
    fs::create_dir_all(&markdown_root).unwrap();
    fs::create_dir_all(&asset_root).unwrap();
    let mut config = sample_config(
        &markdown_root.display().to_string(),
        &asset_root.display().to_string(),
        enable_mcp,
    );
    config.features.enable_pg = enable_pg;
    config.sync.mode = "dual_write".to_string();
    let config_path = root.join("memory-cli-runtime.toml");
    fs::write(&config_path, config.to_toml_string_pretty().unwrap()).unwrap();
    config_path
}

struct EnvVarGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    fn set_path(key: &'static str, value: &Path) -> Self {
        let previous = env::var_os(key);
        // SAFETY: CLI env-based tests hold CLI_ENV_LOCK while mutating process env.
        unsafe {
            env::set_var(key, value);
        }
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        // SAFETY: CLI env-based tests hold CLI_ENV_LOCK while restoring process env.
        unsafe {
            if let Some(previous) = self.previous.as_ref() {
                env::set_var(self.key, previous);
            } else {
                env::remove_var(self.key);
            }
        }
    }
}

async fn lock_cli_env() -> tokio::sync::MutexGuard<'static, ()> {
    static CLI_ENV_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    CLI_ENV_LOCK
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

fn local_pg_test_port_available() -> bool {
    let authority = env::var("MEAT_MEMORY_TEST_DATABASE_URL")
        .ok()
        .and_then(|url| {
            url.split('@')
                .nth(1)
                .map(|tail| tail.split('/').next().unwrap_or("").to_string())
        })
        .filter(|authority| !authority.is_empty())
        .unwrap_or_else(|| "127.0.0.1:5433".to_string());
    let addr = authority
        .to_socket_addrs()
        .ok()
        .and_then(|mut addrs| addrs.next())
        .unwrap_or_else(|| "127.0.0.1:5433".parse::<SocketAddr>().unwrap());
    StdTcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
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
fn parses_management_enum_variants_and_rejects_unknowns() {
    let key_sources = [
        ("cli", KeySourceKind::Cli),
        ("mcp", KeySourceKind::Mcp),
        ("skill", KeySourceKind::Skill),
        ("http", KeySourceKind::Http),
        ("tui", KeySourceKind::Tui),
        ("custom", KeySourceKind::Custom),
    ];
    for (raw, expected) in key_sources {
        assert_eq!(parse_key_source(raw).unwrap(), expected);
    }

    let key_scopes = [
        ("personal", KeyScopeKind::Personal),
        ("team", KeyScopeKind::Team),
    ];
    for (raw, expected) in key_scopes {
        assert_eq!(parse_key_scope(raw).unwrap(), expected);
    }

    let storage_modes = [
        ("file", StorageMode::File),
        ("vector", StorageMode::Vector),
        ("all", StorageMode::All),
    ];
    for (raw, expected) in storage_modes {
        assert_eq!(parse_storage_mode(raw).unwrap(), expected);
    }

    let source_sync_modes = [
        ("read_only", SourceSyncMode::ReadOnly),
        ("index_only", SourceSyncMode::IndexOnly),
        ("two_way", SourceSyncMode::TwoWay),
    ];
    for (raw, expected) in source_sync_modes {
        assert_eq!(parse_source_sync_mode(raw).unwrap(), expected);
    }

    let document_sync_states = [
        ("clean", DocumentSyncState::Clean),
        ("changed", DocumentSyncState::Changed),
        ("deleted", DocumentSyncState::Deleted),
        ("missing", DocumentSyncState::Missing),
        ("conflicted", DocumentSyncState::Conflicted),
    ];
    for (raw, expected) in document_sync_states {
        assert_eq!(parse_document_sync_state(raw).unwrap(), expected);
    }

    let document_conflict_states = [
        ("none", DocumentConflictState::None),
        ("local_changed", DocumentConflictState::LocalChanged),
        ("remote_changed", DocumentConflictState::RemoteChanged),
        ("both_changed", DocumentConflictState::BothChanged),
    ];
    for (raw, expected) in document_conflict_states {
        assert_eq!(parse_document_conflict_state(raw).unwrap(), expected);
    }

    assert!(parse_key_source("unknown").is_err());
    assert!(parse_key_scope("unknown").is_err());
    assert!(parse_storage_mode("unknown").is_err());
    assert!(parse_source_sync_mode("unknown").is_err());
    assert!(parse_document_sync_state("unknown").is_err());
    assert!(parse_document_conflict_state("unknown").is_err());
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
        detect_image_media_type(&PathBuf::from("capture.png"), Some("   ".to_string())).is_err()
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

#[tokio::test]
async fn run_with_cli_handles_static_commands_without_bootstrap() {
    for args in [
        &["memory-cli", "doctor"][..],
        &["memory-cli", "print-plan"][..],
    ] {
        let cli = Cli::try_parse_from(args).unwrap();
        run_with_cli(cli).await.unwrap();
    }
}

#[test]
fn cli_parser_accepts_all_subcommands_and_nested_shapes() {
    let cases: &[&[&str]] = &[
        &["memory-cli", "doctor"],
        &["memory-cli", "print-plan"],
        &["memory-cli", "config", "show", "--json"],
        &["memory-cli", "config", "check", "--database"],
        &["memory-cli", "mcp", "info", "--check-http", "--json"],
        &[
            "memory-cli",
            "skills",
            "export",
            "--target",
            "codex",
            "--output-dir",
            "dist/skills",
            "--force",
            "--json",
        ],
        &[
            "memory-cli",
            "key",
            "create",
            "--name",
            "parser",
            "--source",
            "cli",
            "--owner-principal-id",
            "alice",
            "--owner-scope-id",
            "scp_parser",
            "--scope-kind",
            "team",
            "--storage",
            "all",
            "--isolated",
            "--raw-key",
            "mmk_parser",
            "--json",
        ],
        &["memory-cli", "key", "list", "--limit", "3", "--json"],
        &[
            "memory-cli",
            "key",
            "use",
            "--raw-key",
            "mmk_parser",
            "--output",
            ".env",
            "--force",
        ],
        &["memory-cli", "key", "rotate", "--key-id", "key_parser"],
        &[
            "memory-cli",
            "key",
            "stats",
            "--key-id",
            "key_parser",
            "--json",
        ],
        &[
            "memory-cli",
            "source",
            "create",
            "--key",
            "mmk_parser",
            "--name",
            "docs",
            "--source-kind",
            "local_docs",
            "--source-uri",
            "file:///docs",
            "--sync-mode",
            "index_only",
            "--local-root",
            ".",
            "--json",
        ],
        &["memory-cli", "source", "list", "--key", "mmk_parser"],
        &[
            "memory-cli",
            "source",
            "keys",
            "--key",
            "mmk_parser",
            "--source-id",
            "src_parser",
        ],
        &[
            "memory-cli",
            "source",
            "key-create",
            "--key",
            "mmk_parser",
            "--source-id",
            "src_parser",
            "--name",
            "source key",
            "--source",
            "cli",
            "--scope-kind",
            "personal",
            "--storage",
            "vector",
            "--isolated",
        ],
        &[
            "memory-cli",
            "project",
            "init",
            "--name",
            "Parser Project",
            "--scope-id",
            "scp_parser",
            "--shared",
            "--json",
        ],
        &[
            "memory-cli",
            "context",
            "upsert",
            "--key",
            "mmk_parser",
            "--scope-id",
            "scp_parser",
            "--session-id",
            "session_parser",
            "--task-id",
            "task_parser",
            "--title",
            "Parser context",
            "--body",
            "body",
            "--labels",
            "cli,parser",
            "--json",
        ],
        &[
            "memory-cli",
            "context",
            "list",
            "--key",
            "mmk_parser",
            "--scope-id",
            "scp_parser",
            "--session-id",
            "session_parser",
            "--limit",
            "2",
        ],
        &[
            "memory-cli",
            "context",
            "promote",
            "--key",
            "mmk_parser",
            "--context-id",
            "ctx_parser",
            "--memory-kind",
            "summary",
            "--visibility",
            "team",
            "--sensitivity",
            "private",
        ],
        &[
            "memory-cli",
            "context",
            "delete",
            "--key",
            "mmk_parser",
            "--context-id",
            "ctx_parser",
            "--json",
        ],
        &[
            "memory-cli",
            "docs",
            "import",
            "--key",
            "mmk_parser",
            "--source-id",
            "src_parser",
            "--scope-id",
            "scp_parser",
            "--canonical-uri",
            "file:///README.md",
            "--title",
            "README",
            "--body",
            "body",
            "--local-path",
            "README.md",
            "--sync-state",
            "clean",
            "--conflict-state",
            "none",
            "--json",
        ],
        &[
            "memory-cli",
            "docs",
            "list",
            "--key",
            "mmk_parser",
            "--source-id",
            "src_parser",
            "--query",
            "readme",
        ],
        &[
            "memory-cli",
            "docs",
            "projection",
            "--key",
            "mmk_parser",
            "--source-id",
            "src_parser",
            "--document-id",
            "doc_parser",
        ],
        &[
            "memory-cli",
            "docs",
            "conflicts",
            "--key",
            "mmk_parser",
            "--source-id",
            "src_parser",
            "--limit",
            "4",
        ],
        &[
            "memory-cli",
            "docs",
            "sync",
            "--key",
            "mmk_parser",
            "--source-id",
            "src_parser",
            "--scope-id",
            "scp_parser",
            "--local-root",
            ".",
            "--dry-run",
        ],
        &[
            "memory-cli",
            "docs",
            "status",
            "--key",
            "mmk_parser",
            "--source-id",
            "src_parser",
            "--local-root",
            ".",
            "--json",
        ],
        &[
            "memory-cli",
            "lifecycle",
            "inspect",
            "mem_parser",
            "--key",
            "mmk_parser",
            "--scope-id",
            "scp_parser",
            "--query",
            "parser",
        ],
        &[
            "memory-cli",
            "lifecycle",
            "status",
            "mem_parser",
            "--status",
            "archived",
            "--key",
            "mmk_parser",
            "--reason",
            "parser",
            "--actor",
            "alice",
        ],
        &[
            "memory-cli",
            "lifecycle",
            "forget",
            "mem_parser",
            "--key",
            "mmk_parser",
        ],
        &[
            "memory-cli",
            "lifecycle",
            "restore",
            "mem_parser",
            "--key",
            "mmk_parser",
        ],
        &[
            "memory-cli",
            "lifecycle",
            "report",
            "--scope-id",
            "scp_parser",
            "--limit",
            "5",
        ],
        &[
            "memory-cli",
            "tui",
            "init",
            "--json",
            "--default-locale",
            "zh-CN",
            "--enable-mcp",
            "--write-config",
            "config/local.toml",
            "--force",
        ],
        &[
            "memory-cli",
            "tui",
            "key-create",
            "--name",
            "tui key",
            "--json",
        ],
        &[
            "memory-cli",
            "tui",
            "project-init",
            "--name",
            "TUI Project",
            "--existing",
            "--scope-id",
            "scp_parser",
        ],
        &["memory-cli", "serve", "--bind", "127.0.0.1:0"],
        &[
            "memory-cli",
            "remember",
            "--key",
            "mmk_parser",
            "--scope-id",
            "scp_parser",
            "--title",
            "Parser memory",
            "--body",
            "body",
            "--artifact-kind",
            "document",
            "--memory-kind",
            "decision",
            "--source-refs",
            "api://parser,file://README.md",
            "--visibility",
            "team",
            "--sensitivity",
            "internal",
        ],
        &[
            "memory-cli",
            "remember-image",
            "--key",
            "mmk_parser",
            "--scope-id",
            "scp_parser",
            "--title",
            "Parser image",
            "--body",
            "body",
            "--file",
            "capture.png",
            "--media-type",
            "image/png",
            "--memory-kind",
            "summary",
            "--source-refs",
            "image://parser",
        ],
        &[
            "memory-cli",
            "search",
            "parser query",
            "--key",
            "mmk_parser",
            "--scope-id",
            "scp_parser",
            "--limit",
            "8",
            "--json",
        ],
    ];

    for args in cases {
        Cli::try_parse_from(*args).unwrap_or_else(|error| {
            panic!("failed to parse {:?}: {error}", args);
        });
    }
}

#[test]
fn surface_parity_smoke_covers_mcp_cli_and_http_contracts() {
    let mcp_tools = TOOL_NAMES;
    let http_routes = memory_http::HTTP_ROUTES;

    let parity_cases: &[(&str, &[&str], &[&str])] = &[
        (
            "memory.remember",
            &["memory-cli", "remember", "--body", "body"],
            &["/api/v1/memories"],
        ),
        (
            "memory.search",
            &["memory-cli", "search", "query"],
            &["/api/v1/context", "/api/v1/context/search"],
        ),
        (
            "memory.fetch_context",
            &["memory-cli", "search", "query"],
            &["/api/v1/context", "/api/v1/context/search"],
        ),
        (
            "memory.promote",
            &[
                "memory-cli",
                "remember",
                "--body",
                "placeholder for promote parity",
            ],
            &["/api/v1/memories/promote"],
        ),
        (
            "memory.lifecycle.inspect",
            &["memory-cli", "lifecycle", "inspect", "mem_1"],
            &["/api/v1/lifecycle/memories/{scope_id}/{memory_id}"],
        ),
        (
            "memory.lifecycle.status",
            &[
                "memory-cli",
                "lifecycle",
                "status",
                "mem_1",
                "--status",
                "archived",
            ],
            &["/api/v1/lifecycle/memories/{scope_id}/{memory_id}/status"],
        ),
        (
            "memory.lifecycle.forget",
            &["memory-cli", "lifecycle", "forget", "mem_1"],
            &["/api/v1/lifecycle/memories/{scope_id}/{memory_id}/forget"],
        ),
        (
            "memory.lifecycle.restore",
            &["memory-cli", "lifecycle", "restore", "mem_1"],
            &["/api/v1/lifecycle/memories/{scope_id}/{memory_id}/restore"],
        ),
        (
            "memory.lifecycle.report",
            &["memory-cli", "lifecycle", "report"],
            &["/api/v1/lifecycle/report"],
        ),
        (
            "memory.context.upsert",
            &[
                "memory-cli",
                "context",
                "upsert",
                "--session-id",
                "session",
                "--title",
                "title",
                "--body",
                "body",
            ],
            &["/api/v1/agent-contexts"],
        ),
        (
            "memory.context.list",
            &["memory-cli", "context", "list", "--session-id", "session"],
            &["/api/v1/agent-contexts"],
        ),
        (
            "memory.context.promote",
            &["memory-cli", "context", "promote", "--context-id", "ctx_1"],
            &["/api/v1/agent-contexts/{context_id}/promote"],
        ),
        (
            "memory.context.delete",
            &["memory-cli", "context", "delete", "--context-id", "ctx_1"],
            &["/api/v1/agent-contexts/{context_id}"],
        ),
        (
            "memory.docs.sync",
            &["memory-cli", "docs", "sync", "--source-id", "src_1"],
            &["/api/v1/sources/{source_id}/documents/sync"],
        ),
        (
            "memory.docs.search",
            &["memory-cli", "docs", "list", "--source-id", "src_1"],
            &["/api/v1/sources/{source_id}/documents"],
        ),
        (
            "memory.docs.conflicts",
            &["memory-cli", "docs", "conflicts", "--source-id", "src_1"],
            &["/api/v1/sources/{source_id}/documents/conflicts"],
        ),
    ];

    for (tool, cli_args, routes) in parity_cases {
        assert!(mcp_tools.contains(tool), "missing MCP tool {tool}");
        Cli::try_parse_from(*cli_args).unwrap_or_else(|error| {
            panic!("missing CLI parity for {tool}: {cli_args:?}: {error}");
        });
        for route in *routes {
            assert!(http_routes.contains(route), "missing HTTP route {route}");
        }
    }

    let intentionally_http_or_cli_only = [
        "/api/v1/images",
        "/api/v1/keys",
        "/api/v1/sources",
        "/api/v1/sources/{source_id}/documents/import",
        "/api/v1/sources/{source_id}/documents/{document_id}/projection",
        "/api/v1/explorer/memories",
        "/api/v1/assistant/chat",
        "/api/v1/metrics/keys",
    ];
    for route in intentionally_http_or_cli_only {
        assert!(http_routes.contains(&route));
    }

    for args in [
        &["memory-cli", "remember-image", "--file", "capture.png"][..],
        &["memory-cli", "key", "create", "--name", "operator"],
        &["memory-cli", "source", "create", "--name", "docs"],
        &[
            "memory-cli",
            "docs",
            "import",
            "--source-id",
            "src_1",
            "--canonical-uri",
            "file:///README.md",
            "--title",
            "README",
            "--body",
            "body",
        ],
        &[
            "memory-cli",
            "docs",
            "projection",
            "--source-id",
            "src_1",
            "--document-id",
            "doc_1",
        ],
        &["memory-cli", "config", "show"],
        &["memory-cli", "mcp", "info"],
        &["memory-cli", "skills", "export"],
        &["memory-cli", "project", "init", "--name", "Project"],
        &["memory-cli", "tui", "init"],
    ] {
        Cli::try_parse_from(args).unwrap_or_else(|error| {
            panic!("intentional CLI-only surface should parse {args:?}: {error}");
        });
    }
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
fn lifecycle_helpers_parse_and_render_cli_payloads() {
    assert_eq!(
        parse_record_status("forgotten").unwrap(),
        MemoryRecordStatus::Forgotten
    );
    assert_eq!(
        parse_record_status("conflicted").unwrap(),
        MemoryRecordStatus::NeedsReview
    );
    assert!(parse_record_status("unknown").is_err());

    let mut memory = sample_memory();
    memory.source_refs = vec!["agent-context://ctx_cli".to_string()];
    let record = LifecycleNormalizer::normalize_memory(&memory);
    let explanation = RecallExplainer::explain(&record, "保留", 0.8);
    let inspect = InspectMemoryLifecycleResult {
        memory: memory.clone(),
        record: record.clone(),
        explanation: Some(explanation),
    };
    let inspect_payload = lifecycle_inspect_json(&inspect);
    assert_eq!(inspect_payload["record"]["source_kind"], "conversation");
    assert!(
        inspect_payload["explanation"]["reason"]
            .as_str()
            .unwrap()
            .contains("query matched")
    );

    let mut forgotten_record = record;
    forgotten_record.status = MemoryRecordStatus::Forgotten;
    let audit = AuditLogService::record(
        "memory.lifecycle.forgotten",
        "cli-test",
        &forgotten_record,
        Some(MemoryRecordStatus::Active),
        Some(MemoryRecordStatus::Forgotten),
        Some("test forget".to_string()),
    );
    let status_payload = lifecycle_status_json(&ChangeMemoryLifecycleStatusResult {
        memory,
        record: forgotten_record,
        audit_event: audit,
        wrote_pg: false,
        wrote_markdown: true,
    });
    assert_eq!(status_payload["record_status"], "forgotten");
    assert_eq!(status_payload["audit"]["after_status"], "forgotten");
    assert_eq!(status_payload["wrote_markdown"], true);
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
async fn config_and_mcp_renderers_cover_text_json_and_warning_paths() {
    let mut config = sample_config("/tmp/md", "/tmp/assets", true);
    let summary_json = config_summary_json(&config, "config/app.toml").unwrap();
    let summary_lines = config_summary_lines(&config, "config/app.toml").unwrap();

    assert_eq!(summary_json["config_path"], "config/app.toml");
    assert_eq!(summary_json["features"]["mcp"], true);
    assert!(summary_lines.iter().any(|line| line.contains("Models:")));
    assert!(summary_lines.iter().any(|line| line.contains("Routes:")));

    let healthy = config_check_report(&config, "config/app.toml", false)
        .await
        .unwrap();
    let healthy_json = config_check_json(&healthy);
    let healthy_lines = config_check_lines(&healthy);
    assert!(healthy.ok);
    assert_eq!(healthy_json["ok"], true);
    assert!(healthy_lines.iter().any(|line| line == "Warnings: none"));

    config.features.enable_pg = false;
    config.features.enable_markdown = false;
    config.features.enable_http = false;
    config.features.enable_mcp = true;
    let warning = config_check_report(&config, "config/warning.toml", true)
        .await
        .unwrap();
    let warning_json = config_check_json(&warning);
    let warning_lines = config_check_lines(&warning);
    assert!(!warning.ok);
    assert_eq!(warning_json["database"]["checked"], false);
    assert!(
        warning
            .warnings
            .iter()
            .any(|item| item.contains("No persistence store"))
    );
    assert!(
        warning
            .warnings
            .iter()
            .any(|item| item.contains("MCP is enabled but HTTP is disabled"))
    );
    assert!(warning_lines.iter().any(|line| line == "Warnings:"));

    let disabled_http_check = check_mcp_http_endpoint(&config);
    let mcp_json = mcp_info_json(&config, "config/warning.toml", Some(&disabled_http_check));
    let mcp_lines = mcp_info_lines(&config, "config/warning.toml", Some(&disabled_http_check));
    assert_eq!(mcp_json["http_enabled"], false);
    assert_eq!(mcp_json["http_check"]["message"], "http disabled in config");
    assert!(
        mcp_lines
            .iter()
            .any(|line| line == "HTTP check: http disabled in config")
    );
    assert!(
        mcp_lines
            .iter()
            .any(|line| line.contains("memory.lifecycle.inspect"))
    );
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
        rendered.contains("database_url = \"postgres://postgres:postgres@127.0.0.1:5433/custom\"")
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

#[test]
fn tui_prompt_and_skill_helpers_cover_edge_paths() {
    let config = sample_config("/tmp/md", "/tmp/assets", true);
    let mut args = super::TuiInitArgs {
        json: false,
        interactive: false,
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

    super::apply_tui_profile_defaults(&config, &mut args, 2);
    assert!(args.disable_mcp);
    assert_eq!(args.markdown_root.as_deref(), Some("/tmp/md"));
    assert!(!super::effective_mcp_enabled(&config, &args));
    assert_eq!(
        super::effective_text(Some("override"), "current"),
        "override"
    );
    assert_eq!(super::effective_text(None, "current"), "current");

    let language = super::WizardLanguage::from_choice_index(1);
    assert_eq!(language.as_choice_index(), 1);
    assert_eq!(language.locale(), "en-US");
    assert_eq!(super::wizard_language_from_locale("en-US"), language);
    assert_eq!(super::wizard_text(language, "中文", "English"), "English");
    let summary = super::render_tui_interactive_summary(&config, &args, language);
    assert!(summary.iter().any(|line| line.contains("Language")));
    assert!(summary.iter().any(|line| line.contains("MCP: false")));

    let mut reader = Cursor::new(" trimmed value \n");
    assert_eq!(
        super::read_prompt_line(&mut reader).unwrap(),
        "trimmed value"
    );
    let mut eof_reader = Cursor::new(Vec::<u8>::new());
    assert_eq!(super::read_prompt_line(&mut eof_reader).unwrap(), "");

    let mut output = Vec::new();
    let mut blank = Cursor::new("\n");
    assert_eq!(
        super::prompt_text(&mut blank, &mut output, "Name", "current").unwrap(),
        None
    );
    let mut value = Cursor::new("new-name\n");
    assert_eq!(
        super::prompt_text(&mut value, &mut output, "Name", "current")
            .unwrap()
            .as_deref(),
        Some("new-name")
    );

    let mut path_blank = Cursor::new("\n");
    assert_eq!(
        super::prompt_optional_path(
            &mut path_blank,
            &mut output,
            "Path",
            Some(Path::new("config/app.toml")),
        )
        .unwrap()
        .as_deref(),
        Some(Path::new("config/app.toml"))
    );
    let mut path_value = Cursor::new("config/local.toml\n");
    assert_eq!(
        super::prompt_optional_path(&mut path_value, &mut output, "Path", None)
            .unwrap()
            .as_deref(),
        Some(Path::new("config/local.toml"))
    );

    let mut bool_yes = Cursor::new("yes\n");
    assert_eq!(
        super::prompt_bool(&mut bool_yes, &mut output, "Enable", false).unwrap(),
        Some(true)
    );
    let mut bool_no = Cursor::new("0\n");
    assert_eq!(
        super::prompt_bool(&mut bool_no, &mut output, "Enable", true).unwrap(),
        Some(false)
    );
    let mut bool_blank = Cursor::new("\n");
    assert_eq!(
        super::prompt_bool(&mut bool_blank, &mut output, "Enable", true).unwrap(),
        None
    );
    let mut bool_invalid = Cursor::new("maybe\n");
    assert!(super::prompt_bool(&mut bool_invalid, &mut output, "Enable", true).is_err());

    let choices = [("One", "first"), ("Two", "second")];
    let mut choice_blank = Cursor::new("\n");
    assert_eq!(
        super::prompt_choice(&mut choice_blank, &mut output, "Choice", &choices, 1).unwrap(),
        1
    );
    let mut choice_value = Cursor::new("1\n");
    assert_eq!(
        super::prompt_choice(&mut choice_value, &mut output, "Choice", &choices, 1).unwrap(),
        0
    );
    let mut choice_invalid = Cursor::new("3\n");
    assert!(super::prompt_choice(&mut choice_invalid, &mut output, "Choice", &choices, 0).is_err());

    let aliases =
        super::available_model_alias_choices(&config, memory_models::ModelCapability::Reasoning);
    assert!(!aliases.is_empty());
    assert!(aliases[0].summary().contains("chatgpt_reasoning"));
    let mut model_number = Cursor::new("1\n");
    assert_eq!(
        super::prompt_model_alias(
            &mut model_number,
            &mut output,
            &config,
            language,
            "Reasoning",
            memory_models::ModelCapability::Reasoning,
            "chatgpt_reasoning",
        )
        .unwrap()
        .as_deref(),
        Some("chatgpt_reasoning")
    );
    let mut model_custom = Cursor::new("custom_reasoning\n");
    assert_eq!(
        super::prompt_model_alias(
            &mut model_custom,
            &mut output,
            &config,
            language,
            "Reasoning",
            memory_models::ModelCapability::Reasoning,
            "chatgpt_reasoning",
        )
        .unwrap()
        .as_deref(),
        Some("custom_reasoning")
    );

    assert_eq!(
        super::skill_export_target_label(super::SkillExportTarget::ClaudeCode),
        "claude-code"
    );
    assert_eq!(
        super::skill_export_target_label(super::SkillExportTarget::ExecutionAgent),
        "execution-agent"
    );
    assert_eq!(
        super::selected_skill_templates(super::SkillExportTarget::All).len(),
        3
    );
    let readme = super::render_skill_bundle_readme(
        Path::new("docs/agent-skills"),
        "all",
        &["codex-meat-memory/SKILL.md".to_string()],
    );
    assert!(readme.contains("target: `all`"));
    assert!(readme.contains("codex-meat-memory/SKILL.md"));
}

#[test]
fn mcp_endpoint_probe_covers_reachable_response_paths() {
    fn probe_with_response(response: &'static str) -> super::HttpEndpointCheck {
        let listener = match StdTcpListener::bind("127.0.0.1:0") {
            Ok(listener) => listener,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                return super::HttpEndpointCheck {
                    checked: false,
                    ok: true,
                    message: "skipped: local bind not permitted in current sandbox".to_string(),
                };
            }
            Err(error) => panic!("failed to bind test listener: {error}"),
        };
        let addr = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                use std::io::{Read as _, Write as _};
                let mut buffer = [0_u8; 256];
                let _ = stream.read(&mut buffer);
                stream.write_all(response.as_bytes()).unwrap();
            }
        });

        let mut config = sample_config("/tmp/md", "/tmp/assets", true);
        config.server.bind = addr.to_string();
        let check = check_mcp_http_endpoint(&config);
        handle.join().unwrap();
        check
    }

    let ok = probe_with_response("HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
    if ok.message.contains("skipped:") {
        return;
    }
    assert!(ok.ok);
    assert_eq!(ok.message, "reachable (HTTP 200)");

    let unexpected = probe_with_response("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
    assert!(!unexpected.ok);
    assert!(
        unexpected
            .message
            .contains("reachable but unexpected response")
    );

    let empty = probe_with_response("");
    assert!(!empty.ok);
    assert_eq!(empty.message, "reachable but empty response");
}

#[tokio::test]
async fn config_check_and_tui_render_warning_default_key_paths() {
    let config = sample_config("/tmp/md", "/tmp/assets", true);
    let report = super::ConfigCheckReport {
        path: "config/broken.toml".to_string(),
        ok: false,
        warnings: vec![
            "No model providers configured.".to_string(),
            "No models configured in catalog.".to_string(),
        ],
        database: super::DatabaseCheckReport {
            checked: false,
            ok: None,
            message: "skipped; pass --database or --check-database".to_string(),
        },
        provider_count: 0,
        model_count: 0,
        route_count: 0,
    };
    assert!(!report.ok);
    assert!(!report.database.checked);
    assert_eq!(report.database.ok, None);
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| { warning == "No model providers configured." })
    );
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| { warning == "No models configured in catalog." })
    );

    let default_key = super::DefaultKeyMaterial {
        key_id: "key_default".to_string(),
        raw_key: "mmk_default".to_string(),
        path: "/tmp/key.env".to_string(),
    };
    let payload = tui_init_json(
        &config,
        "config/broken.toml",
        Some("config/generated.toml"),
        &report,
        Some(&default_key),
    );
    assert_eq!(payload["default_key"]["key_id"], "key_default");
    assert_eq!(payload["mode"], "generated_config");

    let zh_lines = tui_init_lines(
        &config,
        "config/broken.toml",
        Some("config/generated.toml"),
        &report,
        Some(&default_key),
        Some(super::WizardLanguage::Zh),
    );
    assert!(zh_lines.iter().any(|line| line.contains("默认 Key")));
    assert!(zh_lines.iter().any(|line| line.contains("告警")));
    assert!(
        zh_lines
            .iter()
            .any(|line| line.contains("MEAT_MEMORY_CONFIG=config/generated.toml"))
    );
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
    if !local_pg_test_port_available() {
        return;
    }

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
async fn cli_command_functions_cover_pg_management_paths() {
    let _lock = lock_cli_env().await;
    let tempdir = tempdir().unwrap();
    let config_path = write_runtime_config(tempdir.path(), true, true);
    let _env = EnvVarGuard::set_path("MEAT_MEMORY_CONFIG", &config_path);
    if !local_pg_test_port_available() {
        return;
    }
    let (_, kernel, service_info) = match bootstrap_runtime().await {
        Ok(runtime) => runtime,
        Err(error) if error.to_string().contains("pool timed out") => return,
        Err(error) => panic!("bootstrap_runtime failed: {error:?}"),
    };
    let scope_id = ScopeId::new();
    let owner = kernel
        .create_access_key(memory_kernel::CreateAccessKeyRequest {
            raw_key: None,
            display_name: "cli command owner".to_string(),
            source_id: None,
            source_kind: KeySourceKind::Cli,
            owner_principal_id: "alice".to_string(),
            owner_scope_id: scope_id.clone(),
            scope_kind: KeyScopeKind::Personal,
            storage_mode: StorageMode::All,
            is_fully_isolated: false,
        })
        .await
        .unwrap();
    let raw_key = owner.raw_key.clone();
    let context = kernel
        .resolve_access_key_context(&raw_key)
        .await
        .unwrap()
        .unwrap();

    key_command(super::KeyArgs {
        command: super::KeyCommand::Create(super::KeyCreateArgs {
            name: "cli direct create".to_string(),
            source: "cli".to_string(),
            owner_principal_id: "alice".to_string(),
            owner_scope_id: Some(scope_id.as_str().to_string()),
            scope_kind: "personal".to_string(),
            storage: "file".to_string(),
            isolated: false,
            raw_key: None,
            json: true,
        }),
    })
    .await
    .unwrap();
    key_command(super::KeyArgs {
        command: super::KeyCommand::List(super::KeyListArgs {
            limit: 20,
            json: false,
        }),
    })
    .await
    .unwrap();
    key_command(super::KeyArgs {
        command: super::KeyCommand::Stats(super::KeyStatsArgs {
            key_id: None,
            json: false,
        }),
    })
    .await
    .unwrap();
    let rotate_target = kernel
        .create_access_key(memory_kernel::CreateAccessKeyRequest {
            raw_key: None,
            display_name: "cli rotate target".to_string(),
            source_id: None,
            source_kind: KeySourceKind::Cli,
            owner_principal_id: "alice".to_string(),
            owner_scope_id: scope_id.clone(),
            scope_kind: KeyScopeKind::Personal,
            storage_mode: StorageMode::All,
            is_fully_isolated: false,
        })
        .await
        .unwrap();
    key_command(super::KeyArgs {
        command: super::KeyCommand::Rotate(super::KeyRotateArgs {
            key_id: rotate_target.access_key.id.as_str().to_string(),
            json: true,
        }),
    })
    .await
    .unwrap();
    let key_file = tempdir.path().join("direct-key.env");
    key_command(super::KeyArgs {
        command: super::KeyCommand::Use(super::KeyUseArgs {
            raw_key: raw_key.clone(),
            output: Some(key_file.clone()),
            force: false,
            json: true,
        }),
    })
    .await
    .unwrap();
    assert!(key_file.exists());

    source_command(super::SourceArgs {
        command: super::SourceCommand::Create(super::SourceCreateArgs {
            key: Some(raw_key.clone()),
            name: "direct source".to_string(),
            source_kind: "local_docs".to_string(),
            source_uri: Some("file:///direct".to_string()),
            sync_mode: "index_only".to_string(),
            local_root: Some(tempdir.path().display().to_string()),
            json: true,
        }),
    })
    .await
    .unwrap();
    let sources = kernel
        .list_memory_sources(scope_id.clone(), 10, Some(&context))
        .await
        .unwrap();
    let source = sources
        .into_iter()
        .find(|source| source.display_name == "direct source")
        .unwrap();
    source_command(super::SourceArgs {
        command: super::SourceCommand::List(super::SourceListArgs {
            key: Some(raw_key.clone()),
            limit: 10,
            json: false,
        }),
    })
    .await
    .unwrap();
    source_command(super::SourceArgs {
        command: super::SourceCommand::KeyCreate(super::SourceKeyCreateArgs {
            key: Some(raw_key.clone()),
            source_id: source.id.as_str().to_string(),
            name: "direct source key".to_string(),
            source: "cli".to_string(),
            scope_kind: "personal".to_string(),
            storage: "all".to_string(),
            isolated: false,
            raw_key: None,
            json: true,
        }),
    })
    .await
    .unwrap();
    source_command(super::SourceArgs {
        command: super::SourceCommand::Keys(super::SourceKeysArgs {
            key: Some(raw_key.clone()),
            source_id: source.id.as_str().to_string(),
            limit: 10,
            json: false,
        }),
    })
    .await
    .unwrap();

    let session_id = format!("session-{}", scope_id.as_str());
    context_command(super::ContextArgs {
        command: super::ContextCommand::Upsert(super::ContextUpsertArgs {
            key: Some(raw_key.clone()),
            scope_id: Some(scope_id.as_str().to_string()),
            session_id: session_id.clone(),
            task_id: Some("cli-direct".to_string()),
            title: "Direct context".to_string(),
            body: Some("Direct context body for command coverage.".to_string()),
            file: None,
            labels: vec!["direct".to_string(), "coverage".to_string()],
            json: true,
        }),
    })
    .await
    .unwrap();
    context_command(super::ContextArgs {
        command: super::ContextCommand::List(super::ContextListArgs {
            key: Some(raw_key.clone()),
            scope_id: Some(scope_id.as_str().to_string()),
            session_id: session_id.clone(),
            task_id: Some("cli-direct".to_string()),
            limit: 10,
            json: false,
        }),
    })
    .await
    .unwrap();
    let contexts = kernel
        .list_agent_contexts(memory_kernel::ListAgentContextsRequest {
            scope_id: scope_id.clone(),
            session_id,
            task_id: Some("cli-direct".to_string()),
            limit: 10,
            context: Some(context.clone()),
        })
        .await
        .unwrap();
    let context_id = contexts[0].id.as_str().to_string();
    context_command(super::ContextArgs {
        command: super::ContextCommand::Promote(super::ContextPromoteArgs {
            key: Some(raw_key.clone()),
            context_id: context_id.clone(),
            memory_kind: Some("summary".to_string()),
            visibility: "private".to_string(),
            sensitivity: "internal".to_string(),
            json: true,
        }),
    })
    .await
    .unwrap();
    context_command(super::ContextArgs {
        command: super::ContextCommand::Delete(super::ContextDeleteArgs {
            key: Some(raw_key.clone()),
            context_id,
            json: false,
        }),
    })
    .await
    .unwrap();

    docs_command(super::DocsArgs {
        command: super::DocsCommand::Import(super::DocsImportArgs {
            key: Some(raw_key.clone()),
            source_id: source.id.as_str().to_string(),
            scope_id: Some(scope_id.as_str().to_string()),
            canonical_uri: "file:///direct/README.md".to_string(),
            title: "Direct README".to_string(),
            body: Some("Direct document body.".to_string()),
            file: None,
            local_path: Some("/direct/README.md".to_string()),
            sync_state: "clean".to_string(),
            conflict_state: "none".to_string(),
            json: true,
        }),
    })
    .await
    .unwrap();
    docs_command(super::DocsArgs {
        command: super::DocsCommand::List(super::DocsListArgs {
            key: Some(raw_key.clone()),
            source_id: source.id.as_str().to_string(),
            query: Some("README".to_string()),
            limit: 10,
            json: false,
        }),
    })
    .await
    .unwrap();
    let documents = kernel
        .list_project_documents(memory_kernel::ListProjectDocumentsRequest {
            source_id: source.id.clone(),
            limit: 10,
            query: Some("README".to_string()),
            context: Some(context.clone()),
        })
        .await
        .unwrap();
    docs_command(super::DocsArgs {
        command: super::DocsCommand::Projection(super::DocsProjectionArgs {
            key: Some(raw_key.clone()),
            source_id: source.id.as_str().to_string(),
            document_id: documents[0].id.as_str().to_string(),
            json: true,
        }),
    })
    .await
    .unwrap();
    docs_command(super::DocsArgs {
        command: super::DocsCommand::Conflicts(super::DocsConflictsArgs {
            key: Some(raw_key.clone()),
            source_id: source.id.as_str().to_string(),
            limit: 10,
            json: false,
        }),
    })
    .await
    .unwrap();
    let docs_root = tempdir.path().join("docs-sync");
    fs::create_dir_all(&docs_root).unwrap();
    fs::write(docs_root.join("SYNC.md"), "# Sync\nDirect sync body.").unwrap();
    docs_command(super::DocsArgs {
        command: super::DocsCommand::Status(super::DocsStatusArgs {
            key: Some(raw_key.clone()),
            source_id: source.id.as_str().to_string(),
            local_root: Some(docs_root.display().to_string()),
            json: true,
        }),
    })
    .await
    .unwrap();
    docs_command(super::DocsArgs {
        command: super::DocsCommand::Sync(super::DocsSyncArgs {
            key: Some(raw_key.clone()),
            source_id: source.id.as_str().to_string(),
            scope_id: Some(scope_id.as_str().to_string()),
            local_root: Some(docs_root.display().to_string()),
            dry_run: true,
            json: false,
        }),
    })
    .await
    .unwrap();

    let mut remember_request = memory_kernel::RememberTextRequest::new(
        scope_id.clone(),
        "Lifecycle direct command body.".to_string(),
    );
    remember_request.title = Some("Lifecycle direct".to_string());
    remember_request.source_refs = vec!["agent-context://cli-direct".to_string()];
    remember_request.context = Some(context.clone());
    let remembered = kernel.remember_text(remember_request).await.unwrap();
    let memory_id = remembered.memory.id.as_str().to_string();
    lifecycle_command(super::LifecycleArgs {
        command: super::LifecycleCommand::Inspect(super::LifecycleInspectArgs {
            memory_id: memory_id.clone(),
            key: Some(raw_key.clone()),
            scope_id: Some(scope_id.as_str().to_string()),
            query: Some("Lifecycle".to_string()),
            json: true,
        }),
    })
    .await
    .unwrap();
    lifecycle_command(super::LifecycleArgs {
        command: super::LifecycleCommand::Status(super::LifecycleStatusArgs {
            memory_id: memory_id.clone(),
            status: "archived".to_string(),
            key: Some(raw_key.clone()),
            scope_id: Some(scope_id.as_str().to_string()),
            reason: Some("coverage".to_string()),
            actor: Some("cli-test".to_string()),
            json: false,
        }),
    })
    .await
    .unwrap();
    lifecycle_command(super::LifecycleArgs {
        command: super::LifecycleCommand::Forget(super::LifecycleActionArgs {
            memory_id: memory_id.clone(),
            key: Some(raw_key.clone()),
            scope_id: Some(scope_id.as_str().to_string()),
            reason: Some("coverage forget".to_string()),
            actor: None,
            json: true,
        }),
    })
    .await
    .unwrap();
    lifecycle_command(super::LifecycleArgs {
        command: super::LifecycleCommand::Restore(super::LifecycleActionArgs {
            memory_id,
            key: Some(raw_key.clone()),
            scope_id: Some(scope_id.as_str().to_string()),
            reason: Some("coverage restore".to_string()),
            actor: None,
            json: false,
        }),
    })
    .await
    .unwrap();
    lifecycle_command(super::LifecycleArgs {
        command: super::LifecycleCommand::Report(super::LifecycleReportArgs {
            scope_id: Some(scope_id.as_str().to_string()),
            limit: 20,
            json: true,
        }),
    })
    .await
    .unwrap();

    config_command(super::ConfigArgs {
        command: super::ConfigCommand::Show(super::InspectArgs {
            json: true,
            check_http: false,
        }),
    })
    .await
    .unwrap();
    config_command(super::ConfigArgs {
        command: super::ConfigCommand::Check(super::CheckArgs {
            json: false,
            database: false,
        }),
    })
    .await
    .unwrap();
    mcp_command(super::McpArgs {
        command: super::McpCommand::Info(super::InspectArgs {
            json: true,
            check_http: false,
        }),
    })
    .unwrap();
    let previous_dir = env::current_dir().unwrap();
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    env::set_current_dir(&repo_root).unwrap();
    skills_command(super::SkillsArgs {
        command: super::SkillsCommand::Export(super::SkillsExportArgs {
            target: super::SkillExportTarget::Codex,
            output_dir: Some(tempdir.path().join("skills-out")),
            force: true,
            json: true,
        }),
    })
    .unwrap();
    env::set_current_dir(previous_dir).unwrap();
    tui_command(super::TuiArgs {
        command: super::TuiCommand::Init(super::TuiInitArgs {
            json: true,
            interactive: false,
            default_locale: Some("zh-CN".to_string()),
            write_config: None,
            force: false,
            check_database: false,
            enable_mcp: true,
            disable_mcp: false,
            database_url: None,
            markdown_root: None,
            assets_root: None,
            reasoning_primary: None,
            extraction_primary: None,
            vision_primary: None,
            embedding_primary: None,
        }),
    })
    .await
    .unwrap();
    project_command(super::ProjectArgs {
        command: super::ProjectCommand::Init(super::ProjectInitArgs {
            json: true,
            interactive: false,
            existing: true,
            name: Some("Existing direct project".to_string()),
            owner_principal_id: "alice".to_string(),
            owner_scope_id: Some(scope_id.as_str().to_string()),
            scope_kind: "team".to_string(),
            storage: "all".to_string(),
            isolated: false,
            shared: false,
            list_limit: 20,
        }),
    })
    .await
    .unwrap();

    remember_command(super::RememberArgs {
        key: Some(raw_key.clone()),
        scope_id: Some(scope_id.as_str().to_string()),
        title: Some("Wrapper remember".to_string()),
        body: Some("Wrapper remember body.".to_string()),
        file: None,
        artifact_kind: "message".to_string(),
        memory_kind: Some("fact".to_string()),
        source_refs: vec!["api://cli-wrapper".to_string()],
        visibility: "private".to_string(),
        sensitivity: "internal".to_string(),
        json: true,
    })
    .await
    .unwrap();
    search_command(super::SearchArgs {
        query: "Wrapper".to_string(),
        key: Some(raw_key),
        scope_id: Some(scope_id.as_str().to_string()),
        limit: 5,
        json: false,
    })
    .await
    .unwrap();

    assert_eq!(service_info.name, "meat-memory");
}

#[tokio::test]
async fn cli_command_functions_cover_alternate_rendering_paths() {
    let _lock = lock_cli_env().await;
    let tempdir = tempdir().unwrap();
    let config_path = write_runtime_config(tempdir.path(), true, true);
    let _env = EnvVarGuard::set_path("MEAT_MEMORY_CONFIG", &config_path);
    if !local_pg_test_port_available() {
        return;
    }
    let (_, kernel, _) = match bootstrap_runtime().await {
        Ok(runtime) => runtime,
        Err(error) if error.to_string().contains("pool timed out") => return,
        Err(error) => panic!("bootstrap_runtime failed: {error:?}"),
    };
    let scope_id = ScopeId::new();
    let owner = kernel
        .create_access_key(memory_kernel::CreateAccessKeyRequest {
            raw_key: None,
            display_name: "alternate owner".to_string(),
            source_id: None,
            source_kind: KeySourceKind::Cli,
            owner_principal_id: "alice".to_string(),
            owner_scope_id: scope_id.clone(),
            scope_kind: KeyScopeKind::Personal,
            storage_mode: StorageMode::All,
            is_fully_isolated: false,
        })
        .await
        .unwrap();
    let raw_key = owner.raw_key.clone();
    let context = kernel
        .resolve_access_key_context(&raw_key)
        .await
        .unwrap()
        .unwrap();

    key_command(super::KeyArgs {
        command: super::KeyCommand::Create(super::KeyCreateArgs {
            name: "alternate text key".to_string(),
            source: "cli".to_string(),
            owner_principal_id: "alice".to_string(),
            owner_scope_id: Some(scope_id.as_str().to_string()),
            scope_kind: "team".to_string(),
            storage: "all".to_string(),
            isolated: true,
            raw_key: None,
            json: false,
        }),
    })
    .await
    .unwrap();
    key_command(super::KeyArgs {
        command: super::KeyCommand::List(super::KeyListArgs {
            limit: 5,
            json: true,
        }),
    })
    .await
    .unwrap();
    let rotate_target = kernel
        .create_access_key(memory_kernel::CreateAccessKeyRequest {
            raw_key: None,
            display_name: "alternate rotate".to_string(),
            source_id: None,
            source_kind: KeySourceKind::Cli,
            owner_principal_id: "alice".to_string(),
            owner_scope_id: scope_id.clone(),
            scope_kind: KeyScopeKind::Personal,
            storage_mode: StorageMode::All,
            is_fully_isolated: false,
        })
        .await
        .unwrap();
    key_command(super::KeyArgs {
        command: super::KeyCommand::Rotate(super::KeyRotateArgs {
            key_id: rotate_target.access_key.id.as_str().to_string(),
            json: false,
        }),
    })
    .await
    .unwrap();
    let key_file = tempdir.path().join("alternate-key.env");
    key_command(super::KeyArgs {
        command: super::KeyCommand::Use(super::KeyUseArgs {
            raw_key: raw_key.clone(),
            output: Some(key_file.clone()),
            force: true,
            json: false,
        }),
    })
    .await
    .unwrap();
    let existing_error = key_command(super::KeyArgs {
        command: super::KeyCommand::Use(super::KeyUseArgs {
            raw_key: raw_key.clone(),
            output: Some(key_file),
            force: false,
            json: false,
        }),
    })
    .await
    .expect_err("existing key env should require --force");
    assert!(existing_error.to_string().contains("already exists"));
    key_command(super::KeyArgs {
        command: super::KeyCommand::Stats(super::KeyStatsArgs {
            key_id: None,
            json: true,
        }),
    })
    .await
    .unwrap();

    source_command(super::SourceArgs {
        command: super::SourceCommand::Create(super::SourceCreateArgs {
            key: Some(raw_key.clone()),
            name: "alternate source".to_string(),
            source_kind: "local_docs".to_string(),
            source_uri: Some("file:///alternate".to_string()),
            sync_mode: "index_only".to_string(),
            local_root: Some(tempdir.path().display().to_string()),
            json: false,
        }),
    })
    .await
    .unwrap();
    let source = kernel
        .list_memory_sources(scope_id.clone(), 10, Some(&context))
        .await
        .unwrap()
        .into_iter()
        .find(|source| source.display_name == "alternate source")
        .unwrap();
    source_command(super::SourceArgs {
        command: super::SourceCommand::List(super::SourceListArgs {
            key: Some(raw_key.clone()),
            limit: 10,
            json: true,
        }),
    })
    .await
    .unwrap();
    source_command(super::SourceArgs {
        command: super::SourceCommand::Keys(super::SourceKeysArgs {
            key: Some(raw_key.clone()),
            source_id: source.id.as_str().to_string(),
            limit: 10,
            json: true,
        }),
    })
    .await
    .unwrap();
    source_command(super::SourceArgs {
        command: super::SourceCommand::KeyCreate(super::SourceKeyCreateArgs {
            key: Some(raw_key.clone()),
            source_id: source.id.as_str().to_string(),
            name: "alternate source key".to_string(),
            source: "cli".to_string(),
            scope_kind: "team".to_string(),
            storage: "all".to_string(),
            isolated: false,
            raw_key: None,
            json: false,
        }),
    })
    .await
    .unwrap();

    context_command(super::ContextArgs {
        command: super::ContextCommand::Upsert(super::ContextUpsertArgs {
            key: Some(raw_key.clone()),
            scope_id: Some(scope_id.as_str().to_string()),
            session_id: "alternate-session".to_string(),
            task_id: None,
            title: "Alternate context".to_string(),
            body: Some("Alternate context body.".to_string()),
            file: None,
            labels: Vec::new(),
            json: false,
        }),
    })
    .await
    .unwrap();
    context_command(super::ContextArgs {
        command: super::ContextCommand::List(super::ContextListArgs {
            key: Some(raw_key.clone()),
            scope_id: Some(scope_id.as_str().to_string()),
            session_id: "alternate-session".to_string(),
            task_id: None,
            limit: 10,
            json: true,
        }),
    })
    .await
    .unwrap();
    let contexts = kernel
        .list_agent_contexts(memory_kernel::ListAgentContextsRequest {
            scope_id: scope_id.clone(),
            session_id: "alternate-session".to_string(),
            task_id: None,
            limit: 10,
            context: Some(context.clone()),
        })
        .await
        .unwrap();
    let context_id = contexts[0].id.as_str().to_string();
    context_command(super::ContextArgs {
        command: super::ContextCommand::Promote(super::ContextPromoteArgs {
            key: Some(raw_key.clone()),
            context_id: context_id.clone(),
            memory_kind: Some("fact".to_string()),
            visibility: "private".to_string(),
            sensitivity: "internal".to_string(),
            json: false,
        }),
    })
    .await
    .unwrap();
    context_command(super::ContextArgs {
        command: super::ContextCommand::Delete(super::ContextDeleteArgs {
            key: Some(raw_key.clone()),
            context_id,
            json: true,
        }),
    })
    .await
    .unwrap();

    docs_command(super::DocsArgs {
        command: super::DocsCommand::Import(super::DocsImportArgs {
            key: Some(raw_key.clone()),
            source_id: source.id.as_str().to_string(),
            scope_id: Some(scope_id.as_str().to_string()),
            canonical_uri: "file:///alternate/README.md".to_string(),
            title: "Alternate README".to_string(),
            body: Some("Alternate document body.".to_string()),
            file: None,
            local_path: Some("/alternate/README.md".to_string()),
            sync_state: "clean".to_string(),
            conflict_state: "none".to_string(),
            json: false,
        }),
    })
    .await
    .unwrap();
    docs_command(super::DocsArgs {
        command: super::DocsCommand::List(super::DocsListArgs {
            key: Some(raw_key.clone()),
            source_id: source.id.as_str().to_string(),
            query: Some("Alternate".to_string()),
            limit: 10,
            json: true,
        }),
    })
    .await
    .unwrap();
    let documents = kernel
        .list_project_documents(memory_kernel::ListProjectDocumentsRequest {
            source_id: source.id.clone(),
            limit: 10,
            query: Some("Alternate".to_string()),
            context: Some(context.clone()),
        })
        .await
        .unwrap();
    docs_command(super::DocsArgs {
        command: super::DocsCommand::Projection(super::DocsProjectionArgs {
            key: Some(raw_key.clone()),
            source_id: source.id.as_str().to_string(),
            document_id: documents[0].id.as_str().to_string(),
            json: false,
        }),
    })
    .await
    .unwrap();
    docs_command(super::DocsArgs {
        command: super::DocsCommand::Conflicts(super::DocsConflictsArgs {
            key: Some(raw_key.clone()),
            source_id: source.id.as_str().to_string(),
            limit: 10,
            json: true,
        }),
    })
    .await
    .unwrap();
    let docs_root = tempdir.path().join("alternate-sync");
    fs::create_dir_all(&docs_root).unwrap();
    fs::write(docs_root.join("ALT.md"), "# Alt\nAlternate sync body.").unwrap();
    docs_command(super::DocsArgs {
        command: super::DocsCommand::Status(super::DocsStatusArgs {
            key: Some(raw_key.clone()),
            source_id: source.id.as_str().to_string(),
            local_root: Some(docs_root.display().to_string()),
            json: false,
        }),
    })
    .await
    .unwrap();
    docs_command(super::DocsArgs {
        command: super::DocsCommand::Sync(super::DocsSyncArgs {
            key: Some(raw_key.clone()),
            source_id: source.id.as_str().to_string(),
            scope_id: Some(scope_id.as_str().to_string()),
            local_root: Some(docs_root.display().to_string()),
            dry_run: true,
            json: true,
        }),
    })
    .await
    .unwrap();
    let mut remember_request = memory_kernel::RememberTextRequest::new(
        scope_id.clone(),
        "Alternate lifecycle command body.".to_string(),
    );
    remember_request.title = Some("Alternate lifecycle".to_string());
    remember_request.context = Some(context);
    let remembered = kernel.remember_text(remember_request).await.unwrap();
    let memory_id = remembered.memory.id.as_str().to_string();
    lifecycle_command(super::LifecycleArgs {
        command: super::LifecycleCommand::Inspect(super::LifecycleInspectArgs {
            memory_id: memory_id.clone(),
            key: Some(raw_key.clone()),
            scope_id: Some(scope_id.as_str().to_string()),
            query: None,
            json: false,
        }),
    })
    .await
    .unwrap();
    lifecycle_command(super::LifecycleArgs {
        command: super::LifecycleCommand::Status(super::LifecycleStatusArgs {
            memory_id: memory_id.clone(),
            status: "active".to_string(),
            key: Some(raw_key.clone()),
            scope_id: Some(scope_id.as_str().to_string()),
            reason: None,
            actor: None,
            json: true,
        }),
    })
    .await
    .unwrap();
    lifecycle_command(super::LifecycleArgs {
        command: super::LifecycleCommand::Forget(super::LifecycleActionArgs {
            memory_id: memory_id.clone(),
            key: Some(raw_key.clone()),
            scope_id: Some(scope_id.as_str().to_string()),
            reason: None,
            actor: Some("alternate-test".to_string()),
            json: false,
        }),
    })
    .await
    .unwrap();
    lifecycle_command(super::LifecycleArgs {
        command: super::LifecycleCommand::Restore(super::LifecycleActionArgs {
            memory_id,
            key: Some(raw_key.clone()),
            scope_id: Some(scope_id.as_str().to_string()),
            reason: None,
            actor: Some("alternate-test".to_string()),
            json: true,
        }),
    })
    .await
    .unwrap();
    lifecycle_command(super::LifecycleArgs {
        command: super::LifecycleCommand::Report(super::LifecycleReportArgs {
            scope_id: Some(scope_id.as_str().to_string()),
            limit: 20,
            json: false,
        }),
    })
    .await
    .unwrap();
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
