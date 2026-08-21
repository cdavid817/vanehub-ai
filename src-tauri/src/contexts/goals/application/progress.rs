use serde::Serialize;

use super::super::domain::{GoalLinkTarget, GoalStatus};
use super::ports::LinkProgress;

/// The status a goal presents, which is not the status it stores.
///
/// `AwaitingAcceptance` exists only here. Persisting it would outlive the
/// condition that produced it: nothing publishes child state changes, so a
/// reopened child would leave the stored value permanently wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DerivedGoalStatus {
    Draft,
    Active,
    AwaitingAcceptance,
    Achieved,
    Abandoned,
}

impl DerivedGoalStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::AwaitingAcceptance => "awaiting_acceptance",
            Self::Achieved => "achieved",
            Self::Abandoned => "abandoned",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkProgressView {
    pub(crate) target_kind: GoalLinkTarget,
    pub(crate) target_id: String,
    pub(crate) progress: LinkProgress,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GoalProgress {
    pub(crate) derived_status: DerivedGoalStatus,
    /// Links that count toward acceptance: neither a session nor unresolvable.
    pub(crate) counted: usize,
    pub(crate) terminal: usize,
    pub(crate) unresolvable: usize,
    pub(crate) links: Vec<LinkProgressView>,
}

impl GoalProgress {
    pub(crate) fn awaiting_acceptance(&self) -> bool {
        self.derived_status == DerivedGoalStatus::AwaitingAcceptance
    }
}

/// Recomputes a goal's presented status from its children on every read.
///
/// Sessions never count, and unresolvable children leave the denominator so a
/// deleted linked object cannot strand a goal short of acceptance.
pub(crate) fn derive(status: GoalStatus, links: &[LinkProgressView]) -> GoalProgress {
    let derivable = links
        .iter()
        .filter(|link| link.target_kind.participates_in_derivation());

    let mut counted = 0_usize;
    let mut terminal = 0_usize;
    let mut unresolvable = 0_usize;

    for link in derivable {
        match link.progress {
            LinkProgress::Unresolvable => unresolvable += 1,
            LinkProgress::Terminal => {
                counted += 1;
                terminal += 1;
            }
            LinkProgress::Active => counted += 1,
        }
    }

    let derived_status = match status {
        GoalStatus::Draft => DerivedGoalStatus::Draft,
        GoalStatus::Achieved => DerivedGoalStatus::Achieved,
        GoalStatus::Abandoned => DerivedGoalStatus::Abandoned,
        GoalStatus::Active => {
            if counted > 0 && terminal == counted {
                DerivedGoalStatus::AwaitingAcceptance
            } else {
                DerivedGoalStatus::Active
            }
        }
    };

    GoalProgress {
        derived_status,
        counted,
        terminal,
        unresolvable,
        links: links.to_vec(),
    }
}
