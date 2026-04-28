# V2.95 Competitor Compatibility Report

schema_version: 2.95
generated_at: 2026-04-28 12:48:38.419894 +00:00:00

## Markdown Compatibility

- memory markdown frontmatter + body -> memory markdown plus V2.9 reports under tests/reports/* (full_read)
  - V2.95 keeps old memory markdown readable and only adds report projections beside it.
- project document projection -> project document projection with optional evidence references (additive)
  - Old docs projection remains valid; evidence metadata is optional and can be absent.

## Competitor Capability Mapping

- Supermemory: container / user / project memory -> ScopeId plus visibility and source_refs (mapped)
  - Containers map to scopes; user/project separation stays explicit instead of SaaS-tenant hidden.
- Supermemory: memory graph -> ContextBundle memories/entities/relations (mapped)
  - Graph retrieval maps to existing entity and relation context payloads.
- Supermemory: document RAG -> project documents, evidence spans, recall traces (mapped)
  - Document chunks become source-backed memory plus traceable evidence.
- Supermemory: connectors -> local connector skeletons (skeleton)
  - V2.95 defines safe local connector shapes; external SaaS sync remains explicit.
- mem0: user / session / agent / org memory -> ScopeId, AgentContext, Visibility (mapped)
  - User and org map to scopes; session and agent memory use short-term AgentContext.
- mem0: add / search / delete -> remember / search / lifecycle forget-delete (mapped)
  - Core operations already exist across CLI, HTTP, and MCP.
- mem0: OSS local runtime -> markdown-first local runtime with optional PostgreSQL (superset)
  - Meat Memory keeps local markdown as a first-class store and can add PG without losing portability.
- MemoryLake: passport -> Memory Passport manifest and bundle (mapped)
  - V2.94 passport export, verify, import, and manifest inspection are reused.
- MemoryLake: provenance -> EvidenceSpan and source_refs (mapped)
  - Each imported draft keeps external source references for later evidence promotion.
- MemoryLake: conflict / version / audit -> document conflicts, timeline, lifecycle audit (mapped)
  - Existing V2.7/V2.8 governance primitives cover the compatibility contract.

## Connector Skeletons

- local-git [repository]: Read repository metadata, commits, and important docs as project context. (read_only_no_remote_push, skeleton)
- markdown-docs [document_tree]: Scan local markdown docs into project documents and evidence refs. (local_files_only, skeleton)
- chat-export [conversation_export]: Normalize chat exports into timestamped message memory drafts. (explicit_import_only, skeleton)

## Adapter Fixture Drafts

- mem0-style-json mem0_fixture_001 -> mem0 memory mem0_fixture_001 (Preference)
- supermemory-style-document sm_doc_001 -> Project memory container (Summary)
- memorylake-style-passport lake_mem_001 -> Portable provenance fact (Fact)

## Coverage Gate

- V2.95 new production regions require 100% targeted test coverage.
