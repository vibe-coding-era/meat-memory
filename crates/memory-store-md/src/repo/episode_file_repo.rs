use crate::render::frontmatter::{EpisodeFrontmatter, render_frontmatter};
use anyhow::{Context, Result};
use memory_domain::Episode;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn write_episode_markdown(root: &Path, tenant: &str, episode: &Episode) -> Result<PathBuf> {
    let path = episode_path(root, tenant, episode);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create episode markdown directory {}",
                parent.display()
            )
        })?;
    }

    let frontmatter = EpisodeFrontmatter::from_episode(episode, tenant)?;
    let mut rendered = render_frontmatter(&frontmatter)?;
    if let Some(summary) = &episode.summary {
        rendered.push_str(summary);
        if !summary.ends_with('\n') {
            rendered.push('\n');
        }
    }

    fs::write(&path, rendered).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}

fn episode_path(root: &Path, tenant: &str, episode: &Episode) -> PathBuf {
    let started_at = episode.started_at.date();
    root.join(tenant)
        .join("episodes")
        .join(format!("{:04}", started_at.year()))
        .join(format!("{:02}", u8::from(started_at.month())))
        .join(format!("{:02}", started_at.day()))
        .join(format!("{}.md", episode.id.as_str()))
}

#[cfg(test)]
mod tests {
    use super::{episode_path, write_episode_markdown};
    use crate::render::frontmatter::{EpisodeFrontmatter, render_frontmatter};
    use memory_domain::{Episode, EpisodeId, EpisodeKind, ScopeId};
    use std::fs;
    use tempfile::tempdir;
    use time::macros::datetime;

    fn sample_episode() -> Episode {
        let mut episode = Episode::new(
            ScopeId::from_string("scp_episode"),
            EpisodeKind::CodingTask,
            "实现 V1",
        )
        .expect("episode should build");
        episode.id = EpisodeId::from_string("epi_20250102");
        episode.started_at = datetime!(2025-01-02 03:04:05 UTC);
        episode
    }

    #[test]
    fn episode_path_uses_tenant_and_date_hierarchy() {
        let root = tempdir().expect("tempdir should build");
        let episode = sample_episode();

        let path = episode_path(root.path(), "tenant-a", &episode);

        assert_eq!(
            path,
            root.path()
                .join("tenant-a")
                .join("episodes")
                .join("2025")
                .join("01")
                .join("02")
                .join("epi_20250102.md")
        );
    }

    #[test]
    fn write_episode_markdown_creates_file_and_appends_summary_newline() {
        let root = tempdir().expect("tempdir should build");
        let mut episode = sample_episode();
        episode.summary = Some("本次完成 V1 接入".to_string());

        let path =
            write_episode_markdown(root.path(), "tenant-a", &episode).expect("write should work");
        let raw = fs::read_to_string(&path).expect("episode file should exist");

        assert!(path.exists());
        assert!(raw.contains("kind: episode"));
        assert!(raw.ends_with("本次完成 V1 接入\n"));
    }

    #[test]
    fn write_episode_markdown_without_summary_matches_rendered_frontmatter() {
        let root = tempdir().expect("tempdir should build");
        let episode = sample_episode();

        let path =
            write_episode_markdown(root.path(), "tenant-a", &episode).expect("write should work");
        let raw = fs::read_to_string(&path).expect("episode file should exist");
        let expected = render_frontmatter(
            &EpisodeFrontmatter::from_episode(&episode, "tenant-a").expect("frontmatter maps"),
        )
        .expect("frontmatter renders");

        assert_eq!(raw, expected);
    }

    #[test]
    fn write_episode_markdown_preserves_existing_summary_newline() {
        let root = tempdir().expect("tempdir should build");
        let mut episode = sample_episode();
        episode.summary = Some("已包含换行\n".to_string());

        let path =
            write_episode_markdown(root.path(), "tenant-a", &episode).expect("write should work");
        let raw = fs::read_to_string(&path).expect("episode file should exist");

        assert!(raw.ends_with("已包含换行\n"));
        assert!(!raw.ends_with("已包含换行\n\n"));
    }

    #[test]
    fn write_episode_markdown_returns_directory_creation_error() {
        let root = tempdir().expect("tempdir should build");
        let episode = sample_episode();
        let tenant_file = root.path().join("tenant-a");
        fs::write(&tenant_file, "occupied").expect("tenant file should exist");

        let error = write_episode_markdown(root.path(), "tenant-a", &episode)
            .expect_err("directory creation should fail");

        assert!(
            error
                .to_string()
                .contains("failed to create episode markdown directory")
        );
    }

    #[test]
    fn write_episode_markdown_returns_write_error_when_target_is_directory() {
        let root = tempdir().expect("tempdir should build");
        let episode = sample_episode();
        let path = episode_path(root.path(), "tenant-a", &episode);
        let parent = path.parent().expect("episode path should have parent");
        fs::create_dir_all(&path).expect("target directory should be created");
        assert!(parent.exists());

        let error = write_episode_markdown(root.path(), "tenant-a", &episode)
            .expect_err("writing into directory should fail");

        assert!(error.to_string().contains("failed to write"));
    }
}
