use super::{EvolutionCheckpointStatus, EvolutionRunStatus, EvolutionStageKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvolutionRunStatusParseError {
    UnknownStatus,
}

impl EvolutionRunStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Requested => "requested",
            Self::WaitingIdle => "waiting_idle",
            Self::Running => "running",
            Self::Partial => "partial",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::CancelRequested => "cancel_requested",
            Self::Cancelled => "cancelled",
            Self::Recovered => "recovered",
        }
    }

    pub(crate) fn from_persisted(value: &str) -> Result<Self, EvolutionRunStatusParseError> {
        match value {
            "requested" => Ok(Self::Requested),
            "waiting_idle" => Ok(Self::WaitingIdle),
            "running" => Ok(Self::Running),
            "partial" => Ok(Self::Partial),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancel_requested" => Ok(Self::CancelRequested),
            "cancelled" => Ok(Self::Cancelled),
            "recovered" => Ok(Self::Recovered),
            _ => Err(EvolutionRunStatusParseError::UnknownStatus),
        }
    }

    pub(crate) const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    pub(crate) const fn can_transition_to(self, next: Self) -> bool {
        match self {
            Self::Requested => matches!(
                next,
                Self::WaitingIdle | Self::Running | Self::CancelRequested | Self::Failed
            ),
            Self::WaitingIdle => matches!(
                next,
                Self::Running | Self::Partial | Self::CancelRequested | Self::Failed
            ),
            Self::Running => matches!(
                next,
                Self::Partial | Self::Completed | Self::CancelRequested | Self::Failed
            ),
            Self::Partial => matches!(
                next,
                Self::WaitingIdle
                    | Self::Running
                    | Self::Completed
                    | Self::CancelRequested
                    | Self::Failed
            ),
            Self::CancelRequested => {
                matches!(next, Self::Cancelled | Self::Recovered | Self::Failed)
            }
            Self::Recovered => matches!(
                next,
                Self::WaitingIdle
                    | Self::Running
                    | Self::Completed
                    | Self::CancelRequested
                    | Self::Failed
            ),
            Self::Completed | Self::Failed | Self::Cancelled => false,
        }
    }
}

impl EvolutionCheckpointStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Committed => "committed",
            Self::ContinuationRequired => "continuation_required",
            Self::Reconciled => "reconciled",
        }
    }

    pub(crate) fn from_persisted(value: &str) -> Result<Self, EvolutionRunStatusParseError> {
        match value {
            "pending" => Ok(Self::Pending),
            "committed" => Ok(Self::Committed),
            "continuation_required" => Ok(Self::ContinuationRequired),
            "reconciled" => Ok(Self::Reconciled),
            _ => Err(EvolutionRunStatusParseError::UnknownStatus),
        }
    }
}

impl EvolutionStageKind {
    pub(crate) fn from_persisted(value: &str) -> Result<Self, EvolutionRunStatusParseError> {
        match value {
            "recover" => Ok(Self::Recover),
            "maintain_evidence" => Ok(Self::MaintainEvidence),
            "build_seeds" => Ok(Self::BuildSeeds),
            "assess" => Ok(Self::Assess),
            "route_governance" => Ok(Self::RouteGovernance),
            "evaluate_auto_apply" => Ok(Self::EvaluateAutoApply),
            "project_results" => Ok(Self::ProjectResults),
            "notify" => Ok(Self::Notify),
            _ => Err(EvolutionRunStatusParseError::UnknownStatus),
        }
    }
}
