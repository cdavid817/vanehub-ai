use serde::{Deserialize, Serialize};

pub(crate) const ORCHESTRATION_SCHEMA_VERSION_V1: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvolutionTriggerFamily {
    StartupRecovery,
    PeriodicMaintenance,
    ApplicationIdleTransition,
    AgentRunCompletion,
    ConversationCompletion,
    ExplicitFeedbackCommit,
    VerificationCompletion,
    DelegatedUtilityCompletion,
    RelevantPolicyOrSkillChange,
    ManualRunRequest,
}

pub(crate) const EVOLUTION_TRIGGER_FAMILIES_V1: [EvolutionTriggerFamily; 10] = [
    EvolutionTriggerFamily::StartupRecovery,
    EvolutionTriggerFamily::PeriodicMaintenance,
    EvolutionTriggerFamily::ApplicationIdleTransition,
    EvolutionTriggerFamily::AgentRunCompletion,
    EvolutionTriggerFamily::ConversationCompletion,
    EvolutionTriggerFamily::ExplicitFeedbackCommit,
    EvolutionTriggerFamily::VerificationCompletion,
    EvolutionTriggerFamily::DelegatedUtilityCompletion,
    EvolutionTriggerFamily::RelevantPolicyOrSkillChange,
    EvolutionTriggerFamily::ManualRunRequest,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvolutionTriggerRegistryError {
    UnsupportedVersion,
    UnknownFamily,
}

impl EvolutionTriggerFamily {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::StartupRecovery => "startup_recovery",
            Self::PeriodicMaintenance => "periodic_maintenance",
            Self::ApplicationIdleTransition => "application_idle_transition",
            Self::AgentRunCompletion => "agent_run_completion",
            Self::ConversationCompletion => "conversation_completion",
            Self::ExplicitFeedbackCommit => "explicit_feedback_commit",
            Self::VerificationCompletion => "verification_completion",
            Self::DelegatedUtilityCompletion => "delegated_utility_completion",
            Self::RelevantPolicyOrSkillChange => "relevant_policy_or_skill_change",
            Self::ManualRunRequest => "manual_run_request",
        }
    }

    pub(crate) fn from_versioned_name(
        schema_version: u16,
        value: &str,
    ) -> Result<Self, EvolutionTriggerRegistryError> {
        if schema_version != ORCHESTRATION_SCHEMA_VERSION_V1 {
            return Err(EvolutionTriggerRegistryError::UnsupportedVersion);
        }
        EVOLUTION_TRIGGER_FAMILIES_V1
            .into_iter()
            .find(|family| family.as_str() == value)
            .ok_or(EvolutionTriggerRegistryError::UnknownFamily)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvolutionPolicyMode {
    Off,
    Observe,
    Enabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvolutionRunStatus {
    Requested,
    WaitingIdle,
    Running,
    Partial,
    Completed,
    Failed,
    CancelRequested,
    Cancelled,
    Recovered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvolutionStageKind {
    Recover,
    MaintainEvidence,
    BuildSeeds,
    Assess,
    RouteGovernance,
    EvaluateAutoApply,
    ProjectResults,
    Notify,
}

pub(crate) const EVOLUTION_STAGE_ORDER_V1: [EvolutionStageKind; 8] = [
    EvolutionStageKind::Recover,
    EvolutionStageKind::MaintainEvidence,
    EvolutionStageKind::BuildSeeds,
    EvolutionStageKind::Assess,
    EvolutionStageKind::RouteGovernance,
    EvolutionStageKind::EvaluateAutoApply,
    EvolutionStageKind::ProjectResults,
    EvolutionStageKind::Notify,
];

impl EvolutionStageKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Recover => "recover",
            Self::MaintainEvidence => "maintain_evidence",
            Self::BuildSeeds => "build_seeds",
            Self::Assess => "assess",
            Self::RouteGovernance => "route_governance",
            Self::EvaluateAutoApply => "evaluate_auto_apply",
            Self::ProjectResults => "project_results",
            Self::Notify => "notify",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvolutionStageStatus {
    Requested,
    Running,
    Completed,
    SkippedEmpty,
    PartialBudget,
    DeferredIdle,
    FailedRetryable,
    FailedTerminal,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvolutionCheckpointStatus {
    Pending,
    Committed,
    ContinuationRequired,
    Reconciled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AutoEligibilityResult {
    Ineligible,
    Waiting,
    RoutedToCurator,
    WouldApply,
    Eligible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RateReservationStatus {
    Reserved,
    Committed,
    Released,
    RecoveryRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CircuitBreakerStatus {
    Closed,
    Open,
    AwaitingHealth,
    AwaitingAcknowledgement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProbationStatus {
    Active,
    Healthy,
    Regressed,
    Expired,
    Suspended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvolutionActorProvenance {
    InteractiveUser,
    SystemPolicy,
    RuntimeTrigger,
    Recovery,
    WebMock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvolutionResourcePressure {
    Unknown,
    Normal,
    Elevated,
    Critical,
}
