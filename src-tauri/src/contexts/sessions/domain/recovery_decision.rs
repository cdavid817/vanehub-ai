use super::evidence::{
    ExecutionEvidenceFidelity, LiveHandleEvidence, OperationTerminalStatus,
    SessionTerminalEvidence, ToolActivityEvidence,
};
use super::recovery::{RecoveryDecision, RecoveryReasonCode};
use super::{MessageRole, MessageStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecoveryDecisionResult {
    pub(crate) decision: RecoveryDecision,
    pub(crate) reason_codes: Vec<RecoveryReasonCode>,
}

pub(crate) fn decide_recovery(evidence: &SessionTerminalEvidence) -> RecoveryDecisionResult {
    if evidence.live_handle == LiveHandleEvidence::Unavailable {
        return result(
            RecoveryDecision::RetryLater,
            RecoveryReasonCode::StorageTemporarilyUnavailable,
        );
    }
    if evidence.live_handle == LiveHandleEvidence::Present {
        return result(
            RecoveryDecision::RetryLater,
            RecoveryReasonCode::LiveRuntimeHandle,
        );
    }
    if !has_valid_sequence(evidence) {
        return result(
            RecoveryDecision::Quarantined,
            RecoveryReasonCode::InvalidMessageSequence,
        );
    }
    let Some(active_run) = evidence.session.active_execution_run_id.as_deref() else {
        return result(
            RecoveryDecision::ActionRequired,
            RecoveryReasonCode::MissingExecutionRun,
        );
    };
    if evidence.observed_execution_run_id.as_deref() != Some(active_run) {
        return result(
            RecoveryDecision::ActionRequired,
            RecoveryReasonCode::ConflictingExecutionRuns,
        );
    }
    if evidence.conflicting_message().is_some() {
        return result(
            RecoveryDecision::ActionRequired,
            RecoveryReasonCode::ConflictingExecutionRuns,
        );
    }
    if evidence.operations().iter().any(|operation| {
        operation
            .execution_run_id
            .as_deref()
            .is_some_and(|run_id| run_id != active_run)
    }) {
        return result(
            RecoveryDecision::ActionRequired,
            RecoveryReasonCode::ConflictingExecutionRuns,
        );
    }
    let assistant_messages = evidence
        .messages()
        .iter()
        .filter(|message| {
            message.role == MessageRole::Assistant
                && message.execution_run_id.as_deref() == Some(active_run)
        })
        .collect::<Vec<_>>();
    if assistant_messages.is_empty() {
        return result(
            RecoveryDecision::ActionRequired,
            RecoveryReasonCode::MissingAssistantMessage,
        );
    }
    if assistant_messages.len() != 1 {
        return result(
            RecoveryDecision::Quarantined,
            RecoveryReasonCode::InvalidExecutionCorrelation,
        );
    }
    if assistant_messages.iter().any(|message| {
        matches!(
            message.tool_activity,
            ToolActivityEvidence::Incomplete { .. }
        )
    }) {
        return result(
            RecoveryDecision::ActionRequired,
            RecoveryReasonCode::UnfinishedToolActivity,
        );
    }

    let mut outcomes = assistant_messages
        .iter()
        .filter_map(|message| message_outcome(message.status))
        .chain(
            evidence
                .operations()
                .iter()
                .filter(|operation| operation.execution_run_id.as_deref() == Some(active_run))
                .filter_map(|operation| operation_outcome(operation.status)),
        )
        .collect::<Vec<_>>();
    outcomes.sort_by_key(|decision| decision_order(*decision));
    outcomes.dedup();
    if outcomes.len() > 1 {
        return result(
            RecoveryDecision::ActionRequired,
            RecoveryReasonCode::ConflictingTerminalOutcomes,
        );
    }
    if let Some(decision) = outcomes.first().copied() {
        return result(decision, terminal_reason(decision));
    }
    if matches!(
        evidence.session.execution_fidelity,
        ExecutionEvidenceFidelity::ManagedCliOpaque | ExecutionEvidenceFidelity::InteractiveOpaque
    ) {
        return result(
            RecoveryDecision::ActionRequired,
            RecoveryReasonCode::OpaqueProviderActivity,
        );
    }
    result(
        RecoveryDecision::InterruptedWithoutToolAmbiguity,
        RecoveryReasonCode::InterruptedToolFreeResponse,
    )
}

fn has_valid_sequence(evidence: &SessionTerminalEvidence) -> bool {
    evidence
        .messages()
        .windows(2)
        .all(|pair| pair[0].session_sequence < pair[1].session_sequence)
}

fn message_outcome(status: MessageStatus) -> Option<RecoveryDecision> {
    match status {
        MessageStatus::Completed => Some(RecoveryDecision::Completed),
        MessageStatus::Failed => Some(RecoveryDecision::Failed),
        MessageStatus::Cancelled => Some(RecoveryDecision::Cancelled),
        MessageStatus::Pending | MessageStatus::Streaming => None,
    }
}

fn operation_outcome(status: OperationTerminalStatus) -> Option<RecoveryDecision> {
    match status {
        OperationTerminalStatus::Succeeded => Some(RecoveryDecision::Completed),
        OperationTerminalStatus::Failed => Some(RecoveryDecision::Failed),
        OperationTerminalStatus::Cancelled => Some(RecoveryDecision::Cancelled),
        OperationTerminalStatus::Running => None,
    }
}

fn terminal_reason(decision: RecoveryDecision) -> RecoveryReasonCode {
    match decision {
        RecoveryDecision::Completed => RecoveryReasonCode::ConfirmedCompletedMessage,
        RecoveryDecision::Failed => RecoveryReasonCode::ConfirmedFailedMessage,
        RecoveryDecision::Cancelled => RecoveryReasonCode::ConfirmedCancelledOperation,
        _ => RecoveryReasonCode::InvalidExecutionCorrelation,
    }
}

fn decision_order(decision: RecoveryDecision) -> u8 {
    match decision {
        RecoveryDecision::Completed => 0,
        RecoveryDecision::Failed => 1,
        RecoveryDecision::Cancelled => 2,
        _ => 3,
    }
}

fn result(decision: RecoveryDecision, reason_code: RecoveryReasonCode) -> RecoveryDecisionResult {
    RecoveryDecisionResult {
        decision,
        reason_codes: vec![reason_code],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::sessions::domain::evidence::{
        MessageTerminalEvidence, OperationTerminalEvidence, ProviderResumeEvidence,
        SessionEvidenceState, ToolEvidenceFidelity,
    };
    use crate::contexts::sessions::domain::{SessionLifecycle, SessionRecoveryStatus};

    fn evidence(
        fidelity: ExecutionEvidenceFidelity,
        message_status: MessageStatus,
        tool_activity: ToolActivityEvidence,
        operations: Vec<OperationTerminalEvidence>,
    ) -> SessionTerminalEvidence {
        SessionTerminalEvidence::try_new(
            SessionEvidenceState {
                session_id: "session-1".to_string(),
                lifecycle: SessionLifecycle::Running,
                recovery_status: SessionRecoveryStatus::Reconciling,
                active_execution_run_id: Some("run-1".to_string()),
                recovery_revision: 1,
                state_revision: 2,
                history_revision: 3,
                execution_fidelity: fidelity,
            },
            Some("run-1".to_string()),
            vec![MessageTerminalEvidence {
                message_id: "message-1".to_string(),
                session_sequence: 1,
                execution_run_id: Some("run-1".to_string()),
                role: MessageRole::Assistant,
                status: message_status,
                has_content: true,
                tool_activity,
            }],
            operations,
            ProviderResumeEvidence {
                metadata_present: true,
            },
            LiveHandleEvidence::Absent,
        )
        .expect("bounded evidence")
    }

    #[test]
    fn terminal_messages_map_to_matching_decisions() {
        for (status, expected) in [
            (MessageStatus::Completed, RecoveryDecision::Completed),
            (MessageStatus::Failed, RecoveryDecision::Failed),
            (MessageStatus::Cancelled, RecoveryDecision::Cancelled),
        ] {
            assert_eq!(
                decide_recovery(&evidence(
                    ExecutionEvidenceFidelity::ManagedApi,
                    status,
                    ToolActivityEvidence::None,
                    Vec::new(),
                ))
                .decision,
                expected
            );
        }
    }

    #[test]
    fn multiple_assistant_messages_for_one_run_are_quarantined() {
        let mut duplicate = evidence(
            ExecutionEvidenceFidelity::ManagedApi,
            MessageStatus::Streaming,
            ToolActivityEvidence::None,
            Vec::new(),
        );
        let messages = vec![
            duplicate.messages()[0].clone(),
            MessageTerminalEvidence {
                message_id: "message-2".to_string(),
                session_sequence: 2,
                execution_run_id: Some("run-1".to_string()),
                role: MessageRole::Assistant,
                status: MessageStatus::Streaming,
                has_content: true,
                tool_activity: ToolActivityEvidence::None,
            },
        ];
        duplicate = SessionTerminalEvidence::try_new(
            duplicate.session.clone(),
            duplicate.observed_execution_run_id.clone(),
            messages,
            duplicate.operations().to_vec(),
            duplicate.provider_resume,
            duplicate.live_handle,
        )
        .expect("bounded duplicate evidence");

        let decision = decide_recovery(&duplicate);

        assert_eq!(decision.decision, RecoveryDecision::Quarantined);
        assert_eq!(
            decision.reason_codes,
            vec![RecoveryReasonCode::InvalidExecutionCorrelation]
        );
    }

    #[test]
    fn incompatible_terminal_facts_require_review() {
        let operation = OperationTerminalEvidence {
            operation_id: "operation-1".to_string(),
            execution_run_id: Some("run-1".to_string()),
            status: OperationTerminalStatus::Failed,
        };
        assert_eq!(
            decide_recovery(&evidence(
                ExecutionEvidenceFidelity::ManagedApi,
                MessageStatus::Completed,
                ToolActivityEvidence::None,
                vec![operation],
            ))
            .decision,
            RecoveryDecision::ActionRequired
        );
    }

    #[test]
    fn managed_tool_free_partial_is_interruptible_but_opaque_activity_is_not() {
        let managed = evidence(
            ExecutionEvidenceFidelity::ManagedApi,
            MessageStatus::Streaming,
            ToolActivityEvidence::None,
            Vec::new(),
        );
        assert_eq!(
            decide_recovery(&managed).decision,
            RecoveryDecision::InterruptedWithoutToolAmbiguity
        );
        let opaque = evidence(
            ExecutionEvidenceFidelity::ManagedCliOpaque,
            MessageStatus::Streaming,
            ToolActivityEvidence::Incomplete {
                count: 1,
                fidelity: ToolEvidenceFidelity::ProviderOpaque,
            },
            Vec::new(),
        );
        assert_eq!(
            decide_recovery(&opaque).reason_codes,
            vec![RecoveryReasonCode::UnfinishedToolActivity]
        );
        let opaque_without_visible_tools = evidence(
            ExecutionEvidenceFidelity::ManagedCliOpaque,
            MessageStatus::Streaming,
            ToolActivityEvidence::None,
            Vec::new(),
        );
        assert_eq!(
            decide_recovery(&opaque_without_visible_tools).reason_codes,
            vec![RecoveryReasonCode::OpaqueProviderActivity]
        );
    }

    #[test]
    fn operation_terminal_statuses_are_typed_and_decidable() {
        for (status, expected) in [
            (
                OperationTerminalStatus::Succeeded,
                RecoveryDecision::Completed,
            ),
            (OperationTerminalStatus::Failed, RecoveryDecision::Failed),
            (
                OperationTerminalStatus::Cancelled,
                RecoveryDecision::Cancelled,
            ),
        ] {
            let operation = OperationTerminalEvidence {
                operation_id: "operation-1".to_string(),
                execution_run_id: Some("run-1".to_string()),
                status,
            };
            assert_eq!(
                decide_recovery(&evidence(
                    ExecutionEvidenceFidelity::ManagedApi,
                    MessageStatus::Streaming,
                    ToolActivityEvidence::None,
                    vec![operation],
                ))
                .decision,
                expected
            );
        }
    }

    #[test]
    fn cross_run_operation_evidence_is_never_merged_into_the_active_run() {
        let operation = OperationTerminalEvidence {
            operation_id: "operation-other".to_string(),
            execution_run_id: Some("run-other".to_string()),
            status: OperationTerminalStatus::Succeeded,
        };
        let decision = decide_recovery(&evidence(
            ExecutionEvidenceFidelity::ManagedApi,
            MessageStatus::Streaming,
            ToolActivityEvidence::None,
            vec![operation],
        ));
        assert_eq!(decision.decision, RecoveryDecision::ActionRequired);
        assert_eq!(
            decision.reason_codes,
            vec![RecoveryReasonCode::ConflictingExecutionRuns]
        );
    }

    #[test]
    fn unfinished_cross_run_message_requires_review() {
        let mut snapshot = evidence(
            ExecutionEvidenceFidelity::ManagedApi,
            MessageStatus::Streaming,
            ToolActivityEvidence::None,
            Vec::new(),
        );
        snapshot.set_conflicting_message(Some(MessageTerminalEvidence {
            message_id: "message-other".to_string(),
            session_sequence: 2,
            execution_run_id: Some("run-other".to_string()),
            role: MessageRole::Assistant,
            status: MessageStatus::Pending,
            has_content: false,
            tool_activity: ToolActivityEvidence::None,
        }));

        let decision = decide_recovery(&snapshot);
        assert_eq!(decision.decision, RecoveryDecision::ActionRequired);
        assert_eq!(
            decision.reason_codes,
            vec![RecoveryReasonCode::ConflictingExecutionRuns]
        );
    }

    #[test]
    fn decision_matrix_covers_runtime_fidelity_and_evidence_availability() {
        let cases = [
            (
                "api partial",
                ExecutionEvidenceFidelity::ManagedApi,
                MessageStatus::Streaming,
                ToolActivityEvidence::None,
                LiveHandleEvidence::Absent,
                RecoveryDecision::InterruptedWithoutToolAmbiguity,
            ),
            (
                "cli opaque",
                ExecutionEvidenceFidelity::ManagedCliOpaque,
                MessageStatus::Streaming,
                ToolActivityEvidence::None,
                LiveHandleEvidence::Absent,
                RecoveryDecision::ActionRequired,
            ),
            (
                "provider resume is continuity only",
                ExecutionEvidenceFidelity::ManagedApi,
                MessageStatus::Completed,
                ToolActivityEvidence::None,
                LiveHandleEvidence::Absent,
                RecoveryDecision::Completed,
            ),
            (
                "incomplete tool",
                ExecutionEvidenceFidelity::ManagedApi,
                MessageStatus::Streaming,
                ToolActivityEvidence::Incomplete {
                    count: 1,
                    fidelity: ToolEvidenceFidelity::Managed,
                },
                LiveHandleEvidence::Absent,
                RecoveryDecision::ActionRequired,
            ),
            (
                "adapter unavailable",
                ExecutionEvidenceFidelity::ManagedApi,
                MessageStatus::Streaming,
                ToolActivityEvidence::None,
                LiveHandleEvidence::Unavailable,
                RecoveryDecision::RetryLater,
            ),
        ];
        for (name, fidelity, status, tools, live_handle, expected) in cases {
            let mut snapshot = evidence(fidelity, status, tools, Vec::new());
            snapshot.live_handle = live_handle;
            assert_eq!(decide_recovery(&snapshot).decision, expected, "{name}");
        }
    }

    #[test]
    fn terminal_outcomes_do_not_regress_and_operation_order_is_irrelevant() {
        for (status, terminal_status, expected) in [
            (
                MessageStatus::Completed,
                OperationTerminalStatus::Succeeded,
                RecoveryDecision::Completed,
            ),
            (
                MessageStatus::Failed,
                OperationTerminalStatus::Failed,
                RecoveryDecision::Failed,
            ),
            (
                MessageStatus::Cancelled,
                OperationTerminalStatus::Cancelled,
                RecoveryDecision::Cancelled,
            ),
        ] {
            let matching = OperationTerminalEvidence {
                operation_id: "operation-terminal".to_string(),
                execution_run_id: Some("run-1".to_string()),
                status: terminal_status,
            };
            let running = OperationTerminalEvidence {
                operation_id: "operation-running".to_string(),
                execution_run_id: Some("run-1".to_string()),
                status: OperationTerminalStatus::Running,
            };
            let forward = evidence(
                ExecutionEvidenceFidelity::ManagedApi,
                status,
                ToolActivityEvidence::None,
                vec![matching.clone(), running.clone()],
            );
            let reverse = evidence(
                ExecutionEvidenceFidelity::ManagedApi,
                status,
                ToolActivityEvidence::None,
                vec![running, matching],
            );
            assert_eq!(decide_recovery(&forward).decision, expected);
            assert_eq!(decide_recovery(&reverse), decide_recovery(&forward));
        }
    }
}
