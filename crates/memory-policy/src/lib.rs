use memory_domain::{Sensitivity, Visibility};

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

pub fn evaluate_write_policy(input: WritePolicyInput) -> PolicyDecision {
    match (input.visibility, input.sensitivity) {
        (Visibility::Organization, Sensitivity::Restricted) => PolicyDecision::Deny,
        (Visibility::Team, Sensitivity::Restricted) => PolicyDecision::Review,
        _ => PolicyDecision::Allow,
    }
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
    use super::{PolicyDecision, WritePolicyInput, evaluate_visibility, evaluate_write_policy};
    use memory_domain::{Sensitivity, Visibility};

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
}
