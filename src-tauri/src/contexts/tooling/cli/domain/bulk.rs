//! Upgrading several CLIs as one reviewed batch.
//!
//! A bulk plan is a plan of plans: each eligible tool gets its own single-use `CliActionPlan`, and
//! the batch records why every skipped tool was skipped. The skip reasons are stable codes rather
//! than sentences so the UI can localize them and a test can assert on them.
//!
//! Execution does not silently recompute. An item whose environment moved is skipped as
//! `plan-stale` and the rest continue; one bad item never erases the outcomes of the others.

use chrono::{DateTime, Utc};

use super::ids::{CliActionPlanId, CliBulkPlanId, CliSourceId, CliToolId};
use super::plan::CliActionPlanState;
use super::snapshot::CliMutationOutcome;

/// Why a tool the user asked to upgrade is not in the batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliBulkSkipReason {
    /// Already at the source's latest version. Not a failure -- the common case.
    AlreadyCurrent,
    /// VaneHub can see the installation but does not manage its source.
    DetectOnlySource,
    CatalogUnavailable,
    NeedsAuth,
    /// The active executable does not run, so upgrading it is not the right next step.
    Broken,
    NotInstalled,
    /// The source declares no upgrade for this platform or this action.
    UnsupportedAction,
    /// Versions could not be ordered, so no upgrade can be proven.
    UnorderedVersions,
    /// Nothing establishes which source owns the active installation.
    SourceOwnershipUnproven,
    /// The environment moved between preparation and execution.
    PlanStale,
    /// The item plan outlived its ten-minute window before the batch reached it.
    PlanExpired,
    /// The item plan had already been run. A plan is single-use, batch or not.
    PlanConsumed,
    /// Another operation already holds this tool.
    OperationConflict,
    /// A structured installation conflict makes the mutation target ambiguous or unsafe.
    InstallationConflict,
}

impl CliBulkSkipReason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::AlreadyCurrent => "already-current",
            Self::DetectOnlySource => "detect-only-source",
            Self::CatalogUnavailable => "catalog-unavailable",
            Self::NeedsAuth => "needs-auth",
            Self::Broken => "broken",
            Self::NotInstalled => "not-installed",
            Self::UnsupportedAction => "unsupported-action",
            Self::UnorderedVersions => "unordered-versions",
            Self::SourceOwnershipUnproven => "source-ownership-unproven",
            Self::PlanStale => "plan-stale",
            Self::PlanExpired => "plan-expired",
            Self::PlanConsumed => "plan-consumed",
            Self::OperationConflict => "operation-conflict",
            Self::InstallationConflict => "installation-conflict",
        }
    }

    /// Whether the user could plausibly act on this. `AlreadyCurrent` cannot be acted on and
    /// should not be presented as something to fix.
    pub(crate) fn is_actionable(self) -> bool {
        !matches!(self, Self::AlreadyCurrent)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliBulkSkip {
    pub(crate) agent_id: CliToolId,
    pub(crate) reason: CliBulkSkipReason,
}

/// One tool's slot in the batch, pointing at the single-use plan that will run for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliBulkActionItem {
    pub(crate) agent_id: CliToolId,
    pub(crate) plan_id: CliActionPlanId,
    pub(crate) source_id: CliSourceId,
    pub(crate) current_version: Option<String>,
    pub(crate) target_version: Option<String>,
    pub(crate) requires_elevation: bool,
    pub(crate) requires_network: bool,
    pub(crate) state: CliActionPlanState,
    /// Set when the item reached a terminal state without running its plan.
    pub(crate) skipped_reason: Option<CliBulkSkipReason>,
}

impl CliBulkActionItem {
    pub(crate) fn is_finished(&self) -> bool {
        self.state.is_terminal()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliBulkActionPlan {
    pub(crate) id: CliBulkPlanId,
    pub(crate) revision: u32,
    pub(crate) items: Vec<CliBulkActionItem>,
    pub(crate) skipped: Vec<CliBulkSkip>,
    pub(crate) environment_fingerprint: String,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) expires_at: DateTime<Utc>,
}

/// Counts for the progress line, computed once so the UI does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CliBulkProgress {
    pub(crate) total: usize,
    pub(crate) finished: usize,
    pub(crate) succeeded: usize,
    pub(crate) failed: usize,
    pub(crate) skipped_during_execution: usize,
}

impl CliBulkActionPlan {
    pub(crate) fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    /// Nothing to run. A batch of only skips is still a valid, reviewable answer -- it tells the
    /// user everything is current -- but it must not create an execution operation.
    pub(crate) fn has_work(&self) -> bool {
        !self.items.is_empty()
    }

    pub(crate) fn progress(&self) -> CliBulkProgress {
        let mut progress = CliBulkProgress {
            total: self.items.len(),
            finished: 0,
            succeeded: 0,
            failed: 0,
            skipped_during_execution: 0,
        };
        for item in &self.items {
            if !item.is_finished() {
                continue;
            }
            progress.finished += 1;
            if item.skipped_reason.is_some() {
                progress.skipped_during_execution += 1;
            } else if item.state == CliActionPlanState::Completed {
                progress.succeeded += 1;
            } else {
                progress.failed += 1;
            }
        }
        progress
    }

    /// Skips worth showing as something to look at, in stable order. `AlreadyCurrent` is filtered
    /// out: listing every up-to-date tool as "skipped" buries the ones that need attention.
    pub(crate) fn actionable_skips(&self) -> Vec<&CliBulkSkip> {
        self.skipped
            .iter()
            .filter(|skip| skip.reason.is_actionable())
            .collect()
    }
}

/// What became of one tool in a batch.
///
/// Two arms, not one string. An item either ran and produced one of the five mutation outcomes, or
/// it did not run and has a stable reason. Collapsing them into a single string vocabulary is what
/// produced the `"ran"` placeholder this replaces: a label that says a process started and nothing
/// about whether the machine changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliBulkItemStatus {
    Completed(CliMutationOutcome),
    Skipped(CliBulkSkipReason),
}

impl CliBulkItemStatus {
    /// The discriminant the wire and the UI switch on.
    pub(crate) fn kind(self) -> &'static str {
        match self {
            Self::Completed(_) => "completed",
            Self::Skipped(_) => "skipped",
        }
    }

    pub(crate) fn outcome(self) -> Option<CliMutationOutcome> {
        match self {
            Self::Completed(outcome) => Some(outcome),
            Self::Skipped(_) => None,
        }
    }

    pub(crate) fn reason(self) -> Option<CliBulkSkipReason> {
        match self {
            Self::Skipped(reason) => Some(reason),
            Self::Completed(_) => None,
        }
    }

    /// Whether the machine may have changed for this item.
    ///
    /// A skipped item never touched it; a completed one did unless its outcome says otherwise.
    pub(crate) fn may_have_changed_the_machine(self) -> bool {
        self.outcome()
            .is_some_and(CliMutationOutcome::may_have_changed_the_machine)
    }
}

/// One tool's terminal result inside a batch.
///
/// Every item the batch knew about gets one of these -- the ones that ran, the ones the plan
/// already excluded, and the ones a cancellation stopped before they started. A missing entry
/// would read as "nothing to report", which is never true of a tool the user asked to upgrade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliBulkItemResult {
    pub(crate) agent_id: CliToolId,
    /// `None` for a tool the plan excluded before an item plan existed for it.
    pub(crate) plan_id: Option<CliActionPlanId>,
    pub(crate) source_id: Option<CliSourceId>,
    pub(crate) target_version: Option<String>,
    pub(crate) status: CliBulkItemStatus,
}

impl CliBulkItemResult {
    pub(crate) fn skipped(agent_id: CliToolId, reason: CliBulkSkipReason) -> Self {
        Self {
            agent_id,
            plan_id: None,
            source_id: None,
            target_version: None,
            status: CliBulkItemStatus::Skipped(reason),
        }
    }

    /// The result for an item that had a plan, whether it ran or was refused.
    pub(crate) fn for_item(item: &CliBulkActionItem, status: CliBulkItemStatus) -> Self {
        Self {
            agent_id: item.agent_id.clone(),
            plan_id: Some(item.plan_id.clone()),
            source_id: Some(item.source_id.clone()),
            target_version: item.target_version.clone(),
            status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(seconds, 0).expect("timestamp")
    }

    fn tool(agent_id: &str) -> CliToolId {
        CliToolId::new(agent_id).expect("tool id")
    }

    fn item(agent_id: &str, state: CliActionPlanState) -> CliBulkActionItem {
        CliBulkActionItem {
            agent_id: tool(agent_id),
            plan_id: CliActionPlanId::new(format!("plan-{agent_id}")).expect("plan id"),
            source_id: CliSourceId::new("npm").expect("source id"),
            current_version: Some("1.0.0".to_string()),
            target_version: Some("2.0.0".to_string()),
            requires_elevation: false,
            requires_network: true,
            state,
            skipped_reason: None,
        }
    }

    fn bulk(items: Vec<CliBulkActionItem>, skipped: Vec<CliBulkSkip>) -> CliBulkActionPlan {
        CliBulkActionPlan {
            id: CliBulkPlanId::new("bulk-1").expect("bulk id"),
            revision: 1,
            items,
            skipped,
            environment_fingerprint: "fingerprint-a".to_string(),
            created_at: timestamp(1_000),
            expires_at: timestamp(1_600),
        }
    }

    #[test]
    fn each_item_points_at_its_own_single_use_plan() {
        let plan = bulk(
            vec![
                item("claude-code", CliActionPlanState::Draft),
                item("codex-cli", CliActionPlanState::Draft),
            ],
            Vec::new(),
        );

        let plan_ids = plan
            .items
            .iter()
            .map(|item| item.plan_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(plan_ids, vec!["plan-claude-code", "plan-codex-cli"]);
        // Distinct plans, so one item's consumption cannot admit another.
        assert_ne!(plan.items[0].plan_id, plan.items[1].plan_id);
        assert!(plan.has_work());
    }

    #[test]
    fn a_batch_of_only_skips_is_reviewable_but_has_no_work() {
        let plan = bulk(
            Vec::new(),
            vec![CliBulkSkip {
                agent_id: tool("codex-cli"),
                reason: CliBulkSkipReason::AlreadyCurrent,
            }],
        );
        assert!(!plan.has_work());
        assert_eq!(plan.progress().total, 0);
    }

    #[test]
    fn up_to_date_tools_are_recorded_but_not_surfaced_as_something_to_fix() {
        let plan = bulk(
            vec![item("claude-code", CliActionPlanState::Draft)],
            vec![
                CliBulkSkip {
                    agent_id: tool("codex-cli"),
                    reason: CliBulkSkipReason::AlreadyCurrent,
                },
                CliBulkSkip {
                    agent_id: tool("opencode"),
                    reason: CliBulkSkipReason::DetectOnlySource,
                },
                CliBulkSkip {
                    agent_id: tool("gemini-cli"),
                    reason: CliBulkSkipReason::NeedsAuth,
                },
            ],
        );

        // All three are recorded, so the dialog can still account for every tool.
        assert_eq!(plan.skipped.len(), 3);
        let actionable = plan
            .actionable_skips()
            .into_iter()
            .map(|skip| skip.agent_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(actionable, vec!["opencode", "gemini-cli"]);
        assert!(!CliBulkSkipReason::AlreadyCurrent.is_actionable());
    }

    #[test]
    fn one_stale_item_does_not_erase_the_outcomes_of_the_others() {
        let mut stale = item("gemini-cli", CliActionPlanState::Cancelled);
        stale.skipped_reason = Some(CliBulkSkipReason::PlanStale);

        let plan = bulk(
            vec![
                item("claude-code", CliActionPlanState::Completed),
                item("codex-cli", CliActionPlanState::Failed),
                stale,
                item("opencode", CliActionPlanState::Executing),
            ],
            Vec::new(),
        );

        let progress = plan.progress();
        assert_eq!(progress.total, 4);
        assert_eq!(progress.finished, 3);
        assert_eq!(progress.succeeded, 1);
        assert_eq!(progress.failed, 1);
        assert_eq!(progress.skipped_during_execution, 1);
    }

    #[test]
    fn a_cancelled_item_without_a_skip_reason_counts_as_failed_not_skipped() {
        // Cancelled because the user stopped it is a different outcome from cancelled because the
        // environment moved, and the skip reason is what distinguishes them.
        let plan = bulk(
            vec![item("claude-code", CliActionPlanState::Cancelled)],
            Vec::new(),
        );
        let progress = plan.progress();
        assert_eq!(progress.failed, 1);
        assert_eq!(progress.skipped_during_execution, 0);
    }

    #[test]
    fn progress_ignores_items_that_have_not_finished() {
        let plan = bulk(
            vec![
                item("claude-code", CliActionPlanState::Draft),
                item("codex-cli", CliActionPlanState::Executing),
            ],
            Vec::new(),
        );
        let progress = plan.progress();
        assert_eq!(progress.finished, 0);
        assert_eq!(progress.succeeded, 0);
        assert_eq!(progress.failed, 0);
        assert!(!plan.items[0].is_finished());
        assert!(!plan.items[1].is_finished());
    }

    #[test]
    fn the_batch_expires_against_a_supplied_clock() {
        let plan = bulk(
            vec![item("claude-code", CliActionPlanState::Draft)],
            Vec::new(),
        );
        assert!(!plan.is_expired(timestamp(1_599)));
        assert!(plan.is_expired(timestamp(1_600)));
        assert_eq!(plan.revision, 1);
        assert_eq!(plan.environment_fingerprint, "fingerprint-a");
    }

    #[test]
    fn every_skip_reason_has_a_stable_wire_code() {
        for (reason, wire) in [
            (CliBulkSkipReason::AlreadyCurrent, "already-current"),
            (CliBulkSkipReason::DetectOnlySource, "detect-only-source"),
            (CliBulkSkipReason::CatalogUnavailable, "catalog-unavailable"),
            (CliBulkSkipReason::NeedsAuth, "needs-auth"),
            (CliBulkSkipReason::Broken, "broken"),
            (CliBulkSkipReason::NotInstalled, "not-installed"),
            (CliBulkSkipReason::UnsupportedAction, "unsupported-action"),
            (CliBulkSkipReason::UnorderedVersions, "unordered-versions"),
            (
                CliBulkSkipReason::SourceOwnershipUnproven,
                "source-ownership-unproven",
            ),
            (CliBulkSkipReason::PlanStale, "plan-stale"),
            (CliBulkSkipReason::PlanExpired, "plan-expired"),
            (CliBulkSkipReason::PlanConsumed, "plan-consumed"),
            (CliBulkSkipReason::OperationConflict, "operation-conflict"),
            (
                CliBulkSkipReason::InstallationConflict,
                "installation-conflict",
            ),
        ] {
            assert_eq!(reason.as_str(), wire);
            // Everything except "already current" is worth showing.
            assert_eq!(
                reason.is_actionable(),
                reason != CliBulkSkipReason::AlreadyCurrent
            );
        }
    }

    #[test]
    fn items_carry_the_transition_the_review_dialog_displays() {
        let item = item("claude-code", CliActionPlanState::Draft);
        assert_eq!(item.current_version.as_deref(), Some("1.0.0"));
        assert_eq!(item.target_version.as_deref(), Some("2.0.0"));
        assert_eq!(item.source_id.as_str(), "npm");
        assert!(item.requires_network);
        assert!(!item.requires_elevation);
        assert_eq!(item.skipped_reason, None);
    }
}
