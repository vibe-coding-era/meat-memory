use crate::{ApplyProjectDocumentSyncPlanRequest, Kernel, RememberTextRequest};
use anyhow::{Context, Result, bail};
use memory_domain::{
    DocumentSyncState, EvidenceId, EvidenceSpan, EvidenceSpanKind, EvidenceSpanLocation, Memory,
    MemoryKind, MemorySource, ProjectDocument, RequestContext, ScopeId, Sensitivity, SourceId,
    Visibility,
};
use memory_sync::{
    LocalProjectDocumentDraft, LocalProjectDocumentSyncEngine, LocalProjectDocumentSyncPlan,
    MissingProjectDocument, ProjectDocumentConflictInput, ProjectDocumentConflictReport,
    ProjectDocumentSnapshot, classify_project_document_conflict,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    io::{Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::Duration,
};
use time::OffsetDateTime;

const WEB_CRAWLER_REMOTE_FETCH_MAX_REQUESTS_PER_RUN: usize = 8;

const MEM0_FIXTURE: &str = r#"{
  "id": "mem0_fixture_001",
  "memory": "User prefers local-first memory with deterministic export.",
  "user_id": "usr_v295",
  "agent_id": "agent_codex",
  "metadata": {
    "kind": "preference",
    "source": "mem0-style-json"
  }
}"#;

const SUPERMEMORY_FIXTURE: &str = r#"{
  "id": "sm_doc_001",
  "container_id": "container_project_v295",
  "title": "Project memory container",
  "content": "Supermemory-style document content maps into project-scoped evidence-backed memory.",
  "metadata": {
    "kind": "summary",
    "source": "supermemory-style-document"
  }
}"#;

const MEMORYLAKE_FIXTURE: &str = r#"{
  "manifest": {
    "passport_id": "lake_passport_001",
    "source_scope_id": "scp_lake"
  },
  "memories": [
    {
      "id": "lake_mem_001",
      "title": "Portable provenance fact",
      "body": "MemoryLake-style passport data maps to Meat Memory passport import drafts.",
      "kind": "fact"
    }
  ]
}"#;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompetitorCapabilityMapping {
    pub competitor: String,
    pub external_concept: String,
    pub meat_memory_mapping: String,
    pub compatibility_level: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownProjectionCompatibility {
    pub legacy_projection: String,
    pub current_projection: String,
    pub compatibility_level: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorSkeleton {
    pub name: String,
    pub source_kind: String,
    pub capability: String,
    pub safe_default: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorDryRunRequest {
    pub connector: String,
    pub root_path: PathBuf,
    pub max_items: usize,
}

impl ConnectorDryRunRequest {
    pub fn new(connector: impl Into<String>, root_path: impl Into<PathBuf>) -> Self {
        Self {
            connector: connector.into(),
            root_path: root_path.into(),
            max_items: 50,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorDryRunItem {
    pub title: String,
    pub source_ref: String,
    pub content_bytes: u64,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorDryRunReport {
    pub schema_version: String,
    pub connector: String,
    pub root_path: PathBuf,
    pub mode: String,
    pub status: String,
    pub candidate_count: usize,
    pub items: Vec<ConnectorDryRunItem>,
    pub failures: Vec<String>,
    pub incremental_checkpoint: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorDryRunReportPaths {
    pub json: PathBuf,
    pub markdown: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorImportDraftRequest {
    pub connector: String,
    pub root_path: PathBuf,
    pub scope_id: ScopeId,
    pub max_items: usize,
    pub proposal_mode: bool,
}

impl ConnectorImportDraftRequest {
    pub fn new(
        connector: impl Into<String>,
        root_path: impl Into<PathBuf>,
        scope_id: ScopeId,
    ) -> Self {
        Self {
            connector: connector.into(),
            root_path: root_path.into(),
            scope_id,
            max_items: 100,
            proposal_mode: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectorImportDraft {
    pub connector: String,
    pub external_id: String,
    pub scope_id: ScopeId,
    pub title: String,
    pub body: String,
    pub memory_kind: MemoryKind,
    pub source_refs: Vec<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectorImportProposalDraft {
    pub connector: String,
    pub draft_external_id: String,
    pub scope_id: ScopeId,
    pub proposal_type: String,
    pub review_level: String,
    pub reason: String,
    pub evidence: Vec<String>,
    pub source_refs: Vec<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectorImportDraftReport {
    pub schema_version: String,
    pub connector: String,
    pub root_path: PathBuf,
    pub mode: String,
    pub draft_count: usize,
    pub drafts: Vec<ConnectorImportDraft>,
    pub proposal_draft_count: usize,
    pub proposal_drafts: Vec<ConnectorImportProposalDraft>,
    pub failures: Vec<String>,
    pub import_policy: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorImportDraftReportPaths {
    pub json: PathBuf,
    pub markdown: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorSyncPlanRequest {
    pub connector: String,
    pub root_path: PathBuf,
    pub scope_id: ScopeId,
    pub previous_snapshots: Vec<ProjectDocumentSnapshot>,
    pub excluded_dir_names: Vec<String>,
    pub max_items: usize,
    pub allow_remote_fetch: bool,
}

impl ConnectorSyncPlanRequest {
    pub fn new(
        connector: impl Into<String>,
        root_path: impl Into<PathBuf>,
        scope_id: ScopeId,
    ) -> Self {
        Self {
            connector: connector.into(),
            root_path: root_path.into(),
            scope_id,
            previous_snapshots: Vec::new(),
            excluded_dir_names: vec![
                "reports".to_string(),
                ".playwright-cli".to_string(),
                "target".to_string(),
            ],
            max_items: 500,
            allow_remote_fetch: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectorSyncPlanDocument {
    pub title: String,
    pub canonical_uri: String,
    pub local_path: PathBuf,
    pub content_hash: String,
    pub sync_state: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectorSyncPlanReport {
    pub schema_version: String,
    pub connector: String,
    pub root_path: PathBuf,
    pub mode: String,
    pub planned_count: usize,
    pub missing_count: usize,
    pub conflict_count: usize,
    pub documents: Vec<ConnectorSyncPlanDocument>,
    pub conflicts: Vec<ProjectDocumentConflictReport>,
    pub evidence_preview: Vec<EvidenceSpan>,
    pub incremental_checkpoint: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectorSyncPlanOutput {
    pub plan: LocalProjectDocumentSyncPlan,
    pub report: ConnectorSyncPlanReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorSyncPlanReportPaths {
    pub json: PathBuf,
    pub markdown: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorProposalQueueRequest {
    pub connector: String,
    pub root_path: PathBuf,
    pub scope_id: ScopeId,
    pub previous_snapshots: Vec<ProjectDocumentSnapshot>,
    pub max_items: usize,
}

impl ConnectorProposalQueueRequest {
    pub fn new(
        connector: impl Into<String>,
        root_path: impl Into<PathBuf>,
        scope_id: ScopeId,
    ) -> Self {
        Self {
            connector: connector.into(),
            root_path: root_path.into(),
            scope_id,
            previous_snapshots: Vec::new(),
            max_items: 100,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectorProposalQueueItem {
    pub queue_item_id: String,
    pub review_token: String,
    pub connector: String,
    pub external_id: String,
    pub scope_id: ScopeId,
    pub title: String,
    pub proposal_type: String,
    pub review_level: String,
    pub review_status: String,
    pub apply_target: String,
    pub blocked: bool,
    pub block_reason: Option<String>,
    pub reason: String,
    pub evidence: Vec<String>,
    pub source_refs: Vec<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectorProposalQueueReport {
    pub schema_version: String,
    pub queue_id: String,
    pub connector: String,
    pub root_path: PathBuf,
    pub mode: String,
    pub queue_item_count: usize,
    pub blocked_count: usize,
    pub queue_items: Vec<ConnectorProposalQueueItem>,
    pub failures: Vec<String>,
    pub source_summary: Value,
    pub queue_policy: Value,
    pub incremental_checkpoint: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorProposalQueueReportPaths {
    pub json: PathBuf,
    pub markdown: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorProposalApplyPlanRequest {
    pub connector: String,
    pub root_path: PathBuf,
    pub scope_id: ScopeId,
    pub approved_queue_item_ids: Vec<String>,
    pub confirmation_token: String,
    pub previous_snapshots: Vec<ProjectDocumentSnapshot>,
    pub max_items: usize,
}

impl ConnectorProposalApplyPlanRequest {
    pub fn new(
        connector: impl Into<String>,
        root_path: impl Into<PathBuf>,
        scope_id: ScopeId,
        approved_queue_item_ids: Vec<String>,
        confirmation_token: impl Into<String>,
    ) -> Self {
        Self {
            connector: connector.into(),
            root_path: root_path.into(),
            scope_id,
            approved_queue_item_ids,
            confirmation_token: confirmation_token.into(),
            previous_snapshots: Vec::new(),
            max_items: 100,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectorProposalApplyPlanItem {
    pub queue_item_id: String,
    pub title: String,
    pub proposal_type: String,
    pub apply_target: String,
    pub can_apply: bool,
    pub blocked: bool,
    pub block_reason: Option<String>,
    pub requires_executor: bool,
    pub source_refs: Vec<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectorProposalApplyPlanReport {
    pub schema_version: String,
    pub queue_id: String,
    pub connector: String,
    pub root_path: PathBuf,
    pub mode: String,
    pub selected_count: usize,
    pub applicable_count: usize,
    pub blocked_count: usize,
    pub skipped_count: usize,
    pub apply_items: Vec<ConnectorProposalApplyPlanItem>,
    pub skipped_queue_item_ids: Vec<String>,
    pub apply_policy: Value,
    pub queue_policy: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorProposalApplyPlanReportPaths {
    pub json: PathBuf,
    pub markdown: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ConnectorProposalApplyExecutorRequest {
    pub report: ConnectorProposalApplyPlanReport,
    pub scope_id: ScopeId,
    pub source_id: Option<SourceId>,
    pub max_items: usize,
    pub context: RequestContext,
}

impl ConnectorProposalApplyExecutorRequest {
    pub fn new(
        report: ConnectorProposalApplyPlanReport,
        scope_id: ScopeId,
        context: RequestContext,
    ) -> Self {
        Self {
            report,
            scope_id,
            source_id: None,
            max_items: 100,
            context,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectorProposalApplyExecutorResult {
    pub applied_memories: Vec<Memory>,
    pub applied_documents: Vec<ProjectDocument>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompetitorAdapterDraft {
    pub adapter: String,
    pub external_id: String,
    pub scope_id: ScopeId,
    pub title: String,
    pub body: String,
    pub memory_kind: MemoryKind,
    pub source_refs: Vec<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitorCompatibilityReport {
    pub schema_version: String,
    pub generated_at: OffsetDateTime,
    pub markdown_compatibility: Vec<MarkdownProjectionCompatibility>,
    pub mappings: Vec<CompetitorCapabilityMapping>,
    pub connector_skeletons: Vec<ConnectorSkeleton>,
    pub adapter_drafts: Vec<CompetitorAdapterDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompetitorCompatibilityReportPaths {
    pub json: PathBuf,
    pub markdown: PathBuf,
}

pub fn markdown_projection_compatibility() -> Vec<MarkdownProjectionCompatibility> {
    vec![
        MarkdownProjectionCompatibility {
            legacy_projection: "memory markdown frontmatter + body".to_string(),
            current_projection: "memory markdown plus V2.9 reports under tests/reports/*".to_string(),
            compatibility_level: "full_read".to_string(),
            notes: "V2.95 keeps old memory markdown readable and only adds report projections beside it."
                .to_string(),
        },
        MarkdownProjectionCompatibility {
            legacy_projection: "project document projection".to_string(),
            current_projection: "project document projection with optional evidence references".to_string(),
            compatibility_level: "additive".to_string(),
            notes: "Old docs projection remains valid; evidence metadata is optional and can be absent."
                .to_string(),
        },
    ]
}

pub fn competitor_capability_mappings() -> Vec<CompetitorCapabilityMapping> {
    vec![
        mapping(
            "Supermemory",
            "container / user / project memory",
            "ScopeId plus visibility and source_refs",
            "mapped",
            "Containers map to scopes; user/project separation stays explicit instead of SaaS-tenant hidden.",
        ),
        mapping(
            "Supermemory",
            "memory graph",
            "ContextBundle memories/entities/relations",
            "mapped",
            "Graph retrieval maps to existing entity and relation context payloads.",
        ),
        mapping(
            "Supermemory",
            "document RAG",
            "project documents, evidence spans, recall traces",
            "mapped",
            "Document chunks become source-backed memory plus traceable evidence.",
        ),
        mapping(
            "Supermemory",
            "connectors",
            "local connector skeletons",
            "skeleton",
            "V2.95 defines safe local connector shapes; external SaaS sync remains explicit.",
        ),
        mapping(
            "mem0",
            "user / session / agent / org memory",
            "ScopeId, AgentContext, Visibility",
            "mapped",
            "User and org map to scopes; session and agent memory use short-term AgentContext.",
        ),
        mapping(
            "mem0",
            "add / search / delete",
            "remember / search / lifecycle forget-delete",
            "mapped",
            "Core operations already exist across CLI, HTTP, and MCP.",
        ),
        mapping(
            "mem0",
            "OSS local runtime",
            "markdown-first local runtime with optional PostgreSQL",
            "superset",
            "Meat Memory keeps local markdown as a first-class store and can add PG without losing portability.",
        ),
        mapping(
            "MemoryLake",
            "passport",
            "Memory Passport manifest and bundle",
            "mapped",
            "V2.94 passport export, verify, import, and manifest inspection are reused.",
        ),
        mapping(
            "MemoryLake",
            "provenance",
            "EvidenceSpan and source_refs",
            "mapped",
            "Each imported draft keeps external source references for later evidence promotion.",
        ),
        mapping(
            "MemoryLake",
            "conflict / version / audit",
            "document conflicts, timeline, lifecycle audit",
            "mapped",
            "Existing V2.7/V2.8 governance primitives cover the compatibility contract.",
        ),
    ]
}

pub fn connector_skeletons() -> Vec<ConnectorSkeleton> {
    vec![
        ConnectorSkeleton {
            name: "local-git".to_string(),
            source_kind: "repository".to_string(),
            capability: "Read repository metadata, commits, and important docs as project context."
                .to_string(),
            safe_default: "read_only_no_remote_push".to_string(),
            status: "skeleton".to_string(),
        },
        ConnectorSkeleton {
            name: "markdown-docs".to_string(),
            source_kind: "document_tree".to_string(),
            capability: "Scan local markdown docs into project documents and evidence refs."
                .to_string(),
            safe_default: "local_files_only".to_string(),
            status: "skeleton".to_string(),
        },
        ConnectorSkeleton {
            name: "chat-export".to_string(),
            source_kind: "conversation_export".to_string(),
            capability: "Normalize chat exports into timestamped message memory drafts."
                .to_string(),
            safe_default: "explicit_import_only".to_string(),
            status: "skeleton".to_string(),
        },
        ConnectorSkeleton {
            name: "web-crawler".to_string(),
            source_kind: "web_snapshot".to_string(),
            capability:
                "Inspect local web page snapshots with canonical URL and allowlist metadata."
                    .to_string(),
            safe_default: "dry_run_only_no_remote_fetch".to_string(),
            status: "skeleton".to_string(),
        },
        ConnectorSkeleton {
            name: "notion".to_string(),
            source_kind: "notion_export".to_string(),
            capability:
                "Scan local Notion exports into page-shaped project document drafts with source refs."
                    .to_string(),
            safe_default: "local_export_only_no_remote_api".to_string(),
            status: "mvp".to_string(),
        },
        ConnectorSkeleton {
            name: "google-drive".to_string(),
            source_kind: "drive_export".to_string(),
            capability:
                "Scan Google Drive document/sheet/PDF export snapshots and sidecar text into sync plans."
                    .to_string(),
            safe_default: "local_export_only_no_remote_api".to_string(),
            status: "mvp".to_string(),
        },
        ConnectorSkeleton {
            name: "onedrive".to_string(),
            source_kind: "onedrive_export".to_string(),
            capability:
                "Scan OneDrive document/sheet/PDF export snapshots and sidecar text into sync plans."
                    .to_string(),
            safe_default: "local_export_only_no_remote_api".to_string(),
            status: "mvp".to_string(),
        },
    ]
}

pub fn run_connector_dry_run(request: ConnectorDryRunRequest) -> Result<ConnectorDryRunReport> {
    let root_path = request.root_path;
    if !root_path.exists() {
        bail!(
            "connector root path does not exist: {}",
            root_path.display()
        );
    }
    if !root_path.is_dir() {
        bail!(
            "connector root path is not a directory: {}",
            root_path.display()
        );
    }

    match request.connector.as_str() {
        "local-git" => local_git_dry_run(root_path, request.max_items),
        "markdown-docs" => markdown_docs_dry_run(root_path, request.max_items),
        "chat-export" => chat_export_dry_run(root_path, request.max_items),
        "web-crawler" => web_crawler_dry_run(root_path, request.max_items),
        "notion" => {
            external_docs_dry_run(root_path, request.max_items, external_connector("notion")?)
        }
        "google-drive" => external_docs_dry_run(
            root_path,
            request.max_items,
            external_connector("google-drive")?,
        ),
        "onedrive" => external_docs_dry_run(
            root_path,
            request.max_items,
            external_connector("onedrive")?,
        ),
        other => bail!("unsupported connector dry-run: {other}"),
    }
}

pub fn connector_dry_run_json(report: &ConnectorDryRunReport) -> Value {
    json!({
        "schema_version": report.schema_version,
        "connector": report.connector,
        "root_path": report.root_path,
        "mode": report.mode,
        "status": report.status,
        "candidate_count": report.candidate_count,
        "items": report.items,
        "failures": report.failures,
        "incremental_checkpoint": report.incremental_checkpoint,
        "coverage_gate": {
            "new_feature_test_coverage_required": "100%",
            "covered_regions": [
                "local_git_dry_run",
                "markdown_docs_dry_run",
                "yaml_frontmatter_compatibility",
                "chat_export_dry_run",
                "web_crawler_dry_run",
                "notion_export_dry_run",
                "drive_export_dry_run",
                "connector_report_projection",
                "cli_parser_and_command"
            ]
        }
    })
}

pub fn write_connector_dry_run_report(
    output_dir: &Path,
    report: &ConnectorDryRunReport,
) -> Result<ConnectorDryRunReportPaths> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;
    let json_path = output_dir.join(format!("{}-dry-run.json", report.connector));
    let markdown_path = output_dir.join(format!("{}-dry-run.md", report.connector));

    fs::write(
        &json_path,
        serde_json::to_string_pretty(&connector_dry_run_json(report))?,
    )
    .with_context(|| format!("failed to write {}", json_path.display()))?;
    fs::write(&markdown_path, render_connector_dry_run_markdown(report))
        .with_context(|| format!("failed to write {}", markdown_path.display()))?;

    Ok(ConnectorDryRunReportPaths {
        json: json_path,
        markdown: markdown_path,
    })
}

pub fn build_connector_import_draft_report(
    request: ConnectorImportDraftRequest,
) -> Result<ConnectorImportDraftReport> {
    let root_path = request.root_path;
    if !root_path.exists() {
        bail!(
            "connector root path does not exist: {}",
            root_path.display()
        );
    }
    if !root_path.is_dir() {
        bail!(
            "connector root path is not a directory: {}",
            root_path.display()
        );
    }
    if request.connector != "chat-export" {
        bail!(
            "unsupported connector import draft: {}; only chat-export is implemented",
            request.connector
        );
    }

    let proposal_mode = request.proposal_mode;
    let (mut drafts, failures) =
        chat_export_import_drafts(&root_path, &request.scope_id, request.max_items)?;
    drafts.truncate(request.max_items);
    let proposal_drafts = if proposal_mode {
        drafts
            .iter()
            .map(connector_import_proposal_draft)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    Ok(ConnectorImportDraftReport {
        schema_version: "2.97-A".to_string(),
        connector: "chat-export".to_string(),
        root_path,
        mode: "import_draft".to_string(),
        draft_count: drafts.len(),
        drafts,
        proposal_draft_count: proposal_drafts.len(),
        proposal_drafts,
        failures,
        import_policy: json!({
            "safe_default": "explicit_import_only",
            "writes_memory": false,
            "proposal_mode": proposal_mode,
            "review_required_before_apply": true,
            "target": if proposal_mode { "governance_proposal_draft" } else { "memory_draft" },
        }),
    })
}

pub fn connector_import_draft_json(report: &ConnectorImportDraftReport) -> Value {
    json!({
        "schema_version": report.schema_version,
        "connector": report.connector,
        "root_path": report.root_path,
        "mode": report.mode,
        "draft_count": report.draft_count,
        "drafts": report.drafts,
        "proposal_draft_count": report.proposal_draft_count,
        "proposal_drafts": report.proposal_drafts,
        "failures": report.failures,
        "import_policy": report.import_policy,
        "coverage_gate": {
            "new_feature_test_coverage_required": "100%",
            "covered_regions": [
                "chat_export_parser",
                "chat_export_import_draft_projection",
                "chat_export_import_proposal_projection",
                "connector_import_draft_report",
                "cli_parser_and_command"
            ]
        }
    })
}

pub fn write_connector_import_draft_report(
    output_dir: &Path,
    report: &ConnectorImportDraftReport,
) -> Result<ConnectorImportDraftReportPaths> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;
    let json_path = output_dir.join(format!("{}-import-draft.json", report.connector));
    let markdown_path = output_dir.join(format!("{}-import-draft.md", report.connector));

    fs::write(
        &json_path,
        serde_json::to_string_pretty(&connector_import_draft_json(report))?,
    )
    .with_context(|| format!("failed to write {}", json_path.display()))?;
    fs::write(
        &markdown_path,
        render_connector_import_draft_markdown(report),
    )
    .with_context(|| format!("failed to write {}", markdown_path.display()))?;

    Ok(ConnectorImportDraftReportPaths {
        json: json_path,
        markdown: markdown_path,
    })
}

pub fn build_connector_sync_plan(
    request: ConnectorSyncPlanRequest,
) -> Result<ConnectorSyncPlanOutput> {
    let root_path = request.root_path.clone();
    if !root_path.exists() {
        bail!(
            "connector root path does not exist: {}",
            root_path.display()
        );
    }
    if !root_path.is_dir() {
        bail!(
            "connector root path is not a directory: {}",
            root_path.display()
        );
    }

    match request.connector.as_str() {
        "markdown-docs" | "local-git" => build_file_connector_sync_plan(request),
        "web-crawler" => build_web_crawler_sync_plan(request),
        "notion" => build_external_docs_sync_plan(request, external_connector("notion")?),
        "google-drive" => {
            build_external_docs_sync_plan(request, external_connector("google-drive")?)
        }
        "onedrive" => build_external_docs_sync_plan(request, external_connector("onedrive")?),
        other => bail!(
            "unsupported connector sync plan: {other}; supported connectors are markdown-docs, local-git, web-crawler, notion, google-drive, and onedrive"
        ),
    }
}

fn build_file_connector_sync_plan(
    request: ConnectorSyncPlanRequest,
) -> Result<ConnectorSyncPlanOutput> {
    let root_path = request.root_path.clone();
    let connector = request.connector.clone();
    let checkpoint = connector_sync_checkpoint(&connector, &root_path);
    let mut plan = LocalProjectDocumentSyncEngine::with_extensions(root_path, ["md", "markdown"])
        .with_excluded_dir_names(request.excluded_dir_names)
        .scan(&request.previous_snapshots)
        .context("failed to build connector sync plan")?;
    if plan.documents.len() > request.max_items {
        plan.documents.truncate(request.max_items);
    }
    for document in &mut plan.documents {
        document.title = markdown_document_title(&document.local_path, &document.content_text);
    }

    let documents = plan
        .documents
        .iter()
        .map(|document| ConnectorSyncPlanDocument {
            title: document.title.clone(),
            canonical_uri: document.canonical_uri.clone(),
            local_path: document.local_path.clone(),
            content_hash: document.content_hash.clone(),
            sync_state: document.sync_state.as_str().to_string(),
            metadata: connector_sync_plan_document_metadata(document),
        })
        .collect::<Vec<_>>();
    let evidence_preview = plan
        .documents
        .iter()
        .map(|document| connector_document_evidence_span(&request.scope_id, document))
        .collect::<Vec<_>>();

    let report = ConnectorSyncPlanReport {
        schema_version: "2.97-A".to_string(),
        connector,
        root_path: plan.root.clone(),
        mode: "sync_plan".to_string(),
        planned_count: plan.documents.len(),
        missing_count: plan.missing.len(),
        conflict_count: plan.conflicts.len(),
        documents,
        conflicts: plan.conflicts.clone(),
        evidence_preview,
        incremental_checkpoint: checkpoint,
    };

    Ok(ConnectorSyncPlanOutput { plan, report })
}

fn build_external_docs_sync_plan(
    request: ConnectorSyncPlanRequest,
    config: ExternalConnectorConfig,
) -> Result<ConnectorSyncPlanOutput> {
    let root = request.root_path.canonicalize()?;
    let previous_by_uri = request
        .previous_snapshots
        .iter()
        .map(|snapshot| {
            (
                snapshot.canonical_uri.clone(),
                snapshot.content_hash.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let (candidates, failures) = external_document_candidates(&root, request.max_items, config)?;
    let mut seen = BTreeSet::new();
    let documents = candidates
        .into_iter()
        .map(|candidate| {
            seen.insert(candidate.canonical_uri.clone());
            let sync_state = match previous_by_uri.get(&candidate.canonical_uri) {
                Some(previous_hash) if previous_hash == &candidate.content_hash => {
                    DocumentSyncState::Clean
                }
                Some(_) => DocumentSyncState::Changed,
                None => DocumentSyncState::Changed,
            };
            LocalProjectDocumentDraft {
                canonical_uri: candidate.canonical_uri,
                local_path: candidate.local_path,
                title: candidate.title,
                content_text: candidate.content_text,
                content_hash: candidate.content_hash,
                sync_state,
                metadata: candidate.metadata,
            }
        })
        .collect::<Vec<_>>();
    let missing = request
        .previous_snapshots
        .iter()
        .filter(|snapshot| !seen.contains(&snapshot.canonical_uri))
        .map(|snapshot| MissingProjectDocument {
            canonical_uri: snapshot.canonical_uri.clone(),
            sync_state: DocumentSyncState::Missing,
        })
        .collect::<Vec<_>>();
    let conflicts =
        connector_sync_conflict_reports(&request.previous_snapshots, &documents, &missing);
    let documents_report = documents
        .iter()
        .map(|document| ConnectorSyncPlanDocument {
            title: document.title.clone(),
            canonical_uri: document.canonical_uri.clone(),
            local_path: document.local_path.clone(),
            content_hash: document.content_hash.clone(),
            sync_state: document.sync_state.as_str().to_string(),
            metadata: connector_sync_plan_document_metadata(document),
        })
        .collect::<Vec<_>>();
    let evidence_preview = documents
        .iter()
        .map(|document| connector_document_evidence_span(&request.scope_id, document))
        .collect::<Vec<_>>();
    let checkpoint = json!({
        "strategy": "export_path_content_hash",
        "apply_target": "Kernel::apply_project_document_sync_plan",
        "source_kind": config.source_kind,
        "safe_default": config.safe_default,
        "remote_network": false,
        "supported_extensions": config.extensions,
        "failure_count": failures.len(),
        "failures": failures,
    });
    let planned_count = documents.len();
    let plan = LocalProjectDocumentSyncPlan {
        root: root.clone(),
        documents,
        missing,
        conflicts: conflicts.clone(),
    };
    let report = ConnectorSyncPlanReport {
        schema_version: "2.97-A".to_string(),
        connector: config.connector.to_string(),
        root_path: root,
        mode: "sync_plan".to_string(),
        planned_count,
        missing_count: plan.missing.len(),
        conflict_count: plan.conflicts.len(),
        documents: documents_report,
        conflicts,
        evidence_preview,
        incremental_checkpoint: checkpoint,
    };

    Ok(ConnectorSyncPlanOutput { plan, report })
}

fn build_web_crawler_sync_plan(
    request: ConnectorSyncPlanRequest,
) -> Result<ConnectorSyncPlanOutput> {
    let root = request.root_path.canonicalize()?;
    let allowlist_domains = web_allowlist_domains(&root)?;
    let mut files = Vec::new();
    let excluded_dir_names = request
        .excluded_dir_names
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    collect_web_page_files_with_excluded(&root, &mut files, &excluded_dir_names)?;
    files.sort();

    let previous_by_uri = request
        .previous_snapshots
        .iter()
        .map(|snapshot| {
            (
                snapshot.canonical_uri.clone(),
                snapshot.content_hash.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    let mut documents = Vec::new();
    let mut blocked_urls = Vec::new();
    let mut fetch_failures = Vec::new();
    let mut not_modified_urls = Vec::new();
    let mut redirect_chains = Vec::new();
    let url_manifest = web_url_manifest(&root)?;
    let url_validators = web_url_validators(&root)?;

    for path in files {
        if documents.len() >= request.max_items {
            break;
        }
        let parsed = web_page_sync_candidate(&root, &path, &allowlist_domains)?;
        seen.insert(parsed.canonical_uri.clone());
        if !parsed.allowlist_allowed {
            blocked_urls.push(json!({
                "canonical_url": parsed.canonical_uri,
                "relative_path": parsed.relative_path,
            }));
            continue;
        }
        let content_hash = memory_domain::Artifact::compute_content_hash(&parsed.content_text);
        let sync_state = match previous_by_uri.get(&parsed.canonical_uri) {
            Some(previous_hash) if previous_hash == &content_hash => DocumentSyncState::Clean,
            Some(_) => DocumentSyncState::Changed,
            None => DocumentSyncState::Changed,
        };
        documents.push(LocalProjectDocumentDraft {
            canonical_uri: parsed.canonical_uri,
            local_path: path,
            title: parsed.title,
            content_text: parsed.content_text,
            content_hash,
            sync_state,
            metadata: parsed.metadata,
        });
    }

    let mut discovered_remote_urls = Vec::new();
    let mut remote_queue = VecDeque::new();
    let mut queued_remote_urls = BTreeSet::new();
    for url in &url_manifest {
        if queued_remote_urls.insert(url.clone()) {
            remote_queue.push_back(url.clone());
        }
    }
    let mut remote_fetch_budget =
        WebRemoteFetchBudget::new(WEB_CRAWLER_REMOTE_FETCH_MAX_REQUESTS_PER_RUN);
    let mut remote_fetch_rate_limited = false;

    if request.allow_remote_fetch {
        while documents.len() < request.max_items {
            if remote_fetch_budget.is_exhausted() {
                remote_fetch_rate_limited = !remote_queue.is_empty();
                break;
            }
            let Some(url) = remote_queue.pop_front() else {
                break;
            };
            match web_page_remote_sync_candidate(
                &root,
                &url,
                &allowlist_domains,
                &url_validators,
                &mut remote_fetch_budget,
            ) {
                Ok(WebRemoteFetchOutcome::Candidate(parsed)) => {
                    seen.insert(parsed.canonical_uri.clone());
                    if !parsed.allowlist_allowed {
                        blocked_urls.push(json!({
                            "canonical_url": parsed.canonical_uri,
                            "relative_path": parsed.relative_path,
                        }));
                        continue;
                    }
                    let links_to_queue = web_metadata_in_scope_links(&parsed.metadata);
                    if parsed.metadata["redirect_count"].as_u64().unwrap_or(0) > 0 {
                        redirect_chains.push(json!({
                            "canonical_url": parsed.canonical_uri,
                            "redirects": parsed.metadata["redirects"].clone(),
                        }));
                    }
                    let content_hash =
                        memory_domain::Artifact::compute_content_hash(&parsed.content_text);
                    let sync_state = match previous_by_uri.get(&parsed.canonical_uri) {
                        Some(previous_hash) if previous_hash == &content_hash => {
                            DocumentSyncState::Clean
                        }
                        Some(_) => DocumentSyncState::Changed,
                        None => DocumentSyncState::Changed,
                    };
                    documents.push(LocalProjectDocumentDraft {
                        canonical_uri: parsed.canonical_uri,
                        local_path: root.join("urls.txt"),
                        title: parsed.title,
                        content_text: parsed.content_text,
                        content_hash,
                        sync_state,
                        metadata: parsed.metadata,
                    });
                    for discovered_url in links_to_queue {
                        if seen.contains(&discovered_url)
                            || !queued_remote_urls.insert(discovered_url.clone())
                        {
                            continue;
                        }
                        remote_queue.push_back(discovered_url.clone());
                        discovered_remote_urls.push(discovered_url);
                    }
                }
                Ok(WebRemoteFetchOutcome::NotModified {
                    canonical_uri,
                    metadata,
                }) => {
                    if metadata["redirect_count"].as_u64().unwrap_or(0) > 0 {
                        redirect_chains.push(json!({
                            "canonical_url": canonical_uri,
                            "redirects": metadata["redirects"].clone(),
                        }));
                    }
                    seen.insert(canonical_uri);
                    not_modified_urls.push(metadata);
                }
                Err(error) => {
                    let error = error.to_string();
                    let rate_limited = error.contains("remote fetch rate limit exceeded");
                    fetch_failures.push(json!({
                        "url": url,
                        "error": error,
                        "rate_limited": rate_limited,
                    }));
                    if rate_limited {
                        remote_queue.push_front(url);
                        remote_fetch_rate_limited = true;
                        break;
                    }
                }
            }
        }
    }

    let missing = request
        .previous_snapshots
        .iter()
        .filter(|snapshot| !seen.contains(&snapshot.canonical_uri))
        .map(|snapshot| MissingProjectDocument {
            canonical_uri: snapshot.canonical_uri.clone(),
            sync_state: DocumentSyncState::Missing,
        })
        .collect::<Vec<_>>();
    let conflicts =
        connector_sync_conflict_reports(&request.previous_snapshots, &documents, &missing);

    let documents_report = documents
        .iter()
        .map(|document| ConnectorSyncPlanDocument {
            title: document.title.clone(),
            canonical_uri: document.canonical_uri.clone(),
            local_path: document.local_path.clone(),
            content_hash: document.content_hash.clone(),
            sync_state: document.sync_state.as_str().to_string(),
            metadata: connector_sync_plan_document_metadata(document),
        })
        .collect::<Vec<_>>();
    let evidence_preview = documents
        .iter()
        .map(|document| connector_document_evidence_span(&request.scope_id, document))
        .collect::<Vec<_>>();

    let crawl_next_frontier_urls = remote_queue.iter().cloned().collect::<Vec<_>>();
    let checkpoint = json!({
        "strategy": "canonical_uri_content_hash",
        "apply_target": "Kernel::apply_project_document_sync_plan",
        "source_kind": "web_page",
        "allowlist_domains": allowlist_domains,
        "blocked_count": blocked_urls.len(),
        "blocked_urls": blocked_urls,
        "fetch_failures": fetch_failures,
        "not_modified_count": not_modified_urls.len(),
        "not_modified_urls": not_modified_urls,
        "redirect_count": redirect_chains.len(),
        "redirects": redirect_chains,
        "remote_fetch_allowed": request.allow_remote_fetch,
        "remote_network": request.allow_remote_fetch,
        "remote_fetch_rate_limit_policy": "max_requests_per_sync_plan",
        "remote_fetch_max_requests_per_run": remote_fetch_budget.max_requests,
        "remote_fetch_attempted_requests": remote_fetch_budget.attempted_requests,
        "remote_fetch_remaining_requests": remote_fetch_budget.remaining_requests(),
        "remote_fetch_rate_limited": remote_fetch_rate_limited,
        "safe_default": "local_snapshot_only_no_remote_fetch",
        "crawl_policy": "urls_txt_seed_plus_in_scope_links",
        "crawl_seed_count": url_manifest.len(),
        "crawl_discovered_count": discovered_remote_urls.len(),
        "crawl_discovered_urls": discovered_remote_urls,
        "crawl_queue_remaining": crawl_next_frontier_urls.len(),
        "crawl_next_frontier_urls": crawl_next_frontier_urls,
        "url_manifest": "urls.txt",
        "url_manifest_count": url_manifest.len(),
        "validator_manifest": "url-validators.json",
        "validator_manifest_count": url_validators.len(),
        "update_detection": ["canonical_url", "content_hash", "etag", "last_modified"],
    });
    let planned_count = documents.len();
    let plan = LocalProjectDocumentSyncPlan {
        root: root.clone(),
        documents,
        missing,
        conflicts: conflicts.clone(),
    };
    let report = ConnectorSyncPlanReport {
        schema_version: "2.97-A".to_string(),
        connector: "web-crawler".to_string(),
        root_path: root,
        mode: "sync_plan".to_string(),
        planned_count,
        missing_count: plan.missing.len(),
        conflict_count: plan.conflicts.len(),
        documents: documents_report,
        conflicts,
        evidence_preview,
        incremental_checkpoint: checkpoint,
    };

    Ok(ConnectorSyncPlanOutput { plan, report })
}

pub fn connector_sync_plan_json(report: &ConnectorSyncPlanReport) -> Value {
    json!({
        "schema_version": report.schema_version,
        "connector": report.connector,
        "root_path": report.root_path,
        "mode": report.mode,
        "planned_count": report.planned_count,
        "missing_count": report.missing_count,
        "conflict_count": report.conflict_count,
        "documents": report.documents,
        "conflicts": report.conflicts,
        "evidence_preview": report.evidence_preview,
        "incremental_checkpoint": report.incremental_checkpoint,
        "coverage_gate": {
            "new_feature_test_coverage_required": "100%",
            "covered_regions": [
                "markdown_docs_sync_plan",
                "frontmatter_metadata_persistence",
                "yaml_frontmatter_compatibility",
                "sync_plan_conflict_review_projection",
                "sync_plan_evidence_preview",
                "web_crawler_sync_plan",
                "web_crawler_remote_fetch_policy",
                "web_crawler_redirect_conditional_request",
                "web_crawler_robots_rule_precedence",
                "web_crawler_link_boundary",
                "web_crawler_multi_page_crawl",
                "web_crawler_https_url_policy",
                "web_crawler_remote_fetch_rate_limit",
                "notion_export_sync_plan",
                "drive_export_sync_plan",
                "connector_sync_plan_projection",
                "cli_parser_and_command"
            ]
        }
    })
}

pub fn write_connector_sync_plan_report(
    output_dir: &Path,
    report: &ConnectorSyncPlanReport,
) -> Result<ConnectorSyncPlanReportPaths> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;
    let json_path = output_dir.join(format!("{}-sync-plan.json", report.connector));
    let markdown_path = output_dir.join(format!("{}-sync-plan.md", report.connector));

    fs::write(
        &json_path,
        serde_json::to_string_pretty(&connector_sync_plan_json(report))?,
    )
    .with_context(|| format!("failed to write {}", json_path.display()))?;
    fs::write(&markdown_path, render_connector_sync_plan_markdown(report))
        .with_context(|| format!("failed to write {}", markdown_path.display()))?;

    Ok(ConnectorSyncPlanReportPaths {
        json: json_path,
        markdown: markdown_path,
    })
}

pub fn build_connector_proposal_queue_report(
    request: ConnectorProposalQueueRequest,
) -> Result<ConnectorProposalQueueReport> {
    match request.connector.as_str() {
        "chat-export" => build_import_proposal_queue_report(request),
        "markdown-docs" | "local-git" => build_sync_proposal_queue_report(request),
        other => bail!(
            "unsupported connector proposal queue: {other}; supported connectors are markdown-docs, local-git, and chat-export"
        ),
    }
}

pub fn connector_proposal_queue_json(report: &ConnectorProposalQueueReport) -> Value {
    json!({
        "schema_version": report.schema_version,
        "queue_id": report.queue_id,
        "connector": report.connector,
        "root_path": report.root_path,
        "mode": report.mode,
        "queue_item_count": report.queue_item_count,
        "blocked_count": report.blocked_count,
        "queue_items": report.queue_items,
        "failures": report.failures,
        "source_summary": report.source_summary,
        "queue_policy": report.queue_policy,
        "incremental_checkpoint": report.incremental_checkpoint,
        "coverage_gate": {
            "new_feature_test_coverage_required": "100%",
            "covered_regions": [
                "connector_proposal_queue_projection",
                "markdown_docs_proposal_queue",
                "local_git_proposal_queue",
                "chat_export_proposal_queue",
                "service_surface_projection"
            ]
        }
    })
}

pub fn connector_proposal_confirmation_token(
    queue_id: &str,
    approved_queue_item_ids: &[String],
) -> String {
    let mut ids = approved_queue_item_ids.to_vec();
    ids.sort();
    format!(
        "confirm_{}",
        stable_hex_hash(&format!("{queue_id}|{}", ids.join(",")))
    )
}

pub fn write_connector_proposal_queue_report(
    output_dir: &Path,
    report: &ConnectorProposalQueueReport,
) -> Result<ConnectorProposalQueueReportPaths> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;
    let json_path = output_dir.join(format!("{}-proposal-queue.json", report.connector));
    let markdown_path = output_dir.join(format!("{}-proposal-queue.md", report.connector));

    fs::write(
        &json_path,
        serde_json::to_string_pretty(&connector_proposal_queue_json(report))?,
    )
    .with_context(|| format!("failed to write {}", json_path.display()))?;
    fs::write(
        &markdown_path,
        render_connector_proposal_queue_markdown(report),
    )
    .with_context(|| format!("failed to write {}", markdown_path.display()))?;

    Ok(ConnectorProposalQueueReportPaths {
        json: json_path,
        markdown: markdown_path,
    })
}

pub fn build_connector_proposal_apply_plan_report(
    request: ConnectorProposalApplyPlanRequest,
) -> Result<ConnectorProposalApplyPlanReport> {
    let mut queue_request =
        ConnectorProposalQueueRequest::new(request.connector, request.root_path, request.scope_id);
    queue_request.max_items = request.max_items;
    queue_request.previous_snapshots = request.previous_snapshots;
    let queue = build_connector_proposal_queue_report(queue_request)?;
    build_connector_proposal_apply_plan_report_from_queue(
        queue,
        request.approved_queue_item_ids,
        request.confirmation_token,
    )
}

pub fn build_connector_proposal_apply_plan_report_from_queue(
    queue: ConnectorProposalQueueReport,
    approved_queue_item_ids: Vec<String>,
    confirmation_token: impl AsRef<str>,
) -> Result<ConnectorProposalApplyPlanReport> {
    if approved_queue_item_ids.is_empty() {
        bail!("approved_queue_item_ids is required for connector proposal apply-plan");
    }

    let expected_token =
        connector_proposal_confirmation_token(&queue.queue_id, &approved_queue_item_ids);
    if confirmation_token.as_ref() != expected_token {
        bail!("connector proposal apply-plan confirmation token mismatch");
    }

    let by_id = queue
        .queue_items
        .iter()
        .map(|item| (item.queue_item_id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let mut apply_items = Vec::new();
    let mut skipped_queue_item_ids = Vec::new();
    for queue_item_id in &approved_queue_item_ids {
        let Some(item) = by_id.get(queue_item_id.as_str()) else {
            bail!("unknown connector proposal queue item: {queue_item_id}");
        };
        if item.review_status != "open" {
            skipped_queue_item_ids.push(queue_item_id.clone());
            continue;
        }
        let can_apply = !item.blocked;
        apply_items.push(ConnectorProposalApplyPlanItem {
            queue_item_id: item.queue_item_id.clone(),
            title: item.title.clone(),
            proposal_type: item.proposal_type.clone(),
            apply_target: item.apply_target.clone(),
            can_apply,
            blocked: item.blocked,
            block_reason: item.block_reason.clone(),
            requires_executor: true,
            source_refs: item.source_refs.clone(),
            metadata: item.metadata.clone(),
        });
    }

    let applicable_count = apply_items.iter().filter(|item| item.can_apply).count();
    let blocked_count = apply_items.iter().filter(|item| item.blocked).count();
    Ok(ConnectorProposalApplyPlanReport {
        schema_version: queue.schema_version,
        queue_id: queue.queue_id,
        connector: queue.connector,
        root_path: queue.root_path,
        mode: "proposal_apply_plan".to_string(),
        selected_count: approved_queue_item_ids.len(),
        applicable_count,
        blocked_count,
        skipped_count: skipped_queue_item_ids.len(),
        apply_items,
        skipped_queue_item_ids,
        apply_policy: json!({
            "safe_default": "plan_only",
            "writes_memory": false,
            "writes_project_documents": false,
            "requires_confirmation_token": true,
            "requires_runtime_key_for_executor": true,
            "executor_not_invoked": true,
        }),
        queue_policy: queue.queue_policy,
    })
}

pub fn connector_proposal_apply_plan_json(report: &ConnectorProposalApplyPlanReport) -> Value {
    json!({
        "schema_version": report.schema_version,
        "queue_id": report.queue_id,
        "connector": report.connector,
        "root_path": report.root_path,
        "mode": report.mode,
        "selected_count": report.selected_count,
        "applicable_count": report.applicable_count,
        "blocked_count": report.blocked_count,
        "skipped_count": report.skipped_count,
        "apply_items": report.apply_items,
        "skipped_queue_item_ids": report.skipped_queue_item_ids,
        "apply_policy": report.apply_policy,
        "queue_policy": report.queue_policy,
        "coverage_gate": {
            "new_feature_test_coverage_required": "100%",
            "covered_regions": [
                "connector_proposal_apply_plan",
                "connector_queue_confirmation_token",
                "connector_persistent_queue_manifest",
                "service_side_confirmed_apply_plan",
                "service_side_confirmed_executor"
            ]
        }
    })
}

pub fn connector_proposal_apply_plan_execution_json(
    report: &ConnectorProposalApplyPlanReport,
    result: &ConnectorProposalApplyExecutorResult,
) -> Value {
    let mut value = connector_proposal_apply_plan_json(report);
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "execution".to_string(),
            json!({
                "requested": true,
                "executor_invoked": true,
                "applied_memory_count": result.applied_memories.len(),
                "applied_project_document_count": result.applied_documents.len(),
                "applied_memory_ids": result
                    .applied_memories
                    .iter()
                    .map(|memory| memory.id.as_str())
                    .collect::<Vec<_>>(),
                "applied_project_document_ids": result
                    .applied_documents
                    .iter()
                    .map(|document| document.id.as_str())
                    .collect::<Vec<_>>(),
                "writes_memory": !result.applied_memories.is_empty(),
                "writes_project_documents": !result.applied_documents.is_empty(),
                "requires_runtime_key": true,
            }),
        );
    }
    value
}

pub fn write_connector_proposal_apply_plan_report(
    output_dir: &Path,
    report: &ConnectorProposalApplyPlanReport,
) -> Result<ConnectorProposalApplyPlanReportPaths> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;
    let json_path = output_dir.join(format!("{}-proposal-apply-plan.json", report.connector));
    let markdown_path = output_dir.join(format!("{}-proposal-apply-plan.md", report.connector));

    fs::write(
        &json_path,
        serde_json::to_string_pretty(&connector_proposal_apply_plan_json(report))?,
    )
    .with_context(|| format!("failed to write {}", json_path.display()))?;
    fs::write(
        &markdown_path,
        render_connector_proposal_apply_plan_markdown(report),
    )
    .with_context(|| format!("failed to write {}", markdown_path.display()))?;

    Ok(ConnectorProposalApplyPlanReportPaths {
        json: json_path,
        markdown: markdown_path,
    })
}

pub async fn apply_connector_proposal_apply_plan(
    kernel: &Kernel,
    request: ConnectorProposalApplyExecutorRequest,
) -> Result<ConnectorProposalApplyExecutorResult> {
    if request.context.owner_scope_id != request.scope_id {
        bail!("scope access forbidden for current meat memory key");
    }
    if request.report.blocked_count > 0 {
        bail!("connector proposal apply-plan contains blocked queue items");
    }

    let mut applied_memories = Vec::new();
    let mut applied_documents = Vec::new();
    let connector = request.report.connector.clone();
    let root_path = request.report.root_path.clone();
    match request.report.connector.as_str() {
        "chat-export" => {
            let mut draft_request =
                ConnectorImportDraftRequest::new("chat-export", root_path, request.scope_id);
            draft_request.max_items = request.max_items;
            let import_report = build_connector_import_draft_report(draft_request)?;
            for item in request
                .report
                .apply_items
                .iter()
                .filter(|item| item.can_apply)
            {
                let draft = import_report
                    .drafts
                    .iter()
                    .find(|draft| draft.source_refs == item.source_refs)
                    .with_context(|| {
                        format!(
                            "approved queue item {} no longer matches a chat export draft",
                            item.queue_item_id
                        )
                    })?;
                let mut remember = RememberTextRequest::new(draft.scope_id.clone(), &draft.body);
                remember.title = Some(draft.title.clone());
                remember.memory_kind = Some(draft.memory_kind);
                remember.source_refs = draft.source_refs.clone();
                remember.visibility = Visibility::Private;
                remember.sensitivity = Sensitivity::Internal;
                remember.context = Some(request.context.clone());
                applied_memories.push(kernel.remember_text(remember).await?.memory);
            }
        }
        "markdown-docs" | "local-git" => {
            let source = resolve_connector_source_for_apply(
                kernel,
                &request.context,
                request.source_id.as_ref(),
                &root_path,
            )
            .await?;
            let approved_refs = request
                .report
                .apply_items
                .iter()
                .filter(|item| item.can_apply)
                .flat_map(|item| item.source_refs.iter().cloned())
                .collect::<BTreeSet<_>>();
            let mut sync_request =
                ConnectorSyncPlanRequest::new(connector, root_path, request.scope_id.clone());
            sync_request.max_items = request.max_items;
            let output = build_connector_sync_plan(sync_request)?;
            let mut apply_plan = output.plan;
            apply_plan
                .documents
                .retain(|document| approved_refs.contains(&document.canonical_uri));
            let result = kernel
                .apply_project_document_sync_plan(ApplyProjectDocumentSyncPlanRequest {
                    source_id: source.id,
                    scope_id: request.scope_id,
                    plan: apply_plan,
                    context: Some(request.context),
                })
                .await?;
            applied_documents = result.imported;
        }
        other => bail!("unsupported connector proposal apply executor: {other}"),
    }

    Ok(ConnectorProposalApplyExecutorResult {
        applied_memories,
        applied_documents,
    })
}

async fn resolve_connector_source_for_apply(
    kernel: &Kernel,
    context: &RequestContext,
    source_id: Option<&SourceId>,
    root_path: &Path,
) -> Result<MemorySource> {
    if let Some(source_id) = source_id {
        let source = kernel
            .get_memory_source(source_id.clone())
            .await?
            .with_context(|| format!("memory source not found: {}", source_id.as_str()))?;
        if source.owner_scope_id != context.owner_scope_id {
            bail!("source access forbidden for current meat memory key");
        }
        return Ok(source);
    }

    let root = root_path
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", root_path.display()))?;
    let sources = kernel
        .list_memory_sources(context.owner_scope_id.clone(), 100, Some(context))
        .await?;
    let mut matches = sources
        .into_iter()
        .filter(|source| {
            source
                .local_root
                .as_deref()
                .and_then(|local_root| Path::new(local_root).canonicalize().ok())
                .map(|local_root| local_root == root)
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => bail!(
            "source_id is required when apply is set and no source local_root matches {}",
            root.display()
        ),
        _ => bail!(
            "source_id is required when apply is set because multiple sources match {}",
            root.display()
        ),
    }
}

fn build_import_proposal_queue_report(
    request: ConnectorProposalQueueRequest,
) -> Result<ConnectorProposalQueueReport> {
    let mut draft_request = ConnectorImportDraftRequest::new(
        request.connector,
        request.root_path,
        request.scope_id.clone(),
    );
    draft_request.max_items = request.max_items;
    draft_request.proposal_mode = true;
    let report = build_connector_import_draft_report(draft_request)?;
    let queue_items = report
        .proposal_drafts
        .iter()
        .enumerate()
        .map(|(index, proposal)| import_proposal_queue_item(index, proposal))
        .collect::<Vec<_>>();

    Ok(connector_proposal_queue_report(
        report.connector,
        report.root_path,
        queue_items,
        report.failures,
        json!({
            "upstream_mode": report.mode,
            "draft_count": report.draft_count,
            "proposal_draft_count": report.proposal_draft_count,
        }),
        connector_import_queue_checkpoint(&report.drafts),
    ))
}

fn build_sync_proposal_queue_report(
    request: ConnectorProposalQueueRequest,
) -> Result<ConnectorProposalQueueReport> {
    let mut sync_request = ConnectorSyncPlanRequest::new(
        request.connector,
        request.root_path,
        request.scope_id.clone(),
    );
    sync_request.previous_snapshots = request.previous_snapshots;
    sync_request.max_items = request.max_items;
    let output = build_connector_sync_plan(sync_request)?;
    let mut queue_items = output
        .report
        .documents
        .iter()
        .enumerate()
        .filter(|(_, document)| document.sync_state != "clean")
        .map(|(index, document)| {
            sync_document_proposal_queue_item(
                index,
                &output.report.connector,
                &request.scope_id,
                document,
                output.report.evidence_preview.get(index),
            )
        })
        .collect::<Vec<_>>();
    let conflict_offset = queue_items.len();
    queue_items.extend(
        output
            .report
            .conflicts
            .iter()
            .enumerate()
            .map(|(index, conflict)| {
                sync_conflict_proposal_queue_item(
                    conflict_offset + index,
                    &output.report.connector,
                    &request.scope_id,
                    conflict,
                )
            }),
    );

    Ok(connector_proposal_queue_report(
        output.report.connector,
        output.report.root_path,
        queue_items,
        Vec::new(),
        json!({
            "upstream_mode": output.report.mode,
            "planned_count": output.report.planned_count,
            "missing_count": output.report.missing_count,
            "conflict_count": output.report.conflict_count,
        }),
        output.report.incremental_checkpoint,
    ))
}

fn connector_proposal_queue_report(
    connector: String,
    root_path: PathBuf,
    mut queue_items: Vec<ConnectorProposalQueueItem>,
    failures: Vec<String>,
    source_summary: Value,
    incremental_checkpoint: Value,
) -> ConnectorProposalQueueReport {
    let queue_id = connector_queue_id(&connector, &root_path, &queue_items);
    for item in &mut queue_items {
        item.review_token = connector_proposal_confirmation_token(
            &queue_id,
            std::slice::from_ref(&item.queue_item_id),
        );
    }
    let blocked_count = queue_items.iter().filter(|item| item.blocked).count();
    ConnectorProposalQueueReport {
        schema_version: "2.97-A".to_string(),
        queue_id: queue_id.clone(),
        connector,
        root_path,
        mode: "proposal_queue".to_string(),
        queue_item_count: queue_items.len(),
        blocked_count,
        queue_items,
        failures,
        source_summary,
        queue_policy: json!({
            "safe_default": "review_queue_only",
            "writes_memory": false,
            "writes_project_documents": false,
            "review_required_before_apply": true,
            "service_apply_exposed": true,
            "queue_id": queue_id,
            "confirmation_token_strategy": "confirm_<stable_hash(queue_id|sorted_queue_item_ids)>",
            "compatible_apply_paths": [
                "memory-cli compat connector-import-draft --apply --proposal",
                "memory-cli compat connector-sync-plan --apply",
                "memory-cli compat connector-proposal-apply-plan --approve-queue-item <id> --confirmation-token <token> --apply --key <key>",
                "POST /api/v1/compat/connectors/proposal-apply-plan/apply",
                "memory.connectors.proposal_apply_plan apply=true"
            ],
        }),
        incremental_checkpoint,
    }
}

fn import_proposal_queue_item(
    index: usize,
    proposal: &ConnectorImportProposalDraft,
) -> ConnectorProposalQueueItem {
    ConnectorProposalQueueItem {
        queue_item_id: connector_queue_item_id(
            &proposal.connector,
            index,
            &proposal.draft_external_id,
        ),
        review_token: String::new(),
        connector: proposal.connector.clone(),
        external_id: proposal.draft_external_id.clone(),
        scope_id: proposal.scope_id.clone(),
        title: proposal.metadata["target_title"]
            .as_str()
            .unwrap_or(&proposal.draft_external_id)
            .to_string(),
        proposal_type: proposal.proposal_type.clone(),
        review_level: proposal.review_level.clone(),
        review_status: "open".to_string(),
        apply_target: "Kernel::remember_text_after_review".to_string(),
        blocked: false,
        block_reason: None,
        reason: proposal.reason.clone(),
        evidence: proposal.evidence.clone(),
        source_refs: proposal.source_refs.clone(),
        metadata: proposal.metadata.clone(),
    }
}

fn sync_document_proposal_queue_item(
    index: usize,
    connector: &str,
    scope_id: &ScopeId,
    document: &ConnectorSyncPlanDocument,
    evidence: Option<&EvidenceSpan>,
) -> ConnectorProposalQueueItem {
    let evidence_text = evidence
        .map(|span| format!("{}: {}", span.source_ref, span.quote))
        .unwrap_or_else(|| document.canonical_uri.clone());
    ConnectorProposalQueueItem {
        queue_item_id: connector_queue_item_id(connector, index, &document.canonical_uri),
        review_token: String::new(),
        connector: connector.to_string(),
        external_id: document.canonical_uri.clone(),
        scope_id: scope_id.clone(),
        title: document.title.clone(),
        proposal_type: "project_document_upsert".to_string(),
        review_level: "suggested".to_string(),
        review_status: "open".to_string(),
        apply_target: "Kernel::apply_project_document_sync_plan".to_string(),
        blocked: false,
        block_reason: None,
        reason: format!(
            "Review {} document '{}' before applying project document sync.",
            connector, document.title
        ),
        evidence: vec![evidence_text],
        source_refs: vec![document.canonical_uri.clone()],
        metadata: json!({
            "sync_state": document.sync_state,
            "content_hash": document.content_hash,
            "local_path": document.local_path,
            "document_metadata": document.metadata,
        }),
    }
}

fn sync_conflict_proposal_queue_item(
    index: usize,
    connector: &str,
    scope_id: &ScopeId,
    conflict: &ProjectDocumentConflictReport,
) -> ConnectorProposalQueueItem {
    let reason = conflict
        .reason
        .clone()
        .unwrap_or_else(|| "Project document conflict requires review before apply.".to_string());
    ConnectorProposalQueueItem {
        queue_item_id: connector_queue_item_id(connector, index, &conflict.canonical_uri),
        review_token: String::new(),
        connector: connector.to_string(),
        external_id: conflict.canonical_uri.clone(),
        scope_id: scope_id.clone(),
        title: format!("Conflict: {}", conflict.canonical_uri),
        proposal_type: "project_document_conflict_review".to_string(),
        review_level: "required".to_string(),
        review_status: "open".to_string(),
        apply_target: "Review::resolve_project_document_conflict".to_string(),
        blocked: true,
        block_reason: Some(reason.clone()),
        reason,
        evidence: vec![format!(
            "{} sync_state={} conflict_state={}",
            conflict.canonical_uri,
            conflict.sync_state.as_str(),
            conflict.conflict_state.as_str()
        )],
        source_refs: vec![conflict.canonical_uri.clone()],
        metadata: json!({
            "sync_state": conflict.sync_state.as_str(),
            "conflict_state": conflict.conflict_state.as_str(),
        }),
    }
}

fn connector_import_queue_checkpoint(drafts: &[ConnectorImportDraft]) -> Value {
    json!({
        "strategy": "chat_export_external_id",
        "apply_target": "Kernel::remember_text_after_review",
        "external_ids": drafts.iter().map(|draft| draft.external_id.clone()).collect::<Vec<_>>(),
    })
}

fn connector_queue_item_id(connector: &str, index: usize, external_id: &str) -> String {
    let slug = external_id
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() {
                value.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .chars()
        .take(48)
        .collect::<String>();
    let suffix = if slug.is_empty() {
        "item".to_string()
    } else {
        slug
    };
    format!("cpq_{connector}_{index}_{suffix}")
}

fn connector_queue_id(
    connector: &str,
    root_path: &Path,
    queue_items: &[ConnectorProposalQueueItem],
) -> String {
    let mut input = format!("{connector}|{}", root_path.display());
    for item in queue_items {
        input.push('|');
        input.push_str(&item.queue_item_id);
        input.push(':');
        input.push_str(&item.external_id);
    }
    format!("cpq_{}_{}", connector, stable_hex_hash(&input))
}

fn stable_hex_hash(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn local_git_dry_run(root_path: PathBuf, max_items: usize) -> Result<ConnectorDryRunReport> {
    let mut failures = Vec::new();
    if !root_path.join(".git").exists() {
        failures.push("missing .git directory; repository metadata is unavailable".to_string());
    }

    let mut items = markdown_items(&root_path, max_items)?;
    let git_log = root_path.join(".git/logs/HEAD");
    if git_log.exists() && items.len() < max_items {
        let metadata = fs::metadata(&git_log)
            .with_context(|| format!("failed to read {}", git_log.display()))?;
        items.push(ConnectorDryRunItem {
            title: "git HEAD reflog".to_string(),
            source_ref: format!("git-log://{}", root_path.display()),
            content_bytes: metadata.len(),
            metadata: json!({
                "source_kind": "repository_log",
                "path": git_log.display().to_string(),
            }),
        });
    }

    let candidate_count = items.len();
    let repository_metadata = local_git_repository_metadata(&root_path);
    Ok(ConnectorDryRunReport {
        schema_version: "2.97-A".to_string(),
        connector: "local-git".to_string(),
        root_path,
        mode: "dry_run".to_string(),
        status: if failures.is_empty() {
            "ready".to_string()
        } else {
            "needs_attention".to_string()
        },
        candidate_count,
        items,
        failures,
        incremental_checkpoint: json!({
            "strategy": "path_mtime_size",
            "remote_network": false,
            "repository_metadata": repository_metadata,
        }),
    })
}

fn markdown_docs_dry_run(root_path: PathBuf, max_items: usize) -> Result<ConnectorDryRunReport> {
    let items = markdown_items(&root_path, max_items)?;
    let candidate_count = items.len();
    Ok(ConnectorDryRunReport {
        schema_version: "2.97-A".to_string(),
        connector: "markdown-docs".to_string(),
        root_path,
        mode: "dry_run".to_string(),
        status: "ready".to_string(),
        candidate_count,
        items,
        failures: Vec::new(),
        incremental_checkpoint: json!({
            "strategy": "path_mtime_size",
            "frontmatter": "parse_yaml_frontmatter_when_present",
        }),
    })
}

fn chat_export_dry_run(root_path: PathBuf, max_items: usize) -> Result<ConnectorDryRunReport> {
    let mut failures = Vec::new();
    let mut files = Vec::new();
    collect_json_files(&root_path, &mut files)?;
    files.sort();

    let mut items = Vec::new();
    for path in files {
        if items.len() >= max_items {
            break;
        }
        match chat_export_items(&root_path, &path) {
            Ok(mut parsed) => {
                let remaining = max_items.saturating_sub(items.len());
                parsed.truncate(remaining);
                items.extend(parsed);
            }
            Err(error) => failures.push(format!("{}: {error}", path.display())),
        }
    }

    let candidate_count = items.len();
    Ok(ConnectorDryRunReport {
        schema_version: "2.97-A".to_string(),
        connector: "chat-export".to_string(),
        root_path,
        mode: "dry_run".to_string(),
        status: if failures.is_empty() {
            "ready".to_string()
        } else {
            "needs_attention".to_string()
        },
        candidate_count,
        items,
        failures,
        incremental_checkpoint: json!({
            "strategy": "path_mtime_size_message_count",
            "formats": ["generic-messages-json", "chatgpt-conversations-json"],
            "safe_default": "explicit_import_only",
        }),
    })
}

fn web_crawler_dry_run(root_path: PathBuf, max_items: usize) -> Result<ConnectorDryRunReport> {
    let allowlist_domains = web_allowlist_domains(&root_path)?;
    let mut files = Vec::new();
    collect_web_page_files(&root_path, &mut files)?;
    files.sort();

    let mut failures = Vec::new();
    let mut items = Vec::new();
    for path in files.into_iter().take(max_items) {
        match web_page_item(&root_path, &path, &allowlist_domains) {
            Ok(item) => {
                if item.metadata["allowlist_allowed"] != true {
                    failures.push(format!(
                        "{} blocked by allowlist: {}",
                        path.display(),
                        item.metadata["canonical_url"]
                            .as_str()
                            .unwrap_or("unknown canonical url")
                    ));
                }
                items.push(item);
            }
            Err(error) => failures.push(format!("{}: {error}", path.display())),
        }
    }

    if items.is_empty() {
        failures.push("no local web snapshots found".to_string());
    }

    let candidate_count = items.len();
    Ok(ConnectorDryRunReport {
        schema_version: "2.97-A".to_string(),
        connector: "web-crawler".to_string(),
        root_path,
        mode: "dry_run".to_string(),
        status: if failures.is_empty() {
            "ready".to_string()
        } else {
            "needs_attention".to_string()
        },
        candidate_count,
        items,
        failures,
        incremental_checkpoint: json!({
            "strategy": "path_mtime_size_canonical_url",
            "allowlist_domains": allowlist_domains,
            "remote_network": false,
            "safe_default": "dry_run_only_no_remote_fetch",
        }),
    })
}

#[derive(Debug, Clone, Copy)]
struct ExternalConnectorConfig {
    connector: &'static str,
    source_kind: &'static str,
    uri_prefix: &'static str,
    safe_default: &'static str,
    extensions: &'static [&'static str],
}

const NOTION_EXTENSIONS: &[&str] = &["md", "markdown", "json", "csv", "txt"];
const DRIVE_EXTENSIONS: &[&str] = &["md", "markdown", "json", "csv", "txt", "pdf", "docx"];

fn external_connector(connector: &str) -> Result<ExternalConnectorConfig> {
    match connector {
        "notion" => Ok(ExternalConnectorConfig {
            connector: "notion",
            source_kind: "notion_page",
            uri_prefix: "notion-export",
            safe_default: "local_export_only_no_remote_api",
            extensions: NOTION_EXTENSIONS,
        }),
        "google-drive" => Ok(ExternalConnectorConfig {
            connector: "google-drive",
            source_kind: "drive_document",
            uri_prefix: "gdrive-export",
            safe_default: "local_export_only_no_remote_api",
            extensions: DRIVE_EXTENSIONS,
        }),
        "onedrive" => Ok(ExternalConnectorConfig {
            connector: "onedrive",
            source_kind: "onedrive_document",
            uri_prefix: "onedrive-export",
            safe_default: "local_export_only_no_remote_api",
            extensions: DRIVE_EXTENSIONS,
        }),
        other => bail!("unsupported external connector config: {other}"),
    }
}

fn external_docs_dry_run(
    root_path: PathBuf,
    max_items: usize,
    config: ExternalConnectorConfig,
) -> Result<ConnectorDryRunReport> {
    let (candidates, failures) = external_document_candidates(&root_path, max_items, config)?;
    let items = candidates
        .into_iter()
        .map(|candidate| ConnectorDryRunItem {
            title: candidate.title,
            source_ref: candidate.canonical_uri,
            content_bytes: candidate.content_bytes,
            metadata: candidate.metadata,
        })
        .collect::<Vec<_>>();
    let candidate_count = items.len();
    let mut failures = failures;
    if items.is_empty() {
        failures.push(format!(
            "no supported {} export documents found",
            config.connector
        ));
    }

    Ok(ConnectorDryRunReport {
        schema_version: "2.97-A".to_string(),
        connector: config.connector.to_string(),
        root_path,
        mode: "dry_run".to_string(),
        status: if failures.is_empty() {
            "ready".to_string()
        } else {
            "needs_attention".to_string()
        },
        candidate_count,
        items,
        failures,
        incremental_checkpoint: json!({
            "strategy": "export_path_content_hash",
            "source_kind": config.source_kind,
            "safe_default": config.safe_default,
            "remote_network": false,
            "supported_extensions": config.extensions,
        }),
    })
}

fn markdown_items(root_path: &Path, max_items: usize) -> Result<Vec<ConnectorDryRunItem>> {
    let mut files = Vec::new();
    collect_markdown_files(root_path, &mut files)?;
    files.sort();

    files
        .into_iter()
        .take(max_items)
        .map(|path| markdown_item(root_path, &path))
        .collect()
}

fn collect_json_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry =
            entry.with_context(|| format!("failed to read entry under {}", dir.display()))?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name.starts_with('.') || file_name == "target" || file_name == "reports" {
            continue;
        }
        if path.is_dir() {
            collect_json_files(&path, files)?;
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        {
            files.push(path);
        }
    }
    Ok(())
}

fn collect_markdown_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry =
            entry.with_context(|| format!("failed to read entry under {}", dir.display()))?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == ".git" || file_name == "target" {
            continue;
        }
        if path.is_dir() {
            collect_markdown_files(&path, files)?;
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        {
            files.push(path);
        }
    }
    Ok(())
}

#[derive(Debug)]
struct ExternalDocumentCandidate {
    title: String,
    canonical_uri: String,
    local_path: PathBuf,
    content_text: String,
    content_hash: String,
    content_bytes: u64,
    metadata: Value,
}

fn external_document_candidates(
    root_path: &Path,
    max_items: usize,
    config: ExternalConnectorConfig,
) -> Result<(Vec<ExternalDocumentCandidate>, Vec<String>)> {
    let mut files = Vec::new();
    collect_external_document_files(root_path, &mut files, config.extensions)?;
    files.sort();

    let mut candidates = Vec::new();
    let mut failures = Vec::new();
    for path in files {
        if candidates.len() >= max_items {
            break;
        }
        match external_document_candidate(root_path, &path, config) {
            Ok(candidate) => candidates.push(candidate),
            Err(error) => failures.push(format!("{}: {error}", path.display())),
        }
    }

    Ok((candidates, failures))
}

fn collect_external_document_files(
    dir: &Path,
    files: &mut Vec<PathBuf>,
    extensions: &[&str],
) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry =
            entry.with_context(|| format!("failed to read entry under {}", dir.display()))?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name.starts_with('.') || file_name == "target" || file_name == "reports" {
            continue;
        }
        if path.is_dir() {
            collect_external_document_files(&path, files, extensions)?;
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extensions
                    .iter()
                    .any(|supported| extension.eq_ignore_ascii_case(supported))
            })
            && !is_external_text_sidecar_path(&path)
        {
            files.push(path);
        }
    }
    Ok(())
}

fn collect_web_page_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let excluded_dir_names = ["target", "reports", ".playwright-cli"]
        .into_iter()
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    collect_web_page_files_with_excluded(dir, files, &excluded_dir_names)
}

fn collect_web_page_files_with_excluded(
    dir: &Path,
    files: &mut Vec<PathBuf>,
    excluded_dir_names: &BTreeSet<String>,
) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry =
            entry.with_context(|| format!("failed to read entry under {}", dir.display()))?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name.starts_with('.') || excluded_dir_names.contains(file_name.as_ref()) {
            continue;
        }
        if path.is_dir() {
            collect_web_page_files_with_excluded(&path, files, excluded_dir_names)?;
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm")
            })
        {
            files.push(path);
        }
    }
    Ok(())
}

fn chat_export_items(root_path: &Path, path: &Path) -> Result<Vec<ConnectorDryRunItem>> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let value: Value = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    let mut items = Vec::new();
    match value {
        Value::Array(conversations) => {
            for (index, conversation) in conversations.iter().enumerate() {
                if let Some(item) = chat_export_item(root_path, path, conversation, index) {
                    items.push(item);
                }
            }
        }
        Value::Object(ref object) => {
            if let Some(Value::Array(conversations)) = object.get("conversations") {
                for (index, conversation) in conversations.iter().enumerate() {
                    if let Some(item) = chat_export_item(root_path, path, conversation, index) {
                        items.push(item);
                    }
                }
            } else if let Some(item) = chat_export_item(root_path, path, &value, 0) {
                items.push(item);
            }
        }
        _ => {}
    }

    if items.is_empty() {
        bail!("no supported chat conversations found");
    }
    Ok(items)
}

fn chat_export_import_drafts(
    root_path: &Path,
    scope_id: &ScopeId,
    max_items: usize,
) -> Result<(Vec<ConnectorImportDraft>, Vec<String>)> {
    let mut failures = Vec::new();
    let mut files = Vec::new();
    collect_json_files(root_path, &mut files)?;
    files.sort();

    let mut drafts = Vec::new();
    for path in files {
        if drafts.len() >= max_items {
            break;
        }
        match chat_export_conversations(root_path, &path) {
            Ok(mut conversations) => {
                let remaining = max_items.saturating_sub(drafts.len());
                conversations.truncate(remaining);
                drafts.extend(
                    conversations
                        .into_iter()
                        .map(|conversation| conversation_import_draft(scope_id, conversation)),
                );
            }
            Err(error) => failures.push(format!("{}: {error}", path.display())),
        }
    }

    Ok((drafts, failures))
}

#[derive(Debug)]
struct ParsedChatConversation {
    title: String,
    external_id: String,
    source_ref: String,
    relative_path: String,
    format: String,
    messages: Vec<ChatExportMessage>,
}

fn chat_export_conversations(root_path: &Path, path: &Path) -> Result<Vec<ParsedChatConversation>> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let value: Value = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    let mut conversations = Vec::new();
    match value {
        Value::Array(items) => {
            for (index, conversation) in items.iter().enumerate() {
                if let Some(parsed) = parsed_chat_conversation(root_path, path, conversation, index)
                {
                    conversations.push(parsed);
                }
            }
        }
        Value::Object(ref object) => {
            if let Some(Value::Array(items)) = object.get("conversations") {
                for (index, conversation) in items.iter().enumerate() {
                    if let Some(parsed) =
                        parsed_chat_conversation(root_path, path, conversation, index)
                    {
                        conversations.push(parsed);
                    }
                }
            } else if let Some(parsed) = parsed_chat_conversation(root_path, path, &value, 0) {
                conversations.push(parsed);
            }
        }
        _ => {}
    }

    if conversations.is_empty() {
        bail!("no supported chat conversations found");
    }
    Ok(conversations)
}

fn parsed_chat_conversation(
    root_path: &Path,
    path: &Path,
    conversation: &Value,
    index: usize,
) -> Option<ParsedChatConversation> {
    let messages = extract_chat_messages(conversation);
    if messages.is_empty() {
        return None;
    }

    let title = conversation_title(conversation);
    let external_id = conversation_external_id(conversation, index);
    let relative_path = path
        .strip_prefix(root_path)
        .unwrap_or(path)
        .display()
        .to_string();
    let format = chat_export_format(conversation).to_string();

    Some(ParsedChatConversation {
        title,
        external_id: external_id.clone(),
        source_ref: format!("file://{}#{}", path.display(), external_id),
        relative_path,
        format,
        messages,
    })
}

fn conversation_import_draft(
    scope_id: &ScopeId,
    conversation: ParsedChatConversation,
) -> ConnectorImportDraft {
    let participants = chat_participants(&conversation.messages);
    let message_count = conversation.messages.len();
    let body = conversation_transcript(&conversation.messages);
    let first_created_at = conversation
        .messages
        .iter()
        .filter_map(|message| message.created_at)
        .min_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let last_created_at = conversation
        .messages
        .iter()
        .filter_map(|message| message.created_at)
        .max_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));

    ConnectorImportDraft {
        connector: "chat-export".to_string(),
        external_id: conversation.external_id.clone(),
        scope_id: scope_id.clone(),
        title: conversation.title,
        body,
        memory_kind: MemoryKind::Summary,
        source_refs: vec![conversation.source_ref],
        metadata: json!({
            "source_kind": "conversation_export",
            "relative_path": conversation.relative_path,
            "external_id": conversation.external_id,
            "message_count": message_count,
            "participants": participants,
            "format": conversation.format,
            "first_created_at": first_created_at,
            "last_created_at": last_created_at,
        }),
    }
}

fn connector_import_proposal_draft(draft: &ConnectorImportDraft) -> ConnectorImportProposalDraft {
    ConnectorImportProposalDraft {
        connector: draft.connector.clone(),
        draft_external_id: draft.external_id.clone(),
        scope_id: draft.scope_id.clone(),
        proposal_type: "distill_upsert".to_string(),
        review_level: "required".to_string(),
        reason: format!(
            "Review chat-export conversation '{}' before importing it as memory.",
            draft.title
        ),
        evidence: vec![format!(
            "connector={} external_id={} source_refs={}",
            draft.connector,
            draft.external_id,
            draft.source_refs.join(",")
        )],
        source_refs: draft.source_refs.clone(),
        metadata: json!({
            "target_memory_kind": format!("{:?}", draft.memory_kind).to_ascii_lowercase(),
            "target_title": draft.title,
            "draft_body_bytes": draft.body.len(),
            "source_metadata": draft.metadata,
        }),
    }
}

fn chat_export_item(
    root_path: &Path,
    path: &Path,
    conversation: &Value,
    index: usize,
) -> Option<ConnectorDryRunItem> {
    let messages = extract_chat_messages(conversation);
    if messages.is_empty() {
        return None;
    }

    let title = conversation_title(conversation);
    let external_id = conversation_external_id(conversation, index);
    let relative_path = path.strip_prefix(root_path).unwrap_or(path);
    let participants = chat_participants(&messages);
    let content_bytes = messages
        .iter()
        .map(|message| message.content.len() as u64)
        .sum();

    Some(ConnectorDryRunItem {
        title,
        source_ref: format!("file://{}#{}", path.display(), external_id),
        content_bytes,
        metadata: json!({
            "source_kind": "conversation_export",
            "relative_path": relative_path.display().to_string(),
            "external_id": external_id,
            "message_count": messages.len(),
            "participants": participants,
            "format": chat_export_format(conversation),
        }),
    })
}

fn conversation_title(conversation: &Value) -> String {
    conversation
        .get("title")
        .and_then(Value::as_str)
        .filter(|title| !title.trim().is_empty())
        .unwrap_or("chat export conversation")
        .to_string()
}

fn conversation_external_id(conversation: &Value, index: usize) -> String {
    conversation
        .get("id")
        .or_else(|| conversation.get("conversation_id"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("conversation-{index}"))
}

fn chat_export_format(conversation: &Value) -> &'static str {
    if conversation.get("mapping").is_some() {
        "chatgpt-conversations-json"
    } else {
        "generic-messages-json"
    }
}

fn chat_participants(messages: &[ChatExportMessage]) -> Vec<&str> {
    messages
        .iter()
        .map(|message| message.role.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn conversation_transcript(messages: &[ChatExportMessage]) -> String {
    messages
        .iter()
        .map(|message| format!("{}: {}", message.role, message.content))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug)]
struct ChatExportMessage {
    role: String,
    content: String,
    created_at: Option<f64>,
}

fn extract_chat_messages(conversation: &Value) -> Vec<ChatExportMessage> {
    if let Some(messages) = conversation.get("messages").and_then(Value::as_array) {
        return messages
            .iter()
            .filter_map(generic_chat_message)
            .collect::<Vec<_>>();
    }

    let Some(mapping) = conversation.get("mapping").and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut messages = mapping
        .values()
        .filter_map(|node| node.get("message"))
        .filter_map(chatgpt_message)
        .collect::<Vec<_>>();
    messages.sort_by(|left, right| {
        left.created_at
            .partial_cmp(&right.created_at)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    messages
}

fn generic_chat_message(value: &Value) -> Option<ChatExportMessage> {
    let role = value
        .get("role")
        .or_else(|| value.get("author"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let content = value
        .get("content")
        .or_else(|| value.get("text"))
        .and_then(chat_content_text)?;
    Some(ChatExportMessage {
        role,
        content,
        created_at: value
            .get("created_at")
            .or_else(|| value.get("create_time"))
            .and_then(Value::as_f64),
    })
}

fn chatgpt_message(value: &Value) -> Option<ChatExportMessage> {
    let role = value
        .pointer("/author/role")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let content = value
        .pointer("/content/parts")
        .and_then(chat_content_text)?;
    Some(ChatExportMessage {
        role,
        content,
        created_at: value.get("create_time").and_then(Value::as_f64),
    })
}

fn chat_content_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => non_empty_text(text),
        Value::Array(parts) => non_empty_text(
            &parts
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        _ => None,
    }
}

fn non_empty_text(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

fn markdown_item(root_path: &Path, path: &Path) -> Result<ConnectorDryRunItem> {
    let metadata =
        fs::metadata(path).with_context(|| format!("failed to read {}", path.display()))?;
    let content_text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let frontmatter = markdown_frontmatter(&content_text);
    let frontmatter_present = frontmatter.is_some();
    let relative_path = path.strip_prefix(root_path).unwrap_or(path);
    let title = frontmatter
        .as_ref()
        .and_then(|value| value.get("title"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("markdown document")
                .replace(['_', '-'], " ")
        });

    Ok(ConnectorDryRunItem {
        title,
        source_ref: format!("file://{}", path.display()),
        content_bytes: metadata.len(),
        metadata: json!({
            "source_kind": "markdown",
            "relative_path": relative_path.display().to_string(),
            "frontmatter": frontmatter,
            "frontmatter_present": frontmatter_present,
        }),
    })
}

fn external_document_candidate(
    root_path: &Path,
    path: &Path,
    config: ExternalConnectorConfig,
) -> Result<ExternalDocumentCandidate> {
    let file_metadata =
        fs::metadata(path).with_context(|| format!("failed to read {}", path.display()))?;
    let relative_path = path
        .strip_prefix(root_path)
        .unwrap_or(path)
        .display()
        .to_string();
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let (title, content_text, extraction_mode, external_metadata) = match extension.as_str() {
        "md" | "markdown" => {
            let content_text = fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let title = markdown_document_title(path, &content_text);
            (
                title,
                content_text.clone(),
                "markdown_text".to_string(),
                markdown_document_metadata(&content_text),
            )
        }
        "txt" | "csv" => {
            let content_text = fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let title = path
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or(config.connector)
                .replace(['_', '-'], " ");
            (
                title,
                content_text,
                format!("{extension}_text"),
                json!({ "visible_content_bytes": file_metadata.len() }),
            )
        }
        "json" => external_json_document(path, config)?,
        "pdf" | "docx" => external_sidecar_document(path, &extension, config)?,
        _ => bail!("unsupported export extension: {extension}"),
    };
    let canonical_uri =
        external_document_uri(path, &relative_path, config, external_metadata.as_object());
    let content_hash = memory_domain::Artifact::compute_content_hash(&content_text);
    let metadata = json!({
        "source_kind": config.source_kind,
        "connector": config.connector,
        "relative_path": relative_path,
        "document_extension": extension,
        "extraction_mode": extraction_mode,
        "content_hash": content_hash,
        "remote_network": false,
        "safe_default": config.safe_default,
        "source_metadata": external_metadata,
    });

    Ok(ExternalDocumentCandidate {
        title,
        canonical_uri,
        local_path: path.to_path_buf(),
        content_text,
        content_hash,
        content_bytes: file_metadata.len(),
        metadata,
    })
}

fn external_json_document(
    path: &Path,
    config: ExternalConnectorConfig,
) -> Result<(String, String, String, Value)> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let value: Value = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let title = value
        .get("title")
        .or_else(|| value.get("name"))
        .or_else(|| value.get("filename"))
        .and_then(Value::as_str)
        .filter(|title| !title.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or(config.connector)
                .replace(['_', '-'], " ")
        });
    let content_text = value
        .get("content")
        .or_else(|| value.get("text"))
        .or_else(|| value.get("body"))
        .or_else(|| value.get("markdown"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| serde_json::to_string_pretty(&value).unwrap_or_else(|_| raw.clone()));

    Ok((title, content_text, "json_text".to_string(), value))
}

fn external_sidecar_document(
    path: &Path,
    extension: &str,
    config: ExternalConnectorConfig,
) -> Result<(String, String, String, Value)> {
    let sidecar = external_text_sidecar(path).ok_or_else(|| {
        anyhow::anyhow!(
            "{} export requires a .txt or .md sidecar for text extraction",
            extension
        )
    })?;
    let content_text = fs::read_to_string(&sidecar)
        .with_context(|| format!("failed to read {}", sidecar.display()))?;
    let title = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(config.connector)
        .replace(['_', '-'], " ");
    Ok((
        title,
        content_text,
        format!("{extension}_sidecar_text"),
        json!({
            "sidecar_path": sidecar.display().to_string(),
            "sidecar_required": true,
        }),
    ))
}

fn external_text_sidecar(path: &Path) -> Option<PathBuf> {
    [path.with_extension("txt"), path.with_extension("md")]
        .into_iter()
        .find(|candidate| candidate.exists())
}

fn is_external_text_sidecar_path(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return false;
    };
    if !extension.eq_ignore_ascii_case("txt") && !extension.eq_ignore_ascii_case("md") {
        return false;
    }
    path.with_extension("pdf").exists() || path.with_extension("docx").exists()
}

fn external_document_uri(
    path: &Path,
    relative_path: &str,
    config: ExternalConnectorConfig,
    metadata: Option<&serde_json::Map<String, Value>>,
) -> String {
    let external_id = metadata
        .and_then(|metadata| {
            metadata
                .get("id")
                .or_else(|| metadata.get("page_id"))
                .or_else(|| metadata.get("file_id"))
                .or_else(|| metadata.get("drive_id"))
                .or_else(|| metadata.get("web_url"))
                .and_then(Value::as_str)
        })
        .map(str::to_string)
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or(relative_path)
                .to_string()
        });
    format!("{}://{}", config.uri_prefix, external_id)
}

fn web_page_item(
    root_path: &Path,
    path: &Path,
    allowlist_domains: &[String],
) -> Result<ConnectorDryRunItem> {
    let metadata =
        fs::metadata(path).with_context(|| format!("failed to read {}", path.display()))?;
    let html =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let relative_path = path.strip_prefix(root_path).unwrap_or(path);
    let canonical_url =
        html_canonical_url(&html).unwrap_or_else(|| format!("file://{}", path.display()));
    let title = html_title(&html).unwrap_or_else(|| {
        path.file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("web page")
            .replace(['_', '-'], " ")
    });
    let visible_text = html_visible_text(&html);
    let allowlist_allowed = web_url_allowed(&canonical_url, allowlist_domains);

    Ok(ConnectorDryRunItem {
        title,
        source_ref: canonical_url.clone(),
        content_bytes: metadata.len(),
        metadata: json!({
            "source_kind": "web_page",
            "relative_path": relative_path.display().to_string(),
            "canonical_url": canonical_url,
            "allowlist_allowed": allowlist_allowed,
            "allowlist_domains": allowlist_domains,
            "link_count": html_link_count(&html),
            "link_boundary": web_link_boundary(&html, &canonical_url, allowlist_domains),
            "visible_text_bytes": visible_text.len(),
            "content_hash": stable_hex_hash(&html),
            "remote_network": false,
        }),
    })
}

#[derive(Debug)]
struct WebPageSyncCandidate {
    canonical_uri: String,
    relative_path: String,
    title: String,
    content_text: String,
    allowlist_allowed: bool,
    metadata: Value,
}

enum WebRemoteFetchOutcome {
    Candidate(WebPageSyncCandidate),
    NotModified {
        canonical_uri: String,
        metadata: Value,
    },
}

fn web_page_sync_candidate(
    root_path: &Path,
    path: &Path,
    allowlist_domains: &[String],
) -> Result<WebPageSyncCandidate> {
    let html =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let relative_path = path
        .strip_prefix(root_path)
        .unwrap_or(path)
        .display()
        .to_string();
    let canonical_uri =
        html_canonical_url(&html).unwrap_or_else(|| format!("file://{}", path.display()));
    let title = html_title(&html).unwrap_or_else(|| {
        path.file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("web page")
            .replace(['_', '-'], " ")
    });
    let visible_text = html_visible_text(&html);
    let allowlist_allowed = web_url_allowed(&canonical_uri, allowlist_domains);
    let content_text = web_page_document_content(&title, &canonical_uri, &visible_text);
    let metadata = json!({
        "source_kind": "web_page",
        "relative_path": relative_path,
        "canonical_url": canonical_uri,
        "allowlist_allowed": allowlist_allowed,
        "allowlist_domains": allowlist_domains,
        "link_count": html_link_count(&html),
        "link_boundary": web_link_boundary(&html, &canonical_uri, allowlist_domains),
        "visible_text_bytes": visible_text.len(),
        "html_content_hash": stable_hex_hash(&html),
        "remote_network": false,
        "fetch_policy": "local_snapshot_only_no_remote_fetch",
    });

    Ok(WebPageSyncCandidate {
        canonical_uri,
        relative_path,
        title,
        content_text,
        allowlist_allowed,
        metadata,
    })
}

fn web_page_remote_sync_candidate(
    _root_path: &Path,
    url: &str,
    allowlist_domains: &[String],
    url_validators: &BTreeMap<String, WebUrlValidator>,
    budget: &mut WebRemoteFetchBudget,
) -> Result<WebRemoteFetchOutcome> {
    let requested = parse_http_url(url)?;
    if !web_url_allowed(url, allowlist_domains) {
        return Ok(WebRemoteFetchOutcome::Candidate(
            web_blocked_sync_candidate(
                url,
                url,
                "allowlist",
                json!({
                    "requested_url": url,
                    "remote_network": false,
                }),
            ),
        ));
    }

    let fetch = web_fetch_remote_url(url, allowlist_domains, url_validators, budget)?;
    let Some(response) = fetch.response else {
        let final_url = fetch.final_url;
        let redirects = fetch.redirects;
        if !fetch.robots_allowed {
            return Ok(WebRemoteFetchOutcome::Candidate(
                web_blocked_sync_candidate(
                    url,
                    &final_url,
                    "robots",
                    json!({
                        "requested_url": url,
                        "canonical_url": final_url,
                        "remote_network": true,
                        "fetch_policy": "remote_fetch_opt_in",
                        "redirect_count": redirects.len(),
                        "redirects": redirects,
                        "robots_allowed": false,
                        "robots_status": fetch.robots_status,
                        "robots_url": fetch.robots_url,
                    }),
                ),
            ));
        }
        return Ok(WebRemoteFetchOutcome::NotModified {
            canonical_uri: final_url.clone(),
            metadata: json!({
                "requested_url": url,
                "canonical_url": final_url,
                "http_status": 304,
                "remote_network": true,
                "fetch_policy": "remote_fetch_opt_in",
                "conditional_request": true,
                "etag": fetch.validator.as_ref().and_then(|validator| validator.etag.as_ref()),
                "last_modified": fetch.validator.as_ref().and_then(|validator| validator.last_modified.as_ref()),
                "redirect_count": redirects.len(),
                "redirects": redirects,
                "robots_allowed": fetch.robots_allowed,
                "robots_status": fetch.robots_status,
                "robots_url": fetch.robots_url,
            }),
        });
    };
    if !(200..300).contains(&response.status_code) {
        bail!("remote fetch returned HTTP {}", response.status_code);
    }

    let canonical_uri =
        html_canonical_url(&response.body).unwrap_or_else(|| fetch.final_url.clone());
    let title = html_title(&response.body).unwrap_or_else(|| {
        parse_http_url(&fetch.final_url)
            .unwrap_or_else(|_| requested.clone())
            .path_without_query()
            .rsplit('/')
            .find(|segment| !segment.is_empty())
            .unwrap_or("web page")
            .replace(['_', '-'], " ")
    });
    let visible_text = html_visible_text(&response.body);
    let allowlist_allowed = web_url_allowed(&canonical_uri, allowlist_domains);
    let content_text = web_page_document_content(&title, &canonical_uri, &visible_text);
    let metadata = json!({
        "source_kind": "web_page",
        "relative_path": url,
        "requested_url": url,
        "canonical_url": canonical_uri,
        "allowlist_allowed": allowlist_allowed,
        "allowlist_domains": allowlist_domains,
        "link_count": html_link_count(&response.body),
        "link_boundary": web_link_boundary(&response.body, &fetch.final_url, allowlist_domains),
        "visible_text_bytes": visible_text.len(),
        "html_content_hash": stable_hex_hash(&response.body),
        "remote_network": true,
        "fetch_policy": "remote_fetch_opt_in",
        "http_status": response.status_code,
        "etag": response.headers.get("etag"),
        "last_modified": response.headers.get("last-modified"),
        "conditional_request": fetch.validator.is_some(),
        "redirect_count": fetch.redirects.len(),
        "redirects": fetch.redirects,
        "final_url": fetch.final_url,
        "robots_allowed": fetch.robots_allowed,
        "robots_status": fetch.robots_status,
        "robots_url": fetch.robots_url,
    });

    Ok(WebRemoteFetchOutcome::Candidate(WebPageSyncCandidate {
        canonical_uri,
        relative_path: url.to_string(),
        title,
        content_text,
        allowlist_allowed,
        metadata,
    }))
}

fn web_blocked_sync_candidate(
    requested_url: &str,
    canonical_uri: &str,
    block_kind: &str,
    metadata: Value,
) -> WebPageSyncCandidate {
    WebPageSyncCandidate {
        canonical_uri: canonical_uri.to_string(),
        relative_path: requested_url.to_string(),
        title: requested_url.to_string(),
        content_text: String::new(),
        allowlist_allowed: false,
        metadata: json!({
            "source_kind": "web_page",
            "relative_path": requested_url,
            "requested_url": requested_url,
            "canonical_url": canonical_uri,
            "allowlist_allowed": false,
            "block_kind": block_kind,
            "details": metadata,
        }),
    }
}

fn web_page_document_content(title: &str, canonical_uri: &str, visible_text: &str) -> String {
    let mut content = format!("# {title}\n\nSource: {canonical_uri}\n\n");
    if visible_text.trim().is_empty() {
        content.push_str("No visible text extracted.\n");
    } else {
        content.push_str(visible_text.trim());
        content.push('\n');
    }
    content
}

fn connector_sync_conflict_reports(
    previous: &[ProjectDocumentSnapshot],
    documents: &[LocalProjectDocumentDraft],
    missing: &[MissingProjectDocument],
) -> Vec<ProjectDocumentConflictReport> {
    let local_by_uri = documents
        .iter()
        .map(|document| {
            (
                document.canonical_uri.clone(),
                document.content_hash.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let missing_uris = missing
        .iter()
        .map(|document| document.canonical_uri.as_str())
        .collect::<BTreeSet<_>>();

    previous
        .iter()
        .filter_map(|snapshot| {
            let report = classify_project_document_conflict(ProjectDocumentConflictInput {
                canonical_uri: snapshot.canonical_uri.clone(),
                base_content_hash: Some(snapshot.content_hash.clone()),
                indexed_content_hash: Some(snapshot.content_hash.clone()),
                local_content_hash: local_by_uri.get(&snapshot.canonical_uri).cloned(),
            });
            if matches!(report.sync_state, DocumentSyncState::Clean)
                && !missing_uris.contains(snapshot.canonical_uri.as_str())
            {
                None
            } else {
                Some(report)
            }
        })
        .collect()
}

fn web_allowlist_domains(root_path: &Path) -> Result<Vec<String>> {
    let path = root_path.join("allowlist.txt");
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", path.display()));
        }
    };
    Ok(raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            line.trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_start_matches("www.")
                .trim_end_matches('/')
                .to_ascii_lowercase()
        })
        .collect())
}

fn web_url_manifest(root_path: &Path) -> Result<Vec<String>> {
    let path = root_path.join("urls.txt");
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", path.display()));
        }
    };
    Ok(raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WebUrlValidator {
    etag: Option<String>,
    last_modified: Option<String>,
}

fn web_url_validators(root_path: &Path) -> Result<BTreeMap<String, WebUrlValidator>> {
    let path = root_path.join("url-validators.json");
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", path.display()));
        }
    };
    let value: Value = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let Some(object) = value.as_object() else {
        bail!("url validators manifest must be a JSON object");
    };
    Ok(object
        .iter()
        .filter_map(|(url, entry)| {
            let entry = entry.as_object()?;
            Some((
                url.to_string(),
                WebUrlValidator {
                    etag: entry
                        .get("etag")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    last_modified: entry
                        .get("last_modified")
                        .or_else(|| entry.get("last-modified"))
                        .and_then(Value::as_str)
                        .map(str::to_string),
                },
            ))
        })
        .collect())
}

fn web_url_allowed(url: &str, allowlist_domains: &[String]) -> bool {
    if allowlist_domains.is_empty() || url.starts_with("file://") {
        return true;
    }
    let Some(host) = web_url_host(url) else {
        return false;
    };
    allowlist_domains
        .iter()
        .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
}

fn web_url_host(url: &str) -> Option<String> {
    let (_, tail) = url.split_once("://")?;
    let host = tail.split(['/', '?', '#']).next()?.trim();
    if host.is_empty() {
        None
    } else {
        Some(
            host.trim_start_matches("www.")
                .split(':')
                .next()
                .unwrap_or(host)
                .to_ascii_lowercase(),
        )
    }
}

fn web_url_scheme(url: &str) -> Option<&str> {
    let (scheme, _) = url.split_once("://")?;
    match scheme {
        "http" | "https" => Some(scheme),
        _ => None,
    }
}

#[derive(Debug, Clone)]
struct ParsedHttpUrl {
    scheme: String,
    host: String,
    port: u16,
    path_and_query: String,
}

impl ParsedHttpUrl {
    fn host_header(&self) -> String {
        let default_port = match self.scheme.as_str() {
            "https" => 443,
            _ => 80,
        };
        if self.port == default_port {
            self.host.clone()
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    fn path_without_query(&self) -> String {
        self.path_and_query
            .split('?')
            .next()
            .filter(|path| !path.is_empty())
            .unwrap_or("/")
            .to_string()
    }

    fn robots_url(&self) -> String {
        format!("{}://{}/robots.txt", self.scheme, self.host_header())
    }
}

#[derive(Debug)]
struct WebHttpResponse {
    status_code: u16,
    headers: BTreeMap<String, String>,
    body: String,
}

struct WebRemoteFetch {
    final_url: String,
    response: Option<WebHttpResponse>,
    redirects: Vec<Value>,
    validator: Option<WebUrlValidator>,
    robots_allowed: bool,
    robots_status: u16,
    robots_url: String,
}

#[derive(Debug, Clone)]
struct WebRemoteFetchBudget {
    max_requests: usize,
    attempted_requests: usize,
}

impl WebRemoteFetchBudget {
    fn new(max_requests: usize) -> Self {
        Self {
            max_requests,
            attempted_requests: 0,
        }
    }

    fn remaining_requests(&self) -> usize {
        self.max_requests.saturating_sub(self.attempted_requests)
    }

    fn is_exhausted(&self) -> bool {
        self.remaining_requests() == 0
    }

    fn reserve(&mut self, url: &str) -> Result<()> {
        if self.is_exhausted() {
            bail!("remote fetch rate limit exceeded before requesting {url}");
        }
        self.attempted_requests += 1;
        Ok(())
    }
}

fn parse_http_url(url: &str) -> Result<ParsedHttpUrl> {
    let (scheme, tail) = url
        .split_once("://")
        .ok_or_else(|| anyhow::anyhow!("remote fetch URL is missing scheme"))?;
    if !matches!(scheme, "http" | "https") {
        bail!("remote fetch currently supports http:// and https:// URLs only");
    }
    let (authority, path) = tail.split_once('/').unwrap_or((tail, ""));
    if authority.trim().is_empty() {
        bail!("remote fetch URL is missing host");
    }
    if authority.contains('@') {
        bail!("remote fetch URL userinfo is not supported");
    }
    let default_port = if scheme == "https" { 443 } else { 80 };
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => (
            host.to_ascii_lowercase(),
            port.parse::<u16>()
                .with_context(|| format!("invalid remote fetch port in {url}"))?,
        ),
        _ => (authority.to_ascii_lowercase(), default_port),
    };
    Ok(ParsedHttpUrl {
        scheme: scheme.to_string(),
        host,
        port,
        path_and_query: format!("/{}", path),
    })
}

fn web_fetch_remote_url(
    url: &str,
    allowlist_domains: &[String],
    url_validators: &BTreeMap<String, WebUrlValidator>,
    budget: &mut WebRemoteFetchBudget,
) -> Result<WebRemoteFetch> {
    let mut current_url = url.to_string();
    let mut redirects = Vec::new();

    for _ in 0..5 {
        if !web_url_allowed(&current_url, allowlist_domains) {
            bail!("redirect target is blocked by allowlist: {current_url}");
        }
        let parsed = parse_http_url(&current_url)?;
        let robots_url = parsed.robots_url();
        let robots_response = web_fetch_url(&robots_url, None, budget)?;
        let robots_body = if (200..300).contains(&robots_response.status_code) {
            robots_response.body.as_str()
        } else {
            ""
        };
        let robots_path = parsed.path_without_query();
        let robots_allowed = web_robots_allows(robots_body, &robots_path);
        if !robots_allowed {
            return Ok(WebRemoteFetch {
                final_url: current_url,
                response: None,
                redirects,
                validator: None,
                robots_allowed: false,
                robots_status: robots_response.status_code,
                robots_url,
            });
        }

        let validator = url_validators
            .get(&current_url)
            .or_else(|| url_validators.get(url))
            .cloned();
        let response = web_fetch_url(&current_url, validator.as_ref(), budget)?;
        if response.status_code == 304 {
            return Ok(WebRemoteFetch {
                final_url: current_url,
                response: None,
                redirects,
                validator,
                robots_allowed: true,
                robots_status: robots_response.status_code,
                robots_url,
            });
        }
        if web_is_redirect_status(response.status_code) {
            let Some(location) = response.headers.get("location") else {
                bail!("remote redirect response missing Location header");
            };
            let next_url = resolve_http_redirect_url(&current_url, location)?;
            redirects.push(json!({
                "from": current_url,
                "to": next_url,
                "http_status": response.status_code,
            }));
            current_url = next_url;
            continue;
        }
        return Ok(WebRemoteFetch {
            final_url: current_url,
            response: Some(response),
            redirects,
            validator,
            robots_allowed: true,
            robots_status: robots_response.status_code,
            robots_url,
        });
    }

    bail!("remote fetch exceeded redirect limit")
}

fn web_fetch_url(
    url: &str,
    validator: Option<&WebUrlValidator>,
    budget: &mut WebRemoteFetchBudget,
) -> Result<WebHttpResponse> {
    let parsed = parse_http_url(url)?;
    budget.reserve(url)?;
    let mut conditional_headers = String::new();
    if let Some(validator) = validator {
        if let Some(etag) = &validator.etag {
            conditional_headers.push_str(&format!("If-None-Match: {etag}\r\n"));
        }
        if let Some(last_modified) = &validator.last_modified {
            conditional_headers.push_str(&format!("If-Modified-Since: {last_modified}\r\n"));
        }
    }
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: meat-memory-web-crawler/2.97\r\nAccept: text/html,text/plain;q=0.9,*/*;q=0.1\r\n{}Connection: close\r\n\r\n",
        parsed.path_and_query,
        parsed.host_header(),
        conditional_headers
    );
    let bytes = if parsed.scheme == "https" {
        web_fetch_https_bytes(&parsed, request.as_bytes(), url)?
    } else {
        web_fetch_http_bytes(&parsed, request.as_bytes(), url)?
    };
    let response = String::from_utf8_lossy(&bytes);
    let (head, body) = response
        .split_once("\r\n\r\n")
        .or_else(|| response.split_once("\n\n"))
        .ok_or_else(|| anyhow::anyhow!("remote fetch response missing headers"))?;
    let mut lines = head.lines();
    let status_line = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("remote fetch response missing status line"))?;
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("remote fetch response missing status code"))?
        .parse::<u16>()
        .context("remote fetch response has invalid status code")?;
    let headers = lines
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect::<BTreeMap<_, _>>();

    Ok(WebHttpResponse {
        status_code,
        headers,
        body: body.to_string(),
    })
}

fn web_open_tcp_stream(parsed: &ParsedHttpUrl) -> Result<TcpStream> {
    let stream = TcpStream::connect((parsed.host.as_str(), parsed.port))
        .with_context(|| format!("failed to connect to {}", parsed.host_header()))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .context("failed to configure remote fetch read timeout")?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .context("failed to configure remote fetch write timeout")?;
    Ok(stream)
}

fn web_fetch_http_bytes(parsed: &ParsedHttpUrl, request: &[u8], url: &str) -> Result<Vec<u8>> {
    let mut stream = web_open_tcp_stream(parsed)?;
    stream
        .write_all(request)
        .with_context(|| format!("failed to send remote fetch request to {url}"))?;
    let mut bytes = Vec::new();
    stream
        .read_to_end(&mut bytes)
        .with_context(|| format!("failed to read remote fetch response from {url}"))?;
    Ok(bytes)
}

fn web_fetch_https_bytes(parsed: &ParsedHttpUrl, request: &[u8], url: &str) -> Result<Vec<u8>> {
    let root_store =
        rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .context("failed to configure HTTPS protocol versions")?
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let server_name = rustls::pki_types::ServerName::try_from(parsed.host.clone())
        .with_context(|| format!("invalid HTTPS server name: {}", parsed.host))?;
    let connection = rustls::ClientConnection::new(Arc::new(config), server_name)
        .with_context(|| format!("failed to initialize HTTPS connection to {}", parsed.host))?;
    let tcp = web_open_tcp_stream(parsed)?;
    let mut stream = rustls::StreamOwned::new(connection, tcp);
    stream
        .write_all(request)
        .with_context(|| format!("failed to send HTTPS remote fetch request to {url}"))?;
    let mut bytes = Vec::new();
    stream
        .read_to_end(&mut bytes)
        .with_context(|| format!("failed to read HTTPS remote fetch response from {url}"))?;
    Ok(bytes)
}

fn web_is_redirect_status(status_code: u16) -> bool {
    matches!(status_code, 301 | 302 | 303 | 307 | 308)
}

fn resolve_http_redirect_url(current_url: &str, location: &str) -> Result<String> {
    let location = location.trim();
    if location.starts_with("http://") {
        return Ok(location.to_string());
    }
    if location.starts_with("https://") {
        return Ok(location.to_string());
    }
    let parsed = parse_http_url(current_url)?;
    if location.starts_with('/') {
        return Ok(format!(
            "{}://{}{}",
            parsed.scheme,
            parsed.host_header(),
            location
        ));
    }
    let current_path = parsed.path_without_query();
    let current_dir = current_path
        .rsplit_once('/')
        .map(|(dir, _)| if dir.is_empty() { "/" } else { dir })
        .unwrap_or("/");
    Ok(format!(
        "{}://{}{}{}{}",
        parsed.scheme,
        parsed.host_header(),
        current_dir,
        if current_dir.ends_with('/') { "" } else { "/" },
        location
    ))
}

fn web_robots_allows(robots_text: &str, path: &str) -> bool {
    let rules = web_robots_rules_for_crawler(robots_text);
    let mut best_rule = None;

    for rule in rules {
        if !web_robots_rule_matches(&rule.pattern, path) {
            continue;
        }
        let specificity = web_robots_rule_specificity(&rule.pattern);
        let allow = matches!(rule.directive, WebRobotsDirective::Allow);
        match best_rule {
            Some((best_specificity, best_allow))
                if best_specificity > specificity
                    || (best_specificity == specificity && best_allow) => {}
            _ => best_rule = Some((specificity, allow)),
        }
    }

    best_rule.map(|(_, allow)| allow).unwrap_or(true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum WebRobotsAgentMatch {
    Wildcard,
    Exact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WebRobotsDirective {
    Allow,
    Disallow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WebRobotsRule {
    directive: WebRobotsDirective,
    pattern: String,
}

fn web_robots_rules_for_crawler(robots_text: &str) -> Vec<WebRobotsRule> {
    let mut exact_rules = Vec::new();
    let mut wildcard_rules = Vec::new();
    let mut group_agent_match = None::<WebRobotsAgentMatch>;
    let mut group_rules = Vec::new();
    let mut group_has_user_agent = false;
    let mut group_has_rules = false;
    let mut saw_exact_group = false;
    let mut saw_wildcard_group = false;

    let flush_group = |agent_match: &mut Option<WebRobotsAgentMatch>,
                       rules: &mut Vec<WebRobotsRule>,
                       exact_rules: &mut Vec<WebRobotsRule>,
                       wildcard_rules: &mut Vec<WebRobotsRule>,
                       saw_exact_group: &mut bool,
                       saw_wildcard_group: &mut bool| {
        match agent_match {
            Some(WebRobotsAgentMatch::Exact) => {
                *saw_exact_group = true;
                exact_rules.append(rules);
            }
            Some(WebRobotsAgentMatch::Wildcard) => {
                *saw_wildcard_group = true;
                wildcard_rules.append(rules);
            }
            None => rules.clear(),
        }
        *agent_match = None;
    };

    for raw_line in robots_text.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            flush_group(
                &mut group_agent_match,
                &mut group_rules,
                &mut exact_rules,
                &mut wildcard_rules,
                &mut saw_exact_group,
                &mut saw_wildcard_group,
            );
            group_has_user_agent = false;
            group_has_rules = false;
            continue;
        }
        let Some((field, value)) = line.split_once(':') else {
            continue;
        };
        let field = field.trim().to_ascii_lowercase();
        let value = value.trim();
        if field == "user-agent" {
            if group_has_rules {
                flush_group(
                    &mut group_agent_match,
                    &mut group_rules,
                    &mut exact_rules,
                    &mut wildcard_rules,
                    &mut saw_exact_group,
                    &mut saw_wildcard_group,
                );
                group_has_rules = false;
            }
            group_has_user_agent = true;
            if let Some(agent_match) = web_robots_agent_match(value) {
                group_agent_match =
                    Some(group_agent_match.map_or(agent_match, |current| current.max(agent_match)));
            }
        } else if group_has_user_agent && matches!(field.as_str(), "allow" | "disallow") {
            group_has_rules = true;
            if value.is_empty() {
                continue;
            }
            group_rules.push(WebRobotsRule {
                directive: if field == "allow" {
                    WebRobotsDirective::Allow
                } else {
                    WebRobotsDirective::Disallow
                },
                pattern: value.to_string(),
            });
        }
    }

    flush_group(
        &mut group_agent_match,
        &mut group_rules,
        &mut exact_rules,
        &mut wildcard_rules,
        &mut saw_exact_group,
        &mut saw_wildcard_group,
    );

    if saw_exact_group {
        exact_rules
    } else if saw_wildcard_group {
        wildcard_rules
    } else {
        Vec::new()
    }
}

fn web_robots_agent_match(value: &str) -> Option<WebRobotsAgentMatch> {
    if value == "*" {
        Some(WebRobotsAgentMatch::Wildcard)
    } else if value.eq_ignore_ascii_case("meat-memory-web-crawler") {
        Some(WebRobotsAgentMatch::Exact)
    } else {
        None
    }
}

fn web_robots_rule_specificity(pattern: &str) -> usize {
    pattern
        .trim_end_matches('$')
        .chars()
        .filter(|character| *character != '*')
        .count()
}

fn web_robots_rule_matches(pattern: &str, path: &str) -> bool {
    let anchored = pattern.ends_with('$');
    let pattern = pattern.trim_end_matches('$');
    if !pattern.contains('*') {
        return if anchored {
            path == pattern
        } else {
            path.starts_with(pattern)
        };
    }

    let starts_with_wildcard = pattern.starts_with('*');
    let ends_with_wildcard = pattern.ends_with('*');
    let mut remainder = path;
    let mut first_part = true;
    let mut matched_any_part = false;

    for part in pattern.split('*').filter(|part| !part.is_empty()) {
        matched_any_part = true;
        if first_part && !starts_with_wildcard {
            let Some(next_remainder) = remainder.strip_prefix(part) else {
                return false;
            };
            remainder = next_remainder;
        } else {
            let Some(position) = remainder.find(part) else {
                return false;
            };
            remainder = &remainder[position + part.len()..];
        }
        first_part = false;
    }

    if !matched_any_part {
        return true;
    }
    !anchored || ends_with_wildcard || remainder.is_empty()
}

fn html_canonical_url(html: &str) -> Option<String> {
    html_tags(html, "link")
        .into_iter()
        .find(|tag| {
            html_attribute(tag, "rel")
                .map(|value| {
                    value
                        .split_whitespace()
                        .any(|part| part.eq_ignore_ascii_case("canonical"))
                })
                .unwrap_or(false)
        })
        .and_then(|tag| html_attribute(tag, "href"))
        .or_else(|| {
            html_tags(html, "meta")
                .into_iter()
                .find(|tag| {
                    html_attribute(tag, "property")
                        .as_deref()
                        .is_some_and(|property| property.eq_ignore_ascii_case("og:url"))
                        || html_attribute(tag, "name")
                            .as_deref()
                            .is_some_and(|name| name.eq_ignore_ascii_case("og:url"))
                })
                .and_then(|tag| html_attribute(tag, "content"))
        })
}

fn html_title(html: &str) -> Option<String> {
    html_tag_text(html, "title").or_else(|| html_tag_text(html, "h1"))
}

fn html_tag_text(html: &str, tag_name: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start_tag = format!("<{tag_name}");
    let close_tag = format!("</{tag_name}>");
    let start = lower.find(&start_tag)?;
    let body_start = lower[start..].find('>').map(|offset| start + offset + 1)?;
    let body_end = lower[body_start..]
        .find(&close_tag)
        .map(|offset| body_start + offset)?;
    let text = html_visible_text(&html[body_start..body_end]);
    if text.is_empty() { None } else { Some(text) }
}

fn html_link_count(html: &str) -> usize {
    html_tags(html, "a").len()
}

fn web_link_boundary(html: &str, base_url: &str, allowlist_domains: &[String]) -> Value {
    let base_host = web_url_host(base_url);
    let mut in_scope = Vec::new();
    let mut out_of_scope = Vec::new();
    let mut blocked = Vec::new();
    let mut unsupported = Vec::new();
    let mut ignored_count = 0usize;

    for href in html_link_hrefs(html) {
        let href = href.trim();
        if href.is_empty() || href.starts_with('#') {
            ignored_count += 1;
            continue;
        }
        let Some(resolved_url) = resolve_web_link_url(base_url, href) else {
            unsupported.push(href.to_string());
            continue;
        };
        if !web_url_allowed(&resolved_url, allowlist_domains) {
            blocked.push(resolved_url);
        } else if base_host.is_some() && web_url_host(&resolved_url) == base_host {
            in_scope.push(resolved_url);
        } else {
            out_of_scope.push(resolved_url);
        }
    }

    json!({
        "base_url": base_url,
        "total_count": html_link_count(html),
        "resolved_count": in_scope.len() + out_of_scope.len() + blocked.len(),
        "in_scope_count": in_scope.len(),
        "out_of_scope_count": out_of_scope.len(),
        "blocked_count": blocked.len(),
        "unsupported_count": unsupported.len(),
        "ignored_count": ignored_count,
        "sample_in_scope": sample_strings(&in_scope, 5),
        "sample_out_of_scope": sample_strings(&out_of_scope, 5),
        "sample_blocked": sample_strings(&blocked, 5),
        "sample_unsupported": sample_strings(&unsupported, 5),
    })
}

fn web_metadata_in_scope_links(metadata: &Value) -> Vec<String> {
    metadata
        .get("link_boundary")
        .and_then(|boundary| boundary.get("sample_in_scope"))
        .and_then(Value::as_array)
        .map(|links| {
            links
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn html_link_hrefs(html: &str) -> Vec<String> {
    html_tags(html, "a")
        .into_iter()
        .filter_map(|tag| html_attribute(tag, "href"))
        .collect()
}

fn sample_strings(values: &[String], max_items: usize) -> Vec<String> {
    values.iter().take(max_items).cloned().collect()
}

fn resolve_web_link_url(base_url: &str, href: &str) -> Option<String> {
    let href = href.split('#').next().unwrap_or(href).trim();
    if href.is_empty() {
        return None;
    }
    if href.starts_with("http://") || href.starts_with("https://") {
        return Some(href.to_string());
    }
    if href.starts_with("//") {
        let scheme = web_url_scheme(base_url).unwrap_or("http");
        return Some(format!("{scheme}:{href}"));
    }
    if href.contains(':') && !href.starts_with("./") && !href.starts_with("../") {
        return None;
    }

    let parts = parse_web_url_parts(base_url)?;
    let joined = if href.starts_with('/') {
        href.to_string()
    } else if href.starts_with('?') {
        format!("{}{}", parts.path_without_query(), href)
    } else {
        let current_dir = parts
            .path_without_query()
            .rsplit_once('/')
            .map(|(dir, _)| if dir.is_empty() { "/" } else { dir })
            .unwrap_or("/");
        format!(
            "{}{}{}",
            current_dir,
            if current_dir.ends_with('/') { "" } else { "/" },
            href
        )
    };
    Some(format!(
        "{}://{}{}",
        parts.scheme,
        parts.authority,
        normalize_web_path_and_query(&joined)
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedWebUrlParts {
    scheme: String,
    authority: String,
    path_and_query: String,
}

impl ParsedWebUrlParts {
    fn path_without_query(&self) -> &str {
        self.path_and_query.split('?').next().unwrap_or("/")
    }
}

fn parse_web_url_parts(url: &str) -> Option<ParsedWebUrlParts> {
    let (scheme, tail) = url.split_once("://")?;
    if !matches!(scheme, "http" | "https") {
        return None;
    }
    let authority_end = tail.find(['/', '?', '#']).unwrap_or(tail.len());
    let authority = tail[..authority_end].trim();
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    let rest = &tail[authority_end..];
    let rest = rest.split('#').next().unwrap_or(rest);
    let path_and_query = if rest.is_empty() {
        "/".to_string()
    } else if rest.starts_with('/') {
        rest.to_string()
    } else {
        format!("/{rest}")
    };
    Some(ParsedWebUrlParts {
        scheme: scheme.to_string(),
        authority: authority.to_ascii_lowercase(),
        path_and_query,
    })
}

fn normalize_web_path_and_query(path_and_query: &str) -> String {
    let (path, query) = path_and_query
        .split_once('?')
        .map(|(path, query)| (path, Some(query)))
        .unwrap_or((path_and_query, None));
    let mut segments = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            value => segments.push(value),
        }
    }
    let normalized = format!("/{}", segments.join("/"));
    match query {
        Some(query) if !query.is_empty() => format!("{normalized}?{query}"),
        Some(_) => format!("{normalized}?"),
        None => normalized,
    }
}

fn html_tags<'a>(html: &'a str, tag_name: &str) -> Vec<&'a str> {
    let mut tags = Vec::new();
    let lower = html.to_ascii_lowercase();
    let mut offset = 0usize;
    let prefix = format!("<{tag_name}");
    while let Some(start) = lower[offset..].find(&prefix) {
        let start = offset + start;
        let after_name = lower[start + prefix.len()..].chars().next();
        if after_name.is_some_and(|value| !value.is_whitespace() && value != '>' && value != '/') {
            offset = start + prefix.len();
            continue;
        }
        let Some(end) = lower[start..].find('>') else {
            break;
        };
        let end = start + end + 1;
        tags.push(&html[start..end]);
        offset = end;
    }
    tags
}

fn html_attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let name = name.to_ascii_lowercase();
    let mut cursor = 0usize;
    while let Some(position) = lower[cursor..].find(&name) {
        let start = cursor + position;
        let before = lower[..start].chars().next_back().unwrap_or(' ');
        let after = lower[start + name.len()..].chars().next().unwrap_or(' ');
        if !before.is_ascii_alphanumeric() && matches!(after, '=' | ' ' | '\t' | '\n' | '\r') {
            let mut value_start = start + name.len();
            while lower[value_start..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
            {
                value_start += 1;
            }
            if !lower[value_start..].starts_with('=') {
                cursor = start + name.len();
                continue;
            }
            value_start += 1;
            while lower[value_start..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
            {
                value_start += 1;
            }
            let quote = tag[value_start..].chars().next()?;
            if quote == '"' || quote == '\'' {
                let value_start = value_start + quote.len_utf8();
                let value_end = tag[value_start..]
                    .find(quote)
                    .map(|offset| value_start + offset)?;
                return Some(html_decode_entities(&tag[value_start..value_end]));
            }
            let value_end = lower[value_start..]
                .find(|value: char| value.is_whitespace() || value == '>')
                .map(|offset| value_start + offset)
                .unwrap_or(tag.len());
            return Some(html_decode_entities(&tag[value_start..value_end]));
        }
        cursor = start + name.len();
    }
    None
}

fn html_visible_text(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for value in html.chars() {
        match value {
            '<' => {
                in_tag = true;
                text.push(' ');
            }
            '>' => in_tag = false,
            _ if !in_tag => text.push(value),
            _ => {}
        }
    }
    html_decode_entities(&text)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn html_decode_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn markdown_frontmatter(content_text: &str) -> Option<Value> {
    memory_sync::markdown_frontmatter(content_text)
}

fn markdown_document_title(path: &Path, content_text: &str) -> String {
    memory_sync::markdown_document_title(path, content_text)
}

fn markdown_document_metadata(content_text: &str) -> Value {
    let frontmatter = markdown_frontmatter(content_text);
    let frontmatter_present = frontmatter.is_some();
    let visible_content = markdown_content_without_frontmatter(content_text);
    json!({
        "source_kind": "markdown",
        "frontmatter_present": frontmatter_present,
        "frontmatter": frontmatter,
        "visible_content_bytes": visible_content.len(),
    })
}

fn connector_sync_plan_document_metadata(
    document: &memory_sync::LocalProjectDocumentDraft,
) -> Value {
    match document.metadata.as_object() {
        Some(metadata) if !metadata.is_empty() => document.metadata.clone(),
        _ => markdown_document_metadata(&document.content_text),
    }
}

fn markdown_content_without_frontmatter(content_text: &str) -> String {
    memory_sync::markdown_content_without_frontmatter(content_text)
}

fn connector_document_evidence_span(
    scope_id: &ScopeId,
    document: &memory_sync::LocalProjectDocumentDraft,
) -> EvidenceSpan {
    let visible_content = markdown_content_without_frontmatter(&document.content_text);
    let quote = evidence_quote(&visible_content);
    EvidenceSpan {
        id: connector_document_evidence_id(&document.content_hash),
        scope_id: scope_id.clone(),
        memory_id: None,
        artifact_id: None,
        source_ref: document.canonical_uri.clone(),
        kind: EvidenceSpanKind::Text,
        quote: quote.clone(),
        content_hash: document.content_hash.clone(),
        location: EvidenceSpanLocation::text(0, quote.len()),
        created_at: OffsetDateTime::UNIX_EPOCH,
    }
}

fn connector_document_evidence_id(content_hash: &str) -> EvidenceId {
    let suffix = content_hash
        .chars()
        .filter(|value| value.is_ascii_alphanumeric())
        .take(26)
        .collect::<String>();
    EvidenceId::from_string(format!("evd_v297_{suffix}"))
}

fn evidence_quote(content_text: &str) -> String {
    content_text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .chars()
        .take(160)
        .collect()
}

fn connector_sync_checkpoint(connector: &str, root_path: &Path) -> Value {
    let mut checkpoint = json!({
        "strategy": "canonical_uri_content_hash",
        "apply_target": "Kernel::apply_project_document_sync_plan",
        "excluded_dir_names": ["reports", ".playwright-cli", "target"],
    });

    if connector == "local-git" {
        if let Some(object) = checkpoint.as_object_mut() {
            object.insert(
                "repository_metadata".to_string(),
                local_git_repository_metadata(root_path),
            );
        }
    }

    checkpoint
}

fn local_git_repository_metadata(root_path: &Path) -> Value {
    let head_path = root_path.join(".git").join("HEAD");
    let head_ref = fs::read_to_string(&head_path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let active_branch = head_ref
        .as_deref()
        .and_then(|value| value.strip_prefix("ref: refs/heads/"))
        .map(str::to_string);
    let recent_commits = local_git_recent_commits(root_path, 5);
    let branches = local_git_branches(root_path);
    let remotes = local_git_remotes(root_path);
    let packed_refs = local_git_packed_refs(root_path);
    let refs = local_git_refs(&branches, &packed_refs);
    let worktree_status = local_git_worktree_status(root_path);
    let important_files = local_git_important_files(root_path);

    json!({
        "git_head_path": head_path.display().to_string(),
        "git_head_ref": head_ref,
        "active_branch": active_branch,
        "remote_network": false,
        "commit_count": recent_commits.len(),
        "recent_commits": recent_commits,
        "branch_count": branches.len(),
        "branches": branches,
        "remote_count": remotes.len(),
        "remotes": remotes,
        "packed_ref_count": packed_refs.len(),
        "packed_refs": packed_refs,
        "ref_count": refs.len(),
        "refs": refs,
        "worktree_status": worktree_status,
        "important_files": important_files,
    })
}

fn local_git_recent_commits(root_path: &Path, limit: usize) -> Vec<Value> {
    let log_path = root_path.join(".git").join("logs").join("HEAD");
    let Ok(log_text) = fs::read_to_string(log_path) else {
        return Vec::new();
    };

    log_text
        .lines()
        .rev()
        .filter_map(local_git_reflog_entry)
        .take(limit)
        .collect()
}

fn local_git_reflog_entry(line: &str) -> Option<Value> {
    let (header, message) = line.split_once('\t').unwrap_or((line, ""));
    let mut parts = header.split_whitespace();
    let _old_sha = parts.next()?;
    let new_sha = parts.next()?.to_string();
    let tokens = header.split_whitespace().collect::<Vec<_>>();
    let committed_at = tokens
        .len()
        .checked_sub(2)
        .and_then(|index| tokens.get(index))
        .and_then(|value| value.parse::<i64>().ok());

    Some(json!({
        "sha": new_sha,
        "committed_at_unix": committed_at,
        "message": message.trim(),
    }))
}

fn local_git_branches(root_path: &Path) -> Vec<Value> {
    let heads_dir = root_path.join(".git").join("refs").join("heads");
    let Ok(entries) = fs::read_dir(heads_dir) else {
        return Vec::new();
    };
    let mut branches = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let sha = fs::read_to_string(&path)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let ref_name = format!("refs/heads/{name}");
            Some(json!({
                "name": name,
                "ref_name": ref_name,
                "sha": sha,
                "kind": "branch",
                "source": "loose",
                "source_ref": format!("git-ref://{}", path.display()),
            }))
        })
        .collect::<Vec<_>>();
    branches.sort_by(|left, right| {
        left["name"]
            .as_str()
            .unwrap_or("")
            .cmp(right["name"].as_str().unwrap_or(""))
    });
    branches
}

fn local_git_remotes(root_path: &Path) -> Vec<Value> {
    let config_path = root_path.join(".git").join("config");
    let Ok(config_text) = fs::read_to_string(config_path) else {
        return Vec::new();
    };
    let mut remotes = Vec::new();
    let mut current_remote: Option<String> = None;

    for line in config_text.lines() {
        let trimmed = line.trim();
        if let Some(section) = trimmed
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
        {
            current_remote = section
                .strip_prefix("remote \"")
                .and_then(|value| value.strip_suffix('"'))
                .map(str::to_string);
            continue;
        }
        let Some(remote_name) = current_remote.as_ref() else {
            continue;
        };
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if key.trim() == "url" {
            remotes.push(json!({
                "name": remote_name,
                "url": value.trim(),
                "remote_network": false,
            }));
        }
    }

    remotes
}

fn local_git_packed_refs(root_path: &Path) -> Vec<Value> {
    let path = root_path.join(".git").join("packed-refs");
    let Ok(text) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('^') {
                return None;
            }
            let mut parts = trimmed.split_whitespace();
            let sha = parts.next()?.to_string();
            let name = parts.next()?.to_string();
            let short_name = local_git_ref_short_name(&name);
            let kind = local_git_ref_kind(&name);
            let source_ref = format!("git-packed-ref://{}#{}", root_path.display(), name);
            Some(json!({
                "name": name,
                "short_name": short_name,
                "sha": sha,
                "kind": kind,
                "source": "packed",
                "source_ref": source_ref,
            }))
        })
        .collect()
}

fn local_git_refs(branches: &[Value], packed_refs: &[Value]) -> Vec<Value> {
    let mut refs = BTreeMap::new();

    for packed_ref in packed_refs {
        let Some(name) = packed_ref["name"].as_str() else {
            continue;
        };
        refs.insert(
            name.to_string(),
            local_git_ref_entry(
                name,
                packed_ref["sha"].clone(),
                "packed",
                packed_ref["source_ref"].as_str(),
            ),
        );
    }

    for branch in branches {
        let Some(short_name) = branch["name"].as_str() else {
            continue;
        };
        let ref_name = branch["ref_name"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("refs/heads/{short_name}"));
        refs.insert(
            ref_name.clone(),
            local_git_ref_entry(
                &ref_name,
                branch["sha"].clone(),
                "loose",
                branch["source_ref"].as_str(),
            ),
        );
    }

    refs.into_values().collect()
}

fn local_git_ref_entry(name: &str, sha: Value, source: &str, source_ref: Option<&str>) -> Value {
    json!({
        "name": name,
        "short_name": local_git_ref_short_name(name),
        "sha": sha,
        "kind": local_git_ref_kind(name),
        "source": source,
        "source_ref": source_ref,
    })
}

fn local_git_ref_kind(name: &str) -> &'static str {
    if name.starts_with("refs/heads/") {
        "branch"
    } else if name.starts_with("refs/remotes/") {
        "remote_ref"
    } else if name.starts_with("refs/tags/") {
        "tag"
    } else {
        "other"
    }
}

fn local_git_ref_short_name(name: &str) -> String {
    ["refs/heads/", "refs/remotes/", "refs/tags/"]
        .iter()
        .find_map(|prefix| name.strip_prefix(prefix))
        .unwrap_or(name)
        .to_string()
}

fn local_git_worktree_status(root_path: &Path) -> Value {
    let index_path = root_path.join(".git").join("index");
    let index_metadata = fs::metadata(&index_path).ok();
    let git_index_present = index_metadata.is_some();
    let git_index_bytes = index_metadata.map(|metadata| metadata.len());

    match local_git_porcelain_status(root_path) {
        Ok(status_entries) => {
            let status_summary = local_git_status_summary(&status_entries);
            json!({
                "strategy": "git_status_porcelain_v1",
                "remote_network": false,
                "status_source": "git_status_porcelain_v1",
                "status_available": true,
                "git_index_present": git_index_present,
                "git_index_bytes": git_index_bytes,
                "dirty_state": if status_entries.is_empty() { "clean" } else { "dirty" },
                "status_entry_count": status_entries.len(),
                "status_entries": status_entries,
                "status_summary": status_summary,
            })
        }
        Err(error) => json!({
            "strategy": "offline_metadata_only",
            "remote_network": false,
            "status_source": "unavailable",
            "status_available": false,
            "status_error": error,
            "git_index_present": git_index_present,
            "git_index_bytes": git_index_bytes,
            "dirty_state": "unknown_offline",
            "status_entry_count": 0,
            "status_entries": [],
            "status_summary": local_git_status_summary(&[]),
        }),
    }
}

fn local_git_porcelain_status(root_path: &Path) -> std::result::Result<Vec<Value>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root_path)
        .args(["status", "--porcelain=v1", "--untracked-files=all"])
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .map_err(|error| format!("git status unavailable: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            return Err(format!("git status exited with {}", output.status));
        }
        return Err(stderr);
    }

    Ok(local_git_porcelain_entries(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn local_git_porcelain_entries(stdout: &str) -> Vec<Value> {
    stdout
        .lines()
        .filter_map(|line| {
            if line.chars().count() < 3 {
                return None;
            }
            let code = line.chars().take(2).collect::<String>();
            let path = line.chars().skip(3).collect::<String>();
            let kind = local_git_porcelain_kind(&code);
            Some(json!({
                "code": code,
                "path": path,
                "kind": kind,
            }))
        })
        .collect()
}

fn local_git_porcelain_kind(code: &str) -> &'static str {
    if code == "??" {
        "untracked"
    } else if code == "!!" {
        "ignored"
    } else if code.contains('U') {
        "conflicted"
    } else if code.contains('D') {
        "deleted"
    } else if code.contains('A') {
        "added"
    } else if code.contains('R') {
        "renamed"
    } else if code.contains('C') {
        "copied"
    } else if code.contains('M') {
        "modified"
    } else {
        "other"
    }
}

fn local_git_status_summary(status_entries: &[Value]) -> Value {
    let mut untracked_count = 0;
    let mut modified_count = 0;
    let mut deleted_count = 0;
    let mut added_count = 0;
    let mut renamed_count = 0;
    let mut copied_count = 0;
    let mut conflicted_count = 0;
    let mut other_count = 0;

    for entry in status_entries {
        match entry["kind"].as_str().unwrap_or("other") {
            "untracked" => untracked_count += 1,
            "modified" => modified_count += 1,
            "deleted" => deleted_count += 1,
            "added" => added_count += 1,
            "renamed" => renamed_count += 1,
            "copied" => copied_count += 1,
            "conflicted" => conflicted_count += 1,
            _ => other_count += 1,
        }
    }

    json!({
        "untracked_count": untracked_count,
        "modified_count": modified_count,
        "deleted_count": deleted_count,
        "added_count": added_count,
        "renamed_count": renamed_count,
        "copied_count": copied_count,
        "conflicted_count": conflicted_count,
        "other_count": other_count,
    })
}

fn local_git_important_files(root_path: &Path) -> Vec<Value> {
    [
        "README.md",
        "AGENTS.md",
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "docs/README.md",
    ]
    .iter()
    .filter_map(|relative_path| {
        let path = root_path.join(relative_path);
        if !path.is_file() {
            return None;
        }
        let metadata = fs::metadata(&path).ok()?;
        Some(json!({
            "relative_path": relative_path,
            "content_bytes": metadata.len(),
            "source_ref": format!("file://{}", path.display()),
        }))
    })
    .collect()
}

fn render_connector_sync_plan_markdown(report: &ConnectorSyncPlanReport) -> String {
    let mut output = format!(
        "# V2.97-A Connector Sync Plan\n\nschema_version: {}\nconnector: {}\nroot_path: {}\nmode: {}\nplanned_count: {}\nmissing_count: {}\nconflict_count: {}\n\n",
        report.schema_version,
        report.connector,
        report.root_path.display(),
        report.mode,
        report.planned_count,
        report.missing_count,
        report.conflict_count
    );

    output.push_str("## Planned Documents\n\n");
    for document in &report.documents {
        output.push_str(&format!(
            "- {} ({})\n  - {}\n",
            document.title, document.sync_state, document.canonical_uri
        ));
        if document.metadata["frontmatter_present"] == true {
            output.push_str("  - frontmatter: present\n");
        }
    }

    output.push_str("\n## Conflict Review\n\n");
    if report.conflicts.is_empty() {
        output.push_str("- none\n");
    } else {
        for conflict in &report.conflicts {
            output.push_str(&format!(
                "- {} ({}/{})\n  - {}\n",
                conflict.canonical_uri,
                conflict.sync_state.as_str(),
                conflict.conflict_state.as_str(),
                conflict.reason.as_deref().unwrap_or("review recommended")
            ));
        }
    }

    output.push_str("\n## Evidence Preview\n\n");
    for evidence in &report.evidence_preview {
        output.push_str(&format!(
            "- {} [{}..{}]\n  - {}\n",
            evidence.source_ref,
            evidence.location.start.unwrap_or_default(),
            evidence.location.end.unwrap_or_default(),
            evidence.quote
        ));
    }

    output.push_str("\n## Coverage Gate\n\n- V2.97-A connector sync-plan production regions require 100% targeted test coverage.\n");
    output
}

fn render_connector_dry_run_markdown(report: &ConnectorDryRunReport) -> String {
    let mut output = format!(
        "# V2.97-A Connector Dry Run\n\nschema_version: {}\nconnector: {}\nroot_path: {}\nmode: {}\nstatus: {}\ncandidate_count: {}\n\n",
        report.schema_version,
        report.connector,
        report.root_path.display(),
        report.mode,
        report.status,
        report.candidate_count
    );

    output.push_str("## Candidates\n\n");
    for item in &report.items {
        output.push_str(&format!(
            "- {} [{} bytes]\n  - {}\n",
            item.title, item.content_bytes, item.source_ref
        ));
    }

    output.push_str("\n## Failures\n\n");
    if report.failures.is_empty() {
        output.push_str("- none\n");
    } else {
        for failure in &report.failures {
            output.push_str(&format!("- {failure}\n"));
        }
    }

    output.push_str("\n## Coverage Gate\n\n- V2.97-A connector dry-run production regions require 100% targeted test coverage.\n");
    output
}

fn render_connector_import_draft_markdown(report: &ConnectorImportDraftReport) -> String {
    let mut output = format!(
        "# V2.97-A Connector Import Draft\n\nschema_version: {}\nconnector: {}\nroot_path: {}\nmode: {}\ndraft_count: {}\n\n",
        report.schema_version,
        report.connector,
        report.root_path.display(),
        report.mode,
        report.draft_count
    );

    output.push_str("## Import Policy\n\n");
    output.push_str("- explicit import only; this report does not write memory records\n");
    output.push_str("- review is required before apply\n");
    if report.import_policy["proposal_mode"] == true {
        output.push_str("- proposal draft mode is enabled; review queue semantics are projected without writing proposals\n");
    }

    output.push_str("\n## Drafts\n\n");
    for draft in &report.drafts {
        output.push_str(&format!(
            "- {} [{}]\n  - external_id: {}\n  - source_ref: {}\n",
            draft.title,
            format!("{:?}", draft.memory_kind).to_ascii_lowercase(),
            draft.external_id,
            draft.source_refs.first().cloned().unwrap_or_default()
        ));
    }

    output.push_str("\n## Proposal Drafts\n\n");
    if report.proposal_drafts.is_empty() {
        output.push_str("- none\n");
    } else {
        for proposal in &report.proposal_drafts {
            output.push_str(&format!(
                "- {} [{}]\n  - draft_external_id: {}\n  - evidence: {}\n",
                proposal.proposal_type,
                proposal.review_level,
                proposal.draft_external_id,
                proposal.evidence.first().cloned().unwrap_or_default()
            ));
        }
    }

    output.push_str("\n## Failures\n\n");
    if report.failures.is_empty() {
        output.push_str("- none\n");
    } else {
        for failure in &report.failures {
            output.push_str(&format!("- {failure}\n"));
        }
    }

    output.push_str("\n## Coverage Gate\n\n- V2.97-A connector import-draft production regions require 100% targeted test coverage.\n");
    output
}

fn render_connector_proposal_queue_markdown(report: &ConnectorProposalQueueReport) -> String {
    let mut output = format!(
        "# V2.97-A Connector Proposal Queue\n\nschema_version: {}\nqueue_id: {}\nconnector: {}\nroot_path: {}\nmode: {}\nqueue_item_count: {}\nblocked_count: {}\n\n",
        report.schema_version,
        report.queue_id,
        report.connector,
        report.root_path.display(),
        report.mode,
        report.queue_item_count,
        report.blocked_count
    );

    output.push_str("## Queue Policy\n\n");
    output
        .push_str("- review queue only; this report does not write memory or project documents\n");
    output.push_str("- review is required before apply\n");
    output.push_str("- service apply is not exposed by this compatibility endpoint\n");

    output.push_str("\n## Queue Items\n\n");
    if report.queue_items.is_empty() {
        output.push_str("- none\n");
    } else {
        for item in &report.queue_items {
            output.push_str(&format!(
                "- {} [{} / {}]\n  - id: {}\n  - review_token: {}\n  - apply_target: {}\n  - blocked: {}\n",
                item.title,
                item.proposal_type,
                item.review_level,
                item.queue_item_id,
                item.review_token,
                item.apply_target,
                item.blocked
            ));
            if let Some(block_reason) = &item.block_reason {
                output.push_str(&format!("  - block_reason: {block_reason}\n"));
            }
        }
    }

    output.push_str("\n## Failures\n\n");
    if report.failures.is_empty() {
        output.push_str("- none\n");
    } else {
        for failure in &report.failures {
            output.push_str(&format!("- {failure}\n"));
        }
    }

    output.push_str("\n## Coverage Gate\n\n- V2.97-A connector proposal-queue production regions require 100% targeted test coverage.\n");
    output
}

fn render_connector_proposal_apply_plan_markdown(
    report: &ConnectorProposalApplyPlanReport,
) -> String {
    let mut output = format!(
        "# V2.97-A Connector Proposal Apply Plan\n\nschema_version: {}\nqueue_id: {}\nconnector: {}\nroot_path: {}\nmode: {}\nselected_count: {}\napplicable_count: {}\nblocked_count: {}\n\n",
        report.schema_version,
        report.queue_id,
        report.connector,
        report.root_path.display(),
        report.mode,
        report.selected_count,
        report.applicable_count,
        report.blocked_count
    );

    output.push_str("## Apply Policy\n\n");
    output.push_str("- plan only; this report does not write memory or project documents\n");
    output.push_str("- confirmation token was verified before generating this plan\n");
    output.push_str("- runtime key and executor are still required for actual apply\n");

    output.push_str("\n## Apply Items\n\n");
    if report.apply_items.is_empty() {
        output.push_str("- none\n");
    } else {
        for item in &report.apply_items {
            output.push_str(&format!(
                "- {} [{}]\n  - id: {}\n  - apply_target: {}\n  - can_apply: {}\n",
                item.title,
                item.proposal_type,
                item.queue_item_id,
                item.apply_target,
                item.can_apply
            ));
            if let Some(block_reason) = &item.block_reason {
                output.push_str(&format!("  - block_reason: {block_reason}\n"));
            }
        }
    }

    output.push_str("\n## Skipped Items\n\n");
    if report.skipped_queue_item_ids.is_empty() {
        output.push_str("- none\n");
    } else {
        for queue_item_id in &report.skipped_queue_item_ids {
            output.push_str(&format!("- {queue_item_id}\n"));
        }
    }

    output.push_str("\n## Coverage Gate\n\n- V2.97-A connector proposal apply-plan production regions require 100% targeted test coverage.\n");
    output
}

pub fn adapt_mem0_memory_json(scope_id: ScopeId, raw: &str) -> Result<CompetitorAdapterDraft> {
    let value: Value = serde_json::from_str(raw).context("failed to parse mem0-style JSON")?;
    let external_id = required_string(&value, "/id")?;
    let body = required_string(&value, "/memory")?;
    let memory_kind = value
        .pointer("/metadata/kind")
        .and_then(Value::as_str)
        .map(parse_memory_kind)
        .transpose()?
        .unwrap_or(MemoryKind::Fact);
    let user_id = value
        .pointer("/user_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown_user");

    Ok(CompetitorAdapterDraft {
        adapter: "mem0-style-json".to_string(),
        external_id: external_id.clone(),
        scope_id,
        title: format!("mem0 memory {external_id}"),
        body,
        memory_kind,
        source_refs: vec![format!("mem0://user/{user_id}/memory/{external_id}")],
        metadata: value
            .pointer("/metadata")
            .cloned()
            .unwrap_or_else(|| json!({})),
    })
}

pub fn adapt_supermemory_document_json(
    scope_id: ScopeId,
    raw: &str,
) -> Result<CompetitorAdapterDraft> {
    let value: Value =
        serde_json::from_str(raw).context("failed to parse Supermemory-style document JSON")?;
    let external_id = required_string(&value, "/id")?;
    let container_id = required_string(&value, "/container_id")?;
    let title = required_string(&value, "/title")?;
    let body = required_string(&value, "/content")?;
    let memory_kind = value
        .pointer("/metadata/kind")
        .and_then(Value::as_str)
        .map(parse_memory_kind)
        .transpose()?
        .unwrap_or(MemoryKind::Summary);

    Ok(CompetitorAdapterDraft {
        adapter: "supermemory-style-document".to_string(),
        external_id: external_id.clone(),
        scope_id,
        title,
        body,
        memory_kind,
        source_refs: vec![format!(
            "supermemory://container/{container_id}/document/{external_id}"
        )],
        metadata: value
            .pointer("/metadata")
            .cloned()
            .unwrap_or_else(|| json!({})),
    })
}

pub fn adapt_memorylake_passport_json(
    scope_id: ScopeId,
    raw: &str,
) -> Result<CompetitorAdapterDraft> {
    let value: Value =
        serde_json::from_str(raw).context("failed to parse MemoryLake-style passport JSON")?;
    let passport_id = required_string(&value, "/manifest/passport_id")?;
    let memory = value
        .pointer("/memories/0")
        .ok_or_else(|| anyhow::anyhow!("MemoryLake-style passport has no memories"))?;
    let external_id = required_string(memory, "/id")?;
    let title = required_string(memory, "/title")?;
    let body = required_string(memory, "/body")?;
    let memory_kind = memory
        .pointer("/kind")
        .and_then(Value::as_str)
        .map(parse_memory_kind)
        .transpose()?
        .unwrap_or(MemoryKind::Fact);

    Ok(CompetitorAdapterDraft {
        adapter: "memorylake-style-passport".to_string(),
        external_id: external_id.clone(),
        scope_id,
        title,
        body,
        memory_kind,
        source_refs: vec![format!(
            "memorylake://passport/{passport_id}/memory/{external_id}"
        )],
        metadata: json!({
            "passport_id": passport_id,
            "source_scope_id": value.pointer("/manifest/source_scope_id").and_then(Value::as_str),
        }),
    })
}

pub fn build_competitor_compatibility_report(
    scope_id: ScopeId,
) -> Result<CompetitorCompatibilityReport> {
    let adapter_drafts = vec![
        adapt_mem0_memory_json(scope_id.clone(), MEM0_FIXTURE)?,
        adapt_supermemory_document_json(scope_id.clone(), SUPERMEMORY_FIXTURE)?,
        adapt_memorylake_passport_json(scope_id, MEMORYLAKE_FIXTURE)?,
    ];

    Ok(CompetitorCompatibilityReport {
        schema_version: "2.95".to_string(),
        generated_at: OffsetDateTime::now_utc(),
        markdown_compatibility: markdown_projection_compatibility(),
        mappings: competitor_capability_mappings(),
        connector_skeletons: connector_skeletons(),
        adapter_drafts,
    })
}

pub fn compatibility_report_json(report: &CompetitorCompatibilityReport) -> Value {
    json!({
        "schema_version": report.schema_version,
        "generated_at": report.generated_at,
        "markdown_compatibility": report.markdown_compatibility,
        "mappings": report.mappings,
        "connector_skeletons": report.connector_skeletons,
        "adapter_drafts": report.adapter_drafts,
        "coverage_gate": {
            "new_feature_test_coverage_required": "100%",
            "covered_regions": [
                "capability_mappings",
                "adapter_fixtures",
                "connector_skeletons",
                "cli_http_mcp_surface_parity"
            ]
        }
    })
}

pub fn write_competitor_compatibility_report(
    output_dir: &Path,
    report: &CompetitorCompatibilityReport,
) -> Result<CompetitorCompatibilityReportPaths> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;
    let json_path = output_dir.join("compatibility.json");
    let markdown_path = output_dir.join("compatibility.md");

    fs::write(
        &json_path,
        serde_json::to_string_pretty(&compatibility_report_json(report))?,
    )
    .with_context(|| format!("failed to write {}", json_path.display()))?;
    fs::write(&markdown_path, render_compatibility_markdown(report))
        .with_context(|| format!("failed to write {}", markdown_path.display()))?;

    Ok(CompetitorCompatibilityReportPaths {
        json: json_path,
        markdown: markdown_path,
    })
}

fn render_compatibility_markdown(report: &CompetitorCompatibilityReport) -> String {
    let mut output = format!(
        "# V2.95 Competitor Compatibility Report\n\nschema_version: {}\ngenerated_at: {}\n\n",
        report.schema_version, report.generated_at
    );

    output.push_str("## Markdown Compatibility\n\n");
    for item in &report.markdown_compatibility {
        output.push_str(&format!(
            "- {} -> {} ({})\n  - {}\n",
            item.legacy_projection, item.current_projection, item.compatibility_level, item.notes
        ));
    }

    output.push_str("\n## Competitor Capability Mapping\n\n");
    for mapping in &report.mappings {
        output.push_str(&format!(
            "- {}: {} -> {} ({})\n  - {}\n",
            mapping.competitor,
            mapping.external_concept,
            mapping.meat_memory_mapping,
            mapping.compatibility_level,
            mapping.notes
        ));
    }

    output.push_str("\n## Connector Skeletons\n\n");
    for connector in &report.connector_skeletons {
        output.push_str(&format!(
            "- {} [{}]: {} ({}, {})\n",
            connector.name,
            connector.source_kind,
            connector.capability,
            connector.safe_default,
            connector.status
        ));
    }

    output.push_str("\n## Adapter Fixture Drafts\n\n");
    for draft in &report.adapter_drafts {
        output.push_str(&format!(
            "- {} {} -> {} ({:?})\n",
            draft.adapter, draft.external_id, draft.title, draft.memory_kind
        ));
    }

    output.push_str("\n## Coverage Gate\n\n- V2.95 new production regions require 100% targeted test coverage.\n");
    output
}

fn mapping(
    competitor: &str,
    external_concept: &str,
    meat_memory_mapping: &str,
    compatibility_level: &str,
    notes: &str,
) -> CompetitorCapabilityMapping {
    CompetitorCapabilityMapping {
        competitor: competitor.to_string(),
        external_concept: external_concept.to_string(),
        meat_memory_mapping: meat_memory_mapping.to_string(),
        compatibility_level: compatibility_level.to_string(),
        notes: notes.to_string(),
    }
}

fn required_string(value: &Value, pointer: &str) -> Result<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|item| !item.trim().is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| anyhow::anyhow!("missing required string at {pointer}"))
}

fn parse_memory_kind(raw: &str) -> Result<MemoryKind> {
    Ok(match raw {
        "fact" | "Fact" => MemoryKind::Fact,
        "preference" | "Preference" => MemoryKind::Preference,
        "decision" | "Decision" => MemoryKind::Decision,
        "procedure" | "Procedure" => MemoryKind::Procedure,
        "constraint" | "Constraint" => MemoryKind::Constraint,
        "risk" | "Risk" => MemoryKind::Risk,
        "summary" | "Summary" => MemoryKind::Summary,
        "insight" | "Insight" => MemoryKind::Insight,
        other => bail!("unsupported memory kind in competitor fixture: {other}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::net::TcpListener;
    use std::thread;
    use std::time::{Duration, Instant};
    use tempfile::tempdir;

    #[test]
    fn v29_compat_mappings_cover_expected_competitors_and_connectors() {
        let competitors = competitor_capability_mappings()
            .into_iter()
            .map(|mapping| mapping.competitor)
            .collect::<BTreeSet<_>>();
        assert!(competitors.contains("Supermemory"));
        assert!(competitors.contains("mem0"));
        assert!(competitors.contains("MemoryLake"));

        let connectors = connector_skeletons()
            .into_iter()
            .map(|connector| connector.name)
            .collect::<BTreeSet<_>>();
        assert_eq!(connectors.len(), 7);
        assert!(connectors.contains("local-git"));
        assert!(connectors.contains("markdown-docs"));
        assert!(connectors.contains("chat-export"));
        assert!(connectors.contains("web-crawler"));
        assert!(connectors.contains("notion"));
        assert!(connectors.contains("google-drive"));
        assert!(connectors.contains("onedrive"));
    }

    #[test]
    fn v29_compat_adapters_parse_competitor_fixtures() {
        let scope_id = ScopeId::from_string("scp_v295_fixture");
        let mem0 = adapt_mem0_memory_json(scope_id.clone(), MEM0_FIXTURE).unwrap();
        let supermemory =
            adapt_supermemory_document_json(scope_id.clone(), SUPERMEMORY_FIXTURE).unwrap();
        let memorylake = adapt_memorylake_passport_json(scope_id, MEMORYLAKE_FIXTURE).unwrap();

        assert_eq!(mem0.memory_kind, MemoryKind::Preference);
        assert!(mem0.source_refs[0].starts_with("mem0://"));
        assert_eq!(supermemory.memory_kind, MemoryKind::Summary);
        assert!(supermemory.source_refs[0].contains("container_project_v295"));
        assert_eq!(memorylake.memory_kind, MemoryKind::Fact);
        assert_eq!(memorylake.metadata["passport_id"], "lake_passport_001");
    }

    #[test]
    fn v29_compat_report_writer_outputs_json_and_markdown() {
        let tempdir = tempdir().unwrap();
        let report =
            build_competitor_compatibility_report(ScopeId::from_string("scp_v295_report")).unwrap();
        let paths = write_competitor_compatibility_report(tempdir.path(), &report).unwrap();

        let json_text = fs::read_to_string(&paths.json).unwrap();
        let markdown_text = fs::read_to_string(&paths.markdown).unwrap();
        let payload: Value = serde_json::from_str(&json_text).unwrap();

        assert_eq!(payload["schema_version"], "2.95");
        assert_eq!(
            payload["coverage_gate"]["new_feature_test_coverage_required"],
            "100%"
        );
        assert!(markdown_text.contains("Supermemory"));
        assert!(markdown_text.contains("Adapter Fixture Drafts"));
    }

    #[test]
    fn v29_compat_rejects_unknown_fixture_kind() {
        let raw = r#"{"id":"bad","memory":"body","metadata":{"kind":"unknown"}}"#;
        let error = adapt_mem0_memory_json(ScopeId::from_string("scp_bad"), raw).unwrap_err();
        assert!(error.to_string().contains("unsupported memory kind"));
    }

    #[test]
    fn v297_connector_dry_run_scans_markdown_docs() {
        let tempdir = tempdir().unwrap();
        let docs_dir = tempdir.path().join("docs");
        fs::create_dir_all(&docs_dir).unwrap();
        fs::write(
            docs_dir.join("alpha-note.md"),
            "---\ntitle: Alpha Frontmatter\ntags:\n  - connector\n  - docs\nsummary: |\n  Parsed by dry-run.\n  Supports real YAML blocks.\nowner:\n  team: memory\n---\n# Alpha\n",
        )
        .unwrap();
        fs::write(docs_dir.join("ignored.txt"), "ignored").unwrap();

        let mut request = ConnectorDryRunRequest::new("markdown-docs", tempdir.path());
        request.max_items = 10;
        let report = run_connector_dry_run(request).unwrap();

        assert_eq!(report.schema_version, "2.97-A");
        assert_eq!(report.connector, "markdown-docs");
        assert_eq!(report.status, "ready");
        assert_eq!(report.candidate_count, 1);
        assert_eq!(report.items[0].title, "Alpha Frontmatter");
        assert_eq!(
            report.items[0].metadata["relative_path"],
            "docs/alpha-note.md"
        );
        assert_eq!(report.items[0].metadata["frontmatter_present"], true);
        assert_eq!(
            report.items[0].metadata["frontmatter"]["tags"][0],
            "connector"
        );
        assert_eq!(
            report.items[0].metadata["frontmatter"]["owner"]["team"],
            "memory"
        );
        assert!(
            report.items[0].metadata["frontmatter"]["summary"]
                .as_str()
                .unwrap()
                .contains("real YAML blocks")
        );
        assert_eq!(
            report.incremental_checkpoint["frontmatter"],
            "parse_yaml_frontmatter_when_present"
        );
    }

    #[test]
    fn v297_notion_connector_dry_run_scans_exported_pages() {
        let tempdir = tempdir().unwrap();
        fs::write(
            tempdir.path().join("roadmap.md"),
            "---\ntitle: Notion Roadmap\n---\n# Roadmap\n\nShip connector MVP.",
        )
        .unwrap();
        fs::write(
            tempdir.path().join("page.json"),
            serde_json::json!({
                "id": "notion_page_1",
                "title": "Notion JSON Page",
                "content": "JSON export body"
            })
            .to_string(),
        )
        .unwrap();

        let mut request = ConnectorDryRunRequest::new("notion", tempdir.path());
        request.max_items = 10;
        let report = run_connector_dry_run(request).unwrap();

        assert_eq!(report.connector, "notion");
        assert_eq!(report.status, "ready");
        assert_eq!(report.candidate_count, 2);
        assert!(
            report
                .items
                .iter()
                .any(|item| item.title == "Notion Roadmap")
        );
        let json_page = report
            .items
            .iter()
            .find(|item| item.title == "Notion JSON Page")
            .unwrap();
        assert_eq!(json_page.source_ref, "notion-export://notion_page_1");
        assert_eq!(json_page.metadata["source_kind"], "notion_page");
        assert_eq!(json_page.metadata["remote_network"], false);
        assert_eq!(
            report.incremental_checkpoint["safe_default"],
            "local_export_only_no_remote_api"
        );
    }

    #[test]
    fn v297_drive_connector_sync_plan_extracts_sidecar_text_and_failures() {
        let tempdir = tempdir().unwrap();
        fs::write(tempdir.path().join("planning.pdf"), "%PDF metadata only").unwrap();
        fs::write(
            tempdir.path().join("planning.txt"),
            "Planning PDF extracted text.",
        )
        .unwrap();
        fs::write(tempdir.path().join("missing-sidecar.pdf"), "%PDF").unwrap();
        fs::write(
            tempdir.path().join("sheet.csv"),
            "name,status\nConnector,done\n",
        )
        .unwrap();
        let mut request = ConnectorSyncPlanRequest::new(
            "google-drive",
            tempdir.path(),
            ScopeId::from_string("scp_drive_sync"),
        );
        request.max_items = 10;

        let output = build_connector_sync_plan(request).unwrap();

        assert_eq!(output.report.connector, "google-drive");
        assert_eq!(output.report.planned_count, 2);
        assert!(
            output
                .report
                .documents
                .iter()
                .any(|document| document.canonical_uri == "gdrive-export://planning")
        );
        assert!(
            output
                .report
                .documents
                .iter()
                .any(|document| document.metadata["extraction_mode"] == "pdf_sidecar_text")
        );
        assert!(
            output.report.incremental_checkpoint["failures"][0]
                .as_str()
                .unwrap()
                .contains("missing-sidecar.pdf")
        );
        assert_eq!(
            output.report.incremental_checkpoint["safe_default"],
            "local_export_only_no_remote_api"
        );
        assert_eq!(output.report.evidence_preview.len(), 2);
    }

    #[test]
    fn v297_onedrive_connector_sync_plan_marks_clean_previous_snapshot() {
        let tempdir = tempdir().unwrap();
        let path = tempdir.path().join("ops.txt");
        fs::write(&path, "OneDrive exported runbook.").unwrap();
        let content_hash =
            memory_domain::Artifact::compute_content_hash(&fs::read_to_string(&path).unwrap());
        let mut request = ConnectorSyncPlanRequest::new(
            "onedrive",
            tempdir.path(),
            ScopeId::from_string("scp_onedrive_sync"),
        );
        request.previous_snapshots = vec![ProjectDocumentSnapshot {
            canonical_uri: "onedrive-export://ops".to_string(),
            content_hash,
        }];

        let output = build_connector_sync_plan(request).unwrap();

        assert_eq!(output.report.connector, "onedrive");
        assert_eq!(output.report.planned_count, 1);
        assert_eq!(output.report.documents[0].sync_state, "clean");
        assert_eq!(output.report.missing_count, 0);
        assert_eq!(
            output.report.documents[0].metadata["source_kind"],
            "onedrive_document"
        );
    }

    #[test]
    fn v297_web_crawler_dry_run_extracts_canonical_allowlist_and_visible_text() {
        let tempdir = tempdir().unwrap();
        fs::write(tempdir.path().join("allowlist.txt"), "example.com\n").unwrap();
        fs::write(
            tempdir.path().join("index.html"),
            r#"<!doctype html>
<html>
  <head>
    <link rel="canonical" href="https://example.com/docs/start?utm_source=test" />
    <title>Start &amp; Guide</title>
  </head>
  <body>
    <h1>Fallback Heading</h1>
    <p>Visible crawler body.</p>
    <a href="/docs/next">Next</a>
    <a href="https://other.example.com/offsite">Other subdomain</a>
    <a href="https://blocked.example.net/private">Blocked</a>
    <a href="mailto:team@example.com">Email</a>
  </body>
</html>"#,
        )
        .unwrap();

        let mut request = ConnectorDryRunRequest::new("web-crawler", tempdir.path());
        request.max_items = 5;
        let report = run_connector_dry_run(request).unwrap();

        assert_eq!(report.connector, "web-crawler");
        assert_eq!(report.status, "ready");
        assert_eq!(report.candidate_count, 1);
        assert_eq!(report.items[0].title, "Start & Guide");
        assert_eq!(
            report.items[0].source_ref,
            "https://example.com/docs/start?utm_source=test"
        );
        assert_eq!(
            report.items[0].metadata["canonical_url"],
            "https://example.com/docs/start?utm_source=test"
        );
        assert_eq!(report.items[0].metadata["allowlist_allowed"], true);
        assert_eq!(
            report.items[0].metadata["allowlist_domains"][0],
            "example.com"
        );
        assert_eq!(report.items[0].metadata["link_count"], 4);
        assert_eq!(
            report.items[0].metadata["link_boundary"]["in_scope_count"],
            1
        );
        assert_eq!(
            report.items[0].metadata["link_boundary"]["sample_in_scope"][0],
            "https://example.com/docs/next"
        );
        assert_eq!(
            report.items[0].metadata["link_boundary"]["out_of_scope_count"],
            1
        );
        assert_eq!(
            report.items[0].metadata["link_boundary"]["blocked_count"],
            1
        );
        assert_eq!(
            report.items[0].metadata["link_boundary"]["unsupported_count"],
            1
        );
        assert_eq!(report.items[0].metadata["remote_network"], false);
        assert!(
            report.items[0].metadata["visible_text_bytes"]
                .as_u64()
                .unwrap()
                > 20
        );
        assert_eq!(report.incremental_checkpoint["remote_network"], false);
    }

    #[test]
    fn v297_web_crawler_dry_run_reports_allowlist_blocks() {
        let tempdir = tempdir().unwrap();
        fs::write(tempdir.path().join("allowlist.txt"), "example.com\n").unwrap();
        fs::write(
            tempdir.path().join("blocked.html"),
            r#"<!doctype html>
<html>
  <head>
    <meta property="OG:URL" content="https://blocked.example.net/private" />
    <title>Blocked Snapshot</title>
  </head>
  <body>
    <article>Article text should not count as a link.</article>
  </body>
</html>"#,
        )
        .unwrap();

        let report =
            run_connector_dry_run(ConnectorDryRunRequest::new("web-crawler", tempdir.path()))
                .unwrap();

        assert_eq!(report.connector, "web-crawler");
        assert_eq!(report.status, "needs_attention");
        assert_eq!(report.candidate_count, 1);
        assert_eq!(report.items[0].metadata["allowlist_allowed"], false);
        assert_eq!(report.items[0].metadata["link_count"], 0);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("blocked by allowlist"))
        );
    }

    #[test]
    fn v297_web_crawler_sync_plan_builds_project_document_plan() {
        let tempdir = tempdir().unwrap();
        fs::write(tempdir.path().join("allowlist.txt"), "example.com\n").unwrap();
        fs::write(
            tempdir.path().join("index.html"),
            r#"<!doctype html>
<html>
  <head>
    <link rel="canonical" href="https://example.com/docs/v297-sync" />
    <title>V2.97 Web Sync</title>
  </head>
  <body>
    <main>
      <p>Visible web sync body for project documents.</p>
      <a href="/next">Next</a>
    </main>
  </body>
</html>"#,
        )
        .unwrap();

        let output = build_connector_sync_plan(ConnectorSyncPlanRequest::new(
            "web-crawler",
            tempdir.path(),
            ScopeId::from_string("scp_connector_sync"),
        ))
        .unwrap();

        assert_eq!(output.report.connector, "web-crawler");
        assert_eq!(output.report.mode, "sync_plan");
        assert_eq!(output.report.planned_count, 1);
        assert_eq!(output.report.documents[0].title, "V2.97 Web Sync");
        assert_eq!(
            output.report.documents[0].canonical_uri,
            "https://example.com/docs/v297-sync"
        );
        assert_eq!(output.report.documents[0].sync_state, "changed");
        assert_eq!(
            output.report.documents[0].metadata["fetch_policy"],
            "local_snapshot_only_no_remote_fetch"
        );
        assert_eq!(
            output.report.documents[0].metadata["allowlist_allowed"],
            true
        );
        assert_eq!(output.report.documents[0].metadata["link_count"], 1);
        assert_eq!(output.report.incremental_checkpoint["blocked_count"], 0);
        assert_eq!(
            output.report.incremental_checkpoint["remote_network"],
            false
        );
        assert_eq!(output.report.evidence_preview[0].quote, "# V2.97 Web Sync");
        assert!(
            output.plan.documents[0]
                .content_text
                .contains("Visible web sync body")
        );
    }

    #[test]
    fn v297_web_crawler_sync_plan_blocks_disallowed_urls_and_records_conflict() {
        let tempdir = tempdir().unwrap();
        fs::write(tempdir.path().join("allowlist.txt"), "example.com\n").unwrap();
        fs::write(
            tempdir.path().join("blocked.html"),
            r#"<!doctype html>
<html>
  <head>
    <meta name="og:url" content="https://blocked.example.net/private" />
    <title>Blocked Web Sync</title>
  </head>
  <body>Blocked page body.</body>
</html>"#,
        )
        .unwrap();

        let mut request = ConnectorSyncPlanRequest::new(
            "web-crawler",
            tempdir.path(),
            ScopeId::from_string("scp_connector_sync"),
        );
        request.previous_snapshots = vec![ProjectDocumentSnapshot {
            canonical_uri: "https://blocked.example.net/private".to_string(),
            content_hash: "previous".to_string(),
        }];
        let output = build_connector_sync_plan(request).unwrap();

        assert_eq!(output.report.connector, "web-crawler");
        assert_eq!(output.report.planned_count, 0);
        assert_eq!(output.report.missing_count, 0);
        assert_eq!(output.report.conflict_count, 1);
        assert_eq!(output.report.incremental_checkpoint["blocked_count"], 1);
        assert_eq!(
            output.report.incremental_checkpoint["blocked_urls"][0]["canonical_url"],
            "https://blocked.example.net/private"
        );
        assert_eq!(
            output.report.conflicts[0].canonical_uri,
            "https://blocked.example.net/private"
        );
    }

    #[test]
    fn v297_web_crawler_remote_fetch_records_robots_and_validators() {
        let tempdir = tempdir().unwrap();
        fs::write(tempdir.path().join("allowlist.txt"), "127.0.0.1\n").unwrap();
        let (base_url, handle) = spawn_web_fixture_server(
            "User-agent: *\nDisallow: /private\n",
            |url| {
                format!(
                    "<!doctype html><html><head><link rel=\"canonical\" href=\"{url}\" /><title>Fetched Page</title></head><body><p>Fetched remote web body.</p><a href=\"/next\">Next</a></body></html>"
                )
            },
            2,
        );
        let page_url = format!("{base_url}/docs/page");
        fs::write(tempdir.path().join("urls.txt"), format!("{page_url}\n")).unwrap();
        let mut request = ConnectorSyncPlanRequest::new(
            "web-crawler",
            tempdir.path(),
            ScopeId::from_string("scp_connector_sync"),
        );
        request.allow_remote_fetch = true;
        request.max_items = 1;
        let output = build_connector_sync_plan(request).unwrap();
        handle.join().unwrap();

        assert_eq!(output.report.connector, "web-crawler");
        assert_eq!(output.report.planned_count, 1);
        assert_eq!(output.report.documents[0].title, "Fetched Page");
        assert_eq!(output.report.documents[0].canonical_uri, page_url);
        assert_eq!(
            output.report.documents[0].metadata["fetch_policy"],
            "remote_fetch_opt_in"
        );
        assert_eq!(output.report.documents[0].metadata["remote_network"], true);
        assert_eq!(output.report.documents[0].metadata["robots_allowed"], true);
        assert_eq!(
            output.report.documents[0].metadata["link_boundary"]["in_scope_count"],
            1
        );
        assert_eq!(output.report.documents[0].metadata["etag"], "\"v297\"");
        assert_eq!(
            output.report.documents[0].metadata["last_modified"],
            "Sun, 10 May 2026 00:00:00 GMT"
        );
        assert_eq!(
            output.report.incremental_checkpoint["remote_fetch_allowed"],
            true
        );
        assert_eq!(output.report.incremental_checkpoint["remote_network"], true);
        assert_eq!(
            output.report.incremental_checkpoint["update_detection"][2],
            "etag"
        );
        assert_eq!(
            output.report.incremental_checkpoint["crawl_queue_remaining"],
            1
        );
        assert_eq!(
            output.report.incremental_checkpoint["crawl_next_frontier_urls"][0],
            format!("{base_url}/next")
        );
    }

    #[test]
    fn v297_web_crawler_remote_fetch_blocks_robots_disallow() {
        let tempdir = tempdir().unwrap();
        fs::write(tempdir.path().join("allowlist.txt"), "127.0.0.1\n").unwrap();
        let (base_url, handle) = spawn_web_fixture_server(
            "User-agent: *\nDisallow: /private\n",
            |_| "<!doctype html><title>Should Not Fetch</title>".to_string(),
            1,
        );
        let page_url = format!("{base_url}/private/page");
        fs::write(tempdir.path().join("urls.txt"), format!("{page_url}\n")).unwrap();
        let mut request = ConnectorSyncPlanRequest::new(
            "web-crawler",
            tempdir.path(),
            ScopeId::from_string("scp_connector_sync"),
        );
        request.allow_remote_fetch = true;
        let output = build_connector_sync_plan(request).unwrap();
        handle.join().unwrap();

        assert_eq!(output.report.planned_count, 0);
        assert_eq!(output.report.incremental_checkpoint["blocked_count"], 1);
        assert_eq!(
            output.report.incremental_checkpoint["blocked_urls"][0]["canonical_url"],
            page_url
        );
        assert_eq!(
            output.report.incremental_checkpoint["remote_fetch_allowed"],
            true
        );
    }

    #[test]
    fn v297_web_crawler_robots_rules_prefer_exact_user_agent() {
        let robots = "\
User-agent: *
Disallow: /

User-agent: meat-memory-web-crawler
Allow: /
";

        assert!(web_robots_allows(robots, "/docs/page"));
    }

    #[test]
    fn v297_web_crawler_robots_rules_keep_empty_exact_group() {
        let robots = "\
User-agent: *
Disallow: /

User-agent: meat-memory-web-crawler
Disallow:
";

        assert!(web_robots_allows(robots, "/docs/page"));
    }

    #[test]
    fn v297_web_crawler_robots_rules_use_allow_longest_match() {
        let robots = "\
User-agent: *
User-agent: other-crawler
Disallow: /docs
Allow: /docs/public
";

        assert!(web_robots_allows(robots, "/docs/public/page"));
        assert!(!web_robots_allows(robots, "/docs/private/page"));
    }

    #[test]
    fn v297_web_crawler_robots_rules_match_wildcard_and_end_anchor() {
        let robots = "\
User-agent: *
Disallow: /*.pdf$
";

        assert!(!web_robots_allows(robots, "/files/report.pdf"));
        assert!(web_robots_allows(robots, "/files/report.pdf/preview"));
    }

    #[test]
    fn v297_web_crawler_link_boundary_normalizes_relative_urls() {
        let html = r#"
<a href="guide">Guide</a>
<a href="?page=2">Page</a>
<a href="//cdn.example.com/asset">CDN</a>
<a href="javascript:void(0)">JS</a>
"#;

        let boundary = web_link_boundary(
            html,
            "https://example.com/docs/start/index.html",
            &["example.com".to_string()],
        );

        assert_eq!(boundary["in_scope_count"], 2);
        assert_eq!(
            boundary["sample_in_scope"][0],
            "https://example.com/docs/start/guide"
        );
        assert_eq!(
            boundary["sample_in_scope"][1],
            "https://example.com/docs/start/index.html?page=2"
        );
        assert_eq!(boundary["out_of_scope_count"], 1);
        assert_eq!(
            boundary["sample_out_of_scope"][0],
            "https://cdn.example.com/asset"
        );
        assert_eq!(boundary["unsupported_count"], 1);
    }

    #[test]
    fn v297_web_crawler_multi_page_crawl_follows_in_scope_links() {
        let tempdir = tempdir().unwrap();
        fs::write(tempdir.path().join("allowlist.txt"), "127.0.0.1\n").unwrap();
        let (base_url, handle) = spawn_web_fixture_server_with_responder(
            4,
            |request, server_base_url| {
                let first_line = request.lines().next().unwrap_or("");
                if first_line.contains("GET /robots.txt ") {
                    return http_response("text/plain", "User-agent: *\nAllow: /\n", &[]);
                }
                if first_line.contains("GET /start ") {
                    return http_response(
                        "text/html",
                        &format!(
                            "<!doctype html><html><head><link rel=\"canonical\" href=\"{server_base_url}/start\" /><title>Start Page</title></head><body><p>Start body.</p><a href=\"/second\">Second</a><a href=\"https://blocked.example.net/private\">Blocked</a></body></html>"
                        ),
                        &[("ETag", "\"start\"")],
                    );
                }
                if first_line.contains("GET /second ") {
                    return http_response(
                        "text/html",
                        &format!(
                            "<!doctype html><html><head><link rel=\"canonical\" href=\"{server_base_url}/second\" /><title>Second Page</title></head><body><p>Second body.</p></body></html>"
                        ),
                        &[("ETag", "\"second\"")],
                    );
                }
                panic!("unexpected fixture request: {first_line}");
            },
        );
        let start_url = format!("{base_url}/start");
        let second_url = format!("{base_url}/second");
        fs::write(tempdir.path().join("urls.txt"), format!("{start_url}\n")).unwrap();
        let mut request = ConnectorSyncPlanRequest::new(
            "web-crawler",
            tempdir.path(),
            ScopeId::from_string("scp_connector_sync"),
        );
        request.allow_remote_fetch = true;
        let output = build_connector_sync_plan(request).unwrap();
        handle.join().unwrap();

        assert_eq!(output.report.planned_count, 2);
        assert_eq!(output.report.documents[0].canonical_uri, start_url);
        assert_eq!(output.report.documents[1].canonical_uri, second_url);
        assert_eq!(output.report.documents[1].title, "Second Page");
        assert_eq!(
            output.report.incremental_checkpoint["crawl_policy"],
            "urls_txt_seed_plus_in_scope_links"
        );
        assert_eq!(output.report.incremental_checkpoint["crawl_seed_count"], 1);
        assert_eq!(
            output.report.incremental_checkpoint["crawl_discovered_count"],
            1
        );
        assert_eq!(
            output.report.incremental_checkpoint["crawl_discovered_urls"][0],
            second_url
        );
        assert_eq!(
            output.report.incremental_checkpoint["crawl_queue_remaining"],
            0
        );
        let payload = connector_sync_plan_json(&output.report);
        assert!(
            payload["coverage_gate"]["covered_regions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|region| region == "web_crawler_multi_page_crawl")
        );
    }

    #[test]
    fn v297_web_crawler_remote_fetch_follows_redirects() {
        let tempdir = tempdir().unwrap();
        fs::write(tempdir.path().join("allowlist.txt"), "127.0.0.1\n").unwrap();
        let (base_url, handle) = spawn_web_fixture_server_with_responder(
            4,
            |request, server_base_url| {
                let first_line = request.lines().next().unwrap_or("");
                if first_line.contains("GET /robots.txt ") {
                    return http_response("text/plain", "User-agent: *\nAllow: /\n", &[]);
                }
                if first_line.contains("GET /start ") {
                    return http_status_response(
                        "302 Found",
                        "text/plain",
                        "",
                        &[("Location", "/final")],
                    );
                }
                if first_line.contains("GET /final ") {
                    return http_response(
                        "text/html",
                        &format!(
                            "<!doctype html><html><head><link rel=\"canonical\" href=\"{server_base_url}/final\" /><title>Redirected Page</title></head><body><p>Redirected body.</p></body></html>"
                        ),
                        &[("ETag", "\"redirected\"")],
                    );
                }
                panic!("unexpected fixture request: {first_line}");
            },
        );
        let start_url = format!("{base_url}/start");
        let final_url = format!("{base_url}/final");
        fs::write(tempdir.path().join("urls.txt"), format!("{start_url}\n")).unwrap();
        let mut request = ConnectorSyncPlanRequest::new(
            "web-crawler",
            tempdir.path(),
            ScopeId::from_string("scp_connector_sync"),
        );
        request.allow_remote_fetch = true;
        let output = build_connector_sync_plan(request).unwrap();
        handle.join().unwrap();

        assert_eq!(output.report.planned_count, 1);
        assert_eq!(output.report.documents[0].canonical_uri, final_url);
        assert_eq!(output.report.documents[0].metadata["redirect_count"], 1);
        assert_eq!(output.report.documents[0].metadata["final_url"], final_url);
        assert_eq!(
            output.report.documents[0].metadata["redirects"][0]["from"],
            start_url
        );
        assert_eq!(
            output.report.documents[0].metadata["redirects"][0]["to"],
            final_url
        );
        assert_eq!(output.report.incremental_checkpoint["redirect_count"], 1);
    }

    #[test]
    fn v297_web_crawler_remote_fetch_sends_conditional_validators() {
        let tempdir = tempdir().unwrap();
        fs::write(tempdir.path().join("allowlist.txt"), "127.0.0.1\n").unwrap();
        let (base_url, handle) =
            spawn_web_fixture_server_with_responder(2, |request, _server_base_url| {
                let first_line = request.lines().next().unwrap_or("");
                if first_line.contains("GET /robots.txt ") {
                    return http_response("text/plain", "User-agent: *\nAllow: /\n", &[]);
                }
                assert!(first_line.contains("GET /docs/page "));
                assert!(
                    request.contains("If-None-Match: \"cached\""),
                    "expected conditional ETag header in request: {request}"
                );
                assert!(
                    request.contains("If-Modified-Since: Sun, 10 May 2026 00:00:00 GMT"),
                    "expected conditional Last-Modified header in request: {request}"
                );
                http_status_response("304 Not Modified", "text/plain", "", &[])
            });
        let page_url = format!("{base_url}/docs/page");
        fs::write(tempdir.path().join("urls.txt"), format!("{page_url}\n")).unwrap();
        let mut validators = serde_json::Map::new();
        validators.insert(
            page_url.clone(),
            json!({
                "etag": "\"cached\"",
                "last_modified": "Sun, 10 May 2026 00:00:00 GMT",
            }),
        );
        fs::write(
            tempdir.path().join("url-validators.json"),
            Value::Object(validators).to_string(),
        )
        .unwrap();
        let mut request = ConnectorSyncPlanRequest::new(
            "web-crawler",
            tempdir.path(),
            ScopeId::from_string("scp_connector_sync"),
        );
        request.allow_remote_fetch = true;
        let output = build_connector_sync_plan(request).unwrap();
        handle.join().unwrap();

        assert_eq!(output.report.planned_count, 0);
        assert_eq!(
            output.report.incremental_checkpoint["validator_manifest_count"],
            1
        );
        assert_eq!(
            output.report.incremental_checkpoint["not_modified_count"],
            1
        );
        assert_eq!(
            output.report.incremental_checkpoint["not_modified_urls"][0]["canonical_url"],
            page_url
        );
        assert_eq!(
            output.report.incremental_checkpoint["not_modified_urls"][0]["conditional_request"],
            true
        );
        assert_eq!(
            output.report.incremental_checkpoint["not_modified_urls"][0]["etag"],
            "\"cached\""
        );
    }

    #[test]
    fn v297_web_crawler_https_urls_preserve_scheme_for_robots_and_redirects() {
        let parsed = parse_http_url("https://Example.com/docs/page?x=1").unwrap();

        assert_eq!(parsed.scheme, "https");
        assert_eq!(parsed.host, "example.com");
        assert_eq!(parsed.port, 443);
        assert_eq!(parsed.host_header(), "example.com");
        assert_eq!(parsed.robots_url(), "https://example.com/robots.txt");
        assert_eq!(
            resolve_http_redirect_url("https://example.com/docs/start", "/next").unwrap(),
            "https://example.com/next"
        );
        assert_eq!(
            resolve_http_redirect_url(
                "https://example.com/docs/start",
                "https://cdn.example.com/final"
            )
            .unwrap(),
            "https://cdn.example.com/final"
        );
    }

    #[test]
    fn v297_web_crawler_remote_fetch_rate_limit_preserves_frontier() {
        let tempdir = tempdir().unwrap();
        fs::write(tempdir.path().join("allowlist.txt"), "127.0.0.1\n").unwrap();
        let (base_url, handle) = spawn_web_fixture_server_with_responder(
            WEB_CRAWLER_REMOTE_FETCH_MAX_REQUESTS_PER_RUN,
            |request, server_base_url| {
                let first_line = request.lines().next().unwrap_or("");
                if first_line.contains("GET /robots.txt ") {
                    return http_response("text/plain", "User-agent: *\nAllow: /\n", &[]);
                }
                let path = first_line.split_whitespace().nth(1).unwrap_or("/");
                let page_index = path
                    .trim_start_matches("/page")
                    .parse::<usize>()
                    .unwrap_or(0);
                http_response(
                    "text/html",
                    &format!(
                        "<!doctype html><html><head><link rel=\"canonical\" href=\"{server_base_url}/page{page_index}\" /><title>Page {page_index}</title></head><body><p>Page body.</p><a href=\"/page{}\">Next</a></body></html>",
                        page_index + 1
                    ),
                    &[("ETag", "\"rate-limit\"")],
                )
            },
        );
        let start_url = format!("{base_url}/page0");
        let next_frontier_url = format!(
            "{base_url}/page{}",
            WEB_CRAWLER_REMOTE_FETCH_MAX_REQUESTS_PER_RUN / 2
        );
        fs::write(tempdir.path().join("urls.txt"), format!("{start_url}\n")).unwrap();
        let mut request = ConnectorSyncPlanRequest::new(
            "web-crawler",
            tempdir.path(),
            ScopeId::from_string("scp_connector_sync"),
        );
        request.allow_remote_fetch = true;
        request.max_items = 100;
        let output = build_connector_sync_plan(request).unwrap();
        handle.join().unwrap();

        assert_eq!(
            output.report.planned_count,
            WEB_CRAWLER_REMOTE_FETCH_MAX_REQUESTS_PER_RUN / 2
        );
        assert_eq!(
            output.report.incremental_checkpoint["remote_fetch_rate_limit_policy"],
            "max_requests_per_sync_plan"
        );
        assert_eq!(
            output.report.incremental_checkpoint["remote_fetch_attempted_requests"],
            WEB_CRAWLER_REMOTE_FETCH_MAX_REQUESTS_PER_RUN
        );
        assert_eq!(
            output.report.incremental_checkpoint["remote_fetch_rate_limited"],
            true
        );
        assert_eq!(
            output.report.incremental_checkpoint["crawl_next_frontier_urls"][0],
            next_frontier_url
        );
        let payload = connector_sync_plan_json(&output.report);
        assert!(
            payload["coverage_gate"]["covered_regions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|region| region == "web_crawler_remote_fetch_rate_limit")
        );
    }

    fn spawn_web_fixture_server<F>(
        robots_body: &'static str,
        page_body: F,
        expected_requests: usize,
    ) -> (String, thread::JoinHandle<()>)
    where
        F: Fn(&str) -> String + Send + 'static,
    {
        spawn_web_fixture_server_with_responder(
            expected_requests,
            move |request, server_base_url| {
                let first_line = request.lines().next().unwrap_or("");
                if first_line.contains("GET /robots.txt ") {
                    http_response("text/plain", robots_body, &[])
                } else {
                    let path = first_line.split_whitespace().nth(1).unwrap_or("/");
                    let url = format!("{server_base_url}{path}");
                    let body = page_body(&url);
                    http_response(
                        "text/html",
                        &body,
                        &[
                            ("ETag", "\"v297\""),
                            ("Last-Modified", "Sun, 10 May 2026 00:00:00 GMT"),
                        ],
                    )
                }
            },
        )
    }

    fn spawn_web_fixture_server_with_responder<F>(
        expected_requests: usize,
        responder: F,
    ) -> (String, thread::JoinHandle<()>)
    where
        F: Fn(&str, &str) -> String + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);
        let server_base_url = base_url.clone();
        let handle = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut served = 0usize;
            while served < expected_requests {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        use std::io::{Read as _, Write as _};
                        let mut buffer = [0_u8; 4096];
                        let bytes_read = stream.read(&mut buffer).unwrap_or(0);
                        let request = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                        let response = responder(&request, &server_base_url);
                        stream.write_all(response.as_bytes()).unwrap();
                        served += 1;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if Instant::now() >= deadline {
                            panic!("test web fixture server timed out");
                        }
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("test web fixture server failed: {error}"),
                }
            }
        });
        (base_url, handle)
    }

    fn http_response(content_type: &str, body: &str, headers: &[(&str, &str)]) -> String {
        http_status_response("200 OK", content_type, body, headers)
    }

    fn http_status_response(
        status: &str,
        content_type: &str,
        body: &str,
        headers: &[(&str, &str)],
    ) -> String {
        let mut response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n",
            body.len()
        );
        for (name, value) in headers {
            response.push_str(&format!("{name}: {value}\r\n"));
        }
        response.push_str("\r\n");
        response.push_str(body);
        response
    }

    #[test]
    fn v297_local_git_dry_run_reports_repository_candidates_and_failures() {
        let tempdir = tempdir().unwrap();
        fs::write(tempdir.path().join("README.md"), "# Project\n").unwrap();

        let report =
            run_connector_dry_run(ConnectorDryRunRequest::new("local-git", tempdir.path()))
                .unwrap();

        assert_eq!(report.connector, "local-git");
        assert_eq!(report.status, "needs_attention");
        assert_eq!(report.candidate_count, 1);
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains("missing .git"))
        );
        assert_eq!(report.incremental_checkpoint["remote_network"], false);
        assert_eq!(
            report.incremental_checkpoint["repository_metadata"]["important_files"][0]["relative_path"],
            "README.md"
        );
    }

    #[test]
    fn v297_chat_export_dry_run_parses_generic_messages_json() {
        let tempdir = tempdir().unwrap();
        fs::write(
            tempdir.path().join("chat.json"),
            r#"{
              "conversations": [
                {
                  "id": "conv_1",
                  "title": "Launch planning",
                  "messages": [
                    {"role": "user", "content": "Ship the connector."},
                    {"role": "assistant", "content": "Drafting a plan."}
                  ]
                }
              ]
            }"#,
        )
        .unwrap();

        let report =
            run_connector_dry_run(ConnectorDryRunRequest::new("chat-export", tempdir.path()))
                .unwrap();

        assert_eq!(report.connector, "chat-export");
        assert_eq!(report.status, "ready");
        assert_eq!(report.candidate_count, 1);
        assert_eq!(report.items[0].title, "Launch planning");
        assert_eq!(report.items[0].metadata["message_count"], 2);
        assert_eq!(report.items[0].metadata["format"], "generic-messages-json");
        assert!(report.items[0].source_ref.ends_with("chat.json#conv_1"));
    }

    #[test]
    fn v297_chat_export_dry_run_parses_chatgpt_mapping_json() {
        let tempdir = tempdir().unwrap();
        fs::write(
            tempdir.path().join("conversations.json"),
            r#"[
              {
                "id": "chatgpt_1",
                "title": "Research thread",
                "mapping": {
                  "a": {
                    "message": {
                      "author": {"role": "user"},
                      "content": {"parts": ["Find connector gaps."]},
                      "create_time": 1.0
                    }
                  },
                  "b": {
                    "message": {
                      "author": {"role": "assistant"},
                      "content": {"parts": ["Supermemory leads on SaaS connectors."]},
                      "create_time": 2.0
                    }
                  }
                }
              }
            ]"#,
        )
        .unwrap();

        let report =
            run_connector_dry_run(ConnectorDryRunRequest::new("chat-export", tempdir.path()))
                .unwrap();

        assert_eq!(report.candidate_count, 1);
        assert_eq!(report.items[0].title, "Research thread");
        assert_eq!(report.items[0].metadata["message_count"], 2);
        assert_eq!(
            report.items[0].metadata["format"],
            "chatgpt-conversations-json"
        );
        assert_eq!(
            report.incremental_checkpoint["safe_default"],
            "explicit_import_only"
        );
    }

    #[test]
    fn v297_chat_export_import_draft_projects_reviewable_memory_drafts() {
        let tempdir = tempdir().unwrap();
        fs::write(
            tempdir.path().join("chat.json"),
            r#"{
              "id": "conv_import",
              "title": "Release decision",
              "messages": [
                {"role": "user", "content": "Should we ship V2.97-A?"},
                {"role": "assistant", "content": "Ship after connector tests pass."}
              ]
            }"#,
        )
        .unwrap();

        let mut request = ConnectorImportDraftRequest::new(
            "chat-export",
            tempdir.path(),
            ScopeId::from_string("scp_chat_import"),
        );
        request.proposal_mode = true;
        let report = build_connector_import_draft_report(request).unwrap();

        assert_eq!(report.connector, "chat-export");
        assert_eq!(report.mode, "import_draft");
        assert_eq!(report.draft_count, 1);
        assert_eq!(report.proposal_draft_count, 1);
        assert_eq!(report.drafts[0].title, "Release decision");
        assert_eq!(report.drafts[0].memory_kind, MemoryKind::Summary);
        assert_eq!(report.drafts[0].scope_id.as_str(), "scp_chat_import");
        assert!(report.drafts[0].body.contains("user: Should we ship"));
        assert_eq!(report.drafts[0].metadata["message_count"], 2);
        assert_eq!(report.proposal_drafts[0].proposal_type, "distill_upsert");
        assert_eq!(report.proposal_drafts[0].review_level, "required");
        assert_eq!(report.proposal_drafts[0].draft_external_id, "conv_import");
        assert_eq!(report.import_policy["writes_memory"], false);
        assert_eq!(report.import_policy["proposal_mode"], true);
    }

    #[test]
    fn v297_connector_import_draft_writer_outputs_json_and_markdown() {
        let tempdir = tempdir().unwrap();
        fs::write(
            tempdir.path().join("chat.json"),
            r#"{"title":"Draft me","messages":[{"role":"user","content":"hello"}]}"#,
        )
        .unwrap();
        let report = build_connector_import_draft_report(ConnectorImportDraftRequest::new(
            "chat-export",
            tempdir.path(),
            ScopeId::from_string("scp_chat_import"),
        ))
        .unwrap();
        let output_dir = tempdir.path().join("reports");
        let paths = write_connector_import_draft_report(&output_dir, &report).unwrap();

        let json_text = fs::read_to_string(&paths.json).unwrap();
        let markdown_text = fs::read_to_string(&paths.markdown).unwrap();
        let payload: Value = serde_json::from_str(&json_text).unwrap();

        assert_eq!(payload["schema_version"], "2.97-A");
        assert_eq!(
            payload["coverage_gate"]["new_feature_test_coverage_required"],
            "100%"
        );
        assert!(markdown_text.contains("Connector Import Draft"));
        assert!(markdown_text.contains("explicit import only"));
    }

    #[test]
    fn v297_connector_proposal_queue_projects_chat_export_review_items() {
        let tempdir = tempdir().unwrap();
        fs::write(
            tempdir.path().join("chat.json"),
            r#"{
              "id": "conv_queue",
              "title": "Queue decision",
              "messages": [
                {"role": "user", "content": "Should this be queued?"},
                {"role": "assistant", "content": "Queue it for review."}
              ]
            }"#,
        )
        .unwrap();

        let report = build_connector_proposal_queue_report(ConnectorProposalQueueRequest::new(
            "chat-export",
            tempdir.path(),
            ScopeId::from_string("scp_chat_queue"),
        ))
        .unwrap();
        let payload = connector_proposal_queue_json(&report);

        assert_eq!(report.connector, "chat-export");
        assert!(report.queue_id.starts_with("cpq_chat-export_"));
        assert_eq!(report.mode, "proposal_queue");
        assert_eq!(report.queue_item_count, 1);
        assert_eq!(report.blocked_count, 0);
        assert!(report.queue_items[0].review_token.starts_with("confirm_"));
        assert_eq!(report.queue_items[0].proposal_type, "distill_upsert");
        assert_eq!(report.queue_items[0].review_level, "required");
        assert_eq!(
            report.queue_items[0].apply_target,
            "Kernel::remember_text_after_review"
        );
        assert_eq!(report.queue_policy["writes_memory"], false);
        assert_eq!(
            report.incremental_checkpoint["external_ids"][0],
            "conv_queue"
        );
        assert_eq!(
            payload["coverage_gate"]["new_feature_test_coverage_required"],
            "100%"
        );
    }

    #[test]
    fn v297_connector_proposal_queue_projects_markdown_and_local_git_sync_items() {
        let tempdir = tempdir().unwrap();
        fs::create_dir_all(tempdir.path().join(".git")).unwrap();
        fs::write(
            tempdir.path().join("design.md"),
            "# Queue Design\n\nQueue this document.",
        )
        .unwrap();

        let markdown_report =
            build_connector_proposal_queue_report(ConnectorProposalQueueRequest::new(
                "markdown-docs",
                tempdir.path(),
                ScopeId::from_string("scp_docs_queue"),
            ))
            .unwrap();
        let local_git_report =
            build_connector_proposal_queue_report(ConnectorProposalQueueRequest::new(
                "local-git",
                tempdir.path(),
                ScopeId::from_string("scp_docs_queue"),
            ))
            .unwrap();

        assert_eq!(markdown_report.queue_item_count, 1);
        assert_eq!(
            markdown_report.queue_items[0].proposal_type,
            "project_document_upsert"
        );
        assert_eq!(markdown_report.queue_items[0].review_level, "suggested");
        assert_eq!(
            markdown_report.queue_items[0].apply_target,
            "Kernel::apply_project_document_sync_plan"
        );
        assert_eq!(
            markdown_report.queue_policy["writes_project_documents"],
            false
        );
        assert_eq!(local_git_report.connector, "local-git");
        assert_eq!(local_git_report.queue_item_count, 1);
        assert_eq!(
            local_git_report.incremental_checkpoint["apply_target"],
            "Kernel::apply_project_document_sync_plan"
        );
    }

    #[test]
    fn v297_connector_proposal_queue_blocks_missing_doc_conflict_items() {
        let tempdir = tempdir().unwrap();
        let missing_uri = "file:///tmp/queue-missing.md".to_string();
        let mut request = ConnectorProposalQueueRequest::new(
            "markdown-docs",
            tempdir.path(),
            ScopeId::from_string("scp_docs_queue"),
        );
        request.previous_snapshots = vec![ProjectDocumentSnapshot {
            canonical_uri: missing_uri.clone(),
            content_hash: "sha256:old".to_string(),
        }];

        let report = build_connector_proposal_queue_report(request).unwrap();

        assert_eq!(report.queue_item_count, 1);
        assert_eq!(report.blocked_count, 1);
        assert_eq!(
            report.queue_items[0].proposal_type,
            "project_document_conflict_review"
        );
        assert_eq!(report.queue_items[0].review_level, "required");
        assert_eq!(report.queue_items[0].external_id, missing_uri);
        assert!(report.queue_items[0].blocked);
    }

    #[test]
    fn v297_connector_proposal_apply_plan_requires_confirmation_and_blocks_conflicts() {
        let tempdir = tempdir().unwrap();
        fs::write(
            tempdir.path().join("chat.json"),
            r#"{"id":"apply_queue","title":"Apply queue","messages":[{"role":"user","content":"confirm me"}]}"#,
        )
        .unwrap();
        let queue = build_connector_proposal_queue_report(ConnectorProposalQueueRequest::new(
            "chat-export",
            tempdir.path(),
            ScopeId::from_string("scp_chat_queue"),
        ))
        .unwrap();
        let queue_item_id = queue.queue_items[0].queue_item_id.clone();
        let token = connector_proposal_confirmation_token(
            &queue.queue_id,
            std::slice::from_ref(&queue_item_id),
        );

        let report =
            build_connector_proposal_apply_plan_report(ConnectorProposalApplyPlanRequest::new(
                "chat-export",
                tempdir.path(),
                ScopeId::from_string("scp_chat_queue"),
                vec![queue_item_id.clone()],
                token,
            ))
            .unwrap();

        assert_eq!(report.mode, "proposal_apply_plan");
        assert_eq!(report.queue_id, queue.queue_id);
        assert_eq!(report.selected_count, 1);
        assert_eq!(report.applicable_count, 1);
        assert_eq!(report.blocked_count, 0);
        assert_eq!(report.apply_items[0].queue_item_id, queue_item_id);
        assert!(report.apply_items[0].can_apply);
        assert_eq!(report.apply_policy["writes_memory"], false);
        assert_eq!(report.apply_policy["executor_not_invoked"], true);

        let error =
            build_connector_proposal_apply_plan_report(ConnectorProposalApplyPlanRequest::new(
                "chat-export",
                tempdir.path(),
                ScopeId::from_string("scp_chat_queue"),
                vec![report.apply_items[0].queue_item_id.clone()],
                "confirm_wrong",
            ))
            .unwrap_err();
        assert!(error.to_string().contains("confirmation token mismatch"));
    }

    #[test]
    fn v297_connector_proposal_apply_plan_accepts_persisted_queue_manifest() {
        let tempdir = tempdir().unwrap();
        fs::write(
            tempdir.path().join("chat.json"),
            r#"{"id":"manifest_apply","title":"Manifest apply","messages":[{"role":"user","content":"hello"}]}"#,
        )
        .unwrap();
        let queue = build_connector_proposal_queue_report(ConnectorProposalQueueRequest::new(
            "chat-export",
            tempdir.path(),
            ScopeId::from_string("scp_v297_manifest"),
        ))
        .unwrap();
        let queue_item_id = queue.queue_items[0].queue_item_id.clone();
        let token = connector_proposal_confirmation_token(
            &queue.queue_id,
            std::slice::from_ref(&queue_item_id),
        );

        let report = build_connector_proposal_apply_plan_report_from_queue(
            queue.clone(),
            vec![queue_item_id.clone()],
            token,
        )
        .unwrap();

        assert_eq!(report.queue_id, queue.queue_id);
        assert_eq!(report.connector, "chat-export");
        assert_eq!(report.apply_items[0].queue_item_id, queue_item_id);
        assert_eq!(
            report.apply_items[0].source_refs,
            queue.queue_items[0].source_refs
        );

        let error = build_connector_proposal_apply_plan_report_from_queue(
            queue,
            vec![queue_item_id],
            "wrong",
        )
        .unwrap_err();
        assert!(error.to_string().contains("confirmation token mismatch"));
    }

    #[test]
    fn v297_connector_proposal_apply_plan_keeps_blocked_conflicts_plan_only() {
        let tempdir = tempdir().unwrap();
        let missing_uri = "file:///tmp/apply-missing.md".to_string();
        let mut queue_request = ConnectorProposalQueueRequest::new(
            "markdown-docs",
            tempdir.path(),
            ScopeId::from_string("scp_docs_queue"),
        );
        queue_request.previous_snapshots = vec![ProjectDocumentSnapshot {
            canonical_uri: missing_uri,
            content_hash: "sha256:old".to_string(),
        }];
        let queue = build_connector_proposal_queue_report(queue_request.clone()).unwrap();
        let queue_item_id = queue.queue_items[0].queue_item_id.clone();
        let token = connector_proposal_confirmation_token(
            &queue.queue_id,
            std::slice::from_ref(&queue_item_id),
        );
        let mut request = ConnectorProposalApplyPlanRequest::new(
            "markdown-docs",
            tempdir.path(),
            ScopeId::from_string("scp_docs_queue"),
            vec![queue_item_id],
            token,
        );
        request.previous_snapshots = queue_request.previous_snapshots;

        let report = build_connector_proposal_apply_plan_report(request).unwrap();

        assert_eq!(report.selected_count, 1);
        assert_eq!(report.applicable_count, 0);
        assert_eq!(report.blocked_count, 1);
        assert!(!report.apply_items[0].can_apply);
        assert!(report.apply_items[0].blocked);
    }

    #[test]
    fn v297_connector_proposal_apply_plan_writer_outputs_json_and_markdown() {
        let tempdir = tempdir().unwrap();
        fs::write(
            tempdir.path().join("chat.json"),
            r#"{"id":"write_apply_plan","title":"Write plan","messages":[{"role":"user","content":"hello"}]}"#,
        )
        .unwrap();
        let queue = build_connector_proposal_queue_report(ConnectorProposalQueueRequest::new(
            "chat-export",
            tempdir.path(),
            ScopeId::from_string("scp_chat_queue"),
        ))
        .unwrap();
        let queue_item_id = queue.queue_items[0].queue_item_id.clone();
        let token = connector_proposal_confirmation_token(
            &queue.queue_id,
            std::slice::from_ref(&queue_item_id),
        );
        let report =
            build_connector_proposal_apply_plan_report(ConnectorProposalApplyPlanRequest::new(
                "chat-export",
                tempdir.path(),
                ScopeId::from_string("scp_chat_queue"),
                vec![queue_item_id],
                token,
            ))
            .unwrap();
        let output_dir = tempdir.path().join("reports");
        let paths = write_connector_proposal_apply_plan_report(&output_dir, &report).unwrap();

        let json_text = fs::read_to_string(&paths.json).unwrap();
        let markdown_text = fs::read_to_string(&paths.markdown).unwrap();
        let payload: Value = serde_json::from_str(&json_text).unwrap();

        assert_eq!(payload["schema_version"], "2.97-A");
        assert_eq!(payload["mode"], "proposal_apply_plan");
        assert_eq!(payload["apply_policy"]["executor_not_invoked"], true);
        assert_eq!(
            payload["coverage_gate"]["new_feature_test_coverage_required"],
            "100%"
        );
        assert!(markdown_text.contains("Connector Proposal Apply Plan"));
        assert!(markdown_text.contains("plan only"));
    }

    #[test]
    fn v297_connector_proposal_queue_writer_outputs_json_and_markdown() {
        let tempdir = tempdir().unwrap();
        fs::write(
            tempdir.path().join("chat.json"),
            r#"{"title":"Queue me","messages":[{"role":"user","content":"hello"}]}"#,
        )
        .unwrap();
        let report = build_connector_proposal_queue_report(ConnectorProposalQueueRequest::new(
            "chat-export",
            tempdir.path(),
            ScopeId::from_string("scp_chat_queue"),
        ))
        .unwrap();
        let output_dir = tempdir.path().join("reports");
        let paths = write_connector_proposal_queue_report(&output_dir, &report).unwrap();

        let json_text = fs::read_to_string(&paths.json).unwrap();
        let markdown_text = fs::read_to_string(&paths.markdown).unwrap();
        let payload: Value = serde_json::from_str(&json_text).unwrap();

        assert_eq!(payload["schema_version"], "2.97-A");
        assert_eq!(payload["queue_policy"]["service_apply_exposed"], true);
        assert_eq!(
            payload["coverage_gate"]["new_feature_test_coverage_required"],
            "100%"
        );
        assert!(markdown_text.contains("Connector Proposal Queue"));
        assert!(markdown_text.contains("review queue only"));
    }

    #[test]
    fn v297_connector_dry_run_writer_outputs_json_and_markdown() {
        let tempdir = tempdir().unwrap();
        fs::write(tempdir.path().join("README.md"), "# Project\n").unwrap();
        let report =
            run_connector_dry_run(ConnectorDryRunRequest::new("markdown-docs", tempdir.path()))
                .unwrap();
        let output_dir = tempdir.path().join("reports");
        let paths = write_connector_dry_run_report(&output_dir, &report).unwrap();

        let json_text = fs::read_to_string(&paths.json).unwrap();
        let markdown_text = fs::read_to_string(&paths.markdown).unwrap();
        let payload: Value = serde_json::from_str(&json_text).unwrap();

        assert_eq!(payload["schema_version"], "2.97-A");
        assert_eq!(
            payload["coverage_gate"]["new_feature_test_coverage_required"],
            "100%"
        );
        assert!(markdown_text.contains("Connector Dry Run"));
        assert!(markdown_text.contains("README"));
    }

    #[test]
    fn v297_connector_dry_run_rejects_unknown_connector() {
        let tempdir = tempdir().unwrap();
        let error = run_connector_dry_run(ConnectorDryRunRequest::new("unknown", tempdir.path()))
            .unwrap_err();
        assert!(error.to_string().contains("unsupported connector dry-run"));
    }

    #[test]
    fn v297_markdown_docs_sync_plan_builds_apply_ready_plan_and_evidence_preview() {
        let tempdir = tempdir().unwrap();
        fs::write(
            tempdir.path().join("design.md"),
            "---\ntitle: Connector Design Frontmatter\ntags:\n  - sync\n  - connector\nsummary: |\n  Sync this into project docs.\nowner:\n  team: memory\n---\n# Connector Design\n\nSync this into project docs.\n",
        )
        .unwrap();
        fs::write(tempdir.path().join("ignored.txt"), "ignored").unwrap();

        let output = build_connector_sync_plan(ConnectorSyncPlanRequest::new(
            "markdown-docs",
            tempdir.path(),
            ScopeId::from_string("scp_connector_sync"),
        ))
        .unwrap();

        assert_eq!(output.report.schema_version, "2.97-A");
        assert_eq!(output.report.connector, "markdown-docs");
        assert_eq!(output.report.mode, "sync_plan");
        assert_eq!(output.plan.documents.len(), 1);
        assert_eq!(
            output.plan.documents[0].metadata["frontmatter"]["tags"][0],
            "sync"
        );
        assert_eq!(
            output.plan.documents[0].metadata["frontmatter"]["owner"]["team"],
            "memory"
        );
        assert_eq!(output.report.planned_count, 1);
        assert_eq!(
            output.report.documents[0].title,
            "Connector Design Frontmatter"
        );
        assert_eq!(
            output.report.documents[0].metadata["frontmatter"]["tags"][0],
            "sync"
        );
        assert!(
            output.report.documents[0].metadata["frontmatter"]["summary"]
                .as_str()
                .unwrap()
                .contains("project docs")
        );
        assert_eq!(
            output.report.documents[0].metadata["frontmatter_present"],
            true
        );
        assert_eq!(output.report.documents[0].sync_state, "changed");
        assert_eq!(output.report.evidence_preview.len(), 1);
        assert_eq!(
            output.report.evidence_preview[0].quote,
            "# Connector Design"
        );
        assert!(
            output.report.evidence_preview[0]
                .source_ref
                .ends_with("design.md")
        );
    }

    #[test]
    fn v297_local_git_sync_plan_includes_repository_checkpoint() {
        let tempdir = tempdir().unwrap();
        fs::create_dir_all(tempdir.path().join(".git")).unwrap();
        fs::write(
            tempdir.path().join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .unwrap();
        fs::create_dir_all(tempdir.path().join(".git").join("refs").join("heads")).unwrap();
        fs::write(
            tempdir
                .path()
                .join(".git")
                .join("refs")
                .join("heads")
                .join("main"),
            "2222222222222222222222222222222222222222\n",
        )
        .unwrap();
        fs::write(
            tempdir.path().join(".git").join("config"),
            "[remote \"origin\"]\n\turl = git@example.test:team/repo.git\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n",
        )
        .unwrap();
        fs::write(
            tempdir.path().join(".git").join("packed-refs"),
            "# pack-refs with: peeled fully-peeled sorted\n3333333333333333333333333333333333333333 refs/tags/v2.97\n1111111111111111111111111111111111111111 refs/heads/main\n4444444444444444444444444444444444444444 refs/remotes/origin/main\n",
        )
        .unwrap();
        fs::write(tempdir.path().join(".git").join("index"), "index fixture").unwrap();
        fs::create_dir_all(tempdir.path().join(".git").join("logs")).unwrap();
        fs::write(
            tempdir.path().join(".git").join("logs").join("HEAD"),
            "0000000000000000000000000000000000000000 1111111111111111111111111111111111111111 Ada <ada@example.test> 1710000000 +0000\tcommit (initial): add repo docs\n1111111111111111111111111111111111111111 2222222222222222222222222222222222222222 Ada <ada@example.test> 1710000100 +0000\tcommit: update connector plan\n",
        )
        .unwrap();
        fs::write(
            tempdir.path().join("README.md"),
            "# Repo\n\nLocal git docs.",
        )
        .unwrap();

        let output = build_connector_sync_plan(ConnectorSyncPlanRequest::new(
            "local-git",
            tempdir.path(),
            ScopeId::from_string("scp_connector_sync"),
        ))
        .unwrap();

        assert_eq!(output.report.connector, "local-git");
        assert_eq!(output.report.planned_count, 1);
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["git_head_ref"],
            "ref: refs/heads/main"
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["active_branch"],
            "main"
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["remote_network"],
            false
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["branch_count"],
            1
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["branches"][0]["name"],
            "main"
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["remote_count"],
            1
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["remotes"][0]["name"],
            "origin"
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["packed_ref_count"],
            3
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["packed_refs"][0]["kind"],
            "tag"
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["ref_count"],
            3
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["refs"][0]["name"],
            "refs/heads/main"
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["refs"][0]["sha"],
            "2222222222222222222222222222222222222222"
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["refs"][0]["source"],
            "loose"
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["refs"][1]["name"],
            "refs/remotes/origin/main"
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["refs"][2]["name"],
            "refs/tags/v2.97"
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["worktree_status"]["git_index_present"],
            true
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["worktree_status"]["dirty_state"],
            "unknown_offline"
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["commit_count"],
            2
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["recent_commits"][0]["sha"],
            "2222222222222222222222222222222222222222"
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["recent_commits"][0]["message"],
            "commit: update connector plan"
        );
        assert_eq!(
            output.report.incremental_checkpoint["repository_metadata"]["important_files"][0]["relative_path"],
            "README.md"
        );
    }

    #[test]
    fn v297_local_git_porcelain_status_summarizes_dirty_entries() {
        let entries = local_git_porcelain_entries(
            " M README.md\n?? scratch.tmp\nD  old.txt\nR  old-name.md -> new-name.md\n",
        );
        let summary = local_git_status_summary(&entries);

        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0]["kind"], "modified");
        assert_eq!(entries[1]["kind"], "untracked");
        assert_eq!(entries[2]["kind"], "deleted");
        assert_eq!(entries[3]["kind"], "renamed");
        assert_eq!(summary["modified_count"], 1);
        assert_eq!(summary["untracked_count"], 1);
        assert_eq!(summary["deleted_count"], 1);
        assert_eq!(summary["renamed_count"], 1);
    }

    #[test]
    fn v297_markdown_docs_sync_plan_marks_previous_missing() {
        let tempdir = tempdir().unwrap();
        let missing_uri = "file:///tmp/missing.md".to_string();
        let mut request = ConnectorSyncPlanRequest::new(
            "markdown-docs",
            tempdir.path(),
            ScopeId::from_string("scp_connector_sync"),
        );
        request.previous_snapshots = vec![ProjectDocumentSnapshot {
            canonical_uri: missing_uri.clone(),
            content_hash: "sha256:old".to_string(),
        }];

        let output = build_connector_sync_plan(request).unwrap();

        assert_eq!(output.report.planned_count, 0);
        assert_eq!(output.report.missing_count, 1);
        assert_eq!(output.report.conflict_count, 1);
        assert_eq!(output.report.conflicts[0].canonical_uri, missing_uri);
        assert_eq!(output.plan.missing[0].canonical_uri, missing_uri);
    }

    #[test]
    fn v297_markdown_docs_sync_plan_excludes_generated_report_dirs() {
        let tempdir = tempdir().unwrap();
        fs::create_dir_all(tempdir.path().join("reports")).unwrap();
        fs::write(tempdir.path().join("README.md"), "# Project\n").unwrap();
        fs::write(
            tempdir
                .path()
                .join("reports")
                .join("markdown-docs-sync-plan.md"),
            "# Generated Report\n",
        )
        .unwrap();

        let output = build_connector_sync_plan(ConnectorSyncPlanRequest::new(
            "markdown-docs",
            tempdir.path(),
            ScopeId::from_string("scp_connector_sync"),
        ))
        .unwrap();

        assert_eq!(output.report.planned_count, 1);
        assert_eq!(output.report.documents[0].title, "Project");
    }

    #[test]
    fn v297_markdown_docs_sync_plan_marks_unchanged_previous_snapshot_clean() {
        let tempdir = tempdir().unwrap();
        let path = tempdir.path().join("README.md");
        let content = "# Project\n";
        fs::write(&path, content).unwrap();
        let mut request = ConnectorSyncPlanRequest::new(
            "markdown-docs",
            tempdir.path(),
            ScopeId::from_string("scp_connector_sync"),
        );
        request.previous_snapshots = vec![ProjectDocumentSnapshot {
            canonical_uri: format!("file://{}", path.canonicalize().unwrap().to_string_lossy()),
            content_hash: memory_domain::Artifact::compute_content_hash(content),
        }];

        let output = build_connector_sync_plan(request).unwrap();

        assert_eq!(output.report.planned_count, 1);
        assert_eq!(output.report.documents[0].sync_state, "clean");
    }

    #[test]
    fn v297_connector_sync_plan_writer_outputs_json_and_markdown() {
        let tempdir = tempdir().unwrap();
        fs::write(tempdir.path().join("README.md"), "# Project\n").unwrap();
        let output = build_connector_sync_plan(ConnectorSyncPlanRequest::new(
            "markdown-docs",
            tempdir.path(),
            ScopeId::from_string("scp_connector_sync"),
        ))
        .unwrap();
        let output_dir = tempdir.path().join("reports");
        let paths = write_connector_sync_plan_report(&output_dir, &output.report).unwrap();

        let json_text = fs::read_to_string(&paths.json).unwrap();
        let markdown_text = fs::read_to_string(&paths.markdown).unwrap();
        let payload: Value = serde_json::from_str(&json_text).unwrap();

        assert_eq!(payload["schema_version"], "2.97-A");
        assert_eq!(
            payload["coverage_gate"]["new_feature_test_coverage_required"],
            "100%"
        );
        assert!(markdown_text.contains("Connector Sync Plan"));
        assert!(markdown_text.contains("Conflict Review"));
        assert!(markdown_text.contains("Evidence Preview"));
    }
}
