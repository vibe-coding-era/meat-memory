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
