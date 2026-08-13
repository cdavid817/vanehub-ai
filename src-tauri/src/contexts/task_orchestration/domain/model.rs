use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub(crate) enum PlanDomainError {
    #[error("{0} is required")]
    Required(&'static str),
    #[error("invalid Plan state transition from {from} to {to}")]
    InvalidPlanTransition {
        from: &'static str,
        to: &'static str,
    },
    #[error("invalid PlanRun state transition from {from} to {to}")]
    InvalidRunTransition {
        from: &'static str,
        to: &'static str,
    },
    #[error("invalid SubTaskRun state transition from {from} to {to}")]
    InvalidTaskTransition {
        from: &'static str,
        to: &'static str,
    },
    #[cfg_attr(not(test), allow(dead_code))]
    #[error("invalid SubTaskAttempt state transition from {from} to {to}")]
    InvalidAttemptTransition {
        from: &'static str,
        to: &'static str,
    },
    #[cfg_attr(not(test), allow(dead_code))]
    #[error("invalid control request state transition from {from} to {to}")]
    InvalidControlTransition {
        from: &'static str,
        to: &'static str,
    },
    #[error("invalid resource limit: {0}")]
    InvalidLimit(&'static str),
    #[error("invalid execution policy: {0}")]
    InvalidExecutionPolicy(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PlanStatus {
    Draft,
    Approved,
    Archived,
}

impl PlanStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Approved => "approved",
            Self::Archived => "archived",
        }
    }

    pub(crate) fn approve(self) -> Result<Self, PlanDomainError> {
        match self {
            Self::Draft => Ok(Self::Approved),
            state => Err(PlanDomainError::InvalidPlanTransition {
                from: state.as_str(),
                to: Self::Approved.as_str(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PlanRunStatus {
    Queued,
    Preparing,
    Running,
    Verifying,
    Repairing,
    PauseRequested,
    Paused,
    CancelRequested,
    FinalVerifying,
    ActionRequired,
    AwaitingAcceptance,
    RecoveryRequired,
    Completed,
    Failed,
    Cancelled,
}

impl PlanRunStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Preparing => "preparing",
            Self::Running => "running",
            Self::Verifying => "verifying",
            Self::Repairing => "repairing",
            Self::PauseRequested => "pause_requested",
            Self::Paused => "paused",
            Self::CancelRequested => "cancel_requested",
            Self::FinalVerifying => "final_verifying",
            Self::ActionRequired => "action_required",
            Self::AwaitingAcceptance => "awaiting_acceptance",
            Self::RecoveryRequired => "recovery_required",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "queued" => Self::Queued,
            "preparing" => Self::Preparing,
            "running" => Self::Running,
            "verifying" => Self::Verifying,
            "repairing" => Self::Repairing,
            "pause_requested" => Self::PauseRequested,
            "paused" => Self::Paused,
            "cancel_requested" => Self::CancelRequested,
            "final_verifying" => Self::FinalVerifying,
            "action_required" => Self::ActionRequired,
            "awaiting_acceptance" => Self::AwaitingAcceptance,
            "recovery_required" => Self::RecoveryRequired,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => return None,
        })
    }

    pub(crate) fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Queued, Self::Preparing)
                | (
                    Self::Preparing,
                    Self::Running | Self::Failed | Self::RecoveryRequired
                )
                | (
                    Self::Running,
                    Self::Verifying
                        | Self::Repairing
                        | Self::PauseRequested
                        | Self::CancelRequested
                        | Self::FinalVerifying
                        | Self::ActionRequired
                )
                | (
                    Self::Verifying,
                    Self::Running
                        | Self::Repairing
                        | Self::PauseRequested
                        | Self::CancelRequested
                        | Self::FinalVerifying
                        | Self::ActionRequired
                        | Self::Failed
                        | Self::RecoveryRequired
                )
                | (
                    Self::Repairing,
                    Self::Verifying
                        | Self::PauseRequested
                        | Self::CancelRequested
                        | Self::ActionRequired
                        | Self::Failed
                        | Self::RecoveryRequired
                )
                | (
                    Self::Running,
                    Self::AwaitingAcceptance | Self::Failed | Self::RecoveryRequired
                )
                | (Self::PauseRequested, Self::Paused | Self::CancelRequested)
                | (Self::Paused, Self::Running | Self::CancelRequested)
                | (Self::CancelRequested, Self::Cancelled)
                | (
                    Self::FinalVerifying,
                    Self::AwaitingAcceptance
                        | Self::ActionRequired
                        | Self::CancelRequested
                        | Self::RecoveryRequired
                )
                | (Self::ActionRequired, Self::Running | Self::CancelRequested)
                | (Self::AwaitingAcceptance, Self::Completed | Self::Failed)
                | (
                    Self::RecoveryRequired,
                    Self::Running | Self::Paused | Self::CancelRequested
                )
                | (Self::Failed, Self::Running)
        )
    }

    pub(crate) fn transition(self, next: Self) -> Result<Self, PlanDomainError> {
        self.can_transition_to(next)
            .then_some(next)
            .ok_or(PlanDomainError::InvalidRunTransition {
                from: self.as_str(),
                to: next.as_str(),
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SubTaskRunStatus {
    Pending,
    Ready,
    Dispatching,
    Running,
    Verifying,
    Succeeded,
    Failed,
    Cancelled,
    Interrupted,
    Blocked,
    Skipped,
}

impl SubTaskRunStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Ready => "ready",
            Self::Dispatching => "dispatching",
            Self::Running => "running",
            Self::Verifying => "verifying",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
            Self::Blocked => "blocked",
            Self::Skipped => "skipped",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "pending" => Self::Pending,
            "ready" => Self::Ready,
            "dispatching" => Self::Dispatching,
            "running" => Self::Running,
            "verifying" => Self::Verifying,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            "interrupted" => Self::Interrupted,
            "blocked" => Self::Blocked,
            "skipped" => Self::Skipped,
            _ => return None,
        })
    }

    pub(crate) fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (
                Self::Pending,
                Self::Ready | Self::Blocked | Self::Skipped | Self::Cancelled
            ) | (Self::Ready, Self::Dispatching | Self::Cancelled)
                | (
                    Self::Dispatching,
                    Self::Running | Self::Failed | Self::Cancelled | Self::Interrupted
                )
                | (
                    Self::Running,
                    Self::Verifying | Self::Failed | Self::Cancelled | Self::Interrupted
                )
                | (
                    Self::Verifying,
                    Self::Succeeded | Self::Failed | Self::Cancelled | Self::Interrupted
                )
                | (Self::Failed | Self::Interrupted, Self::Ready)
        )
    }

    pub(crate) fn transition(self, next: Self) -> Result<Self, PlanDomainError> {
        self.can_transition_to(next)
            .then_some(next)
            .ok_or(PlanDomainError::InvalidTaskTransition {
                from: self.as_str(),
                to: next.as_str(),
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceLimits {
    pub(crate) token_budget: Option<u32>,
    pub(crate) tool_call_limit: Option<u32>,
    pub(crate) timeout_seconds: Option<u64>,
}

impl ResourceLimits {
    pub(crate) fn validate(&self) -> Result<(), PlanDomainError> {
        if self.token_budget == Some(0) {
            return Err(PlanDomainError::InvalidLimit("token budget"));
        }
        if self.tool_call_limit == Some(0) {
            return Err(PlanDomainError::InvalidLimit("tool call limit"));
        }
        if self.timeout_seconds == Some(0) {
            return Err(PlanDomainError::InvalidLimit("timeout"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VerificationCommand {
    pub(crate) id: String,
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) working_directory: Option<String>,
    pub(crate) timeout_seconds: u64,
    pub(crate) required: bool,
}

impl VerificationCommand {
    pub(crate) fn validate(&self) -> Result<(), PlanDomainError> {
        if self.id.trim().is_empty() {
            return Err(PlanDomainError::Required("validation command id"));
        }
        if self.program.trim().is_empty() {
            return Err(PlanDomainError::Required("validation command program"));
        }
        if self.timeout_seconds == 0 {
            return Err(PlanDomainError::InvalidLimit("validation command timeout"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CriterionEvidenceKind {
    Automated,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CriterionEvidenceBinding {
    pub(crate) criterion_index: u16,
    pub(crate) kind: CriterionEvidenceKind,
    pub(crate) command_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PlanDiscoveryStatus {
    #[default]
    NotStarted,
    Complete,
    Degraded,
    Limited,
}

impl PlanDiscoveryStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::Complete => "complete",
            Self::Degraded => "degraded",
            Self::Limited => "limited",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "not_started" => Self::NotStarted,
            "complete" => Self::Complete,
            "degraded" => Self::Degraded,
            "limited" => Self::Limited,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlanDiscoveryMetadata {
    pub(crate) status: PlanDiscoveryStatus,
    pub(crate) limitations: Vec<String>,
}

fn default_max_attempts() -> u16 {
    3
}

fn default_repair_classes() -> Vec<String> {
    vec!["verification_failed".to_string()]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlanExecutionPolicy {
    #[serde(default = "default_max_attempts")]
    pub(crate) max_attempts_per_subtask: u16,
    #[serde(default = "default_repair_classes")]
    pub(crate) repair_eligible_classes: Vec<String>,
    #[serde(default)]
    pub(crate) final_validation_commands: Vec<VerificationCommand>,
}

impl Default for PlanExecutionPolicy {
    fn default() -> Self {
        Self {
            max_attempts_per_subtask: default_max_attempts(),
            repair_eligible_classes: default_repair_classes(),
            final_validation_commands: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SubTaskSpec {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) acceptance_criteria: Vec<String>,
    #[serde(default)]
    pub(crate) criterion_evidence: Vec<CriterionEvidenceBinding>,
    pub(crate) ordinal: u16,
    pub(crate) assigned_role: String,
    pub(crate) limits: ResourceLimits,
    pub(crate) validation_commands: Vec<VerificationCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DependencyEdge {
    pub(crate) predecessor_id: String,
    pub(crate) successor_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlanVersion {
    pub(crate) id: String,
    pub(crate) version_id: String,
    pub(crate) version: u32,
    pub(crate) goal: String,
    pub(crate) project_path: String,
    pub(crate) base_ref: String,
    pub(crate) planner_profile_id: Option<String>,
    #[serde(default)]
    pub(crate) discovery: PlanDiscoveryMetadata,
    #[serde(default)]
    pub(crate) execution_policy: PlanExecutionPolicy,
    pub(crate) subtasks: Vec<SubTaskSpec>,
    pub(crate) dependencies: Vec<DependencyEdge>,
}

pub(crate) fn validate_plan_execution_policy(version: &PlanVersion) -> Result<(), PlanDomainError> {
    if !(1..=5).contains(&version.execution_policy.max_attempts_per_subtask) {
        return Err(PlanDomainError::InvalidExecutionPolicy(
            "maximum attempts must be between one and five".to_string(),
        ));
    }
    if version.execution_policy.repair_eligible_classes.is_empty()
        || version
            .execution_policy
            .repair_eligible_classes
            .iter()
            .any(|value| value.trim().is_empty())
    {
        return Err(PlanDomainError::InvalidExecutionPolicy(
            "at least one non-empty repair class is required".to_string(),
        ));
    }
    validate_commands(
        &version.execution_policy.final_validation_commands,
        "final validation",
    )?;
    if !version
        .execution_policy
        .final_validation_commands
        .iter()
        .any(|command| command.required)
    {
        return Err(PlanDomainError::InvalidExecutionPolicy(
            "at least one required final validation command is required".to_string(),
        ));
    }
    for task in &version.subtasks {
        validate_commands(&task.validation_commands, "SubTask validation")?;
        if !task
            .validation_commands
            .iter()
            .any(|command| command.required)
        {
            return Err(PlanDomainError::InvalidExecutionPolicy(format!(
                "SubTask `{}` requires at least one required validation command",
                task.id
            )));
        }
        if task.criterion_evidence.len() != task.acceptance_criteria.len() {
            return Err(PlanDomainError::InvalidExecutionPolicy(format!(
                "SubTask `{}` must bind every acceptance criterion to evidence",
                task.id
            )));
        }
        let mut indexes = std::collections::BTreeSet::new();
        for binding in &task.criterion_evidence {
            if usize::from(binding.criterion_index) >= task.acceptance_criteria.len()
                || !indexes.insert(binding.criterion_index)
            {
                return Err(PlanDomainError::InvalidExecutionPolicy(format!(
                    "SubTask `{}` has an invalid criterion evidence index",
                    task.id
                )));
            }
            match binding.kind {
                CriterionEvidenceKind::Automated => {
                    let Some(command_id) = binding.command_id.as_deref() else {
                        return Err(PlanDomainError::InvalidExecutionPolicy(format!(
                            "SubTask `{}` automated evidence requires a command id",
                            task.id
                        )));
                    };
                    if !task
                        .validation_commands
                        .iter()
                        .any(|command| command.id == command_id && command.required)
                    {
                        return Err(PlanDomainError::InvalidExecutionPolicy(format!(
                            "SubTask `{}` references unknown required command `{command_id}`",
                            task.id
                        )));
                    }
                }
                CriterionEvidenceKind::Manual if binding.command_id.is_some() => {
                    return Err(PlanDomainError::InvalidExecutionPolicy(format!(
                        "SubTask `{}` manual evidence cannot reference a command",
                        task.id
                    )));
                }
                CriterionEvidenceKind::Manual => {}
            }
        }
    }
    Ok(())
}

fn validate_commands(commands: &[VerificationCommand], label: &str) -> Result<(), PlanDomainError> {
    let mut ids = std::collections::BTreeSet::new();
    for command in commands {
        command.validate()?;
        if !ids.insert(command.id.as_str()) {
            return Err(PlanDomainError::InvalidExecutionPolicy(format!(
                "{label} command id `{}` is duplicated",
                command.id
            )));
        }
    }
    Ok(())
}

pub(crate) type PlanDraft = PlanVersion;

impl PlanVersion {
    pub(crate) fn validate_required_fields(&self) -> Result<(), PlanDomainError> {
        for (value, field) in [
            (self.id.as_str(), "Plan id"),
            (self.version_id.as_str(), "Plan version id"),
            (self.goal.as_str(), "goal"),
            (self.project_path.as_str(), "project path"),
            (self.base_ref.as_str(), "base ref"),
        ] {
            if value.trim().is_empty() {
                return Err(PlanDomainError::Required(field));
            }
        }
        if self.version == 0 {
            return Err(PlanDomainError::InvalidLimit("version"));
        }
        for subtask in &self.subtasks {
            if subtask.id.trim().is_empty()
                || subtask.title.trim().is_empty()
                || subtask.description.trim().is_empty()
                || subtask.assigned_role.trim().is_empty()
            {
                return Err(PlanDomainError::Required("SubTask field"));
            }
            subtask.limits.validate()?;
        }
        Ok(())
    }
}

// Aggregate-shaped contracts live beside the row-oriented MVP repository so later repository
// refactors cannot invent different lifecycle rules while the serial engine remains SQL-driven.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct Plan {
    pub(crate) id: String,
    pub(crate) status: PlanStatus,
    pub(crate) current_version: u32,
}

#[cfg_attr(not(test), allow(dead_code))]
impl Plan {
    pub(crate) fn new(id: String, current_version: u32) -> Result<Self, PlanDomainError> {
        if id.trim().is_empty() {
            return Err(PlanDomainError::Required("Plan id"));
        }
        if current_version == 0 {
            return Err(PlanDomainError::InvalidLimit("version"));
        }
        Ok(Self {
            id,
            status: PlanStatus::Draft,
            current_version,
        })
    }

    pub(crate) fn approve(&mut self) -> Result<(), PlanDomainError> {
        self.status = self.status.approve()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct ApprovedPlanSnapshot {
    version: PlanVersion,
}

#[cfg_attr(not(test), allow(dead_code))]
impl ApprovedPlanSnapshot {
    pub(crate) fn capture(version: &PlanVersion) -> Result<Self, PlanDomainError> {
        version.validate_required_fields()?;
        validate_plan_execution_policy(version)?;
        Ok(Self {
            version: version.clone(),
        })
    }

    pub(crate) fn version(&self) -> &PlanVersion {
        &self.version
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct PlanRun {
    pub(crate) id: String,
    pub(crate) plan_id: String,
    pub(crate) status: PlanRunStatus,
    pub(crate) snapshot: ApprovedPlanSnapshot,
}

#[cfg_attr(not(test), allow(dead_code))]
impl PlanRun {
    pub(crate) fn transition(&mut self, next: PlanRunStatus) -> Result<(), PlanDomainError> {
        self.status = self.status.transition(next)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct SubTaskRun {
    pub(crate) id: String,
    pub(crate) plan_run_id: String,
    pub(crate) subtask_id: String,
    pub(crate) ordinal: u16,
    pub(crate) topological_rank: u16,
    pub(crate) status: SubTaskRunStatus,
}

#[cfg_attr(not(test), allow(dead_code))]
impl SubTaskRun {
    pub(crate) fn transition(&mut self, next: SubTaskRunStatus) -> Result<(), PlanDomainError> {
        self.status = self.status.transition(next)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) enum SubTaskAttemptStatus {
    Dispatching,
    Running,
    Verifying,
    Succeeded,
    Failed,
    Cancelled,
    Interrupted,
}

#[cfg_attr(not(test), allow(dead_code))]
impl SubTaskAttemptStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Dispatching => "dispatching",
            Self::Running => "running",
            Self::Verifying => "verifying",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
        }
    }

    pub(crate) fn transition(self, next: Self) -> Result<Self, PlanDomainError> {
        let valid = matches!(
            (self, next),
            (
                Self::Dispatching,
                Self::Running | Self::Failed | Self::Cancelled | Self::Interrupted
            ) | (
                Self::Running,
                Self::Verifying | Self::Failed | Self::Cancelled | Self::Interrupted
            ) | (
                Self::Verifying,
                Self::Succeeded | Self::Failed | Self::Cancelled | Self::Interrupted
            )
        );
        valid
            .then_some(next)
            .ok_or(PlanDomainError::InvalidAttemptTransition {
                from: self.as_str(),
                to: next.as_str(),
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct SubTaskAttempt {
    pub(crate) id: String,
    pub(crate) subtask_run_id: String,
    pub(crate) sequence: u32,
    pub(crate) status: SubTaskAttemptStatus,
    pub(crate) session_id: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) execution_run_id: Option<String>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl SubTaskAttempt {
    pub(crate) fn transition(&mut self, next: SubTaskAttemptStatus) -> Result<(), PlanDomainError> {
        self.status = self.status.transition(next)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct VerificationEvidence {
    pub(crate) id: String,
    pub(crate) attempt_id: String,
    pub(crate) command_id: String,
    pub(crate) status: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) output_summary: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) enum PlanControlKind {
    Pause,
    Resume,
    Cancel,
    Retry,
    Recover,
    Accept,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) enum PlanControlStatus {
    Pending,
    Applied,
    Rejected,
}

#[cfg_attr(not(test), allow(dead_code))]
impl PlanControlStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Applied => "applied",
            Self::Rejected => "rejected",
        }
    }

    pub(crate) fn resolve(self, next: Self) -> Result<Self, PlanDomainError> {
        matches!(
            (self, next),
            (Self::Pending, Self::Applied | Self::Rejected)
        )
        .then_some(next)
        .ok_or(PlanDomainError::InvalidControlTransition {
            from: self.as_str(),
            to: next.as_str(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct PlanControlRequest {
    pub(crate) id: String,
    pub(crate) plan_run_id: String,
    pub(crate) kind: PlanControlKind,
    pub(crate) status: PlanControlStatus,
}

#[cfg_attr(not(test), allow(dead_code))]
impl PlanControlRequest {
    pub(crate) fn resolve(&mut self, next: PlanControlStatus) -> Result<(), PlanDomainError> {
        self.status = self.status.resolve(next)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_machines_reject_unsafe_transitions() {
        assert!(PlanStatus::Draft.approve().is_ok());
        assert!(PlanStatus::Approved.approve().is_err());
        assert!(PlanRunStatus::Queued.can_transition_to(PlanRunStatus::Preparing));
        assert!(!PlanRunStatus::Queued.can_transition_to(PlanRunStatus::Completed));
        assert!(PlanRunStatus::Queued
            .transition(PlanRunStatus::Preparing)
            .is_ok());
        assert!(PlanRunStatus::Queued
            .transition(PlanRunStatus::Completed)
            .is_err());
        assert!(SubTaskRunStatus::Verifying.can_transition_to(SubTaskRunStatus::Succeeded));
        assert!(!SubTaskRunStatus::Pending.can_transition_to(SubTaskRunStatus::Succeeded));
        assert!(SubTaskRunStatus::Pending
            .transition(SubTaskRunStatus::Succeeded)
            .is_err());
    }

    #[test]
    fn limits_must_be_positive_when_present() {
        let limits = ResourceLimits {
            token_budget: Some(0),
            tool_call_limit: Some(1),
            timeout_seconds: Some(1),
        };
        assert_eq!(
            limits.validate(),
            Err(PlanDomainError::InvalidLimit("token budget"))
        );
    }

    #[test]
    fn final_verification_and_action_required_transitions_are_explicit() {
        assert!(PlanRunStatus::Running
            .transition(PlanRunStatus::FinalVerifying)
            .is_ok());
        assert!(PlanRunStatus::FinalVerifying
            .transition(PlanRunStatus::AwaitingAcceptance)
            .is_ok());
        assert!(PlanRunStatus::FinalVerifying
            .transition(PlanRunStatus::ActionRequired)
            .is_ok());
        assert!(PlanRunStatus::ActionRequired
            .transition(PlanRunStatus::Running)
            .is_ok());
        assert!(PlanRunStatus::ActionRequired
            .transition(PlanRunStatus::Completed)
            .is_err());
    }

    #[test]
    fn verification_and_repair_are_first_class_run_transitions() {
        assert_eq!(
            PlanRunStatus::parse("verifying"),
            Some(PlanRunStatus::Verifying)
        );
        assert_eq!(
            PlanRunStatus::parse("repairing"),
            Some(PlanRunStatus::Repairing)
        );
        assert!(PlanRunStatus::Running
            .transition(PlanRunStatus::Verifying)
            .is_ok());
        assert!(PlanRunStatus::Verifying
            .transition(PlanRunStatus::Repairing)
            .is_ok());
        assert!(PlanRunStatus::Repairing
            .transition(PlanRunStatus::Verifying)
            .is_ok());
        assert!(PlanRunStatus::Repairing
            .transition(PlanRunStatus::Completed)
            .is_err());
    }

    fn version() -> PlanVersion {
        let final_command = VerificationCommand {
            id: "final-test".into(),
            program: "cargo".into(),
            args: vec!["test".into()],
            working_directory: None,
            timeout_seconds: 300,
            required: true,
        };
        PlanVersion {
            id: "plan-1".into(),
            version_id: "plan-version-1".into(),
            version: 1,
            goal: "Ship safely".into(),
            project_path: "C:\\code\\app".into(),
            base_ref: "main".into(),
            planner_profile_id: Some("profile-1".into()),
            discovery: PlanDiscoveryMetadata {
                status: PlanDiscoveryStatus::Complete,
                limitations: Vec::new(),
            },
            execution_policy: PlanExecutionPolicy {
                final_validation_commands: vec![final_command],
                ..PlanExecutionPolicy::default()
            },
            subtasks: vec![
                SubTaskSpec {
                    id: "task-a".into(),
                    title: "Analyze".into(),
                    description: "Analyze the change".into(),
                    acceptance_criteria: vec!["Analysis is reviewed".into()],
                    criterion_evidence: vec![CriterionEvidenceBinding {
                        criterion_index: 0,
                        kind: CriterionEvidenceKind::Manual,
                        command_id: None,
                    }],
                    ordinal: 0,
                    assigned_role: "worker".into(),
                    limits: ResourceLimits {
                        token_budget: Some(1_000),
                        tool_call_limit: Some(10),
                        timeout_seconds: Some(60),
                    },
                    validation_commands: vec![VerificationCommand {
                        id: "review-analysis".into(),
                        program: "cargo".into(),
                        args: vec!["check".into()],
                        working_directory: None,
                        timeout_seconds: 300,
                        required: true,
                    }],
                },
                SubTaskSpec {
                    id: "task-b".into(),
                    title: "Build".into(),
                    description: "Build the change".into(),
                    acceptance_criteria: vec!["Tests pass".into()],
                    criterion_evidence: vec![CriterionEvidenceBinding {
                        criterion_index: 0,
                        kind: CriterionEvidenceKind::Automated,
                        command_id: Some("build-tests".into()),
                    }],
                    ordinal: 1,
                    assigned_role: "worker".into(),
                    limits: ResourceLimits {
                        token_budget: None,
                        tool_call_limit: None,
                        timeout_seconds: None,
                    },
                    validation_commands: vec![VerificationCommand {
                        id: "build-tests".into(),
                        program: "cargo".into(),
                        args: vec!["test".into()],
                        working_directory: None,
                        timeout_seconds: 300,
                        required: true,
                    }],
                },
            ],
            dependencies: vec![DependencyEdge {
                predecessor_id: "task-a".into(),
                successor_id: "task-b".into(),
            }],
        }
    }

    #[test]
    fn execution_policy_accepts_resolved_automated_and_manual_evidence() {
        validate_plan_execution_policy(&version()).expect("valid policy");
    }

    #[test]
    fn execution_policy_rejects_invalid_attempt_and_repair_bounds() {
        let mut invalid_attempts = version();
        invalid_attempts.execution_policy.max_attempts_per_subtask = 0;
        assert!(validate_plan_execution_policy(&invalid_attempts).is_err());

        let mut missing_classes = version();
        missing_classes
            .execution_policy
            .repair_eligible_classes
            .clear();
        assert!(validate_plan_execution_policy(&missing_classes).is_err());
    }

    #[test]
    fn execution_policy_requires_guarded_task_and_final_commands() {
        let mut missing_final = version();
        missing_final
            .execution_policy
            .final_validation_commands
            .clear();
        assert!(validate_plan_execution_policy(&missing_final).is_err());

        let mut optional_task = version();
        optional_task.subtasks[0].validation_commands[0].required = false;
        assert!(validate_plan_execution_policy(&optional_task).is_err());
    }

    #[test]
    fn execution_policy_rejects_unresolved_or_mismatched_evidence() {
        let mut unknown_command = version();
        unknown_command.subtasks[1].criterion_evidence[0].command_id = Some("unknown".into());
        assert!(validate_plan_execution_policy(&unknown_command).is_err());

        let mut missing_binding = version();
        missing_binding.subtasks[0].criterion_evidence.clear();
        assert!(validate_plan_execution_policy(&missing_binding).is_err());

        let mut manual_with_command = version();
        manual_with_command.subtasks[0].criterion_evidence[0].command_id = Some("review".into());
        assert!(validate_plan_execution_policy(&manual_with_command).is_err());
    }

    #[test]
    fn plan_identity_version_order_and_approved_snapshot_are_isolated() {
        let mut plan = Plan::new("plan-1".into(), 1).expect("plan");
        let mut editable = version();
        let snapshot = ApprovedPlanSnapshot::capture(&editable).expect("snapshot");
        editable.goal = "A later edit".into();
        editable.subtasks.swap(0, 1);

        assert_eq!(plan.id, "plan-1");
        assert_eq!(plan.current_version, 1);
        assert_eq!(snapshot.version().goal, "Ship safely");
        assert_eq!(
            snapshot
                .version()
                .subtasks
                .iter()
                .map(|task| (task.id.as_str(), task.ordinal))
                .collect::<Vec<_>>(),
            vec![("task-a", 0), ("task-b", 1)]
        );
        plan.approve().expect("approve");
        assert_eq!(plan.status, PlanStatus::Approved);
        assert!(plan.approve().is_err());
    }

    #[test]
    fn run_task_attempt_and_control_transitions_are_explicit() {
        let snapshot = ApprovedPlanSnapshot::capture(&version()).expect("snapshot");
        let mut run = PlanRun {
            id: "run-1".into(),
            plan_id: "plan-1".into(),
            status: PlanRunStatus::Queued,
            snapshot,
        };
        run.transition(PlanRunStatus::Preparing).expect("prepare");
        assert_eq!(run.id, "run-1");
        assert_eq!(run.plan_id, "plan-1");
        assert_eq!(run.snapshot.version().version_id, "plan-version-1");
        assert!(run.transition(PlanRunStatus::Completed).is_err());

        let mut task_run = SubTaskRun {
            id: "subtask-run-1".into(),
            plan_run_id: run.id.clone(),
            subtask_id: "task-a".into(),
            ordinal: 0,
            topological_rank: 0,
            status: SubTaskRunStatus::Pending,
        };
        task_run.transition(SubTaskRunStatus::Ready).expect("ready");
        assert_eq!(task_run.plan_run_id, "run-1");
        assert_eq!(task_run.subtask_id, "task-a");
        assert_eq!((task_run.ordinal, task_run.topological_rank), (0, 0));
        assert!(task_run.transition(SubTaskRunStatus::Succeeded).is_err());

        let mut attempt = SubTaskAttempt {
            id: "attempt-1".into(),
            subtask_run_id: task_run.id.clone(),
            sequence: 1,
            status: SubTaskAttemptStatus::Dispatching,
            session_id: Some("session-1".into()),
            operation_id: Some("operation-1".into()),
            execution_run_id: Some("execution-1".into()),
        };
        attempt
            .transition(SubTaskAttemptStatus::Running)
            .expect("running");
        attempt
            .transition(SubTaskAttemptStatus::Verifying)
            .expect("verifying");
        attempt
            .transition(SubTaskAttemptStatus::Succeeded)
            .expect("succeeded");
        assert_eq!(attempt.sequence, 1);
        assert_eq!(attempt.subtask_run_id, "subtask-run-1");
        assert_eq!(attempt.session_id.as_deref(), Some("session-1"));
        assert_eq!(attempt.operation_id.as_deref(), Some("operation-1"));
        assert_eq!(attempt.execution_run_id.as_deref(), Some("execution-1"));
        assert!(attempt.transition(SubTaskAttemptStatus::Failed).is_err());

        let mut control = PlanControlRequest {
            id: "control-1".into(),
            plan_run_id: run.id,
            kind: PlanControlKind::Pause,
            status: PlanControlStatus::Pending,
        };
        control
            .resolve(PlanControlStatus::Applied)
            .expect("applied");
        assert_eq!(control.id, "control-1");
        assert_eq!(control.plan_run_id, "run-1");
        assert_eq!(control.kind, PlanControlKind::Pause);
        assert!(control.resolve(PlanControlStatus::Rejected).is_err());

        let evidence = VerificationEvidence {
            id: "evidence-1".into(),
            attempt_id: attempt.id,
            command_id: "tests".into(),
            status: "passed".into(),
            exit_code: Some(0),
            duration_ms: Some(25),
            output_summary: Some("bounded summary".into()),
        };
        assert_eq!(evidence.attempt_id, "attempt-1");
        assert_eq!(evidence.id, "evidence-1");
        assert_eq!(evidence.command_id, "tests");
        assert_eq!(evidence.status, "passed");
        assert_eq!(evidence.exit_code, Some(0));
        assert_eq!(evidence.duration_ms, Some(25));
        assert_eq!(evidence.output_summary.as_deref(), Some("bounded summary"));

        for kind in [
            PlanControlKind::Resume,
            PlanControlKind::Cancel,
            PlanControlKind::Retry,
            PlanControlKind::Recover,
            PlanControlKind::Accept,
        ] {
            assert_ne!(kind, PlanControlKind::Pause);
        }
        for terminal in [
            SubTaskAttemptStatus::Failed,
            SubTaskAttemptStatus::Cancelled,
            SubTaskAttemptStatus::Interrupted,
        ] {
            assert_ne!(terminal, SubTaskAttemptStatus::Succeeded);
        }
    }
}
