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
