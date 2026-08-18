use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_ID_LENGTH: usize = 128;
const MAX_REASON_LENGTH: usize = 64;
const MAX_LINKS: usize = 8;
const MAX_RUNNER_LABEL_LENGTH: usize = 160;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RunState {
    Created,
    Preparing,
    Running,
    WaitingApproval,
    WaitingUser,
    Paused,
    Retrying,
    Blocked,
    Stuck,
    Verifying,
    Completed,
    Failed,
    Cancelled,
}

impl RunState {
    pub(crate) fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RunTrigger {
    Prepare,
    Start,
    RequestApproval,
    ApprovalGranted,
    ApprovalRejected,
    AskUser,
    UserAnswered,
    Pause,
    Resume,
    Retry,
    RetryReady,
    Block,
    MarkStuck,
    Verify,
    Continue,
    Complete,
    Fail,
    CancelUser,
    CancelParent,
    CancelTimeout,
    CancelShutdown,
    InterruptRestart,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RunRecoveryPolicy {
    NotRecoverable,
    OwnerReconciles,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RunRunnerKind {
    Local,
    Ssh,
}

impl RunRunnerKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Ssh => "ssh",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RunRunnerRecovery {
    None,
    InspectOnly,
    Reattach,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunRunner {
    pub(crate) kind: RunRunnerKind,
    pub(crate) target_id: String,
    pub(crate) target_revision: Option<i64>,
    pub(crate) label: String,
    pub(crate) host_label: Option<String>,
    pub(crate) recovery: RunRunnerRecovery,
    pub(crate) capability_witness: String,
    pub(crate) authority_witness: String,
    pub(crate) recovery_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunOwner {
    pub(crate) owner_type: String,
    pub(crate) owner_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunLink {
    pub(crate) link_type: String,
    pub(crate) link_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunEvent {
    pub(crate) sequence: u64,
    pub(crate) state: RunState,
    pub(crate) trigger: RunTrigger,
    pub(crate) timestamp: String,
    pub(crate) reason_code: Option<String>,
    pub(crate) witness: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunTransition {
    pub(crate) trigger: RunTrigger,
    pub(crate) timestamp: String,
    pub(crate) reason_code: Option<String>,
    pub(crate) witness: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentRun {
    pub(crate) id: String,
    pub(crate) owner: RunOwner,
    pub(crate) links: Vec<RunLink>,
    pub(crate) parent_run_id: Option<String>,
    pub(crate) state: RunState,
    pub(crate) recovery_policy: RunRecoveryPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) runner: Option<RunRunner>,
    pub(crate) retry_count: u32,
    pub(crate) max_retries: u32,
    pub(crate) reason_code: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) version: u64,
    pub(crate) last_witness: String,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub(crate) enum RunDomainError {
    #[error("run field is invalid: {0}")]
    InvalidField(&'static str),
    #[error("run transition from {from:?} using {trigger:?} is not allowed")]
    InvalidTransition { from: RunState, trigger: RunTrigger },
    #[error("run retry policy is exhausted")]
    RetryExhausted,
    #[error("run terminal outcome conflicts with existing outcome")]
    TerminalConflict,
}

pub(crate) struct RunCreation {
    pub(crate) id: String,
    pub(crate) owner: RunOwner,
    pub(crate) links: Vec<RunLink>,
    pub(crate) parent_run_id: Option<String>,
    pub(crate) recovery_policy: RunRecoveryPolicy,
    pub(crate) runner: Option<RunRunner>,
    pub(crate) max_retries: u32,
    pub(crate) timestamp: String,
    pub(crate) witness: String,
}

impl AgentRun {
    pub(crate) fn create(input: RunCreation) -> Result<(Self, RunEvent), RunDomainError> {
        let RunCreation {
            id,
            owner,
            links,
            parent_run_id,
            recovery_policy,
            runner,
            max_retries,
            timestamp,
            witness,
        } = input;
        validate_run_id(&id, "id")?;
        validate_safe_token(&owner.owner_type, "owner_type", MAX_ID_LENGTH)?;
        validate_safe_token(&owner.owner_id, "owner_id", MAX_ID_LENGTH)?;
        if links.len() > MAX_LINKS {
            return Err(RunDomainError::InvalidField("links"));
        }
        for link in &links {
            validate_safe_token(&link.link_type, "link_type", MAX_ID_LENGTH)?;
            validate_safe_token(&link.link_id, "link_id", MAX_ID_LENGTH)?;
        }
        if let Some(parent) = &parent_run_id {
            validate_run_id(parent, "parent_run_id")?;
            if parent == &id {
                return Err(RunDomainError::InvalidField("parent_run_id"));
            }
        }
        validate_timestamp(&timestamp)?;
        validate_safe_token(&witness, "witness", MAX_ID_LENGTH)?;
        if let Some(runner) = &runner {
            validate_runner(runner)?;
        }
        let run = Self {
            id,
            owner,
            links,
            parent_run_id,
            state: RunState::Created,
            recovery_policy,
            runner,
            retry_count: 0,
            max_retries,
            reason_code: None,
            created_at: timestamp.clone(),
            updated_at: timestamp.clone(),
            version: 1,
            last_witness: witness.clone(),
        };
        Ok((
            run,
            RunEvent {
                sequence: 1,
                state: RunState::Created,
                trigger: RunTrigger::Prepare,
                timestamp,
                reason_code: None,
                witness,
            },
        ))
    }

    pub(crate) fn transition(
        &mut self,
        input: RunTransition,
    ) -> Result<Option<RunEvent>, RunDomainError> {
        validate_timestamp(&input.timestamp)?;
        validate_safe_token(&input.witness, "witness", MAX_ID_LENGTH)?;
        if let Some(reason) = &input.reason_code {
            validate_safe_token(reason, "reason_code", MAX_REASON_LENGTH)?;
        }
        if self.state.is_terminal() {
            return if self.last_witness == input.witness
                && terminal_trigger_matches(self.state, input.trigger)
            {
                Ok(None)
            } else {
                Err(RunDomainError::TerminalConflict)
            };
        }
        let next =
            next_state(self.state, input.trigger).ok_or(RunDomainError::InvalidTransition {
                from: self.state,
                trigger: input.trigger,
            })?;
        if input.trigger == RunTrigger::Retry {
            if self.retry_count >= self.max_retries {
                return Err(RunDomainError::RetryExhausted);
            }
            self.retry_count += 1;
        }
        self.state = next;
        self.reason_code = input.reason_code.clone();
        self.updated_at = input.timestamp.clone();
        self.version += 1;
        self.last_witness.clone_from(&input.witness);
        Ok(Some(RunEvent {
            sequence: self.version,
            state: next,
            trigger: input.trigger,
            timestamp: input.timestamp,
            reason_code: input.reason_code,
            witness: input.witness,
        }))
    }
}

fn validate_runner(runner: &RunRunner) -> Result<(), RunDomainError> {
    validate_safe_token(&runner.target_id, "runner_target_id", MAX_ID_LENGTH)?;
    validate_display(&runner.label, "runner_label")?;
    if let Some(host) = &runner.host_label {
        validate_display(host, "runner_host_label")?;
    }
    validate_safe_token(
        &runner.capability_witness,
        "runner_capability_witness",
        MAX_ID_LENGTH,
    )?;
    validate_safe_token(
        &runner.authority_witness,
        "runner_authority_witness",
        MAX_ID_LENGTH,
    )?;
    if let Some(reference) = &runner.recovery_reference {
        validate_safe_token(reference, "runner_recovery_reference", MAX_ID_LENGTH)?;
    }
    match runner.kind {
        RunRunnerKind::Local if runner.target_revision.is_some() => {
            Err(RunDomainError::InvalidField("runner_target_revision"))
        }
        RunRunnerKind::Ssh if !runner.target_revision.is_some_and(|revision| revision > 0) => {
            Err(RunDomainError::InvalidField("runner_target_revision"))
        }
        _ => Ok(()),
    }
}

fn next_state(from: RunState, trigger: RunTrigger) -> Option<RunState> {
    use RunState as S;
    use RunTrigger as T;
    match (from, trigger) {
        (S::Created, T::Prepare) => Some(S::Preparing),
        (S::Preparing, T::Start) | (S::Retrying, T::RetryReady) => Some(S::Running),
        (S::Running, T::RequestApproval) => Some(S::WaitingApproval),
        (S::WaitingApproval, T::ApprovalGranted) | (S::WaitingUser, T::UserAnswered) => {
            Some(S::Running)
        }
        (S::Running, T::AskUser) => Some(S::WaitingUser),
        (S::Running | S::Verifying, T::Retry) => Some(S::Retrying),
        (S::Running, T::Verify) => Some(S::Verifying),
        (S::Verifying, T::Continue) => Some(S::Running),
        (S::Running | S::Verifying, T::Complete) => Some(S::Completed),
        (S::Running | S::Preparing | S::Verifying | S::Retrying, T::Pause) => Some(S::Paused),
        (S::Paused | S::Blocked | S::Stuck, T::Resume) => Some(S::Running),
        (_, T::Block) => Some(S::Blocked),
        (S::Running | S::Blocked | S::Retrying, T::MarkStuck) => Some(S::Stuck),
        (_, T::Fail | T::ApprovalRejected | T::InterruptRestart) => Some(S::Failed),
        (_, T::CancelUser | T::CancelParent | T::CancelTimeout | T::CancelShutdown) => {
            Some(S::Cancelled)
        }
        _ => None,
    }
}

fn terminal_trigger_matches(state: RunState, trigger: RunTrigger) -> bool {
    match state {
        RunState::Completed => trigger == RunTrigger::Complete,
        RunState::Failed => matches!(
            trigger,
            RunTrigger::Fail | RunTrigger::ApprovalRejected | RunTrigger::InterruptRestart
        ),
        RunState::Cancelled => matches!(
            trigger,
            RunTrigger::CancelUser
                | RunTrigger::CancelParent
                | RunTrigger::CancelTimeout
                | RunTrigger::CancelShutdown
        ),
        _ => false,
    }
}

fn validate_run_id(value: &str, field: &'static str) -> Result<(), RunDomainError> {
    validate_required(value, field, MAX_ID_LENGTH)?;
    uuid::Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| RunDomainError::InvalidField(field))
}

fn validate_required(value: &str, field: &'static str, max: usize) -> Result<(), RunDomainError> {
    if value.trim().is_empty() || value.chars().count() > max || value.chars().any(char::is_control)
    {
        Err(RunDomainError::InvalidField(field))
    } else {
        Ok(())
    }
}

fn validate_safe_token(value: &str, field: &'static str, max: usize) -> Result<(), RunDomainError> {
    validate_required(value, field, max)?;
    if value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        Ok(())
    } else {
        Err(RunDomainError::InvalidField(field))
    }
}

fn validate_timestamp(value: &str) -> Result<(), RunDomainError> {
    validate_required(value, "timestamp", MAX_ID_LENGTH)?;
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|_| ())
        .map_err(|_| RunDomainError::InvalidField("timestamp"))
}

fn validate_display(value: &str, field: &'static str) -> Result<(), RunDomainError> {
    if value.trim().is_empty()
        || value.chars().count() > MAX_RUNNER_LABEL_LENGTH
        || value.chars().any(char::is_control)
    {
        Err(RunDomainError::InvalidField(field))
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
