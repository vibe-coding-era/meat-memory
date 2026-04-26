use crate::{DomainError, MemoryId, ProposalId, ScopeId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalType {
    Merge,
    NewVersion,
    Supersede,
    ConflictMark,
    Archive,
    Forget,
    Restore,
    HardDelete,
    DistillUpsert,
}

impl ProposalType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::NewVersion => "new_version",
            Self::Supersede => "supersede",
            Self::ConflictMark => "conflict_mark",
            Self::Archive => "archive",
            Self::Forget => "forget",
            Self::Restore => "restore",
            Self::HardDelete => "hard_delete",
            Self::DistillUpsert => "distill_upsert",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewLevel {
    Auto,
    Suggested,
    Required,
    Blocked,
}

impl ReviewLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Suggested => "suggested",
            Self::Required => "required",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    Open,
    Approved,
    Rejected,
    Applied,
    Canceled,
    Expired,
}

impl ProposalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Applied => "applied",
            Self::Canceled => "canceled",
            Self::Expired => "expired",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryProposal {
    pub id: ProposalId,
    pub scope_id: ScopeId,
    pub proposal_type: ProposalType,
    pub status: ProposalStatus,
    pub review_level: ReviewLevel,
    pub subject_memory_id: Option<MemoryId>,
    pub target_memory_ids: Vec<MemoryId>,
    pub reason: String,
    pub evidence: Vec<String>,
    pub decided_by: Option<String>,
    pub decided_at: Option<OffsetDateTime>,
    pub applied_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl MemoryProposal {
    pub fn new(
        scope_id: ScopeId,
        proposal_type: ProposalType,
        review_level: ReviewLevel,
        reason: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let now = OffsetDateTime::now_utc();
        Ok(Self {
            id: ProposalId::new(),
            scope_id,
            proposal_type,
            status: ProposalStatus::Open,
            review_level,
            subject_memory_id: None,
            target_memory_ids: Vec::new(),
            reason: non_empty(reason.into(), "memory_proposal.reason")?,
            evidence: Vec::new(),
            decided_by: None,
            decided_at: None,
            applied_at: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn with_subject_memory(mut self, subject_memory_id: MemoryId) -> Self {
        self.subject_memory_id = Some(subject_memory_id);
        self.updated_at = OffsetDateTime::now_utc();
        self
    }

    pub fn add_target_memory(&mut self, target_memory_id: MemoryId) {
        self.target_memory_ids.push(target_memory_id);
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn add_evidence(&mut self, evidence: String) -> Result<(), DomainError> {
        self.evidence
            .push(non_empty(evidence, "memory_proposal.evidence")?);
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn approve(&mut self, actor: impl Into<String>) -> Result<(), DomainError> {
        self.transition_from_open(ProposalStatus::Approved, actor)
    }

    pub fn reject(&mut self, actor: impl Into<String>) -> Result<(), DomainError> {
        self.transition_from_open(ProposalStatus::Rejected, actor)
    }

    pub fn expire(&mut self) -> Result<(), DomainError> {
        if self.status != ProposalStatus::Open {
            return Err(DomainError::InvalidTransition {
                from: self.status.as_str(),
                to: ProposalStatus::Expired.as_str(),
            });
        }
        self.status = ProposalStatus::Expired;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn mark_applied(&mut self) -> Result<(), DomainError> {
        if self.status != ProposalStatus::Approved {
            return Err(DomainError::InvalidTransition {
                from: self.status.as_str(),
                to: ProposalStatus::Applied.as_str(),
            });
        }
        let now = OffsetDateTime::now_utc();
        self.status = ProposalStatus::Applied;
        self.applied_at = Some(now);
        self.updated_at = now;
        Ok(())
    }

    fn transition_from_open(
        &mut self,
        target_status: ProposalStatus,
        actor: impl Into<String>,
    ) -> Result<(), DomainError> {
        if self.status != ProposalStatus::Open {
            return Err(DomainError::InvalidTransition {
                from: self.status.as_str(),
                to: target_status.as_str(),
            });
        }
        let now = OffsetDateTime::now_utc();
        self.status = target_status;
        self.decided_by = Some(non_empty(actor.into(), "memory_proposal.actor")?);
        self.decided_at = Some(now);
        self.updated_at = now;
        Ok(())
    }
}

fn non_empty(value: String, field: &'static str) -> Result<String, DomainError> {
    if value.trim().is_empty() {
        return Err(DomainError::EmptyField { field });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{MemoryProposal, ProposalStatus, ProposalType, ReviewLevel};
    use crate::{MemoryId, ScopeId};

    #[test]
    fn proposal_rejects_empty_reason_and_evidence() {
        assert!(
            MemoryProposal::new(
                ScopeId::from_string("scp_v28"),
                ProposalType::Merge,
                ReviewLevel::Suggested,
                " ",
            )
            .is_err()
        );

        let mut proposal = MemoryProposal::new(
            ScopeId::from_string("scp_v28"),
            ProposalType::Merge,
            ReviewLevel::Suggested,
            "merge repeated summaries",
        )
        .unwrap();
        assert!(proposal.add_evidence(" ".to_string()).is_err());
    }

    #[test]
    fn proposal_transitions_from_open_to_approved_to_applied() {
        let mut proposal = MemoryProposal::new(
            ScopeId::from_string("scp_v28"),
            ProposalType::Supersede,
            ReviewLevel::Required,
            "new decision replaces old decision",
        )
        .unwrap()
        .with_subject_memory(MemoryId::from_string("mem_new"));
        proposal.add_target_memory(MemoryId::from_string("mem_old"));
        proposal
            .add_evidence("same entity and decision polarity changed".to_string())
            .unwrap();

        proposal.approve("user").unwrap();
        proposal.mark_applied().unwrap();

        assert!(proposal.id.as_str().starts_with("prp_"));
        assert_eq!(proposal.status, ProposalStatus::Applied);
        assert_eq!(proposal.review_level.as_str(), "required");
        assert_eq!(proposal.subject_memory_id.unwrap().as_str(), "mem_new");
        assert_eq!(proposal.target_memory_ids.len(), 1);
        assert_eq!(proposal.decided_by.as_deref(), Some("user"));
    }

    #[test]
    fn proposal_reject_and_expire_follow_state_rules() {
        let mut rejected = MemoryProposal::new(
            ScopeId::from_string("scp_v28"),
            ProposalType::ConflictMark,
            ReviewLevel::Required,
            "facts disagree",
        )
        .unwrap();
        rejected.reject("user").unwrap();
        assert_eq!(rejected.status, ProposalStatus::Rejected);
        assert!(rejected.approve("user").is_err());

        let mut expired = MemoryProposal::new(
            ScopeId::from_string("scp_v28"),
            ProposalType::Archive,
            ReviewLevel::Suggested,
            "stale memory cleanup",
        )
        .unwrap();
        expired.expire().unwrap();
        assert_eq!(expired.status, ProposalStatus::Expired);
        assert!(expired.mark_applied().is_err());
    }

    #[test]
    fn proposal_type_and_status_render_expected_strings() {
        assert_eq!(ProposalType::Merge.as_str(), "merge");
        assert_eq!(ProposalType::NewVersion.as_str(), "new_version");
        assert_eq!(ProposalType::Supersede.as_str(), "supersede");
        assert_eq!(ProposalType::ConflictMark.as_str(), "conflict_mark");
        assert_eq!(ProposalType::Archive.as_str(), "archive");
        assert_eq!(ProposalType::Forget.as_str(), "forget");
        assert_eq!(ProposalType::Restore.as_str(), "restore");
        assert_eq!(ProposalType::HardDelete.as_str(), "hard_delete");
        assert_eq!(ProposalType::DistillUpsert.as_str(), "distill_upsert");
        assert_eq!(ReviewLevel::Auto.as_str(), "auto");
        assert_eq!(ReviewLevel::Suggested.as_str(), "suggested");
        assert_eq!(ReviewLevel::Required.as_str(), "required");
        assert_eq!(ReviewLevel::Blocked.as_str(), "blocked");
        assert_eq!(ProposalStatus::Open.as_str(), "open");
        assert_eq!(ProposalStatus::Approved.as_str(), "approved");
        assert_eq!(ProposalStatus::Rejected.as_str(), "rejected");
        assert_eq!(ProposalStatus::Applied.as_str(), "applied");
        assert_eq!(ProposalStatus::Canceled.as_str(), "canceled");
        assert_eq!(ProposalStatus::Expired.as_str(), "expired");
    }

    #[test]
    fn proposal_approve_reject_validate_actor() {
        let mut proposal = MemoryProposal::new(
            ScopeId::from_string("scp_v28"),
            ProposalType::Merge,
            ReviewLevel::Suggested,
            "merge repeated summaries",
        )
        .unwrap();

        assert!(proposal.approve(" ").is_err());

        let mut rejected = MemoryProposal::new(
            ScopeId::from_string("scp_v28"),
            ProposalType::Merge,
            ReviewLevel::Suggested,
            "merge repeated summaries",
        )
        .unwrap();
        assert!(rejected.reject(" ").is_err());
    }

    #[test]
    fn proposal_cannot_expire_after_leaving_open_state() {
        let mut proposal = MemoryProposal::new(
            ScopeId::from_string("scp_v28"),
            ProposalType::Merge,
            ReviewLevel::Suggested,
            "merge repeated summaries",
        )
        .unwrap();
        proposal.approve("user").unwrap();

        assert!(proposal.expire().is_err());
    }
}
