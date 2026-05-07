use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "memory-cli",
    about = "Local operator entrypoint for Meat Memory"
)]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Command,
}

#[derive(Debug, Subcommand)]
pub(super) enum Command {
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
    Lifecycle(LifecycleArgs),
    Proposals(ProposalArgs),
    Versions(VersionsArgs),
    Timeline(TimelineArgs),
    Rollback(RollbackArgs),
    Profiles(ProfilesArgs),
    Distill(DistillArgs),
    Tui(TuiArgs),
    Serve(ServeArgs),
    Remember(RememberArgs),
    RememberImage(RememberImageArgs),
    Search(SearchArgs),
    Benchmark(BenchmarkArgs),
    Trace(TraceArgs),
    Health(HealthArgs),
    Passport(PassportArgs),
    Compat(CompatArgs),
}

#[derive(Debug, Clone, Args)]
pub(super) struct ConfigArgs {
    #[command(subcommand)]
    pub(super) command: ConfigCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum ConfigCommand {
    Show(InspectArgs),
    Check(CheckArgs),
}

#[derive(Debug, Clone, Args)]
pub(super) struct McpArgs {
    #[command(subcommand)]
    pub(super) command: McpCommand,
}

#[derive(Debug, Clone, Args)]
pub(super) struct SkillsArgs {
    #[command(subcommand)]
    pub(super) command: SkillsCommand,
}

#[derive(Debug, Clone, Args)]
pub(super) struct KeyArgs {
    #[command(subcommand)]
    pub(super) command: KeyCommand,
}

#[derive(Debug, Clone, Args)]
pub(super) struct SourceArgs {
    #[command(subcommand)]
    pub(super) command: SourceCommand,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ProjectArgs {
    #[command(subcommand)]
    pub(super) command: ProjectCommand,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ContextArgs {
    #[command(subcommand)]
    pub(super) command: ContextCommand,
}

#[derive(Debug, Clone, Args)]
pub(super) struct DocsArgs {
    #[command(subcommand)]
    pub(super) command: DocsCommand,
}

#[derive(Debug, Clone, Args)]
pub(super) struct LifecycleArgs {
    #[command(subcommand)]
    pub(super) command: LifecycleCommand,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ProposalArgs {
    #[command(subcommand)]
    pub(super) command: ProposalCommand,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ProfilesArgs {
    #[command(subcommand)]
    pub(super) command: ProfilesCommand,
}

#[derive(Debug, Clone, Args)]
pub(super) struct DistillArgs {
    #[command(subcommand)]
    pub(super) command: DistillCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum KeyCommand {
    Create(KeyCreateArgs),
    List(KeyListArgs),
    Rotate(KeyRotateArgs),
    Use(KeyUseArgs),
    Stats(KeyStatsArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum SourceCommand {
    Create(SourceCreateArgs),
    List(SourceListArgs),
    Keys(SourceKeysArgs),
    KeyCreate(SourceKeyCreateArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum ProjectCommand {
    Init(ProjectInitArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum ContextCommand {
    Upsert(ContextUpsertArgs),
    List(ContextListArgs),
    Promote(ContextPromoteArgs),
    Delete(ContextDeleteArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum DocsCommand {
    Import(DocsImportArgs),
    List(DocsListArgs),
    Projection(DocsProjectionArgs),
    Conflicts(DocsConflictsArgs),
    Sync(DocsSyncArgs),
    Status(DocsStatusArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum LifecycleCommand {
    Inspect(LifecycleInspectArgs),
    Status(LifecycleStatusArgs),
    Forget(LifecycleActionArgs),
    Restore(LifecycleActionArgs),
    Report(LifecycleReportArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum ProposalCommand {
    List(ProposalListArgs),
    Inspect(ProposalInspectArgs),
    Approve(ProposalDecisionArgs),
    Reject(ProposalRejectArgs),
    Apply(ProposalDecisionArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum ProfilesCommand {
    List(ProfilesListArgs),
    Upsert(ProfileUpsertArgs),
    Archive(ProfileArchiveArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum DistillCommand {
    Preview(DistillPreviewArgs),
}

#[derive(Debug, Clone, Args)]
pub(super) struct TuiArgs {
    #[command(subcommand)]
    pub(super) command: TuiCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum TuiCommand {
    Init(TuiInitArgs),
    KeyCreate(KeyCreateArgs),
    ProjectInit(ProjectInitArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum SkillsCommand {
    Export(SkillsExportArgs),
}

#[derive(Debug, Clone, Args)]
pub(super) struct KeyCreateArgs {
    #[arg(long)]
    pub(super) name: String,
    #[arg(long, default_value = "cli")]
    pub(super) source: String,
    #[arg(long, default_value = "local-user")]
    pub(super) owner_principal_id: String,
    #[arg(long)]
    pub(super) owner_scope_id: Option<String>,
    #[arg(long, default_value = "personal")]
    pub(super) scope_kind: String,
    #[arg(long, default_value = "all")]
    pub(super) storage: String,
    #[arg(long)]
    pub(super) isolated: bool,
    #[arg(long)]
    pub(super) raw_key: Option<String>,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct KeyListArgs {
    #[arg(long, default_value_t = 200)]
    pub(super) limit: usize,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct KeyUseArgs {
    #[arg(long)]
    pub(super) raw_key: String,
    #[arg(long)]
    pub(super) output: Option<PathBuf>,
    #[arg(long)]
    pub(super) force: bool,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct KeyRotateArgs {
    #[arg(long)]
    pub(super) key_id: String,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct KeyStatsArgs {
    #[arg(long)]
    pub(super) key_id: Option<String>,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct SourceCreateArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) name: String,
    #[arg(long, default_value = "custom")]
    pub(super) source_kind: String,
    #[arg(long)]
    pub(super) source_uri: Option<String>,
    #[arg(long, default_value = "read_only")]
    pub(super) sync_mode: String,
    #[arg(long)]
    pub(super) local_root: Option<String>,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct SourceListArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long, default_value_t = 100)]
    pub(super) limit: usize,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct SourceKeysArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) source_id: String,
    #[arg(long, default_value_t = 100)]
    pub(super) limit: usize,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct SourceKeyCreateArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) source_id: String,
    #[arg(long)]
    pub(super) name: String,
    #[arg(long, default_value = "custom")]
    pub(super) source: String,
    #[arg(long, default_value = "personal")]
    pub(super) scope_kind: String,
    #[arg(long, default_value = "all")]
    pub(super) storage: String,
    #[arg(long)]
    pub(super) isolated: bool,
    #[arg(long)]
    pub(super) raw_key: Option<String>,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ProjectInitArgs {
    #[arg(long)]
    pub(super) json: bool,
    #[arg(long)]
    pub(super) interactive: bool,
    #[arg(long)]
    pub(super) existing: bool,
    #[arg(long)]
    pub(super) name: Option<String>,
    #[arg(long, default_value = "local-user")]
    pub(super) owner_principal_id: String,
    #[arg(long, visible_alias = "scope-id")]
    pub(super) owner_scope_id: Option<String>,
    #[arg(long, default_value = "team")]
    pub(super) scope_kind: String,
    #[arg(long, default_value = "all")]
    pub(super) storage: String,
    #[arg(long, conflicts_with = "shared")]
    pub(super) isolated: bool,
    #[arg(long, conflicts_with = "isolated")]
    pub(super) shared: bool,
    #[arg(long, default_value_t = 20)]
    pub(super) list_limit: usize,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ContextUpsertArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long)]
    pub(super) session_id: String,
    #[arg(long)]
    pub(super) task_id: Option<String>,
    #[arg(long)]
    pub(super) title: String,
    #[arg(long, conflicts_with = "file")]
    pub(super) body: Option<String>,
    #[arg(long)]
    pub(super) file: Option<PathBuf>,
    #[arg(long, value_delimiter = ',')]
    pub(super) labels: Vec<String>,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ContextListArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long)]
    pub(super) session_id: String,
    #[arg(long)]
    pub(super) task_id: Option<String>,
    #[arg(long, default_value_t = 20)]
    pub(super) limit: usize,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ContextPromoteArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) context_id: String,
    #[arg(long)]
    pub(super) memory_kind: Option<String>,
    #[arg(long, default_value = "private")]
    pub(super) visibility: String,
    #[arg(long, default_value = "internal")]
    pub(super) sensitivity: String,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ContextDeleteArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) context_id: String,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct DocsImportArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) source_id: String,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long)]
    pub(super) canonical_uri: String,
    #[arg(long)]
    pub(super) title: String,
    #[arg(long, conflicts_with = "file")]
    pub(super) body: Option<String>,
    #[arg(long)]
    pub(super) file: Option<PathBuf>,
    #[arg(long)]
    pub(super) local_path: Option<String>,
    #[arg(long, default_value = "clean")]
    pub(super) sync_state: String,
    #[arg(long, default_value = "none")]
    pub(super) conflict_state: String,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct DocsListArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) source_id: String,
    #[arg(long)]
    pub(super) query: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub(super) limit: usize,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct DocsProjectionArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) source_id: String,
    #[arg(long)]
    pub(super) document_id: String,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct DocsConflictsArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) source_id: String,
    #[arg(long, default_value_t = 50)]
    pub(super) limit: usize,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct DocsSyncArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) source_id: String,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long)]
    pub(super) local_root: Option<String>,
    #[arg(long)]
    pub(super) dry_run: bool,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct DocsStatusArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) source_id: String,
    #[arg(long)]
    pub(super) local_root: Option<String>,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum McpCommand {
    Info(InspectArgs),
}

#[derive(Debug, Clone, Args)]
pub(super) struct InspectArgs {
    #[arg(long)]
    pub(super) json: bool,
    #[arg(long)]
    pub(super) check_http: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct CheckArgs {
    #[arg(long)]
    pub(super) json: bool,
    #[arg(long)]
    pub(super) database: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct SkillsExportArgs {
    #[arg(long, value_enum, default_value_t = SkillExportTarget::All)]
    pub(super) target: SkillExportTarget,
    #[arg(long)]
    pub(super) output_dir: Option<PathBuf>,
    #[arg(long)]
    pub(super) force: bool,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum SkillExportTarget {
    Codex,
    ClaudeCode,
    ExecutionAgent,
    All,
}

#[derive(Debug, Clone, Args)]
pub(super) struct TuiInitArgs {
    #[arg(long)]
    pub(super) json: bool,
    #[arg(long)]
    pub(super) interactive: bool,
    #[arg(long)]
    pub(super) default_locale: Option<String>,
    #[arg(long)]
    pub(super) write_config: Option<PathBuf>,
    #[arg(long)]
    pub(super) force: bool,
    #[arg(long)]
    pub(super) check_database: bool,
    #[arg(long, conflicts_with = "disable_mcp")]
    pub(super) enable_mcp: bool,
    #[arg(long, conflicts_with = "enable_mcp")]
    pub(super) disable_mcp: bool,
    #[arg(long)]
    pub(super) database_url: Option<String>,
    #[arg(long)]
    pub(super) markdown_root: Option<String>,
    #[arg(long)]
    pub(super) assets_root: Option<String>,
    #[arg(long)]
    pub(super) reasoning_primary: Option<String>,
    #[arg(long)]
    pub(super) extraction_primary: Option<String>,
    #[arg(long)]
    pub(super) vision_primary: Option<String>,
    #[arg(long)]
    pub(super) embedding_primary: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ServeArgs {
    #[arg(long)]
    pub(super) bind: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub(super) struct RememberArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long)]
    pub(super) title: Option<String>,
    #[arg(long, conflicts_with = "file")]
    pub(super) body: Option<String>,
    #[arg(long)]
    pub(super) file: Option<PathBuf>,
    #[arg(long, default_value = "message")]
    pub(super) artifact_kind: String,
    #[arg(long)]
    pub(super) memory_kind: Option<String>,
    #[arg(long, value_delimiter = ',')]
    pub(super) source_refs: Vec<String>,
    #[arg(long, default_value = "private")]
    pub(super) visibility: String,
    #[arg(long, default_value = "internal")]
    pub(super) sensitivity: String,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct RememberImageArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long)]
    pub(super) title: Option<String>,
    #[arg(long)]
    pub(super) body: Option<String>,
    #[arg(long)]
    pub(super) file: PathBuf,
    #[arg(long)]
    pub(super) media_type: Option<String>,
    #[arg(long)]
    pub(super) memory_kind: Option<String>,
    #[arg(long, value_delimiter = ',')]
    pub(super) source_refs: Vec<String>,
    #[arg(long, default_value = "private")]
    pub(super) visibility: String,
    #[arg(long, default_value = "internal")]
    pub(super) sensitivity: String,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct SearchArgs {
    pub(super) query: String,
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long, default_value_t = 10)]
    pub(super) limit: usize,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct BenchmarkArgs {
    #[command(subcommand)]
    pub(super) command: BenchmarkCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum BenchmarkCommand {
    Run(BenchmarkRunArgs),
    Report(BenchmarkReportArgs),
}

#[derive(Debug, Clone, Args)]
pub(super) struct BenchmarkRunArgs {
    #[arg(long, default_value = "meat-code-zh")]
    pub(super) suite: String,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long, default_value = "tests/reports/benchmark/latest")]
    pub(super) output_dir: PathBuf,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct BenchmarkReportArgs {
    #[arg(long, default_value = "tests/reports/benchmark/latest")]
    pub(super) input_dir: PathBuf,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct TraceArgs {
    #[command(subcommand)]
    pub(super) command: TraceCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum TraceCommand {
    Latest(TraceLatestArgs),
    Inspect(TraceInspectArgs),
}

#[derive(Debug, Clone, Args)]
pub(super) struct TraceLatestArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long)]
    pub(super) query: Option<String>,
    #[arg(long, default_value_t = 10)]
    pub(super) limit: usize,
    #[arg(long, default_value_t = 5)]
    pub(super) max_records: usize,
    #[arg(long, default_value_t = 2000)]
    pub(super) max_chars: usize,
    #[arg(long)]
    pub(super) debug_candidates: bool,
    #[arg(long, default_value = "tests/reports/trace/latest")]
    pub(super) output_dir: PathBuf,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct TraceInspectArgs {
    #[arg(long)]
    pub(super) trace_id: String,
    #[arg(long, default_value = "tests/reports/trace/latest")]
    pub(super) input_dir: PathBuf,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct HealthArgs {
    #[command(subcommand)]
    pub(super) command: HealthCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum HealthCommand {
    Report(HealthReportArgs),
}

#[derive(Debug, Clone, Args)]
pub(super) struct PassportArgs {
    #[command(subcommand)]
    pub(super) command: PassportCommand,
}

#[derive(Debug, Clone, Args)]
pub(super) struct CompatArgs {
    #[command(subcommand)]
    pub(super) command: CompatCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum PassportCommand {
    Export(PassportExportArgs),
    Verify(PassportVerifyArgs),
    Import(PassportImportArgs),
    Provenance(PassportProvenanceArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum CompatCommand {
    Report(CompatReportArgs),
    ConnectorDryRun(CompatConnectorDryRunArgs),
    ConnectorImportDraft(CompatConnectorImportDraftArgs),
    ConnectorProposalApplyPlan(CompatConnectorProposalApplyPlanArgs),
    ConnectorProposalQueue(CompatConnectorProposalQueueArgs),
    ConnectorSyncPlan(CompatConnectorSyncPlanArgs),
}

#[derive(Debug, Clone, Args)]
pub(super) struct HealthReportArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long, default_value_t = 100)]
    pub(super) limit: usize,
    #[arg(long, default_value = "tests/reports/health/latest")]
    pub(super) output_dir: PathBuf,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct PassportExportArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long, default_value_t = 100)]
    pub(super) limit: usize,
    #[arg(long, default_value = "tests/reports/passport/latest")]
    pub(super) output_dir: PathBuf,
    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    pub(super) redact_sensitive: bool,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct PassportVerifyArgs {
    #[arg(long, default_value = "tests/reports/passport/latest")]
    pub(super) input_dir: PathBuf,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct PassportImportArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long, default_value = "tests/reports/passport/latest")]
    pub(super) input_dir: PathBuf,
    #[arg(long)]
    pub(super) target_scope_id: Option<String>,
    #[arg(long)]
    pub(super) dry_run: bool,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct PassportProvenanceArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long)]
    pub(super) memory_id: String,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct CompatReportArgs {
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long, default_value = "tests/reports/compat/latest")]
    pub(super) output_dir: PathBuf,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct CompatConnectorDryRunArgs {
    #[arg(long)]
    pub(super) connector: String,
    #[arg(long, default_value = ".")]
    pub(super) root_path: PathBuf,
    #[arg(long, default_value = "tests/reports/compat/latest")]
    pub(super) output_dir: PathBuf,
    #[arg(long, default_value_t = 50)]
    pub(super) max_items: usize,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct CompatConnectorImportDraftArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) connector: String,
    #[arg(long, default_value = ".")]
    pub(super) root_path: PathBuf,
    #[arg(long, default_value = "scp_v297_connector")]
    pub(super) scope_id: String,
    #[arg(long, default_value = "tests/reports/compat/latest")]
    pub(super) output_dir: PathBuf,
    #[arg(long, default_value_t = 100)]
    pub(super) max_items: usize,
    #[arg(long)]
    pub(super) apply: bool,
    #[arg(long)]
    pub(super) proposal: bool,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct CompatConnectorProposalQueueArgs {
    #[arg(long)]
    pub(super) connector: String,
    #[arg(long, default_value = ".")]
    pub(super) root_path: PathBuf,
    #[arg(long, default_value = "scp_v297_connector")]
    pub(super) scope_id: String,
    #[arg(long, default_value = "tests/reports/compat/latest")]
    pub(super) output_dir: PathBuf,
    #[arg(long, default_value_t = 100)]
    pub(super) max_items: usize,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct CompatConnectorProposalApplyPlanArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) connector: String,
    #[arg(long, default_value = ".")]
    pub(super) root_path: PathBuf,
    #[arg(long)]
    pub(super) queue_file: Option<PathBuf>,
    #[arg(long)]
    pub(super) source_id: Option<String>,
    #[arg(long, default_value = "scp_v297_connector")]
    pub(super) scope_id: String,
    #[arg(long, value_delimiter = ',')]
    pub(super) approve_queue_item: Vec<String>,
    #[arg(long)]
    pub(super) confirmation_token: String,
    #[arg(long, default_value = "tests/reports/compat/latest")]
    pub(super) output_dir: PathBuf,
    #[arg(long, default_value_t = 100)]
    pub(super) max_items: usize,
    #[arg(long)]
    pub(super) apply: bool,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct CompatConnectorSyncPlanArgs {
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) connector: String,
    #[arg(long, default_value = ".")]
    pub(super) root_path: PathBuf,
    #[arg(long)]
    pub(super) source_id: Option<String>,
    #[arg(long, default_value = "scp_v297_connector")]
    pub(super) scope_id: String,
    #[arg(long, default_value = "tests/reports/compat/latest")]
    pub(super) output_dir: PathBuf,
    #[arg(long, default_value_t = 500)]
    pub(super) max_items: usize,
    #[arg(long)]
    pub(super) apply: bool,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct LifecycleInspectArgs {
    pub(super) memory_id: String,
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long)]
    pub(super) query: Option<String>,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct LifecycleStatusArgs {
    pub(super) memory_id: String,
    #[arg(long)]
    pub(super) status: String,
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long)]
    pub(super) reason: Option<String>,
    #[arg(long)]
    pub(super) actor: Option<String>,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct LifecycleActionArgs {
    pub(super) memory_id: String,
    #[arg(long)]
    pub(super) key: Option<String>,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long)]
    pub(super) reason: Option<String>,
    #[arg(long)]
    pub(super) actor: Option<String>,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct LifecycleReportArgs {
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long, default_value_t = 500)]
    pub(super) limit: usize,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ProposalListArgs {
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub(super) limit: usize,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ProposalInspectArgs {
    pub(super) proposal_id: String,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ProposalDecisionArgs {
    pub(super) proposal_id: String,
    #[arg(long, default_value = "local-user")]
    pub(super) actor: String,
    #[arg(long, default_value = "user")]
    pub(super) actor_kind: String,
    #[arg(long)]
    pub(super) user_authorized: bool,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ProposalRejectArgs {
    pub(super) proposal_id: String,
    #[arg(long, default_value = "local-user")]
    pub(super) actor: String,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct VersionsArgs {
    pub(super) memory_id: String,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub(super) limit: usize,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct TimelineArgs {
    pub(super) memory_id: String,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub(super) limit: usize,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct RollbackArgs {
    pub(super) memory_id: String,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long)]
    pub(super) target_version: i32,
    #[arg(long, default_value = "local-user")]
    pub(super) actor: String,
    #[arg(long)]
    pub(super) reason: String,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ProfilesListArgs {
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub(super) limit: usize,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ProfileUpsertArgs {
    #[arg(long)]
    pub(super) profile_id: Option<String>,
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long, default_value = "project")]
    pub(super) level: String,
    #[arg(long, default_value = "active")]
    pub(super) status: String,
    #[arg(long)]
    pub(super) name: String,
    #[arg(long)]
    pub(super) prompt_text: String,
    #[arg(long, value_delimiter = ',')]
    pub(super) focus_topics: Vec<String>,
    #[arg(long, value_delimiter = ',')]
    pub(super) prefer_memory_kinds: Vec<String>,
    #[arg(long, default_value = "local-user")]
    pub(super) created_by: String,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ProfileArchiveArgs {
    pub(super) profile_id: String,
    #[arg(long, default_value = "local-user")]
    pub(super) actor: String,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct DistillPreviewArgs {
    #[arg(long)]
    pub(super) scope_id: Option<String>,
    #[arg(long, conflicts_with = "file")]
    pub(super) input: Option<String>,
    #[arg(long)]
    pub(super) file: Option<PathBuf>,
    #[arg(long, value_delimiter = ',')]
    pub(super) evidence_refs: Vec<String>,
    #[arg(long)]
    pub(super) prompt_text: Option<String>,
    #[arg(long, value_delimiter = ',')]
    pub(super) focus_topics: Vec<String>,
    #[arg(long, value_delimiter = ',')]
    pub(super) prefer_memory_kinds: Vec<String>,
    #[arg(long)]
    pub(super) json: bool,
}
