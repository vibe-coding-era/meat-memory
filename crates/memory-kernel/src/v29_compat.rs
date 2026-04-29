use anyhow::{Context, Result, bail};
use memory_domain::{EvidenceSpan, MemoryKind, ScopeId};
use memory_sync::{
    LocalProjectDocumentSyncEngine, LocalProjectDocumentSyncPlan, ProjectDocumentSnapshot,
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
    if request.connector != "markdown-docs" {
        bail!(
            "unsupported connector sync plan: {}; only markdown-docs is implemented",
            request.connector
        );
    }

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
        connector: "markdown-docs".to_string(),
        root_path: plan.root.clone(),
        mode: "sync_plan".to_string(),
        planned_count: plan.documents.len(),
        missing_count: plan.missing.len(),
        conflict_count: plan.conflicts.len(),
        documents,
        evidence_preview,
        incremental_checkpoint: json!({
            "strategy": "canonical_uri_content_hash",
            "apply_target": "Kernel::apply_project_document_sync_plan",
            "excluded_dir_names": ["reports", ".playwright-cli", "target"],
        }),
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
        "evidence_preview": report.evidence_preview,
        "incremental_checkpoint": report.incremental_checkpoint,
        "coverage_gate": {
            "new_feature_test_coverage_required": "100%",
            "covered_regions": [
                "markdown_docs_sync_plan",
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

    let title = conversation
        .get("title")
        .and_then(Value::as_str)
        .filter(|title| !title.trim().is_empty())
        .unwrap_or("chat export conversation")
        .to_string();
    let external_id = conversation
        .get("id")
        .or_else(|| conversation.get("conversation_id"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("conversation-{index}"));
    let relative_path = path.strip_prefix(root_path).unwrap_or(path);
    let participants = messages
        .iter()
        .map(|message| message.role.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
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
            "format": if conversation.get("mapping").is_some() {
                "chatgpt-conversations-json"
            } else {
                "generic-messages-json"
            },
        }),
    })
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
        assert!(markdown_text.contains("Evidence Preview"));
    }
}
