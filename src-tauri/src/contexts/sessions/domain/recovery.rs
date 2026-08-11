use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SessionRecoveryStatus {
    Clean,
    Reconciling,
    ActionRequired,
    Quarantined,
}

impl SessionRecoveryStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Reconciling => "reconciling",
            Self::ActionRequired => "action_required",
            Self::Quarantined => "quarantined",
        }
    }

    pub(crate) fn from_storage(value: &str) -> Option<Self> {
        match value {
            "clean" => Some(Self::Clean),
            "reconciling" => Some(Self::Reconciling),
            "action_required" => Some(Self::ActionRequired),
            "quarantined" => Some(Self::Quarantined),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionRecoveryMetadata {
    status: SessionRecoveryStatus,
    recovery_revision: u64,
    state_revision: u64,
    history_revision: u64,
    active_execution_run_id: Option<String>,
    next_message_sequence: u64,
}

impl Default for SessionRecoveryMetadata {
    fn default() -> Self {
        Self {
            status: SessionRecoveryStatus::Clean,
            recovery_revision: 0,
            state_revision: 0,
            history_revision: 0,
            active_execution_run_id: None,
            next_message_sequence: 1,
        }
    }
}

impl SessionRecoveryMetadata {
    pub(crate) fn rehydrate(
        status: SessionRecoveryStatus,
        recovery_revision: u64,
        state_revision: u64,
        history_revision: u64,
        active_execution_run_id: Option<String>,
        next_message_sequence: u64,
    ) -> Self {
        Self {
            status,
            recovery_revision,
            state_revision,
            history_revision,
            active_execution_run_id,
            next_message_sequence,
        }
    }

    pub(crate) fn status(&self) -> SessionRecoveryStatus {
        self.status
    }

    pub(crate) fn recovery_revision(&self) -> u64 {
        self.recovery_revision
    }

    pub(crate) fn state_revision(&self) -> u64 {
        self.state_revision
    }

    pub(crate) fn history_revision(&self) -> u64 {
        self.history_revision
    }

    pub(crate) fn active_execution_run_id(&self) -> Option<&str> {
        self.active_execution_run_id.as_deref()
    }

    pub(crate) fn next_message_sequence(&self) -> u64 {
        self.next_message_sequence
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RecoveryDecision {
    Completed,
    Failed,
    Cancelled,
    InterruptedWithoutToolAmbiguity,
    ActionRequired,
    Quarantined,
    RetryLater,
    Acknowledged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RecoveryReasonCode {
    ConfirmedCompletedMessage,
    ConfirmedFailedMessage,
    ConfirmedCancelledOperation,
    InterruptedToolFreeResponse,
    MissingExecutionRun,
    MissingAssistantMessage,
    UnfinishedToolActivity,
    OpaqueProviderActivity,
    ConflictingExecutionRuns,
    ConflictingTerminalOutcomes,
    InvalidMessageSequence,
    InvalidExecutionCorrelation,
    LiveRuntimeHandle,
    StorageTemporarilyUnavailable,
    AcknowledgedByUser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RecoveryTrigger {
    Startup,
    ExplicitRetry,
    UserAcknowledgement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum RecoveryEvidenceReference {
    Session {
        session_id: String,
        state_revision: u64,
        history_revision: u64,
    },
    Message {
        message_id: String,
        execution_run_id: Option<String>,
        status: String,
    },
    Operation {
        operation_id: String,
        execution_run_id: Option<String>,
        status: String,
    },
    ToolActivity {
        tool_use_id: String,
        execution_run_id: Option<String>,
        status: String,
    },
    ProviderResumeMetadata {
        present: bool,
    },
    LiveRuntimeHandle {
        execution_run_id: Option<String>,
        present: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionRecoveryReport {
    report_id: String,
    session_id: String,
    recovery_revision: u64,
    trigger: RecoveryTrigger,
    observed_lifecycle: String,
    observed_execution_run_id: Option<String>,
    decision: RecoveryDecision,
    reason_codes: Vec<RecoveryReasonCode>,
    evidence_refs: Vec<RecoveryEvidenceReference>,
    created_at: String,
}

impl SessionRecoveryReport {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        report_id: String,
        session_id: String,
        recovery_revision: u64,
        trigger: RecoveryTrigger,
        observed_lifecycle: String,
        observed_execution_run_id: Option<String>,
        decision: RecoveryDecision,
        reason_codes: Vec<RecoveryReasonCode>,
        evidence_refs: Vec<RecoveryEvidenceReference>,
        created_at: String,
    ) -> Self {
        Self {
            report_id,
            session_id,
            recovery_revision,
            trigger,
            observed_lifecycle,
            observed_execution_run_id,
            decision,
            reason_codes,
            evidence_refs,
            created_at,
        }
    }

    pub(crate) fn report_id(&self) -> &str {
        &self.report_id
    }

    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }

    pub(crate) fn recovery_revision(&self) -> u64 {
        self.recovery_revision
    }

    pub(crate) fn trigger(&self) -> RecoveryTrigger {
        self.trigger
    }

    pub(crate) fn observed_lifecycle(&self) -> &str {
        &self.observed_lifecycle
    }

    pub(crate) fn observed_execution_run_id(&self) -> Option<&str> {
        self.observed_execution_run_id.as_deref()
    }

    pub(crate) fn decision(&self) -> RecoveryDecision {
        self.decision
    }

    pub(crate) fn reason_codes(&self) -> &[RecoveryReasonCode] {
        &self.reason_codes
    }

    pub(crate) fn evidence_refs(&self) -> &[RecoveryEvidenceReference] {
        &self.evidence_refs
    }

    pub(crate) fn created_at(&self) -> &str {
        &self.created_at
    }
}

#[cfg(test)]
mod tests {
    use serde::{de::DeserializeOwned, Serialize};
    use serde_json::{json, Value};

    use super::*;

    fn assert_scalar_round_trip<T>(cases: &[(T, &str)])
    where
        T: Copy + std::fmt::Debug + PartialEq + Serialize + DeserializeOwned,
    {
        for (value, expected) in cases {
            let encoded = serde_json::to_value(value).expect("serialize recovery value");
            assert_eq!(encoded, Value::String((*expected).to_string()));
            assert_eq!(
                serde_json::from_value::<T>(encoded).expect("deserialize recovery value"),
                *value
            );
        }
    }

    #[test]
    fn recovery_status_serialization_is_exhaustive() {
        let cases = [
            (SessionRecoveryStatus::Clean, "clean"),
            (SessionRecoveryStatus::Reconciling, "reconciling"),
            (SessionRecoveryStatus::ActionRequired, "action_required"),
            (SessionRecoveryStatus::Quarantined, "quarantined"),
        ];
        assert_scalar_round_trip(&cases);
        for (value, encoded) in cases {
            assert_eq!(value.as_str(), encoded);
            assert_eq!(SessionRecoveryStatus::from_storage(encoded), Some(value));
        }
        assert_eq!(SessionRecoveryStatus::from_storage("unknown"), None);
    }

    #[test]
    fn recovery_decision_serialization_is_exhaustive() {
        assert_scalar_round_trip(&[
            (RecoveryDecision::Completed, "completed"),
            (RecoveryDecision::Failed, "failed"),
            (RecoveryDecision::Cancelled, "cancelled"),
            (
                RecoveryDecision::InterruptedWithoutToolAmbiguity,
                "interrupted_without_tool_ambiguity",
            ),
            (RecoveryDecision::ActionRequired, "action_required"),
            (RecoveryDecision::Quarantined, "quarantined"),
            (RecoveryDecision::RetryLater, "retry_later"),
            (RecoveryDecision::Acknowledged, "acknowledged"),
        ]);
    }

    #[test]
    fn recovery_reason_serialization_is_exhaustive() {
        assert_scalar_round_trip(&[
            (
                RecoveryReasonCode::ConfirmedCompletedMessage,
                "confirmed_completed_message",
            ),
            (
                RecoveryReasonCode::ConfirmedFailedMessage,
                "confirmed_failed_message",
            ),
            (
                RecoveryReasonCode::ConfirmedCancelledOperation,
                "confirmed_cancelled_operation",
            ),
            (
                RecoveryReasonCode::InterruptedToolFreeResponse,
                "interrupted_tool_free_response",
            ),
            (
                RecoveryReasonCode::MissingExecutionRun,
                "missing_execution_run",
            ),
            (
                RecoveryReasonCode::MissingAssistantMessage,
                "missing_assistant_message",
            ),
            (
                RecoveryReasonCode::UnfinishedToolActivity,
                "unfinished_tool_activity",
            ),
            (
                RecoveryReasonCode::OpaqueProviderActivity,
                "opaque_provider_activity",
            ),
            (
                RecoveryReasonCode::ConflictingExecutionRuns,
                "conflicting_execution_runs",
            ),
            (
                RecoveryReasonCode::ConflictingTerminalOutcomes,
                "conflicting_terminal_outcomes",
            ),
            (
                RecoveryReasonCode::InvalidMessageSequence,
                "invalid_message_sequence",
            ),
            (
                RecoveryReasonCode::InvalidExecutionCorrelation,
                "invalid_execution_correlation",
            ),
            (RecoveryReasonCode::LiveRuntimeHandle, "live_runtime_handle"),
            (
                RecoveryReasonCode::StorageTemporarilyUnavailable,
                "storage_temporarily_unavailable",
            ),
            (
                RecoveryReasonCode::AcknowledgedByUser,
                "acknowledged_by_user",
            ),
        ]);
    }

    #[test]
    fn recovery_trigger_serialization_is_exhaustive() {
        assert_scalar_round_trip(&[
            (RecoveryTrigger::Startup, "startup"),
            (RecoveryTrigger::ExplicitRetry, "explicit_retry"),
            (RecoveryTrigger::UserAcknowledgement, "user_acknowledgement"),
        ]);
    }

    #[test]
    fn evidence_reference_serialization_is_exhaustive() {
        let references = vec![
            RecoveryEvidenceReference::Session {
                session_id: "session-1".to_string(),
                state_revision: 4,
                history_revision: 9,
            },
            RecoveryEvidenceReference::Message {
                message_id: "message-1".to_string(),
                execution_run_id: Some("run-1".to_string()),
                status: "streaming".to_string(),
            },
            RecoveryEvidenceReference::Operation {
                operation_id: "operation-1".to_string(),
                execution_run_id: None,
                status: "failed".to_string(),
            },
            RecoveryEvidenceReference::ToolActivity {
                tool_use_id: "tool-1".to_string(),
                execution_run_id: Some("run-1".to_string()),
                status: "unfinished".to_string(),
            },
            RecoveryEvidenceReference::ProviderResumeMetadata { present: true },
            RecoveryEvidenceReference::LiveRuntimeHandle {
                execution_run_id: Some("run-1".to_string()),
                present: false,
            },
        ];

        for reference in references {
            let encoded = serde_json::to_value(&reference).expect("serialize evidence reference");
            let decoded = serde_json::from_value(encoded).expect("deserialize evidence reference");
            assert_eq!(reference, decoded);
        }
    }

    #[test]
    fn report_serialization_preserves_all_immutable_metadata() {
        let report = SessionRecoveryReport::new(
            "report-1".to_string(),
            "session-1".to_string(),
            3,
            RecoveryTrigger::Startup,
            "running".to_string(),
            Some("run-1".to_string()),
            RecoveryDecision::ActionRequired,
            vec![RecoveryReasonCode::UnfinishedToolActivity],
            vec![RecoveryEvidenceReference::ToolActivity {
                tool_use_id: "tool-1".to_string(),
                execution_run_id: Some("run-1".to_string()),
                status: "unfinished".to_string(),
            }],
            "2026-08-09T00:00:00Z".to_string(),
        );

        let encoded = serde_json::to_value(&report).expect("serialize recovery report");
        assert_eq!(
            encoded,
            json!({
                "reportId": "report-1",
                "sessionId": "session-1",
                "recoveryRevision": 3,
                "trigger": "startup",
                "observedLifecycle": "running",
                "observedExecutionRunId": "run-1",
                "decision": "action_required",
                "reasonCodes": ["unfinished_tool_activity"],
                "evidenceRefs": [{
                    "kind": "tool_activity",
                    "toolUseId": "tool-1",
                    "executionRunId": "run-1",
                    "status": "unfinished"
                }],
                "createdAt": "2026-08-09T00:00:00Z"
            })
        );

        let decoded: SessionRecoveryReport =
            serde_json::from_value(encoded).expect("deserialize recovery report");
        assert_eq!(decoded, report);
        assert_eq!(decoded.report_id(), "report-1");
        assert_eq!(decoded.session_id(), "session-1");
        assert_eq!(decoded.recovery_revision(), 3);
        assert_eq!(decoded.trigger(), RecoveryTrigger::Startup);
        assert_eq!(decoded.observed_lifecycle(), "running");
        assert_eq!(decoded.observed_execution_run_id(), Some("run-1"));
        assert_eq!(decoded.decision(), RecoveryDecision::ActionRequired);
        assert_eq!(
            decoded.reason_codes(),
            &[RecoveryReasonCode::UnfinishedToolActivity]
        );
        assert_eq!(decoded.evidence_refs().len(), 1);
        assert_eq!(decoded.created_at(), "2026-08-09T00:00:00Z");
    }
}
