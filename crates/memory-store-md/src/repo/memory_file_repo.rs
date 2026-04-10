use crate::render::frontmatter::{MemoryFrontmatter, render_frontmatter};
use crate::repo::parser::{ParsedMemoryMarkdown, parse_memory_markdown};
use anyhow::{Context, Result, anyhow};
use memory_domain::{Memory, MemoryId, ScopeId};
use std::{
    fs,
    path::{Path, PathBuf},
};

use super::episode_file_repo::write_episode_markdown;

const ENTRY_START_PREFIX: &str = "<!-- memory-entry:start ";
const ENTRY_END_PREFIX: &str = "<!-- memory-entry:end ";
const ROLLUP_HEADER: &str = "# Scope Memory Rollup\n";

#[derive(Debug, Clone)]
pub struct MarkdownStore {
    root: PathBuf,
    tenant: String,
}

impl MarkdownStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self> {
        Self::with_tenant(root, "default")
    }

    pub fn with_tenant(root: impl Into<PathBuf>, tenant: impl Into<String>) -> Result<Self> {
        let root = root.into();
        if root.as_os_str().is_empty() {
            return Err(anyhow!("markdown projection root cannot be empty"));
        }

        let tenant = sanitize_segment(&tenant.into());
        if tenant.is_empty() {
            return Err(anyhow!("markdown tenant cannot be empty"));
        }

        Ok(Self { root, tenant })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn tenant(&self) -> &str {
        &self.tenant
    }

    pub fn scope_memory_rollup_path(&self, scope_id: &ScopeId) -> PathBuf {
        scope_directory(&self.root, &self.tenant, scope_id).join("MEMORY.md")
    }

    pub fn write_memory_markdown(&self, memory: &Memory) -> Result<PathBuf> {
        let path = self.scope_memory_rollup_path(&memory.scope_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create markdown projection directory {}",
                    parent.display()
                )
            })?;
        }

        let entry = render_memory_markdown(memory, &self.tenant)?;
        let next = match fs::read_to_string(&path) {
            Ok(existing) => upsert_entry(&existing, memory.id.as_str(), &entry),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                format!("{ROLLUP_HEADER}\n{entry}")
            }
            Err(error) => {
                return Err(error).with_context(|| format!("failed to read {}", path.display()));
            }
        };

        fs::write(&path, next).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(path)
    }

    pub fn read_memory_markdown(
        &self,
        scope_id: &ScopeId,
        memory_id: &MemoryId,
    ) -> Result<Option<ParsedMemoryMarkdown>> {
        let path = self.scope_memory_rollup_path(scope_id);
        let raw = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(error).with_context(|| format!("failed to read {}", path.display()));
            }
        };

        let entry = extract_entry(&raw, memory_id.as_str());
        entry.as_deref().map(parse_memory_markdown).transpose()
    }

    pub fn list_memories_by_scope(&self, scope_id: &ScopeId) -> Result<Vec<Memory>> {
        let path = self.scope_memory_rollup_path(scope_id);
        let raw = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(error).with_context(|| format!("failed to read {}", path.display()));
            }
        };

        parse_memory_entries(&raw)
    }

    pub fn list_all_memories(&self) -> Result<Vec<Memory>> {
        let scopes_root = self.root.join(&self.tenant).join("scopes");
        let entries = match fs::read_dir(&scopes_root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to read {}", scopes_root.display()));
            }
        };

        let mut memories = Vec::new();
        for entry in entries {
            let entry = entry.with_context(|| {
                format!("failed to read scope entry under {}", scopes_root.display())
            })?;
            let file_type = entry.file_type().with_context(|| {
                format!("failed to inspect scope entry {}", entry.path().display())
            })?;
            if !file_type.is_dir() {
                continue;
            }

            let rollup_path = entry.path().join("MEMORY.md");
            let raw = match fs::read_to_string(&rollup_path) {
                Ok(content) => content,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("failed to read {}", rollup_path.display()));
                }
            };
            memories.extend(parse_memory_entries(&raw)?);
        }

        memories.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(memories)
    }

    pub fn write_episode_markdown(&self, episode: &memory_domain::Episode) -> Result<PathBuf> {
        write_episode_markdown(&self.root, &self.tenant, episode)
    }
}

pub fn render_memory_markdown(memory: &Memory, tenant: &str) -> Result<String> {
    let frontmatter = MemoryFrontmatter::from_memory(memory, tenant)?;
    let start_marker = format!("{ENTRY_START_PREFIX}{} -->\n", memory.id.as_str());
    let end_marker = format!("{ENTRY_END_PREFIX}{} -->\n", memory.id.as_str());
    let mut rendered = String::new();
    rendered.push_str(&start_marker);
    rendered.push_str(&render_frontmatter(&frontmatter)?);
    rendered.push_str(&memory.body);
    if !memory.body.ends_with('\n') {
        rendered.push('\n');
    }
    rendered.push_str(&end_marker);
    Ok(rendered)
}

pub(crate) fn scope_directory(root: &Path, tenant: &str, scope_id: &ScopeId) -> PathBuf {
    root.join(tenant).join("scopes").join(scope_id.as_str())
}

pub(crate) fn sanitize_segment(raw: &str) -> String {
    let mut sanitized = String::with_capacity(raw.len());
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            sanitized.push(ch);
        } else {
            sanitized.push('-');
        }
    }

    sanitized.trim_matches('-').to_string()
}

fn upsert_entry(existing: &str, memory_id: &str, replacement: &str) -> String {
    let start_marker = format!("{ENTRY_START_PREFIX}{memory_id} -->");
    let end_marker = format!("{ENTRY_END_PREFIX}{memory_id} -->");
    if let (Some(start), Some(end)) = (existing.find(&start_marker), existing.find(&end_marker)) {
        let end = end + end_marker.len();
        let trailing_newline = if existing[end..].starts_with('\n') {
            end + 1
        } else {
            end
        };
        let mut output = String::with_capacity(existing.len() + replacement.len());
        output.push_str(&existing[..start]);
        output.push_str(replacement);
        output.push_str(&existing[trailing_newline..]);
        return output;
    }

    let trimmed = existing.trim_end();
    if trimmed.is_empty() {
        return format!("{ROLLUP_HEADER}\n{replacement}");
    }

    format!("{trimmed}\n\n{replacement}")
}

fn extract_entry(existing: &str, memory_id: &str) -> Option<String> {
    let start_marker = format!("{ENTRY_START_PREFIX}{memory_id} -->");
    let end_marker = format!("{ENTRY_END_PREFIX}{memory_id} -->");
    let start = existing.find(&start_marker)?;
    let end = existing.find(&end_marker)? + end_marker.len();
    Some(existing[start..end].to_string())
}

fn parse_memory_entries(raw: &str) -> Result<Vec<Memory>> {
    let entries = extract_entries(raw);
    let mut memories = entries
        .iter()
        .map(|entry| parse_memory_markdown(entry).and_then(ParsedMemoryMarkdown::into_memory))
        .collect::<Result<Vec<_>>>()?;
    memories.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(memories)
}

fn extract_entries(raw: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut current = Vec::new();
    let mut in_entry = false;

    for line in raw.lines() {
        if line.starts_with(ENTRY_START_PREFIX) {
            current.clear();
            in_entry = true;
        }

        if in_entry {
            current.push(line);
        }

        if in_entry && line.starts_with(ENTRY_END_PREFIX) {
            let mut entry = current.join("\n");
            entry.push('\n');
            entries.push(entry);
            current.clear();
            in_entry = false;
        }
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::{
        MarkdownStore, ROLLUP_HEADER, extract_entries, extract_entry, parse_memory_entries,
        render_memory_markdown, sanitize_segment, scope_directory, upsert_entry,
    };
    use memory_domain::{Episode, EpisodeId, EpisodeKind, Memory, MemoryId, MemoryKind, ScopeId};
    use std::fs;
    use tempfile::tempdir;
    use time::macros::datetime;

    fn sample_memory() -> Memory {
        let mut memory = Memory::new(
            ScopeId::from_string("scp_store"),
            MemoryKind::Decision,
            "保留 PG 与 MD 双写",
            "首版继续双写，便于回放与排查",
        )
        .expect("memory should build");
        memory.id = MemoryId::from_string("mem_store");
        memory.created_at = datetime!(2025-01-02 03:04:05 UTC);
        memory.updated_at = datetime!(2025-01-03 04:05:06 UTC);
        memory
    }

    fn sample_episode() -> Episode {
        let mut episode = Episode::new(
            ScopeId::from_string("scp_store"),
            EpisodeKind::CodingTask,
            "补测试",
        )
        .expect("episode should build");
        episode.id = EpisodeId::from_string("epi_store");
        episode.started_at = datetime!(2025-01-02 03:04:05 UTC);
        episode
    }

    #[test]
    fn markdown_store_validates_root_and_tenant() {
        assert!(MarkdownStore::with_tenant(std::path::PathBuf::new(), "tenant-a").is_err());
        assert!(MarkdownStore::with_tenant("/tmp", "   ").is_err());
    }

    #[test]
    fn markdown_store_sanitizes_tenant_and_computes_rollup_path() {
        let store = MarkdownStore::with_tenant("/tmp/meat-memory", "tenant alpha")
            .expect("store should build");
        let scope_id = ScopeId::from_string("scp_store");

        assert_eq!(store.tenant(), "tenant-alpha");
        assert_eq!(store.root(), std::path::Path::new("/tmp/meat-memory"));
        assert_eq!(
            scope_directory(store.root(), store.tenant(), &scope_id),
            std::path::Path::new("/tmp/meat-memory")
                .join("tenant-alpha")
                .join("scopes")
                .join("scp_store")
        );
        assert_eq!(
            store.scope_memory_rollup_path(&scope_id),
            std::path::Path::new("/tmp/meat-memory")
                .join("tenant-alpha")
                .join("scopes")
                .join("scp_store")
                .join("MEMORY.md")
        );
    }

    #[test]
    fn sanitize_segment_replaces_unsafe_characters() {
        assert_eq!(sanitize_segment(" tenant.alpha "), "tenant.alpha");
        assert_eq!(sanitize_segment("tenant / alpha"), "tenant---alpha");
        assert_eq!(sanitize_segment("///tenant///"), "tenant");
    }

    #[test]
    fn render_memory_markdown_wraps_entry_markers_and_body() {
        let rendered =
            render_memory_markdown(&sample_memory(), "tenant-a").expect("markdown should render");

        assert!(rendered.starts_with("<!-- memory-entry:start mem_store -->\n---\n"));
        assert!(rendered.contains("kind: memory"));
        assert!(rendered.contains("tenant: tenant-a"));
        assert!(rendered.ends_with("<!-- memory-entry:end mem_store -->\n"));
    }

    #[test]
    fn upsert_entry_bootstraps_and_replaces_rollup_entries() {
        let original = "<!-- memory-entry:start mem_store -->\nold\n<!-- memory-entry:end mem_store -->\n\n<!-- memory-entry:start mem_other -->\nkeep\n<!-- memory-entry:end mem_other -->\n";
        let replacement =
            "<!-- memory-entry:start mem_store -->\nnew\n<!-- memory-entry:end mem_store -->\n";
        let updated = upsert_entry(original, "mem_store", replacement);

        assert!(updated.contains("new"));
        assert!(updated.contains("keep"));
        assert_eq!(
            upsert_entry("", "mem_store", replacement),
            format!("{ROLLUP_HEADER}\n{replacement}")
        );
    }

    #[test]
    fn extract_entry_returns_only_requested_block() {
        let existing = "<!-- memory-entry:start mem_store -->\nblock\n<!-- memory-entry:end mem_store -->\n\n<!-- memory-entry:start mem_other -->\nother\n<!-- memory-entry:end mem_other -->\n";

        let entry = extract_entry(existing, "mem_store").expect("entry should exist");

        assert!(entry.contains("mem_store"));
        assert!(!entry.contains("mem_other"));
        assert_eq!(extract_entry(existing, "missing"), None);
    }

    #[test]
    fn extract_entries_collects_multiple_memory_blocks() {
        let raw = format!(
            "{}\n{}",
            render_memory_markdown(&sample_memory(), "tenant-a").unwrap(),
            render_memory_markdown(
                &Memory::new(
                    ScopeId::from_string("scp_store"),
                    MemoryKind::Summary,
                    "第二条",
                    "第二条正文",
                )
                .unwrap(),
                "tenant-a",
            )
            .unwrap()
        );

        let entries = extract_entries(&raw);

        assert_eq!(entries.len(), 2);
        assert!(entries[0].contains("mem_store"));
        assert!(entries[1].contains("第二条"));
    }

    #[test]
    fn parse_memory_entries_returns_memories_sorted_by_updated_at() {
        let older = sample_memory();
        let mut newer = Memory::new(
            ScopeId::from_string("scp_store"),
            MemoryKind::Summary,
            "较新",
            "较新的正文",
        )
        .unwrap();
        newer.updated_at = datetime!(2025-01-04 05:06:07 UTC);
        let raw = format!(
            "{}\n{}",
            render_memory_markdown(&older, "tenant-a").unwrap(),
            render_memory_markdown(&newer, "tenant-a").unwrap()
        );

        let memories = parse_memory_entries(&raw).unwrap();

        assert_eq!(memories.len(), 2);
        assert_eq!(memories[0].title, "较新");
        assert_eq!(memories[1].title, "保留 PG 与 MD 双写");
    }

    #[test]
    fn write_and_read_memory_markdown_round_trips_and_upserts() {
        let root = tempdir().expect("tempdir should build");
        let store =
            MarkdownStore::with_tenant(root.path(), "tenant-a").expect("store should build");
        let mut memory = sample_memory();

        let path = store
            .write_memory_markdown(&memory)
            .expect("first write should work");
        assert!(path.exists());

        let parsed = store
            .read_memory_markdown(&memory.scope_id, &memory.id)
            .expect("read should work")
            .expect("entry should exist");
        assert_eq!(parsed.body, "首版继续双写，便于回放与排查");

        memory.title = "只保留最新正文".to_string();
        memory.body = "更新后的正文".to_string();
        store
            .write_memory_markdown(&memory)
            .expect("second write should work");

        let raw = fs::read_to_string(store.scope_memory_rollup_path(&memory.scope_id))
            .expect("rollup file should exist");
        assert_eq!(
            raw.matches("<!-- memory-entry:start mem_store -->").count(),
            1
        );
        assert_eq!(
            raw.matches("<!-- memory-entry:end mem_store -->").count(),
            1
        );
        assert!(raw.contains("更新后的正文"));
        assert!(!raw.contains("首版继续双写，便于回放与排查"));
    }

    #[test]
    fn read_memory_markdown_returns_none_when_rollup_is_missing() {
        let root = tempdir().expect("tempdir should build");
        let store =
            MarkdownStore::with_tenant(root.path(), "tenant-a").expect("store should build");

        let result = store
            .read_memory_markdown(
                &ScopeId::from_string("scp_missing"),
                &MemoryId::from_string("mem_missing"),
            )
            .expect("read should succeed");

        assert_eq!(result, None);
    }

    #[test]
    fn markdown_store_lists_memories_across_scopes() {
        let root = tempdir().expect("tempdir should build");
        let store =
            MarkdownStore::with_tenant(root.path(), "tenant-a").expect("store should build");
        let first = sample_memory();
        let second = Memory::new(
            ScopeId::from_string("scp_other"),
            MemoryKind::Summary,
            "跨 scope",
            "第二个 scope 的正文",
        )
        .unwrap();

        store.write_memory_markdown(&first).unwrap();
        store.write_memory_markdown(&second).unwrap();

        let memories = store.list_all_memories().unwrap();

        assert_eq!(memories.len(), 2);
        assert!(
            memories
                .iter()
                .any(|memory| memory.scope_id.as_str() == "scp_store")
        );
        assert!(
            memories
                .iter()
                .any(|memory| memory.scope_id.as_str() == "scp_other")
        );
        assert_eq!(
            store
                .list_memories_by_scope(&ScopeId::from_string("scp_store"))
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn store_delegates_episode_markdown_writes() {
        let root = tempdir().expect("tempdir should build");
        let store =
            MarkdownStore::with_tenant(root.path(), "tenant-a").expect("store should build");
        let episode = sample_episode();

        let path = store
            .write_episode_markdown(&episode)
            .expect("episode write should work");

        assert!(path.exists());
        assert!(path.ends_with("epi_store.md"));
    }
}
