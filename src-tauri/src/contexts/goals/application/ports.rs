use super::super::domain::{Goal, GoalLink, GoalLinkTarget};

/// What a single linked child contributes to acceptance readiness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinkProgress {
    /// The child reached a state it cannot leave on its own.
    Terminal,
    /// The child is still in play, including a Loop parked at its own
    /// awaiting-acceptance state.
    Active,
    /// The child was deleted, or its status could not be read. It is dropped
    /// from the denominator rather than blocking the goal forever.
    Unresolvable,
}

impl LinkProgress {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Active => "active",
            Self::Unresolvable => "unresolvable",
        }
    }
}

/// Reads one executor kind's status. Implementations live in `infrastructure`
/// so the domain and application layers never import an executor context.
///
/// A probe returns `Unresolvable` rather than an error: one failing lookup must
/// not fail the whole goal read.
pub(crate) trait LinkProgressProbe: Send + Sync {
    fn kind(&self) -> GoalLinkTarget;
    fn probe(&self, target_id: &str) -> LinkProgress;
}

pub(crate) trait GoalRepository: Send + Sync {
    fn insert_goal(&self, goal: &Goal) -> Result<(), String>;
    fn update_goal(&self, goal: &Goal) -> Result<(), String>;
    fn delete_goal(&self, goal_id: &str) -> Result<(), String>;
    fn find_goal(&self, goal_id: &str) -> Result<Option<Goal>, String>;
    fn list_goals(&self) -> Result<Vec<Goal>, String>;

    fn insert_link(&self, link: &GoalLink) -> Result<(), String>;
    fn delete_link(
        &self,
        goal_id: &str,
        target_kind: GoalLinkTarget,
        target_id: &str,
    ) -> Result<(), String>;
    fn list_links(&self, goal_id: &str) -> Result<Vec<GoalLink>, String>;
    fn link_exists(
        &self,
        goal_id: &str,
        target_kind: GoalLinkTarget,
        target_id: &str,
    ) -> Result<bool, String>;
}
