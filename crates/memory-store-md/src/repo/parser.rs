use crate::render::frontmatter::MemoryFrontmatter;
use anyhow::{Context, Result, anyhow, bail};
use memory_domain::{
    Memory, MemoryId, MemoryKind, MemoryScores, MemoryState, ScopeId, Sensitivity, Visibility,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedMemoryMarkdown {
    pub frontmatter: MemoryFrontmatter,
    pub body: String,
}

impl ParsedMemoryMarkdown {
    pub fn into_memory(self) -> Result<Memory> {
        if self.frontmatter.kind != "memory" {
            bail!("unsupported markdown kind: {}", self.frontmatter.kind);
        }

        let scores = MemoryScores {
            confidence: self.frontmatter.scores.confidence,
            importance: self.frontmatter.scores.importance,
            stability: self.frontmatter.scores.stability,
            freshness: self.frontmatter.scores.freshness,
        }
        .validate()?;

        Ok(Memory {
            id: MemoryId::from_string(self.frontmatter.id),
            scope_id: ScopeId::from_string(self.frontmatter.scope),
            kind: parse_memory_kind(&self.frontmatter.memory_kind)?,
            state: parse_memory_state(&self.frontmatter.status)?,
            title: self.frontmatter.title,
            body: self.body,
            scores,
            visibility: parse_visibility(&self.frontmatter.visibility)?,
            sensitivity: parse_sensitivity(&self.frontmatter.sensitivity)?,
            evidence_count: self.frontmatter.evidence_count,
            created_at: parse_timestamp(&self.frontmatter.created_at)?,
            updated_at: parse_timestamp(&self.frontmatter.updated_at)?,
        })
    }
}

pub fn parse_memory_markdown(input: &str) -> Result<ParsedMemoryMarkdown> {
    let normalized = normalize_lines(input);
    let filtered = normalized
        .lines()
        .filter(|line| {
            !line.starts_with("<!-- memory-entry:start ")
                && !line.starts_with("<!-- memory-entry:end ")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let stripped = filtered.trim();
    let Some(remainder) = stripped.strip_prefix("---\n") else {
        return Err(anyhow!(
            "memory markdown is missing opening frontmatter delimiter"
        ));
    };

    let Some((frontmatter_raw, body)) = remainder.split_once("\n---\n") else {
        return Err(anyhow!(
            "memory markdown is missing closing frontmatter delimiter"
        ));
    };

    let frontmatter: MemoryFrontmatter =
        serde_yaml::from_str(frontmatter_raw).context("failed to parse memory frontmatter")?;

    Ok(ParsedMemoryMarkdown {
        frontmatter,
        body: body.trim_end_matches('\n').to_string(),
    })
}

fn parse_timestamp(raw: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(raw, &Rfc3339)
        .with_context(|| format!("failed to parse RFC3339 timestamp: {raw}"))
}

fn parse_memory_kind(raw: &str) -> Result<MemoryKind> {
    match raw {
        "fact" => Ok(MemoryKind::Fact),
        "preference" => Ok(MemoryKind::Preference),
        "decision" => Ok(MemoryKind::Decision),
        "procedure" => Ok(MemoryKind::Procedure),
        "constraint" => Ok(MemoryKind::Constraint),
        "risk" => Ok(MemoryKind::Risk),
        "summary" => Ok(MemoryKind::Summary),
        "insight" => Ok(MemoryKind::Insight),
        _ => bail!("unknown memory kind: {raw}"),
    }
}

fn parse_memory_state(raw: &str) -> Result<MemoryState> {
    match raw {
        "candidate" => Ok(MemoryState::Candidate),
        "active" => Ok(MemoryState::Active),
        "deprecated" => Ok(MemoryState::Deprecated),
        "conflicted" => Ok(MemoryState::Conflicted),
        "archived" => Ok(MemoryState::Archived),
        "deleted" => Ok(MemoryState::Deleted),
        _ => bail!("unknown memory state: {raw}"),
    }
}

fn parse_visibility(raw: &str) -> Result<Visibility> {
    match raw {
        "private" => Ok(Visibility::Private),
        "project" => Ok(Visibility::Project),
        "team" => Ok(Visibility::Team),
        "organization" => Ok(Visibility::Organization),
        _ => bail!("unknown visibility: {raw}"),
    }
}

fn parse_sensitivity(raw: &str) -> Result<Sensitivity> {
    match raw {
        "public" => Ok(Sensitivity::Public),
        "internal" => Ok(Sensitivity::Internal),
        "private" => Ok(Sensitivity::Private),
        "restricted" => Ok(Sensitivity::Restricted),
        _ => bail!("unknown sensitivity: {raw}"),
    }
}

fn normalize_lines(input: &str) -> String {
    input.replace("\r\n", "\n").replace('\r', "\n")
}
