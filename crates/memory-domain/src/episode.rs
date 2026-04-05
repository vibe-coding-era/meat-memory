use crate::{DomainError, EpisodeId, ScopeId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpisodeKind {
    ChatSession,
    CodingTask,
    Meeting,
    ResearchRun,
    AutomationRun,
    Incident,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpisodeState {
    Open,
    Idle,
    Closed,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: EpisodeId,
    pub scope_id: ScopeId,
    pub kind: EpisodeKind,
    pub state: EpisodeState,
    pub title: String,
    pub summary: Option<String>,
    pub participants: Vec<String>,
    pub started_at: OffsetDateTime,
    pub ended_at: Option<OffsetDateTime>,
}

impl Episode {
    pub fn new(
        scope_id: ScopeId,
        kind: EpisodeKind,
        title: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let title = title.into();
        if title.trim().is_empty() {
            return Err(DomainError::EmptyField {
                field: "episode.title",
            });
        }

        Ok(Self {
            id: EpisodeId::new(),
            scope_id,
            kind,
            state: EpisodeState::Open,
            title,
            summary: None,
            participants: Vec::new(),
            started_at: OffsetDateTime::now_utc(),
            ended_at: None,
        })
    }

    pub fn transition_to(&mut self, next: EpisodeState) -> Result<(), DomainError> {
        let allowed = matches!(
            (self.state, next),
            (EpisodeState::Open, EpisodeState::Idle)
                | (EpisodeState::Idle, EpisodeState::Closed)
                | (EpisodeState::Closed, EpisodeState::Archived)
        );

        if !allowed {
            return Err(DomainError::InvalidTransition {
                from: self.state.as_str(),
                to: next.as_str(),
            });
        }

        self.state = next;
        if matches!(next, EpisodeState::Closed | EpisodeState::Archived) {
            self.ended_at = Some(OffsetDateTime::now_utc());
        }
        Ok(())
    }
}

impl EpisodeState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Idle => "idle",
            Self::Closed => "closed",
            Self::Archived => "archived",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Episode, EpisodeKind, EpisodeState};
    use crate::ScopeId;

    #[test]
    fn only_allows_linear_episode_transitions() {
        let mut episode =
            Episode::new(ScopeId::new(), EpisodeKind::CodingTask, "Fix search").unwrap();
        assert!(episode.transition_to(EpisodeState::Idle).is_ok());
        assert!(episode.transition_to(EpisodeState::Archived).is_err());
    }
}
