use super::{DelegationMode, DelegationTarget};
use serde::{Deserialize, Serialize};

pub(crate) const MAX_DELEGATION_ATTEMPTS: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DelegationRequestSnapshot {
    pub(crate) task: String,
    pub(crate) context_summary: Option<String>,
    pub(crate) artifact_hashes: Vec<String>,
    pub(crate) repository_identity: String,
    pub(crate) base_commit: String,
    pub(crate) instruction_hashes: Vec<String>,
    pub(crate) provider_configuration_hash: String,
    pub(crate) limits_hash: String,
    pub(crate) adapter_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Interrupted,
}

impl DelegationStatus {
    const fn terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::Interrupted
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationAttemptStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationAttempt {
    pub(crate) id: String,
    pub(crate) number: u8,
    pub(crate) target: DelegationTarget,
    pub(crate) mode: DelegationMode,
    pub(crate) status: DelegationAttemptStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationResult {
    pub(crate) attempt_id: String,
    pub(crate) report_artifact_id: Option<String>,
    pub(crate) change_set_artifact_id: Option<String>,
    pub(crate) error_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationError {
    InvalidSnapshot,
    AlreadyTerminal,
    AttemptAlreadyActive,
    AttemptLimitReached,
    AttemptNotActive,
    InvalidTerminalResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Delegation {
    pub(crate) id: String,
    pub(crate) session_id: String,
    pub(crate) snapshot: DelegationRequestSnapshot,
    pub(crate) status: DelegationStatus,
    pub(crate) attempts: Vec<DelegationAttempt>,
    pub(crate) result: Option<DelegationResult>,
}

impl Delegation {
    pub(crate) fn queued(
        id: String,
        session_id: String,
        snapshot: DelegationRequestSnapshot,
    ) -> Result<Self, DelegationError> {
        if id.trim().is_empty()
            || session_id.trim().is_empty()
            || snapshot.task.trim().is_empty()
            || snapshot.repository_identity.trim().is_empty()
            || snapshot.base_commit.trim().is_empty()
            || snapshot.adapter_version.trim().is_empty()
        {
            return Err(DelegationError::InvalidSnapshot);
        }
        Ok(Self {
            id,
            session_id,
            snapshot,
            status: DelegationStatus::Queued,
            attempts: Vec::new(),
            result: None,
        })
    }

    pub(crate) fn queue_attempt(
        &mut self,
        id: String,
        target: DelegationTarget,
        mode: DelegationMode,
    ) -> Result<&DelegationAttempt, DelegationError> {
        if self.status.terminal() {
            return Err(DelegationError::AlreadyTerminal);
        }
        if self.attempts.iter().any(|attempt| {
            matches!(
                attempt.status,
                DelegationAttemptStatus::Queued | DelegationAttemptStatus::Running
            )
        }) {
            return Err(DelegationError::AttemptAlreadyActive);
        }
        if self.attempts.len() >= MAX_DELEGATION_ATTEMPTS {
            return Err(DelegationError::AttemptLimitReached);
        }
        let number = u8::try_from(self.attempts.len() + 1)
            .map_err(|_| DelegationError::AttemptLimitReached)?;
        self.attempts.push(DelegationAttempt {
            id,
            number,
            target,
            mode,
            status: DelegationAttemptStatus::Queued,
        });
        self.status = DelegationStatus::Queued;
        self.attempts
            .last()
            .ok_or(DelegationError::AttemptLimitReached)
    }

    pub(crate) fn start_attempt(&mut self, id: &str) -> Result<(), DelegationError> {
        let attempt = self.active_attempt_mut(id, DelegationAttemptStatus::Queued)?;
        attempt.status = DelegationAttemptStatus::Running;
        self.status = DelegationStatus::Running;
        Ok(())
    }

    pub(crate) fn fail_attempt(&mut self, id: &str) -> Result<(), DelegationError> {
        let attempt = self.active_attempt_mut(id, DelegationAttemptStatus::Running)?;
        attempt.status = DelegationAttemptStatus::Failed;
        self.status = DelegationStatus::Queued;
        Ok(())
    }

    pub(crate) fn complete(
        &mut self,
        status: DelegationStatus,
        result: DelegationResult,
    ) -> Result<(), DelegationError> {
        if self.status.terminal() {
            return Err(DelegationError::AlreadyTerminal);
        }
        if !status.terminal() || result.attempt_id.trim().is_empty() {
            return Err(DelegationError::InvalidTerminalResult);
        }
        let attempt = self
            .attempts
            .iter_mut()
            .find(|attempt| {
                attempt.id == result.attempt_id
                    && attempt.status == DelegationAttemptStatus::Running
            })
            .ok_or(DelegationError::AttemptNotActive)?;
        attempt.status = match status {
            DelegationStatus::Succeeded => DelegationAttemptStatus::Succeeded,
            DelegationStatus::Failed => DelegationAttemptStatus::Failed,
            DelegationStatus::Cancelled => DelegationAttemptStatus::Cancelled,
            DelegationStatus::Interrupted => DelegationAttemptStatus::Interrupted,
            DelegationStatus::Queued | DelegationStatus::Running => {
                return Err(DelegationError::InvalidTerminalResult)
            }
        };
        self.status = status;
        self.result = Some(result);
        Ok(())
    }

    pub(crate) fn interrupt_after_restart(&mut self) -> bool {
        if self.status.terminal() {
            return false;
        }
        if let Some(attempt) = self.attempts.iter_mut().find(|attempt| {
            matches!(
                attempt.status,
                DelegationAttemptStatus::Queued | DelegationAttemptStatus::Running
            )
        }) {
            attempt.status = DelegationAttemptStatus::Interrupted;
        }
        self.status = DelegationStatus::Interrupted;
        self.result = None;
        true
    }

    fn active_attempt_mut(
        &mut self,
        id: &str,
        expected: DelegationAttemptStatus,
    ) -> Result<&mut DelegationAttempt, DelegationError> {
        self.attempts
            .iter_mut()
            .find(|attempt| attempt.id == id && attempt.status == expected)
            .ok_or(DelegationError::AttemptNotActive)
    }
}

#[cfg(test)]
#[path = "state_machine_tests.rs"]
mod tests;
