use memory_domain::{ScopeType, Sensitivity, Visibility};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    Review,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WritePolicyInput {
    pub visibility: Visibility,
    pub sensitivity: Sensitivity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublishPolicyInput {
    pub source_scope_type: ScopeType,
    pub target_scope_type: ScopeType,
    pub target_visibility: Visibility,
    pub sensitivity: Sensitivity,
}

pub fn evaluate_write_policy(input: WritePolicyInput) -> PolicyDecision {
    match (input.visibility, input.sensitivity) {
        (Visibility::Organization, Sensitivity::Restricted) => PolicyDecision::Deny,
        (Visibility::Team, Sensitivity::Restricted) => PolicyDecision::Review,
        _ => PolicyDecision::Allow,
    }
}

pub fn evaluate_publish_policy(input: PublishPolicyInput) -> PolicyDecision {
    use ScopeType::{Org, Project, Session, Team, User, Workspace};

    if matches!(input.target_visibility, Visibility::Organization)
        && matches!(input.sensitivity, Sensitivity::Restricted)
    {
        return PolicyDecision::Deny;
    }

    match (input.source_scope_type, input.target_scope_type) {
        (Session, User | Project) => PolicyDecision::Review,
        (Session, Team | Org | Workspace) => PolicyDecision::Deny,
        (User, Project) | (Project, Team) | (Team, Org) => {
            if matches!(
                input.sensitivity,
                Sensitivity::Private | Sensitivity::Restricted
            ) {
                PolicyDecision::Review
            } else {
                PolicyDecision::Allow
            }
        }
        (Workspace, Project) | (Org, Team) | (Team, Workspace) | (Workspace, User) => {
            PolicyDecision::Allow
        }
        (left, right) if left == right => evaluate_write_policy(WritePolicyInput {
            visibility: input.target_visibility,
            sensitivity: input.sensitivity,
        }),
        _ => PolicyDecision::Deny,
    }
}

pub fn redact_for_shared_scope(body: &str, sensitivity: Sensitivity) -> String {
    if matches!(sensitivity, Sensitivity::Public | Sensitivity::Internal) {
        return body.to_string();
    }

    let mut redacted = body.to_string();
    for marker in [
        "token",
        "secret",
        "password",
        "api key",
        "access key",
        "凭证",
        "密钥",
        "密码",
    ] {
        redacted = redact_marker(&redacted, marker);
    }
    redacted = redact_emails(&redacted);
    redacted
}

fn redact_marker(body: &str, marker: &str) -> String {
    let lowered = body.to_lowercase();
    let lowered_marker = marker.to_lowercase();
    let mut output = String::with_capacity(body.len());
    let mut index = 0usize;

    while let Some(found) = lowered[index..].find(&lowered_marker) {
        let start = index + found;
        let marker_end = start + lowered_marker.len();
        output.push_str(&body[index..marker_end]);

        let tail = &body[marker_end..];
        let trimmed_tail = tail.trim_start_matches([' ', '\t', ':', '：', '=', '-']);
        let consumed = tail.len() - trimmed_tail.len();
        output.push_str(&tail[..consumed]);

        let secret_len = trimmed_tail
            .chars()
            .take_while(|ch| !ch.is_whitespace() && !matches!(ch, ',' | '，' | ';' | '；'))
            .count();
        if secret_len > 0 {
            let secret_bytes = trimmed_tail
                .char_indices()
                .nth(secret_len)
                .map(|(offset, _)| offset)
                .unwrap_or(trimmed_tail.len());
            output.push_str("[REDACTED]");
            index = marker_end + consumed + secret_bytes;
        } else {
            index = marker_end;
        }
    }

    output.push_str(&body[index..]);
    output
}

fn redact_emails(body: &str) -> String {
    body.split_whitespace()
        .map(|token| {
            if token.contains('@') && token.contains('.') {
                "[REDACTED_EMAIL]".to_string()
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn evaluate_visibility(visibility: &str, sensitivity: &str) -> PolicyDecision {
    match (visibility, sensitivity) {
        ("private", _) => PolicyDecision::Allow,
        ("team", "restricted") => PolicyDecision::Review,
        ("organization", "restricted") => PolicyDecision::Deny,
        _ => PolicyDecision::Allow,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PolicyDecision, PublishPolicyInput, WritePolicyInput, evaluate_publish_policy,
        evaluate_visibility, evaluate_write_policy, redact_for_shared_scope,
    };
    use memory_domain::{ScopeType, Sensitivity, Visibility};

    #[test]
    fn restricts_org_restricted_content() {
        assert_eq!(
            evaluate_visibility("organization", "restricted"),
            PolicyDecision::Deny
        );
    }

    #[test]
    fn enum_policy_matches_string_policy() {
        assert_eq!(
            evaluate_write_policy(WritePolicyInput {
                visibility: Visibility::Team,
                sensitivity: Sensitivity::Restricted,
            }),
            PolicyDecision::Review
        );
    }

    #[test]
    fn publish_policy_respects_scope_ladder_and_sensitivity() {
        assert_eq!(
            evaluate_publish_policy(PublishPolicyInput {
                source_scope_type: ScopeType::User,
                target_scope_type: ScopeType::Project,
                target_visibility: Visibility::Project,
                sensitivity: Sensitivity::Internal,
            }),
            PolicyDecision::Allow
        );
        assert_eq!(
            evaluate_publish_policy(PublishPolicyInput {
                source_scope_type: ScopeType::Project,
                target_scope_type: ScopeType::Team,
                target_visibility: Visibility::Team,
                sensitivity: Sensitivity::Private,
            }),
            PolicyDecision::Review
        );
        assert_eq!(
            evaluate_publish_policy(PublishPolicyInput {
                source_scope_type: ScopeType::Session,
                target_scope_type: ScopeType::Team,
                target_visibility: Visibility::Team,
                sensitivity: Sensitivity::Internal,
            }),
            PolicyDecision::Deny
        );
    }

    #[test]
    fn redaction_masks_simple_secrets_and_emails() {
        let body = "owner rou@example.com token: abc123 密钥 sk-live-1";
        let redacted = redact_for_shared_scope(body, Sensitivity::Restricted);

        assert!(redacted.contains("[REDACTED_EMAIL]"));
        assert!(redacted.contains("token: [REDACTED]"));
        assert!(redacted.contains("密钥 [REDACTED]"));
    }
}
