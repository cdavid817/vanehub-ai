use super::{MessageRole, MessageStatus, SessionLifecycle, SessionRecoveryStatus};

pub(crate) const MAX_RECOVERY_EVIDENCE_MESSAGES: usize = 256;
pub(crate) const MAX_RECOVERY_EVIDENCE_OPERATIONS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionEvidenceState {
    pub(crate) session_id: String,
    pub(crate) lifecycle: SessionLifecycle,
    pub(crate) recovery_status: SessionRecoveryStatus,
    pub(crate) active_execution_run_id: Option<String>,
    pub(crate) recovery_revision: u64,
    pub(crate) state_revision: u64,
    pub(crate) history_revision: u64,
    pub(crate) execution_fidelity: ExecutionEvidenceFidelity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionEvidenceFidelity {
    ManagedApi,
    ManagedCliOpaque,
    InteractiveOpaque,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MessageTerminalEvidence {
    pub(crate) message_id: String,
    pub(crate) session_sequence: u64,
    pub(crate) execution_run_id: Option<String>,
    pub(crate) role: MessageRole,
    pub(crate) status: MessageStatus,
    pub(crate) has_content: bool,
    pub(crate) tool_activity: ToolActivityEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OperationTerminalStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OperationTerminalEvidence {
    pub(crate) operation_id: String,
    pub(crate) execution_run_id: Option<String>,
    pub(crate) status: OperationTerminalStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolEvidenceFidelity {
    Managed,
    ProviderOpaque,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolActivityEvidence {
    None,
    Complete {
        count: u32,
        fidelity: ToolEvidenceFidelity,
    },
    Incomplete {
        count: u32,
        fidelity: ToolEvidenceFidelity,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LiveHandleEvidence {
    Present,
    Absent,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProviderResumeEvidence {
    pub(crate) metadata_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionTerminalEvidence {
    pub(crate) session: SessionEvidenceState,
    pub(crate) observed_execution_run_id: Option<String>,
    messages: Vec<MessageTerminalEvidence>,
    conflicting_message: Option<MessageTerminalEvidence>,
    operations: Vec<OperationTerminalEvidence>,
    pub(crate) provider_resume: ProviderResumeEvidence,
    pub(crate) live_handle: LiveHandleEvidence,
}

impl SessionTerminalEvidence {
    pub(crate) fn try_new(
        session: SessionEvidenceState,
        observed_execution_run_id: Option<String>,
        messages: Vec<MessageTerminalEvidence>,
        operations: Vec<OperationTerminalEvidence>,
        provider_resume: ProviderResumeEvidence,
        live_handle: LiveHandleEvidence,
    ) -> Result<Self, EvidenceBoundError> {
        if messages.len() > MAX_RECOVERY_EVIDENCE_MESSAGES {
            return Err(EvidenceBoundError::Messages(messages.len()));
        }
        if operations.len() > MAX_RECOVERY_EVIDENCE_OPERATIONS {
            return Err(EvidenceBoundError::Operations(operations.len()));
        }
        Ok(Self {
            session,
            observed_execution_run_id,
            messages,
            conflicting_message: None,
            operations,
            provider_resume,
            live_handle,
        })
    }

    pub(crate) fn messages(&self) -> &[MessageTerminalEvidence] {
        &self.messages
    }

    pub(crate) fn operations(&self) -> &[OperationTerminalEvidence] {
        &self.operations
    }

    pub(crate) fn conflicting_message(&self) -> Option<&MessageTerminalEvidence> {
        self.conflicting_message.as_ref()
    }

    pub(crate) fn set_conflicting_message(&mut self, message: Option<MessageTerminalEvidence>) {
        self.conflicting_message = message;
    }

    pub(crate) fn replace_operations(
        &mut self,
        operations: Vec<OperationTerminalEvidence>,
    ) -> Result<(), EvidenceBoundError> {
        if operations.len() > MAX_RECOVERY_EVIDENCE_OPERATIONS {
            return Err(EvidenceBoundError::Operations(operations.len()));
        }
        self.operations = operations;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EvidenceBoundError {
    Messages(usize),
    Operations(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> SessionEvidenceState {
        SessionEvidenceState {
            session_id: "session-1".to_string(),
            lifecycle: SessionLifecycle::Running,
            recovery_status: SessionRecoveryStatus::Reconciling,
            active_execution_run_id: Some("run-1".to_string()),
            recovery_revision: 1,
            state_revision: 2,
            history_revision: 3,
            execution_fidelity: ExecutionEvidenceFidelity::ManagedApi,
        }
    }

    #[test]
    fn evidence_snapshot_enforces_message_and_operation_bounds() {
        let message = MessageTerminalEvidence {
            message_id: "message-1".to_string(),
            session_sequence: 1,
            execution_run_id: Some("run-1".to_string()),
            role: MessageRole::Assistant,
            status: MessageStatus::Streaming,
            has_content: true,
            tool_activity: ToolActivityEvidence::None,
        };
        let operation = OperationTerminalEvidence {
            operation_id: "operation-1".to_string(),
            execution_run_id: Some("run-1".to_string()),
            status: OperationTerminalStatus::Running,
        };
        let messages = vec![message; MAX_RECOVERY_EVIDENCE_MESSAGES + 1];
        assert_eq!(
            SessionTerminalEvidence::try_new(
                state(),
                Some("run-1".to_string()),
                messages,
                Vec::new(),
                ProviderResumeEvidence {
                    metadata_present: false,
                },
                LiveHandleEvidence::Absent,
            ),
            Err(EvidenceBoundError::Messages(
                MAX_RECOVERY_EVIDENCE_MESSAGES + 1
            ))
        );
        let operations = vec![operation; MAX_RECOVERY_EVIDENCE_OPERATIONS + 1];
        assert!(matches!(
            SessionTerminalEvidence::try_new(
                state(),
                Some("run-1".to_string()),
                Vec::new(),
                operations,
                ProviderResumeEvidence {
                    metadata_present: true,
                },
                LiveHandleEvidence::Present,
            ),
            Err(EvidenceBoundError::Operations(_))
        ));
    }
}
