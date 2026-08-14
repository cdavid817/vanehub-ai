use super::AgentRuntimeDomainError;

pub(crate) const MAX_UTILITY_TASK_CHARS: usize = 4_000;
pub(crate) const MAX_UTILITY_INSTRUCTION_CHARS: usize = 12_000;
pub(crate) const MAX_UTILITY_RESULT_CHARS: usize = 4_000;
pub(crate) const MAX_UTILITY_DURATION_MS: u64 = 300_000;
pub(crate) const MAX_UTILITY_TOOL_CALLS: u32 = 32;
pub(crate) const MAX_UTILITY_APPROVALS: u32 = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UtilityDelegationLimits {
    pub(crate) duration_ms: u64,
    pub(crate) tool_calls: u32,
    pub(crate) approvals: u32,
    pub(crate) result_chars: usize,
}

impl UtilityDelegationLimits {
    pub(crate) fn bounded(
        duration_ms: u64,
        tool_calls: u32,
        approvals: u32,
        result_chars: usize,
    ) -> Result<Self, AgentRuntimeDomainError> {
        if duration_ms == 0 || duration_ms > MAX_UTILITY_DURATION_MS {
            return Err(AgentRuntimeDomainError::InvalidUtilityDelegationLimit(
                "duration",
            ));
        }
        if tool_calls > MAX_UTILITY_TOOL_CALLS {
            return Err(AgentRuntimeDomainError::InvalidUtilityDelegationLimit(
                "tool calls",
            ));
        }
        if approvals > MAX_UTILITY_APPROVALS {
            return Err(AgentRuntimeDomainError::InvalidUtilityDelegationLimit(
                "approvals",
            ));
        }
        if result_chars == 0 || result_chars > MAX_UTILITY_RESULT_CHARS {
            return Err(AgentRuntimeDomainError::InvalidUtilityDelegationLimit(
                "result characters",
            ));
        }
        Ok(Self {
            duration_ms,
            tool_calls,
            approvals,
            result_chars,
        })
    }
}

impl Default for UtilityDelegationLimits {
    fn default() -> Self {
        Self {
            duration_ms: 120_000,
            tool_calls: 16,
            approvals: 4,
            result_chars: 2_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UtilityDelegationRequest {
    pub(crate) agent_id: String,
    pub(crate) skill_id: String,
    pub(crate) task: String,
    pub(crate) parent_run_id: String,
    pub(crate) parent_span_id: String,
    pub(crate) session_id: String,
    pub(crate) message_id: String,
    pub(crate) canonical_workspace: Option<String>,
    pub(crate) depth: u8,
    pub(crate) limits: UtilityDelegationLimits,
}

impl UtilityDelegationRequest {
    pub(crate) fn validate(&self) -> Result<(), AgentRuntimeDomainError> {
        validate_identity(&self.agent_id, "Agent id")?;
        validate_identity(&self.skill_id, "Skill id")?;
        validate_identity(&self.parent_run_id, "Parent run id")?;
        validate_identity(&self.parent_span_id, "Parent span id")?;
        validate_identity(&self.session_id, "Session id")?;
        validate_identity(&self.message_id, "Message id")?;
        if self.task.trim().is_empty()
            || self.task.chars().count() > MAX_UTILITY_TASK_CHARS
            || self.task.chars().any(|value| value == '\0')
        {
            return Err(AgentRuntimeDomainError::InvalidUtilityDelegationValue(
                "task",
            ));
        }
        if self.depth != 0 {
            return Err(AgentRuntimeDomainError::NestedUtilityDelegation);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UtilityDelegationSnapshot {
    pub(crate) skill_id: String,
    pub(crate) revision: String,
    pub(crate) instructions: String,
    pub(crate) canonical_workspace: Option<String>,
}

impl UtilityDelegationSnapshot {
    pub(crate) fn validate(&self) -> Result<(), AgentRuntimeDomainError> {
        validate_identity(&self.skill_id, "Skill id")?;
        validate_identity(&self.revision, "Skill revision")?;
        if self.instructions.trim().is_empty()
            || self.instructions.chars().count() > MAX_UTILITY_INSTRUCTION_CHARS
        {
            return Err(AgentRuntimeDomainError::InvalidUtilityDelegationValue(
                "instructions",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UtilityDelegationTerminal {
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    Limited,
    #[allow(dead_code)]
    Refused,
}

impl UtilityDelegationTerminal {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed-out",
            Self::Limited => "limited",
            Self::Refused => "refused",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UtilityDelegationState {
    Admitted,
    Running,
    Terminal(UtilityDelegationTerminal),
}

impl UtilityDelegationState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Admitted => "admitted",
            Self::Running => "running",
            Self::Terminal(value) => value.as_str(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct UtilityDelegationCounts {
    pub(crate) tool_calls: u32,
    pub(crate) approvals: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UtilityDelegationResult {
    pub(crate) delegation_id: String,
    pub(crate) attempt_id: String,
    pub(crate) skill_id: String,
    pub(crate) revision: String,
    pub(crate) terminal: UtilityDelegationTerminal,
    pub(crate) summary: Option<String>,
    pub(crate) duration_ms: u64,
    pub(crate) counts: UtilityDelegationCounts,
    pub(crate) limit_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UtilityDelegationAttempt {
    pub(crate) delegation_id: String,
    pub(crate) attempt_id: String,
    pub(crate) snapshot: UtilityDelegationSnapshot,
    state: UtilityDelegationState,
}

impl UtilityDelegationAttempt {
    pub(crate) fn admit(
        delegation_id: String,
        attempt_id: String,
        request: &UtilityDelegationRequest,
        snapshot: UtilityDelegationSnapshot,
        current_revision: &str,
    ) -> Result<Self, AgentRuntimeDomainError> {
        request.validate()?;
        snapshot.validate()?;
        validate_identity(&delegation_id, "Delegation id")?;
        validate_identity(&attempt_id, "Attempt id")?;
        if snapshot.revision != current_revision {
            return Err(AgentRuntimeDomainError::StaleUtilityRevision);
        }
        Ok(Self {
            delegation_id,
            attempt_id,
            snapshot,
            state: UtilityDelegationState::Admitted,
        })
    }

    pub(crate) fn start(&mut self) -> Result<bool, AgentRuntimeDomainError> {
        match self.state {
            UtilityDelegationState::Admitted => {
                self.state = UtilityDelegationState::Running;
                Ok(true)
            }
            UtilityDelegationState::Running => Ok(false),
            state => Err(invalid_transition(state, "running")),
        }
    }

    pub(crate) fn finish(
        &mut self,
        terminal: UtilityDelegationTerminal,
    ) -> Result<bool, AgentRuntimeDomainError> {
        match self.state {
            UtilityDelegationState::Running => {
                self.state = UtilityDelegationState::Terminal(terminal);
                Ok(true)
            }
            UtilityDelegationState::Terminal(_) => Ok(false),
            state => Err(invalid_transition(state, terminal.as_str())),
        }
    }

    #[cfg(test)]
    pub(crate) fn state(&self) -> UtilityDelegationState {
        self.state
    }
}

fn validate_identity(value: &str, name: &'static str) -> Result<(), AgentRuntimeDomainError> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        Err(AgentRuntimeDomainError::InvalidUtilityDelegationValue(name))
    } else {
        Ok(())
    }
}

fn invalid_transition(from: UtilityDelegationState, to: &str) -> AgentRuntimeDomainError {
    AgentRuntimeDomainError::InvalidUtilityDelegationTransition {
        from: from.as_str().to_string(),
        to: to.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> UtilityDelegationRequest {
        UtilityDelegationRequest {
            agent_id: "onepiece".to_string(),
            skill_id: "review-code".to_string(),
            task: "Review the current change.".to_string(),
            parent_run_id: "run-1".to_string(),
            parent_span_id: "1111111111111111".to_string(),
            session_id: "session-1".to_string(),
            message_id: "message-1".to_string(),
            canonical_workspace: Some("workspace-1".to_string()),
            depth: 0,
            limits: UtilityDelegationLimits::default(),
        }
    }

    fn snapshot() -> UtilityDelegationSnapshot {
        UtilityDelegationSnapshot {
            skill_id: "review-code".to_string(),
            revision: "sha256:abc".to_string(),
            instructions: "Review code without modifying it.".to_string(),
            canonical_workspace: Some("workspace-1".to_string()),
        }
    }

    #[test]
    fn validates_limits_content_and_nesting() {
        assert!(UtilityDelegationLimits::bounded(1, 0, 0, 1).is_ok());
        assert!(UtilityDelegationLimits::bounded(MAX_UTILITY_DURATION_MS + 1, 0, 0, 1).is_err());
        let mut nested = request();
        nested.depth = 1;
        assert_eq!(
            nested.validate(),
            Err(AgentRuntimeDomainError::NestedUtilityDelegation)
        );
        let mut oversized = request();
        oversized.task = "x".repeat(MAX_UTILITY_TASK_CHARS + 1);
        assert!(oversized.validate().is_err());
    }

    #[test]
    fn rejects_revision_drift_before_admission() {
        assert_eq!(
            UtilityDelegationAttempt::admit(
                "delegation-1".to_string(),
                "attempt-1".to_string(),
                &request(),
                snapshot(),
                "sha256:new",
            ),
            Err(AgentRuntimeDomainError::StaleUtilityRevision)
        );
    }

    #[test]
    fn lifecycle_start_and_terminal_callbacks_are_idempotent() {
        let mut attempt = UtilityDelegationAttempt::admit(
            "delegation-1".to_string(),
            "attempt-1".to_string(),
            &request(),
            snapshot(),
            "sha256:abc",
        )
        .expect("admit");
        assert_eq!(attempt.start(), Ok(true));
        assert_eq!(attempt.start(), Ok(false));
        assert_eq!(
            attempt.finish(UtilityDelegationTerminal::Succeeded),
            Ok(true)
        );
        assert_eq!(attempt.finish(UtilityDelegationTerminal::Failed), Ok(false));
        assert_eq!(
            attempt.state(),
            UtilityDelegationState::Terminal(UtilityDelegationTerminal::Succeeded)
        );
    }
}
