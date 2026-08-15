use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use super::super::domain::{Goal, GoalInput, GoalLink, GoalLinkTarget, GoalStatus};
use super::ports::{GoalRepository, LinkProgress, LinkProgressProbe};
use super::progress::{derive, GoalProgress, LinkProgressView};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GoalDetail {
    pub(crate) goal: Goal,
    pub(crate) progress: GoalProgress,
}

pub(crate) struct GoalApplicationService {
    repository: Arc<dyn GoalRepository>,
    probes: Vec<Arc<dyn LinkProgressProbe>>,
}

impl GoalApplicationService {
    pub(crate) fn new(
        repository: Arc<dyn GoalRepository>,
        probes: Vec<Arc<dyn LinkProgressProbe>>,
    ) -> Self {
        Self { repository, probes }
    }

    pub(crate) fn create(&self, input: GoalInput) -> Result<GoalDetail, String> {
        let now = timestamp();
        let seeded = GoalInput {
            id: format!("goal-{}", Uuid::new_v4()),
            ..input
        };
        let goal = Goal::new(seeded, &now).map_err(|error| error.to_string())?;
        self.repository.insert_goal(&goal)?;
        self.detail(goal)
    }

    pub(crate) fn update(&self, goal_id: &str, input: GoalInput) -> Result<GoalDetail, String> {
        let mut goal = self.require_goal(goal_id)?;
        goal.apply_edits(input, &timestamp())
            .map_err(|error| error.to_string())?;
        self.repository.update_goal(&goal)?;
        self.detail(goal)
    }

    pub(crate) fn delete(&self, goal_id: &str) -> Result<(), String> {
        self.require_goal(goal_id)?;
        self.repository.delete_goal(goal_id)
    }

    pub(crate) fn get(&self, goal_id: &str) -> Result<GoalDetail, String> {
        let goal = self.require_goal(goal_id)?;
        self.detail(goal)
    }

    pub(crate) fn list(&self) -> Result<Vec<GoalDetail>, String> {
        self.repository
            .list_goals()?
            .into_iter()
            .map(|goal| self.detail(goal))
            .collect()
    }

    pub(crate) fn link(
        &self,
        goal_id: &str,
        target_kind: GoalLinkTarget,
        target_id: &str,
    ) -> Result<GoalDetail, String> {
        let goal = self.require_goal(goal_id)?;
        let target_id = target_id.trim();
        if target_id.is_empty() {
            return Err("A link target id is required.".to_string());
        }
        if self
            .repository
            .link_exists(goal_id, target_kind, target_id)?
        {
            return Err(format!(
                "This {} is already linked to the goal.",
                target_kind.as_str()
            ));
        }
        self.repository.insert_link(&GoalLink {
            goal_id: goal_id.to_string(),
            target_kind,
            target_id: target_id.to_string(),
            linked_at: timestamp(),
        })?;
        self.detail(goal)
    }

    pub(crate) fn unlink(
        &self,
        goal_id: &str,
        target_kind: GoalLinkTarget,
        target_id: &str,
    ) -> Result<GoalDetail, String> {
        let goal = self.require_goal(goal_id)?;
        self.repository
            .delete_link(goal_id, target_kind, target_id)?;
        self.detail(goal)
    }

    pub(crate) fn activate(&self, goal_id: &str) -> Result<GoalDetail, String> {
        self.move_to(goal_id, GoalStatus::Active)
    }

    pub(crate) fn abandon(&self, goal_id: &str) -> Result<GoalDetail, String> {
        self.move_to(goal_id, GoalStatus::Abandoned)
    }

    /// Reopening is the same transition as activating; it exists as its own
    /// entry point so the command surface reads the way the UI does.
    pub(crate) fn reopen(&self, goal_id: &str) -> Result<GoalDetail, String> {
        self.move_to(goal_id, GoalStatus::Active)
    }

    pub(crate) fn accept(&self, goal_id: &str) -> Result<GoalDetail, String> {
        let mut goal = self.require_goal(goal_id)?;
        let progress = self.progress_for(&goal)?;
        goal.accept(progress.awaiting_acceptance(), &timestamp())
            .map_err(|error| error.to_string())?;
        self.repository.update_goal(&goal)?;
        self.detail(goal)
    }

    fn move_to(&self, goal_id: &str, next: GoalStatus) -> Result<GoalDetail, String> {
        let mut goal = self.require_goal(goal_id)?;
        goal.move_to(next, &timestamp())
            .map_err(|error| error.to_string())?;
        self.repository.update_goal(&goal)?;
        self.detail(goal)
    }

    fn require_goal(&self, goal_id: &str) -> Result<Goal, String> {
        self.repository
            .find_goal(goal_id)?
            .ok_or_else(|| "The goal was not found.".to_string())
    }

    fn detail(&self, goal: Goal) -> Result<GoalDetail, String> {
        let progress = self.progress_for(&goal)?;
        Ok(GoalDetail { goal, progress })
    }

    fn progress_for(&self, goal: &Goal) -> Result<GoalProgress, String> {
        let views = self
            .repository
            .list_links(&goal.id)?
            .into_iter()
            .map(|link| LinkProgressView {
                progress: self.probe(link.target_kind, &link.target_id),
                target_kind: link.target_kind,
                target_id: link.target_id,
            })
            .collect::<Vec<_>>();

        Ok(derive(goal.status, &views))
    }

    /// A kind with no registered probe reads as unresolvable rather than as
    /// finished work, so a composition gap can never accept a goal by itself.
    fn probe(&self, target_kind: GoalLinkTarget, target_id: &str) -> LinkProgress {
        self.probes
            .iter()
            .find(|probe| probe.kind() == target_kind)
            .map_or(LinkProgress::Unresolvable, |probe| probe.probe(target_id))
    }
}

fn timestamp() -> String {
    Utc::now().to_rfc3339()
}
