use crate::{render::frontmatter::MemoryFrontmatter, repo::memory_file_repo::unescape_rollup_body};
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
            owner_scope_id: ScopeId::from_string(self.frontmatter.owner_scope),
            published_from_scope_id: self
                .frontmatter
                .published_from_scope
                .map(ScopeId::from_string),
            kind: parse_memory_kind(&self.frontmatter.memory_kind)?,
            state: parse_memory_state(&self.frontmatter.status)?,
            title: self.frontmatter.title,
            body: self.body,
            language_code: self.frontmatter.language_code,
            source_refs: self.frontmatter.source_refs,
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
        body: unescape_rollup_body(body.trim_end_matches('\n')),
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
        "forgotten" => Ok(MemoryState::Forgotten),
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

#[cfg(test)]
mod tests {
    use super::{
        ParsedMemoryMarkdown, parse_memory_kind, parse_memory_markdown, parse_memory_state,
        parse_sensitivity, parse_timestamp, parse_visibility,
    };
    use crate::render::frontmatter::{
        MemoryFrontmatter, MemoryFrontmatterScores, render_frontmatter,
    };
    use memory_domain::{MemoryKind, MemoryState, Sensitivity, Visibility};

    fn sample_parsed_markdown() -> ParsedMemoryMarkdown {
        ParsedMemoryMarkdown {
            frontmatter: MemoryFrontmatter {
                id: "mem_parser".to_string(),
                kind: "memory".to_string(),
                tenant: "tenant-a".to_string(),
                scope: "scp_parser".to_string(),
                owner_scope: "scp_parser".to_string(),
                published_from_scope: None,
                memory_kind: "fact".to_string(),
                title: "解析成功".to_string(),
                status: "active".to_string(),
                language_code: Some("zh-CN".to_string()),
                visibility: "team".to_string(),
                sensitivity: "internal".to_string(),
                created_at: "2025-01-02T03:04:05Z".to_string(),
                updated_at: "2025-01-03T04:05:06Z".to_string(),
                source_refs: Vec::new(),
                evidence: Vec::new(),
                entities: Vec::new(),
                tags: Vec::new(),
                scores: MemoryFrontmatterScores {
                    confidence: 0.8,
                    importance: 0.7,
                    stability: 0.6,
                    freshness: 0.5,
                },
                evidence_count: 3,
            },
            body: "这是正文".to_string(),
        }
    }

    #[test]
    fn parsed_markdown_converts_into_memory() {
        let memory = sample_parsed_markdown()
            .into_memory()
            .expect("markdown should convert");

        assert_eq!(memory.id.as_str(), "mem_parser");
        assert_eq!(memory.scope_id.as_str(), "scp_parser");
        assert_eq!(memory.owner_scope_id.as_str(), "scp_parser");
        assert_eq!(memory.published_from_scope_id, None);
        assert_eq!(memory.kind, MemoryKind::Fact);
        assert_eq!(memory.state, MemoryState::Active);
        assert_eq!(memory.title, "解析成功");
        assert_eq!(memory.body, "这是正文");
        assert_eq!(memory.language_code.as_deref(), Some("zh-CN"));
        assert_eq!(memory.visibility, Visibility::Team);
        assert_eq!(memory.sensitivity, Sensitivity::Internal);
        assert_eq!(memory.evidence_count, 3);
        assert_eq!(memory.created_at.unix_timestamp(), 1_735_787_045);
        assert_eq!(memory.updated_at.unix_timestamp(), 1_735_877_106);
    }

    #[test]
    fn parse_memory_markdown_supports_markers_and_crlf() {
        let parsed = sample_parsed_markdown();
        let frontmatter = render_frontmatter(&parsed.frontmatter)
            .expect("frontmatter should render")
            .replace('\n', "\r\n");
        let raw = format!(
            "<!-- memory-entry:start mem_parser -->\r\n{}{}\r\n<!-- memory-entry:end mem_parser -->\r\n",
            frontmatter,
            parsed.body.replace('\n', "\r\n")
        );

        let actual = parse_memory_markdown(&raw).expect("markdown should parse");

        assert_eq!(actual.frontmatter.id, "mem_parser");
        assert_eq!(actual.body, "这是正文");
    }

    #[test]
    fn parse_memory_markdown_requires_opening_frontmatter() {
        let error = parse_memory_markdown("kind: memory")
            .expect_err("missing opening delimiter should fail");
        assert!(
            error
                .to_string()
                .contains("missing opening frontmatter delimiter")
        );
    }

    #[test]
    fn parse_memory_markdown_requires_closing_frontmatter() {
        let error = parse_memory_markdown("---\nkind: memory\n")
            .expect_err("missing closing delimiter should fail");
        assert!(
            error
                .to_string()
                .contains("missing closing frontmatter delimiter")
        );
    }

    #[test]
    fn parse_memory_markdown_reports_invalid_frontmatter_yaml() {
        let error =
            parse_memory_markdown("---\nid: [\n---\nbody").expect_err("invalid yaml should fail");
        assert!(
            error
                .to_string()
                .contains("failed to parse memory frontmatter")
        );
    }

    #[test]
    fn into_memory_rejects_invalid_frontmatter_variants() {
        let mut invalid_kind = sample_parsed_markdown();
        invalid_kind.frontmatter.kind = "episode".to_string();
        assert!(invalid_kind.into_memory().is_err());

        let mut invalid_memory_kind = sample_parsed_markdown();
        invalid_memory_kind.frontmatter.memory_kind = "unknown".to_string();
        assert!(invalid_memory_kind.into_memory().is_err());

        let mut invalid_state = sample_parsed_markdown();
        invalid_state.frontmatter.status = "unknown".to_string();
        assert!(invalid_state.into_memory().is_err());

        let mut invalid_visibility = sample_parsed_markdown();
        invalid_visibility.frontmatter.visibility = "unknown".to_string();
        assert!(invalid_visibility.into_memory().is_err());

        let mut invalid_sensitivity = sample_parsed_markdown();
        invalid_sensitivity.frontmatter.sensitivity = "unknown".to_string();
        assert!(invalid_sensitivity.into_memory().is_err());

        let mut invalid_created_at = sample_parsed_markdown();
        invalid_created_at.frontmatter.created_at = "not-a-timestamp".to_string();
        assert!(invalid_created_at.into_memory().is_err());

        let mut invalid_scores = sample_parsed_markdown();
        invalid_scores.frontmatter.scores.confidence = 1.5;
        assert!(invalid_scores.into_memory().is_err());
    }

    #[test]
    fn parser_helpers_cover_supported_variants_and_errors() {
        let memory_kind_cases = [
            ("fact", MemoryKind::Fact),
            ("preference", MemoryKind::Preference),
            ("decision", MemoryKind::Decision),
            ("procedure", MemoryKind::Procedure),
            ("constraint", MemoryKind::Constraint),
            ("risk", MemoryKind::Risk),
            ("summary", MemoryKind::Summary),
            ("insight", MemoryKind::Insight),
        ];
        for (raw, expected) in memory_kind_cases {
            assert_eq!(
                parse_memory_kind(raw).expect("memory kind should parse"),
                expected
            );
        }
        assert!(parse_memory_kind("invalid").is_err());

        let state_cases = [
            ("candidate", MemoryState::Candidate),
            ("active", MemoryState::Active),
            ("deprecated", MemoryState::Deprecated),
            ("conflicted", MemoryState::Conflicted),
            ("archived", MemoryState::Archived),
            ("forgotten", MemoryState::Forgotten),
            ("deleted", MemoryState::Deleted),
        ];
        for (raw, expected) in state_cases {
            assert_eq!(
                parse_memory_state(raw).expect("state should parse"),
                expected
            );
        }
        assert!(parse_memory_state("invalid").is_err());

        let visibility_cases = [
            ("private", Visibility::Private),
            ("project", Visibility::Project),
            ("team", Visibility::Team),
            ("organization", Visibility::Organization),
        ];
        for (raw, expected) in visibility_cases {
            assert_eq!(
                parse_visibility(raw).expect("visibility should parse"),
                expected
            );
        }
        assert!(parse_visibility("invalid").is_err());

        let sensitivity_cases = [
            ("public", Sensitivity::Public),
            ("internal", Sensitivity::Internal),
            ("private", Sensitivity::Private),
            ("restricted", Sensitivity::Restricted),
        ];
        for (raw, expected) in sensitivity_cases {
            assert_eq!(
                parse_sensitivity(raw).expect("sensitivity should parse"),
                expected
            );
        }
        assert!(parse_sensitivity("invalid").is_err());
    }

    #[test]
    fn parse_timestamp_reports_invalid_rfc3339() {
        let error =
            parse_timestamp("2025/01/02 03:04:05").expect_err("invalid timestamp should fail");
        assert!(
            error
                .to_string()
                .contains("failed to parse RFC3339 timestamp")
        );
    }
}
