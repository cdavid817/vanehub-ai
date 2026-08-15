use serde::{Deserialize, Serialize};

use super::goal::GoalDomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GoalLinkTarget {
    Plan,
    Loop,
    WorkItem,
    Session,
}

impl GoalLinkTarget {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Loop => "loop",
            Self::WorkItem => "work_item",
            Self::Session => "session",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, GoalDomainError> {
        match value {
            "plan" => Ok(Self::Plan),
            "loop" => Ok(Self::Loop),
            "work_item" => Ok(Self::WorkItem),
            "session" => Ok(Self::Session),
            other => Err(GoalDomainError::InvalidStatus(other.to_string())),
        }
    }

    /// Sessions are linked for navigation only. They have no completion
    /// semantics, so counting them would leave every goal permanently short of
    /// acceptance.
    pub(crate) fn participates_in_derivation(self) -> bool {
        !matches!(self, Self::Session)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GoalLink {
    pub(crate) goal_id: String,
    pub(crate) target_kind: GoalLinkTarget,
    pub(crate) target_id: String,
    pub(crate) linked_at: String,
}
