use anyhow::{Context, Result, bail};
use memory_domain::{MemoryKind, ScopeId};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
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
}
