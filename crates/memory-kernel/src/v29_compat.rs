use anyhow::{Context, Result, bail};
use memory_domain::{EvidenceSpan, MemoryKind, ScopeId};
use memory_sync::{
    LocalProjectDocumentSyncEngine, LocalProjectDocumentSyncPlan, ProjectDocumentConflictReport,
    ProjectDocumentSnapshot,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use time::OffsetDateTime;

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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorSyncPlanDocument {
    pub title: String,
    pub canonical_uri: String,
    pub local_path: PathBuf,
    pub content_hash: String,
    pub sync_state: String,
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
                "chat_export_dry_run",
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
    if !matches!(request.connector.as_str(), "markdown-docs" | "local-git") {
        bail!(
            "unsupported connector sync plan: {}; only markdown-docs and local-git are implemented",
            request.connector
        );
    }

    let connector = request.connector.clone();
    let checkpoint = connector_sync_checkpoint(&connector, &root_path);
    let mut plan = LocalProjectDocumentSyncEngine::with_extensions(root_path, ["md", "markdown"])
        .with_excluded_dir_names(request.excluded_dir_names)
        .scan(&request.previous_snapshots)
        .context("failed to build connector sync plan")?;
    if plan.documents.len() > request.max_items {
        plan.documents.truncate(request.max_items);
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
                "sync_plan_conflict_review_projection",
                "sync_plan_evidence_preview",
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
            "frontmatter": "preserve_when_present",
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
    let relative_path = path.strip_prefix(root_path).unwrap_or(path);
    let title = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("markdown document")
        .replace(['_', '-'], " ");

    Ok(ConnectorDryRunItem {
        title,
        source_ref: format!("file://{}", path.display()),
        content_bytes: metadata.len(),
        metadata: json!({
            "source_kind": "markdown",
            "relative_path": relative_path.display().to_string(),
        }),
    })
}

fn connector_document_evidence_span(
    scope_id: &ScopeId,
    document: &memory_sync::LocalProjectDocumentDraft,
) -> EvidenceSpan {
    let quote = evidence_quote(&document.content_text);
    EvidenceSpan::new_text(
        scope_id.clone(),
        None,
        None,
        document.canonical_uri.clone(),
        quote.clone(),
        document.content_hash.clone(),
        0,
        quote.len(),
    )
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
            Some(json!({
                "name": name,
                "sha": sha,
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
        assert_eq!(connectors.len(), 3);
        assert!(connectors.contains("local-git"));
        assert!(connectors.contains("markdown-docs"));
        assert!(connectors.contains("chat-export"));
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
        fs::write(docs_dir.join("alpha-note.md"), "# Alpha\n").unwrap();
        fs::write(docs_dir.join("ignored.txt"), "ignored").unwrap();

        let mut request = ConnectorDryRunRequest::new("markdown-docs", tempdir.path());
        request.max_items = 10;
        let report = run_connector_dry_run(request).unwrap();

        assert_eq!(report.schema_version, "2.97-A");
        assert_eq!(report.connector, "markdown-docs");
        assert_eq!(report.status, "ready");
        assert_eq!(report.candidate_count, 1);
        assert_eq!(report.items[0].title, "alpha note");
        assert_eq!(
            report.items[0].metadata["relative_path"],
            "docs/alpha-note.md"
        );
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
            "# Connector Design\n\nSync this into project docs.\n",
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
        assert_eq!(output.report.planned_count, 1);
        assert_eq!(output.report.documents[0].title, "Connector Design");
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
