use crate::{LifecycleNormalizer, MemoryHealthReport};
use anyhow::Result;
use memory_domain::{
    Artifact, Memory, MemoryHealthRisk, MemoryHealthRiskKind, MemoryHealthSeverity,
    MemoryHealthSuggestedAction, MemoryId, MemoryRecordStatus, ScopeId, SecretFinding,
    SecretFindingAction, SecretFindingKind, SecretFindingLocation, SecretRiskLevel, Sensitivity,
};
use serde_json::json;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct SensitiveIngestGuardResult {
    pub body: String,
    pub findings: Vec<SecretFinding>,
    pub denied: bool,
    pub proposal_required: bool,
}

#[derive(Debug, Clone)]
pub struct SecretRecallGuardResult {
    pub memories: Vec<Memory>,
    pub findings: Vec<SecretFinding>,
    pub blocked_count: usize,
}

#[derive(Debug, Clone)]
pub struct MemoryHealthReportPaths {
    pub json: PathBuf,
    pub markdown: PathBuf,
}

#[derive(Debug, Clone)]
struct DetectedSecret {
    finding: SecretFinding,
}

#[derive(Debug, Clone, Copy)]
struct DetectionContext<'a> {
    scope_id: &'a ScopeId,
    memory_id: Option<&'a MemoryId>,
    source_ref: &'a str,
    field: &'a str,
    text: &'a str,
}

pub struct SecretDetector;

impl SecretDetector {
    pub fn detect_text(
        scope_id: &ScopeId,
        memory_id: Option<&MemoryId>,
        source_ref: &str,
        field: &str,
        text: &str,
    ) -> Vec<SecretFinding> {
        detect_secrets(scope_id, memory_id, source_ref, field, text)
            .into_iter()
            .map(|detected| detected.finding)
            .collect()
    }
}

pub fn apply_sensitive_ingest_guard(
    scope_id: &ScopeId,
    source_ref: &str,
    body: &str,
) -> SensitiveIngestGuardResult {
    let detected = detect_secrets(scope_id, None, source_ref, "body", body);
    let findings = detected
        .iter()
        .map(|detected| detected.finding.clone())
        .collect::<Vec<_>>();
    let denied = findings
        .iter()
        .any(|finding| finding.action == SecretFindingAction::Deny);
    let proposal_required = findings
        .iter()
        .any(|finding| finding.action == SecretFindingAction::Proposal);
    let body = if denied {
        body.to_string()
    } else {
        redact_detected(body, &detected)
    };

    SensitiveIngestGuardResult {
        body,
        findings,
        denied,
        proposal_required,
    }
}

pub fn apply_secret_recall_guard(memories: Vec<Memory>) -> SecretRecallGuardResult {
    let mut allowed = Vec::new();
    let mut findings = Vec::new();
    let mut blocked_count = 0;

    for memory in memories {
        let memory_findings = secret_recall_findings(&memory);
        if memory_findings
            .iter()
            .any(|finding| finding.action == SecretFindingAction::RecallBlock)
        {
            blocked_count += 1;
            findings.extend(memory_findings);
        } else {
            findings.extend(memory_findings);
            allowed.push(memory);
        }
    }

    SecretRecallGuardResult {
        memories: allowed,
        findings,
        blocked_count,
    }
}

pub fn redact_hard_deleted_memory_body(memory: &mut Memory) -> bool {
    let findings = SecretDetector::detect_text(
        &memory.scope_id,
        Some(&memory.id),
        &format!("memory:{}", memory.id.as_str()),
        "body",
        &memory.body,
    );
    let should_redact = findings.iter().any(|finding| {
        matches!(
            finding.risk_level,
            SecretRiskLevel::Medium | SecretRiskLevel::High | SecretRiskLevel::Critical
        )
    });
    if should_redact {
        memory.body = "[REDACTED:HARD_DELETE_SECRET]".to_string();
        memory.updated_at = OffsetDateTime::now_utc();
    }
    should_redact
}

pub(crate) fn secret_recall_block_reason(memory: &Memory) -> Option<String> {
    let findings = secret_recall_findings(memory);
    findings
        .iter()
        .find(|finding| finding.action == SecretFindingAction::RecallBlock)
        .map(|finding| format!("secret recall block: {}", finding.kind.as_str()))
}

pub fn analyze_memory_health(
    scope_id: Option<ScopeId>,
    memories: &[Memory],
    generated_at: OffsetDateTime,
) -> MemoryHealthReport {
    let mut report = MemoryHealthReport {
        scope_id,
        total: memories.len(),
        active: 0,
        candidate: 0,
        needs_review: 0,
        archived: 0,
        deprecated: 0,
        forgotten: 0,
        deleted: 0,
        restricted: 0,
        stale: 0,
        source_backed: 0,
        low_confidence: 0,
        secret_findings: 0,
        high_risk_secret_findings: 0,
        risks: Vec::new(),
        suggested_actions: Vec::new(),
        generated_at,
    };
    let mut titles: HashMap<String, Vec<&Memory>> = HashMap::new();

    for memory in memories {
        let record = LifecycleNormalizer::normalize_memory(memory);
        match record.status {
            MemoryRecordStatus::Active => report.active += 1,
            MemoryRecordStatus::Candidate => report.candidate += 1,
            MemoryRecordStatus::NeedsReview => report.needs_review += 1,
            MemoryRecordStatus::Archived => report.archived += 1,
            MemoryRecordStatus::Deprecated => report.deprecated += 1,
            MemoryRecordStatus::Forgotten => report.forgotten += 1,
            MemoryRecordStatus::Deleted => report.deleted += 1,
        }
        if matches!(record.sensitivity, Sensitivity::Restricted) {
            report.restricted += 1;
            push_risk(
                &mut report.risks,
                memory,
                MemoryHealthRiskKind::HighSensitivity,
                MemoryHealthSeverity::Warning,
                "restricted memory requires owner-scope recall access",
                MemoryHealthSuggestedAction::Review,
            );
        }
        if record.freshness < 0.3 {
            report.stale += 1;
            push_risk(
                &mut report.risks,
                memory,
                MemoryHealthRiskKind::Stale,
                MemoryHealthSeverity::Warning,
                "freshness score is below 0.3",
                MemoryHealthSuggestedAction::Archive,
            );
        }
        if memory.scores.confidence < 0.35 {
            report.low_confidence += 1;
            push_risk(
                &mut report.risks,
                memory,
                MemoryHealthRiskKind::LowConfidence,
                MemoryHealthSeverity::Warning,
                "confidence score is below 0.35",
                MemoryHealthSuggestedAction::Review,
            );
        }
        if matches!(
            record.status,
            MemoryRecordStatus::Candidate | MemoryRecordStatus::NeedsReview
        ) {
            push_risk(
                &mut report.risks,
                memory,
                MemoryHealthRiskKind::Unreviewed,
                MemoryHealthSeverity::Info,
                "memory is waiting for review",
                MemoryHealthSuggestedAction::Review,
            );
        }
        if matches!(record.status, MemoryRecordStatus::NeedsReview) {
            push_risk(
                &mut report.risks,
                memory,
                MemoryHealthRiskKind::Conflict,
                MemoryHealthSeverity::Warning,
                "memory lifecycle status requires conflict review",
                MemoryHealthSuggestedAction::Review,
            );
        }
        if memory.evidence_count == 0 && memory.source_refs.is_empty() {
            push_risk(
                &mut report.risks,
                memory,
                MemoryHealthRiskKind::OrphanEvidence,
                MemoryHealthSeverity::Info,
                "memory has no evidence link or source reference",
                MemoryHealthSuggestedAction::AddEvidence,
            );
        }
        if record.source_ref.is_some() {
            report.source_backed += 1;
        }

        let findings = SecretDetector::detect_text(
            &memory.scope_id,
            Some(&memory.id),
            &format!("memory:{}", memory.id.as_str()),
            "body",
            &format!("{}\n{}", memory.title, memory.body),
        );
        report.secret_findings += findings.len();
        for finding in findings {
            if matches!(
                finding.risk_level,
                SecretRiskLevel::High | SecretRiskLevel::Critical
            ) {
                report.high_risk_secret_findings += 1;
                push_risk(
                    &mut report.risks,
                    memory,
                    MemoryHealthRiskKind::SecretFinding,
                    MemoryHealthSeverity::Critical,
                    &format!("{} detected in memory body", finding.kind.as_str()),
                    MemoryHealthSuggestedAction::Redact,
                );
            }
        }

        titles
            .entry(normalize_title(&memory.title))
            .or_default()
            .push(memory);
    }

    for (title, matches) in titles {
        if !title.is_empty() && matches.len() > 1 {
            let detail = format!(
                "{} memories share normalized title '{}'",
                matches.len(),
                title
            );
            push_risk(
                &mut report.risks,
                matches[0],
                MemoryHealthRiskKind::Duplicate,
                MemoryHealthSeverity::Warning,
                &detail,
                MemoryHealthSuggestedAction::Merge,
            );
        }
    }

    report.suggested_actions = report
        .risks
        .iter()
        .map(|risk| risk.suggested_action.as_str().to_string())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    report.suggested_actions.sort();
    report
}

pub fn write_memory_health_report(
    report_dir: &Path,
    report: &MemoryHealthReport,
) -> Result<MemoryHealthReportPaths> {
    fs::create_dir_all(report_dir)?;
    let paths = MemoryHealthReportPaths {
        json: report_dir.join("health.json"),
        markdown: report_dir.join("health.md"),
    };
    fs::write(
        &paths.json,
        serde_json::to_string_pretty(&health_json(report))?,
    )?;
    fs::write(&paths.markdown, render_health_markdown(report))?;
    Ok(paths)
}

pub fn health_json(report: &MemoryHealthReport) -> serde_json::Value {
    json!({
        "scope_id": report.scope_id.as_ref().map(|scope| scope.as_str()),
        "total": report.total,
        "active": report.active,
        "candidate": report.candidate,
        "needs_review": report.needs_review,
        "archived": report.archived,
        "deprecated": report.deprecated,
        "forgotten": report.forgotten,
        "deleted": report.deleted,
        "restricted": report.restricted,
        "stale": report.stale,
        "source_backed": report.source_backed,
        "low_confidence": report.low_confidence,
        "secret_findings": report.secret_findings,
        "high_risk_secret_findings": report.high_risk_secret_findings,
        "suggested_actions": report.suggested_actions,
        "risks": report.risks.iter().map(health_risk_json).collect::<Vec<_>>(),
        "generated_at": report.generated_at,
    })
}

fn render_health_markdown(report: &MemoryHealthReport) -> String {
    let mut output = format!(
        "# Memory Health Report\n\nscope_id: {}\ntotal: {}\nactive: {}\nneeds_review: {}\nrestricted: {}\nstale: {}\nlow_confidence: {}\nsecret_findings: {}\nhigh_risk_secret_findings: {}\n\n",
        report
            .scope_id
            .as_ref()
            .map(ScopeId::as_str)
            .unwrap_or("all"),
        report.total,
        report.active,
        report.needs_review,
        report.restricted,
        report.stale,
        report.low_confidence,
        report.secret_findings,
        report.high_risk_secret_findings
    );

    output.push_str("## Suggested Actions\n\n");
    if report.suggested_actions.is_empty() {
        output.push_str("- keep\n");
    } else {
        for action in &report.suggested_actions {
            output.push_str(&format!("- {action}\n"));
        }
    }

    output.push_str("\n## Risks\n\n");
    if report.risks.is_empty() {
        output.push_str("- none\n");
    } else {
        for risk in &report.risks {
            output.push_str(&format!(
                "- kind={} severity={} action={} memory={} title={} detail={}\n",
                risk.kind.as_str(),
                risk.severity.as_str(),
                risk.suggested_action.as_str(),
                risk.memory_id
                    .as_ref()
                    .map(MemoryId::as_str)
                    .unwrap_or("n/a"),
                risk.title.as_deref().unwrap_or("n/a"),
                risk.detail
            ));
        }
    }
    output
}

fn health_risk_json(risk: &MemoryHealthRisk) -> serde_json::Value {
    json!({
        "kind": risk.kind.as_str(),
        "severity": risk.severity.as_str(),
        "memory_id": risk.memory_id.as_ref().map(|id| id.as_str()),
        "title": risk.title.as_deref(),
        "detail": risk.detail.as_str(),
        "suggested_action": risk.suggested_action.as_str(),
    })
}

fn detect_secrets(
    scope_id: &ScopeId,
    memory_id: Option<&MemoryId>,
    source_ref: &str,
    field: &str,
    text: &str,
) -> Vec<DetectedSecret> {
    let mut detected = Vec::new();
    let context = DetectionContext {
        scope_id,
        memory_id,
        source_ref,
        field,
        text,
    };
    detect_private_key(context, &mut detected);
    detect_line_assignments(context, &mut detected);
    detect_redacted_markers(context, &mut detected);
    detect_tokens(context, &mut detected);
    detected
}

fn detect_private_key(context: DetectionContext<'_>, detected: &mut Vec<DetectedSecret>) {
    let lower = context.text.to_ascii_lowercase();
    let Some(start) = lower.find("-----begin ") else {
        return;
    };
    if !lower[start..].contains(" private key-----") {
        return;
    }
    let end = lower[start..]
        .find("-----end ")
        .and_then(|end_start| {
            lower[start + end_start..]
                .find("-----")
                .map(|end| start + end_start + end + 5)
        })
        .unwrap_or_else(|| {
            context.text[start..]
                .find('\n')
                .map(|line| start + line)
                .unwrap_or(context.text.len())
        });
    push_detected(
        detected,
        context,
        start,
        end.min(context.text.len()),
        SecretFindingKind::PrivateKey,
    );
}

fn detect_line_assignments(context: DetectionContext<'_>, detected: &mut Vec<DetectedSecret>) {
    let mut offset = 0;
    for line in context.text.lines() {
        for (kind, labels, min_len) in [
            (
                SecretFindingKind::ApiKey,
                &["api_key", "apikey", "secret_key", "access_token"][..],
                12,
            ),
            (
                SecretFindingKind::Password,
                &["password", "passwd", "pwd"][..],
                8,
            ),
        ] {
            if let Some((start, end)) = assignment_value_range(line, labels, min_len) {
                push_detected(detected, context, offset + start, offset + end, kind);
            }
        }

        let lower = line.to_ascii_lowercase();
        if let Some(index) = lower.find("bearer ") {
            let start = index + "bearer ".len();
            let end = value_end(line, start);
            if end.saturating_sub(start) >= 12 {
                push_detected(
                    detected,
                    context,
                    offset + start,
                    offset + end,
                    SecretFindingKind::BearerToken,
                );
            }
        }
        offset += line.len() + 1;
    }
}

fn detect_tokens(context: DetectionContext<'_>, detected: &mut Vec<DetectedSecret>) {
    for (start, end) in token_ranges(context.text) {
        let token = &context.text[start..end];
        if looks_like_email(token) {
            push_detected(detected, context, start, end, SecretFindingKind::Email);
        }
        if looks_like_phone(token) {
            push_detected(
                detected,
                context,
                start,
                end,
                SecretFindingKind::PhoneNumber,
            );
        }
        if looks_like_private_path(token) {
            push_detected(
                detected,
                context,
                start,
                end,
                SecretFindingKind::PrivatePath,
            );
        }
    }
}

fn detect_redacted_markers(context: DetectionContext<'_>, detected: &mut Vec<DetectedSecret>) {
    for (marker, kind) in [
        ("[REDACTED:API_KEY]", SecretFindingKind::ApiKey),
        ("[REDACTED:BEARER_TOKEN]", SecretFindingKind::BearerToken),
        ("[REDACTED:PRIVATE_KEY]", SecretFindingKind::PrivateKey),
        ("[REDACTED:PASSWORD]", SecretFindingKind::Password),
        ("[REDACTED:PRIVATE_PATH]", SecretFindingKind::PrivatePath),
    ] {
        let mut search_start = 0;
        while let Some(relative_start) = context.text[search_start..].find(marker) {
            let start = search_start + relative_start;
            let end = start + marker.len();
            push_detected_with_action(
                detected,
                context,
                start,
                end,
                kind,
                SecretFindingAction::Allow,
                risk_level(kind),
            );
            search_start = end;
        }
    }
}

fn push_detected(
    detected: &mut Vec<DetectedSecret>,
    context: DetectionContext<'_>,
    start: usize,
    end: usize,
    kind: SecretFindingKind,
) {
    push_detected_with_action(
        detected,
        context,
        start,
        end,
        kind,
        ingest_action(kind),
        risk_level(kind),
    );
}

fn push_detected_with_action(
    detected: &mut Vec<DetectedSecret>,
    context: DetectionContext<'_>,
    start: usize,
    end: usize,
    kind: SecretFindingKind,
    action: SecretFindingAction,
    risk_level: SecretRiskLevel,
) {
    if start >= end || end > context.text.len() {
        return;
    }
    if detected.iter().any(|existing| {
        existing.finding.location.start == start && existing.finding.location.end == end
    }) {
        return;
    }
    let matched_text = &context.text[start..end];
    detected.push(DetectedSecret {
        finding: SecretFinding::new(
            context.scope_id.clone(),
            context.memory_id.cloned(),
            context.source_ref,
            kind,
            action,
            risk_level,
            SecretFindingLocation {
                field: context.field.to_string(),
                start,
                end,
            },
            matched_text,
        ),
    });
}

fn redact_detected(text: &str, detected: &[DetectedSecret]) -> String {
    let mut output = text.to_string();
    let mut ranges = detected
        .iter()
        .filter(|detected| detected.finding.action != SecretFindingAction::Allow)
        .collect::<Vec<_>>();
    ranges.sort_by(|left, right| {
        right
            .finding
            .location
            .start
            .cmp(&left.finding.location.start)
    });
    for detected in ranges {
        output.replace_range(
            detected.finding.location.start..detected.finding.location.end,
            &detected.finding.redacted_preview,
        );
    }
    output
}

fn assignment_value_range(line: &str, labels: &[&str], min_len: usize) -> Option<(usize, usize)> {
    let lower = line.to_ascii_lowercase();
    for label in labels {
        let Some(label_start) = lower.find(label) else {
            continue;
        };
        let after_label = label_start + label.len();
        let Some(separator_offset) = line[after_label..].find(['=', ':']) else {
            continue;
        };
        let value_start = after_label + separator_offset + 1;
        let value_start = skip_value_prefix(line, value_start);
        let value_end = value_end(line, value_start);
        if line[value_start..value_end].starts_with("[REDACTED:") {
            continue;
        }
        if value_end.saturating_sub(value_start) >= min_len {
            return Some((value_start, value_end));
        }
    }
    None
}

fn skip_value_prefix(line: &str, start: usize) -> usize {
    line[start..]
        .char_indices()
        .find(|(_, ch)| !ch.is_whitespace() && !matches!(ch, '"' | '\'' | '`'))
        .map(|(index, _)| start + index)
        .unwrap_or(line.len())
}

fn value_end(line: &str, start: usize) -> usize {
    line[start..]
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace() || matches!(ch, '"' | '\'' | '`' | ',' | ';'))
        .map(|(index, _)| start + index)
        .unwrap_or(line.len())
}

fn token_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = None;
    for (index, ch) in text.char_indices() {
        if ch.is_whitespace() || matches!(ch, '"' | '\'' | '`' | '<' | '>' | '(' | ')' | '[' | ']')
        {
            if let Some(token_start) = start.take() {
                ranges.push(trim_token_range(text, token_start, index));
            }
        } else if start.is_none() {
            start = Some(index);
        }
    }
    if let Some(token_start) = start {
        ranges.push(trim_token_range(text, token_start, text.len()));
    }
    ranges
        .into_iter()
        .filter(|(start, end)| start < end)
        .collect()
}

fn trim_token_range(text: &str, mut start: usize, mut end: usize) -> (usize, usize) {
    while start < end {
        let Some(ch) = text[start..end].chars().next() else {
            break;
        };
        if !matches!(ch, ',' | ';' | ':' | '.') {
            break;
        }
        start += ch.len_utf8();
    }
    while start < end {
        let Some(ch) = text[start..end].chars().next_back() else {
            break;
        };
        if !matches!(ch, ',' | ';' | ':' | '.') {
            break;
        }
        end -= ch.len_utf8();
    }
    (start, end)
}

fn looks_like_email(token: &str) -> bool {
    let Some(at) = token.find('@') else {
        return false;
    };
    at > 0 && token[at + 1..].contains('.') && token.len() >= 6
}

fn looks_like_phone(token: &str) -> bool {
    let digits = token.chars().filter(|ch| ch.is_ascii_digit()).count();
    digits >= 10 && (token.starts_with('+') || token.contains('-'))
}

fn looks_like_private_path(token: &str) -> bool {
    token.contains("/.ssh/")
        || token.contains("\\.ssh\\")
        || token.ends_with("/id_rsa")
        || token.ends_with("\\id_rsa")
        || (token.starts_with("/Users/") && token.ends_with(".pem"))
}

fn ingest_action(kind: SecretFindingKind) -> SecretFindingAction {
    match kind {
        SecretFindingKind::PrivateKey => SecretFindingAction::Deny,
        SecretFindingKind::ApiKey | SecretFindingKind::BearerToken => SecretFindingAction::Redact,
        SecretFindingKind::Password | SecretFindingKind::PrivatePath => {
            SecretFindingAction::Proposal
        }
        SecretFindingKind::Email | SecretFindingKind::PhoneNumber => SecretFindingAction::Allow,
    }
}

fn risk_level(kind: SecretFindingKind) -> SecretRiskLevel {
    match kind {
        SecretFindingKind::PrivateKey => SecretRiskLevel::Critical,
        SecretFindingKind::ApiKey | SecretFindingKind::BearerToken => SecretRiskLevel::High,
        SecretFindingKind::Password | SecretFindingKind::PrivatePath => SecretRiskLevel::Medium,
        SecretFindingKind::Email | SecretFindingKind::PhoneNumber => SecretRiskLevel::Low,
    }
}

fn secret_recall_findings(memory: &Memory) -> Vec<SecretFinding> {
    SecretDetector::detect_text(
        &memory.scope_id,
        Some(&memory.id),
        &format!("memory:{}", memory.id.as_str()),
        "body",
        &format!("{}\n{}", memory.title, memory.body),
    )
    .into_iter()
    .map(|finding| {
        if finding.action == SecretFindingAction::Allow {
            finding
        } else {
            finding.with_action(SecretFindingAction::RecallBlock)
        }
    })
    .collect()
}

fn push_risk(
    risks: &mut Vec<MemoryHealthRisk>,
    memory: &Memory,
    kind: MemoryHealthRiskKind,
    severity: MemoryHealthSeverity,
    detail: &str,
    suggested_action: MemoryHealthSuggestedAction,
) {
    risks.push(MemoryHealthRisk {
        kind,
        severity,
        memory_id: Some(memory.id.clone()),
        title: Some(memory.title.clone()),
        detail: detail.to_string(),
        suggested_action,
    });
}

fn normalize_title(title: &str) -> String {
    title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

pub(crate) fn rewrite_artifact_body(artifact: &mut Artifact, body: String) {
    artifact.content_text = body;
    artifact.content_hash = Artifact::compute_content_hash(&artifact.content_text);
    artifact.updated_at = OffsetDateTime::now_utc();
}

#[cfg(test)]
mod tests {
    use super::{
        SecretDetector, analyze_memory_health, apply_secret_recall_guard,
        apply_sensitive_ingest_guard, redact_hard_deleted_memory_body, write_memory_health_report,
    };
    use crate::MemoryHealthReport;
    use memory_domain::{
        Memory, MemoryHealthRiskKind, MemoryKind, MemoryState, ScopeId, SecretFindingAction,
        SecretFindingKind, Sensitivity,
    };
    use tempfile::tempdir;

    #[test]
    fn detector_classifies_secret_and_pii_actions() {
        let scope_id = ScopeId::from_string("scp_secret_test");
        let findings = SecretDetector::detect_text(
            &scope_id,
            None,
            "artifact:art_1",
            "body",
            "api_key = sk_test_1234567890abcdef owner rou@example.com path /Users/rou/.ssh/id_rsa",
        );

        assert!(findings.iter().any(|finding| {
            finding.kind == SecretFindingKind::ApiKey
                && finding.action == SecretFindingAction::Redact
        }));
        assert!(findings.iter().any(|finding| {
            finding.kind == SecretFindingKind::Email && finding.action == SecretFindingAction::Allow
        }));
        assert!(findings.iter().any(|finding| {
            finding.kind == SecretFindingKind::PrivatePath
                && finding.action == SecretFindingAction::Proposal
        }));

        let redacted = SecretDetector::detect_text(
            &scope_id,
            None,
            "artifact:art_2",
            "body",
            "service [REDACTED:API_KEY]",
        );
        assert!(redacted.iter().any(|finding| {
            finding.kind == SecretFindingKind::ApiKey
                && finding.action == SecretFindingAction::Allow
        }));
    }

    #[test]
    fn ingest_guard_redacts_and_denies_expected_inputs() {
        let scope_id = ScopeId::from_string("scp_guard");
        let redacted = apply_sensitive_ingest_guard(
            &scope_id,
            "artifact:art_1",
            "api_key: sk_test_1234567890abcdef",
        );
        assert!(!redacted.denied);
        assert!(redacted.body.contains("[REDACTED:API_KEY]"));

        let denied = apply_sensitive_ingest_guard(
            &scope_id,
            "artifact:art_2",
            "-----BEGIN PRIVATE KEY-----\nabc\n-----END PRIVATE KEY-----",
        );
        assert!(denied.denied);
    }

    #[test]
    fn recall_guard_blocks_secret_but_keeps_low_risk_pii() {
        let scope_id = ScopeId::from_string("scp_recall_guard");
        let secret = Memory::new(
            scope_id.clone(),
            MemoryKind::Fact,
            "api key",
            "api_key=sk_test_1234567890abcdef",
        )
        .unwrap();
        let pii = Memory::new(
            scope_id,
            MemoryKind::Fact,
            "contact",
            "owner rou@example.com",
        )
        .unwrap();

        let guarded = apply_secret_recall_guard(vec![secret, pii]);
        assert_eq!(guarded.memories.len(), 1);
        assert_eq!(guarded.blocked_count, 1);
        assert!(
            guarded
                .findings
                .iter()
                .any(|finding| { finding.action == SecretFindingAction::RecallBlock })
        );

        let redacted = Memory::new(
            ScopeId::from_string("scp_recall_guard_redacted"),
            MemoryKind::Fact,
            "redacted key",
            "api_key=[REDACTED:API_KEY]",
        )
        .unwrap();
        let guarded_redacted = apply_secret_recall_guard(vec![redacted]);
        assert_eq!(guarded_redacted.memories.len(), 1);
        assert_eq!(guarded_redacted.blocked_count, 0);
    }

    #[test]
    fn hard_delete_redaction_clears_secret_body_only() {
        let scope_id = ScopeId::from_string("scp_hard_delete");
        let mut secret = Memory::new(
            scope_id.clone(),
            MemoryKind::Fact,
            "legacy secret",
            "api_key=sk_test_1234567890abcdef",
        )
        .unwrap();
        let mut safe = Memory::new(scope_id, MemoryKind::Fact, "safe", "plain note").unwrap();

        assert!(redact_hard_deleted_memory_body(&mut secret));
        assert_eq!(secret.body, "[REDACTED:HARD_DELETE_SECRET]");
        assert!(!redact_hard_deleted_memory_body(&mut safe));
        assert_eq!(safe.body, "plain note");
    }

    #[test]
    fn health_report_classifies_operational_risks() {
        let scope_id = ScopeId::from_string("scp_health");
        let mut memory = Memory::new(
            scope_id.clone(),
            MemoryKind::Risk,
            "Duplicate Risk",
            "api_key=sk_test_1234567890abcdef",
        )
        .unwrap();
        memory.sensitivity = Sensitivity::Restricted;
        memory.scores.confidence = 0.2;
        memory.scores.freshness = 0.1;
        memory.state = MemoryState::Candidate;
        let duplicate =
            Memory::new(scope_id.clone(), MemoryKind::Fact, "duplicate risk", "safe").unwrap();

        let report = analyze_memory_health(
            Some(scope_id),
            &[memory, duplicate],
            time::OffsetDateTime::now_utc(),
        );

        assert_eq!(report.total, 2);
        assert_eq!(report.restricted, 1);
        assert_eq!(report.low_confidence, 1);
        assert!(report.secret_findings >= 1);
        assert!(report.high_risk_secret_findings >= 1);
        assert!(
            report
                .risks
                .iter()
                .any(|risk| risk.kind == MemoryHealthRiskKind::Duplicate)
        );
    }

    #[test]
    fn health_report_writer_outputs_json_and_markdown() {
        let tempdir = tempdir().unwrap();
        let report = MemoryHealthReport {
            scope_id: Some(ScopeId::from_string("scp_report")),
            total: 0,
            active: 0,
            candidate: 0,
            needs_review: 0,
            archived: 0,
            deprecated: 0,
            forgotten: 0,
            deleted: 0,
            restricted: 0,
            stale: 0,
            source_backed: 0,
            low_confidence: 0,
            secret_findings: 0,
            high_risk_secret_findings: 0,
            risks: Vec::new(),
            suggested_actions: Vec::new(),
            generated_at: time::OffsetDateTime::now_utc(),
        };
        let paths = write_memory_health_report(tempdir.path(), &report).unwrap();

        assert!(paths.json.exists());
        assert!(paths.markdown.exists());
        let markdown = std::fs::read_to_string(paths.markdown).unwrap();
        assert!(markdown.contains("Memory Health Report"));
    }
}
