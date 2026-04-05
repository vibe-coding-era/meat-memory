pub mod render;
pub mod repo;

pub use render::frontmatter::{
    EpisodeFrontmatter, MemoryFrontmatter, MemoryFrontmatterScores, render_frontmatter,
};
pub use repo::{
    MarkdownStore, ParsedMemoryMarkdown, parse_memory_markdown, render_memory_markdown,
};

pub fn default_projection_root() -> &'static str {
    "docs/tenants"
}

#[cfg(test)]
mod tests {
    use super::default_projection_root;

    #[test]
    fn exposes_docs_projection_root() {
        assert_eq!(default_projection_root(), "docs/tenants");
    }
}
