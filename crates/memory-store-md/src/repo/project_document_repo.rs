use crate::render::frontmatter::{ProjectDocumentFrontmatter, render_frontmatter};
use anyhow::{Context, Result, anyhow};
use memory_domain::{ProjectDocument, ProjectDocumentId, ScopeId, SourceId};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use super::memory_file_repo::sanitize_segment;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedProjectDocumentMarkdown {
    pub frontmatter: ProjectDocumentFrontmatter,
    pub body: String,
}

pub fn render_project_document_markdown(
    document: &ProjectDocument,
    tenant: &str,
    source_id: &SourceId,
    body: &str,
) -> Result<String> {
    let mut rendered = render_frontmatter(&ProjectDocumentFrontmatter::from_project_document(
        document, tenant, source_id,
    )?)?;
    rendered.push_str(body);
    if !body.ends_with('\n') {
        rendered.push('\n');
    }
    Ok(rendered)
}

pub fn parse_project_document_markdown(raw: &str) -> Result<ParsedProjectDocumentMarkdown> {
    let remainder = raw.strip_prefix("---\n").ok_or_else(|| {
        anyhow!("project document markdown is missing opening frontmatter delimiter")
    })?;
    let Some((frontmatter_raw, body)) = remainder.split_once("\n---\n") else {
        return Err(anyhow!(
            "project document markdown is missing closing frontmatter delimiter"
        ));
    };
    let frontmatter: ProjectDocumentFrontmatter = serde_yaml::from_str(frontmatter_raw)
        .context("failed to parse project document frontmatter")?;
    Ok(ParsedProjectDocumentMarkdown {
        frontmatter,
        body: body.to_string(),
    })
}

pub fn project_documents_root(root: &Path, tenant: &str, source_id: &SourceId) -> PathBuf {
    root.join(tenant)
        .join("sources")
        .join(source_id.as_str())
        .join("documents")
}

pub fn project_document_projection_path(
    root: &Path,
    tenant: &str,
    source_id: &SourceId,
    document: &ProjectDocument,
) -> PathBuf {
    let docs_root = project_documents_root(root, tenant, source_id);
    let relative = document
        .local_path
        .as_deref()
        .map(mapped_local_path)
        .unwrap_or_else(|| {
            PathBuf::from(format!(
                "{}.md",
                sanitize_segment(document.canonical_uri.as_str())
            ))
        });
    docs_root.join(relative)
}

pub fn mapped_local_path(local_path: &str) -> PathBuf {
    let mut mapped = PathBuf::new();
    for component in Path::new(local_path).components() {
        match component {
            Component::RootDir | Component::Prefix(_) | Component::CurDir => {}
            Component::ParentDir => mapped.push("parent"),
            Component::Normal(segment) => {
                let sanitized = sanitize_segment(&segment.to_string_lossy());
                if !sanitized.is_empty() {
                    mapped.push(sanitized);
                }
            }
        }
    }

    if mapped.as_os_str().is_empty() {
        PathBuf::from("document.md")
    } else {
        mapped
    }
}

impl super::memory_file_repo::MarkdownStore {
    pub fn project_documents_root(&self, source_id: &SourceId) -> PathBuf {
        project_documents_root(self.root(), self.tenant(), source_id)
    }

    pub fn project_document_projection_path(
        &self,
        source_id: &SourceId,
        document: &ProjectDocument,
    ) -> PathBuf {
        project_document_projection_path(self.root(), self.tenant(), source_id, document)
    }

    pub fn write_project_document_markdown(
        &self,
        source_id: &SourceId,
        document: &ProjectDocument,
        body: &str,
    ) -> Result<PathBuf> {
        let path = self.project_document_projection_path(source_id, document);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create project document projection directory {}",
                    parent.display()
                )
            })?;
        }
        let rendered = render_project_document_markdown(document, self.tenant(), source_id, body)?;
        fs::write(&path, rendered)
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(path)
    }

    pub fn read_project_document_markdown(
        &self,
        source_id: &SourceId,
        document: &ProjectDocument,
    ) -> Result<Option<ParsedProjectDocumentMarkdown>> {
        let path = self.project_document_projection_path(source_id, document);
        let raw = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(error).with_context(|| format!("failed to read {}", path.display()));
            }
        };
        parse_project_document_markdown(&raw).map(Some)
    }

    pub fn find_project_document_markdown(
        &self,
        source_id: &SourceId,
        document_id: &ProjectDocumentId,
    ) -> Result<Option<ParsedProjectDocumentMarkdown>> {
        let docs_root = self.project_documents_root(source_id);
        let mut stack = vec![docs_root];
        while let Some(path) = stack.pop() {
            let entries = match fs::read_dir(&path) {
                Ok(entries) => entries,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("failed to read {}", path.display()));
                }
            };
            for entry in entries {
                let entry =
                    entry.with_context(|| format!("failed to inspect {}", path.display()))?;
                let file_type = entry.file_type().with_context(|| {
                    format!(
                        "failed to inspect project document entry {}",
                        entry.path().display()
                    )
                })?;
                if file_type.is_dir() {
                    stack.push(entry.path());
                    continue;
                }
                if !file_type.is_file() {
                    continue;
                }
                let raw = fs::read_to_string(entry.path()).with_context(|| {
                    format!(
                        "failed to read project document file {}",
                        entry.path().display()
                    )
                })?;
                let parsed = parse_project_document_markdown(&raw)?;
                if parsed.frontmatter.id == document_id.as_str() {
                    return Ok(Some(parsed));
                }
            }
        }

        Ok(None)
    }

    pub fn project_document_scope_directory(&self, scope_id: &ScopeId) -> PathBuf {
        self.root()
            .join(self.tenant())
            .join("scopes")
            .join(scope_id.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        mapped_local_path, parse_project_document_markdown, project_document_projection_path,
        project_documents_root, render_project_document_markdown,
    };
    use crate::repo::MarkdownStore;
    use memory_domain::{
        DocumentConflictState, DocumentSyncState, ProjectDocument, ProjectDocumentId, ScopeId,
        SourceId,
    };
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;
    use time::macros::datetime;

    fn sample_document() -> ProjectDocument {
        let mut document = ProjectDocument::new(
            SourceId::from_string("src_docs"),
            ScopeId::from_string("scp_docs"),
            "file:///workspace/docs/README.md",
            "README",
            "sha256:docs",
        )
        .expect("document should build");
        document.id = ProjectDocumentId::from_string("doc_docs");
        document.local_path = Some("/workspace/docs/README.md".to_string());
        document.sync_state = DocumentSyncState::Changed;
        document.conflict_state = DocumentConflictState::LocalChanged;
        document.created_at = datetime!(2025-01-02 03:04:05 UTC);
        document.updated_at = datetime!(2025-01-03 04:05:06 UTC);
        document
    }

    #[test]
    fn local_path_mapping_sanitizes_components() {
        assert_eq!(
            mapped_local_path("/workspace/docs/API Reference.md"),
            PathBuf::from("workspace")
                .join("docs")
                .join("API-Reference.md")
        );
    }

    #[test]
    fn project_document_paths_include_source_and_local_mapping() {
        let document = sample_document();
        let source_id = SourceId::from_string("src_docs");
        let root = Path::new("/tmp/meat-memory");

        assert_eq!(
            project_documents_root(root, "tenant-a", &source_id),
            root.join("tenant-a")
                .join("sources")
                .join("src_docs")
                .join("documents")
        );
        assert_eq!(
            project_document_projection_path(root, "tenant-a", &source_id, &document),
            root.join("tenant-a")
                .join("sources")
                .join("src_docs")
                .join("documents")
                .join("workspace")
                .join("docs")
                .join("README.md")
        );
    }

    #[test]
    fn project_document_markdown_round_trips() {
        let document = sample_document();
        let rendered = render_project_document_markdown(
            &document,
            "tenant-a",
            &SourceId::from_string("src_docs"),
            "# README\n",
        )
        .expect("markdown should render");

        let parsed = parse_project_document_markdown(&rendered).expect("markdown should parse");

        assert_eq!(parsed.frontmatter.kind, "project_document");
        assert_eq!(parsed.frontmatter.id, "doc_docs");
        assert_eq!(parsed.frontmatter.source_id, "src_docs");
        assert_eq!(
            parsed.frontmatter.local_path.as_deref(),
            Some("/workspace/docs/README.md")
        );
        assert_eq!(parsed.body, "# README\n");
    }

    #[test]
    fn store_writes_and_finds_project_document_projection() {
        let root = tempdir().expect("tempdir should build");
        let store =
            MarkdownStore::with_tenant(root.path(), "tenant-a").expect("store should build");
        let document = sample_document();
        let source_id = SourceId::from_string("src_docs");

        let path = store
            .write_project_document_markdown(&source_id, &document, "# README\n")
            .expect("project document write should work");
        let parsed = store
            .read_project_document_markdown(&source_id, &document)
            .expect("read should work")
            .expect("document should exist");
        let found = store
            .find_project_document_markdown(&source_id, &document.id)
            .expect("find should work")
            .expect("document should be discoverable");

        assert!(path.exists());
        assert!(path.ends_with("workspace/docs/README.md"));
        assert_eq!(parsed.frontmatter.title, "README");
        assert_eq!(found.frontmatter.id, "doc_docs");
    }
}
