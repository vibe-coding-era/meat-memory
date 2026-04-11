use memory_domain::{Episode, EpisodeKind, Memory, MemoryKind, ScopeId};
use memory_store_md::{MarkdownStore, parse_memory_markdown};
use std::fs;
use tempfile::tempdir;

#[test]
fn memory_markdown_roundtrip_is_stable() {
    let tempdir = tempdir().unwrap();
    let store = MarkdownStore::with_tenant(tempdir.path(), "team-alpha").unwrap();
    let scope_id = ScopeId::new();

    let mut memory = Memory::new(
        scope_id.clone(),
        MemoryKind::Preference,
        "Concise summaries",
        "Codex prefers concise summaries in final responses.",
    )
    .unwrap();
    memory.activate().unwrap();

    let path = store.write_memory_markdown(&memory).unwrap();
    let parsed = store
        .read_memory_markdown(&scope_id, &memory.id)
        .unwrap()
        .unwrap();
    let restored = parsed.clone().into_memory().unwrap();
    let raw = fs::read_to_string(&path).unwrap();

    assert_eq!(path.file_name().unwrap().to_string_lossy(), "MEMORY.md");
    assert!(raw.contains(memory.id.as_str()));
    assert_eq!(restored.id, memory.id);
    assert_eq!(restored.scope_id, memory.scope_id);
    assert_eq!(restored.title, memory.title);
    assert_eq!(restored.body, memory.body);
    assert_eq!(restored.state, memory.state);
    assert_eq!(restored.kind, memory.kind);
    assert_eq!(restored.evidence_count, memory.evidence_count);
    assert_eq!(parsed.frontmatter.tenant, "team-alpha");
}

#[test]
fn writing_same_memory_updates_existing_rollup_entry() {
    let tempdir = tempdir().unwrap();
    let store = MarkdownStore::new(tempdir.path()).unwrap();
    let scope_id = ScopeId::new();

    let mut memory = Memory::new(
        scope_id.clone(),
        MemoryKind::Fact,
        "Primary branch",
        "The default branch is main.",
    )
    .unwrap();
    memory.activate().unwrap();
    let path = store.write_memory_markdown(&memory).unwrap();

    memory.body = "The default branch is main and protected.".to_string();
    memory.updated_at = time::OffsetDateTime::now_utc();
    store.write_memory_markdown(&memory).unwrap();

    let raw = fs::read_to_string(path).unwrap();
    assert_eq!(
        raw.matches(&format!(
            "<!-- memory-entry:start {} -->",
            memory.id.as_str()
        ))
        .count(),
        1
    );
    assert!(raw.contains("The default branch is main and protected."));
}

#[test]
fn parser_handles_single_entry_document() {
    let tempdir = tempdir().unwrap();
    let store = MarkdownStore::new(tempdir.path()).unwrap();
    let scope_id = ScopeId::new();

    let memory = Memory::new(
        scope_id,
        MemoryKind::Decision,
        "Storage mode",
        "PostgreSQL remains the canonical mutable backend.",
    )
    .unwrap();
    let rendered = memory_store_md::render_memory_markdown(&memory, store.tenant()).unwrap();
    let parsed = parse_memory_markdown(&rendered).unwrap();

    assert_eq!(parsed.frontmatter.kind, "memory");
    assert_eq!(parsed.frontmatter.title, "Storage mode");
    assert_eq!(
        parsed.body,
        "PostgreSQL remains the canonical mutable backend."
    );
}

#[test]
fn markdown_roundtrip_preserves_reserved_marker_lines_in_body() {
    let tempdir = tempdir().unwrap();
    let store = MarkdownStore::new(tempdir.path()).unwrap();
    let scope_id = ScopeId::new();

    let mut memory = Memory::new(
        scope_id.clone(),
        MemoryKind::Fact,
        "Marker-safe body",
        "line1\n---\n<!-- memory-entry:start fake -->\n\\literal",
    )
    .unwrap();
    memory.activate().unwrap();

    store.write_memory_markdown(&memory).unwrap();
    let restored = store
        .read_memory_markdown(&scope_id, &memory.id)
        .unwrap()
        .unwrap()
        .into_memory()
        .unwrap();

    assert_eq!(restored.body, memory.body);
}

#[test]
fn episode_writer_creates_partitioned_daily_paths() {
    let tempdir = tempdir().unwrap();
    let store = MarkdownStore::with_tenant(tempdir.path(), "org-demo").unwrap();
    let scope_id = ScopeId::new();
    let episode = Episode::new(
        scope_id,
        EpisodeKind::CodingTask,
        "Implement markdown store",
    )
    .unwrap();

    let path = store.write_episode_markdown(&episode).unwrap();
    let raw = fs::read_to_string(&path).unwrap();

    assert!(path.exists());
    assert!(path.to_string_lossy().contains("/org-demo/episodes/"));
    assert!(
        path.to_string_lossy()
            .ends_with(&format!("{}.md", episode.id.as_str()))
    );
    assert!(raw.contains("kind: episode"));
    assert!(raw.contains(&format!("id: {}", episode.id.as_str())));
}
