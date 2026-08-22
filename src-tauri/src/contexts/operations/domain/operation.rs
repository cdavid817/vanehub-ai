use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OperationKind {
    Sdk,
    Mcp,
    Agent,
    Workspace,
    Extension,
    Cli,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OperationStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl OperationStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OperationRecoveryEvidence {
    pub(crate) operation_id: String,
    pub(crate) execution_run_id: String,
    pub(crate) status: OperationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationLogEntry {
    pub operation_id: String,
    pub line: String,
    pub timestamp: String,
}

/// A partial progress report. Every field is optional so a caller can move one dimension without
/// clearing the others -- see `OperationTask::report_progress`.
///
/// The first production caller arrives with CLI lifecycle phases (task 8.1 of
/// `add-source-aware-cli-environment-management`). `expect` rather than `allow`: once that caller
/// lands, the unfulfilled expectation becomes an error and forces this attribute to be deleted.
#[expect(
    dead_code,
    reason = "consumed by CLI lifecycle phases in task 8.1; remove this attribute with that task"
)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OperationProgress {
    pub phase: Option<String>,
    pub completed_units: Option<u32>,
    pub total_units: Option<u32>,
    pub cancellable: Option<bool>,
}

#[expect(
    dead_code,
    reason = "consumed by CLI lifecycle phases in task 8.1; remove this attribute with that task"
)]
impl OperationProgress {
    pub(crate) fn phase(phase: impl Into<String>) -> Self {
        Self {
            phase: Some(phase.into()),
            ..Self::default()
        }
    }

    pub(crate) fn with_cancellable(mut self, cancellable: bool) -> Self {
        self.cancellable = Some(cancellable);
        self
    }

    pub(crate) fn with_units(mut self, completed: u32, total: u32) -> Self {
        self.completed_units = Some(completed);
        self.total_units = Some(total);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OperationTask {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    pub kind: OperationKind,
    pub status: OperationStatus,
    pub related_entity_id: Option<String>,
    pub message: Option<String>,
    pub logs: Vec<OperationLogEntry>,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// Descriptive stage of a long-running operation. `status` stays authoritative -- a phase is
    /// for the user reading progress, never for a caller deciding whether work finished.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_units: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_units: Option<u32>,
    /// Whether cancellation can be requested *now*. Absent means the operation never declared one
    /// way or the other; it does not mean cancelling would undo an external effect that already
    /// happened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellable: Option<bool>,
}

impl OperationTask {
    pub(crate) fn start(
        id: String,
        kind: OperationKind,
        related_entity_id: Option<String>,
        message: Option<String>,
        now: String,
    ) -> Self {
        Self {
            id,
            execution_run_id: None,
            trace_id: None,
            kind,
            status: OperationStatus::Running,
            related_entity_id,
            message,
            logs: Vec::new(),
            result: None,
            error: None,
            created_at: now.clone(),
            updated_at: now,
            phase: None,
            completed_units: None,
            total_units: None,
            cancellable: None,
        }
    }

    /// Records descriptive progress. Only the supplied fields move: a caller reporting a phase
    /// change must not have to restate unit counts it does not own, and vice versa.
    #[expect(
        dead_code,
        reason = "consumed by CLI lifecycle phases in task 8.1; remove this attribute with that task"
    )]
    pub(crate) fn report_progress(&mut self, progress: OperationProgress, updated_at: String) {
        if let Some(phase) = progress.phase {
            self.phase = Some(phase);
        }
        if let Some(completed) = progress.completed_units {
            self.completed_units = Some(completed);
        }
        if let Some(total) = progress.total_units {
            self.total_units = Some(total);
        }
        if let Some(cancellable) = progress.cancellable {
            self.cancellable = Some(cancellable);
        }
        self.updated_at = updated_at;
    }

    pub(crate) fn correlate_execution(&mut self, run_id: String, trace_id: String) {
        self.execution_run_id = Some(run_id);
        self.trace_id = Some(trace_id);
    }

    pub(crate) fn append_log(&mut self, line: String, log_timestamp: String, updated_at: String) {
        self.logs.push(OperationLogEntry {
            operation_id: self.id.clone(),
            line,
            timestamp: log_timestamp,
        });
        self.updated_at = updated_at;
    }

    pub(crate) fn succeed(&mut self, result: Option<Value>, updated_at: String) {
        self.status = OperationStatus::Succeeded;
        self.result = result;
        self.error = None;
        self.updated_at = updated_at;
    }

    pub(crate) fn fail(&mut self, error: String, updated_at: String) {
        self.status = OperationStatus::Failed;
        self.error = Some(error);
        self.updated_at = updated_at;
    }

    pub(crate) fn cancel(&mut self, updated_at: String) {
        self.status = OperationStatus::Cancelled;
        self.error = None;
        self.updated_at = updated_at;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_preserves_observable_lifecycle_fields() {
        let mut operation = OperationTask::start(
            "op-fixed-1".to_string(),
            OperationKind::Mcp,
            Some("server-1".to_string()),
            Some("Testing".to_string()),
            "100".to_string(),
        );
        operation.append_log(
            "connected".to_string(),
            "101".to_string(),
            "102".to_string(),
        );
        operation.succeed(Some(serde_json::json!({ "ok": true })), "103".to_string());

        assert_eq!(operation.id, "op-fixed-1");
        assert_eq!(operation.status, OperationStatus::Succeeded);
        assert_eq!(operation.created_at, "100");
        assert_eq!(operation.updated_at, "103");
        assert_eq!(operation.logs[0].operation_id, "op-fixed-1");
        assert_eq!(operation.logs[0].timestamp, "101");
        assert_eq!(operation.result, Some(serde_json::json!({ "ok": true })));
        assert!(operation.error.is_none());
    }

    #[test]
    fn failure_keeps_the_existing_result_semantics() {
        let mut operation = OperationTask::start(
            "op-fixed-2".to_string(),
            OperationKind::Sdk,
            None,
            None,
            "200".to_string(),
        );
        operation.result = Some(serde_json::json!({ "partial": true }));
        operation.fail("install failed".to_string(), "201".to_string());

        assert_eq!(operation.status, OperationStatus::Failed);
        assert_eq!(operation.error.as_deref(), Some("install failed"));
        assert_eq!(
            operation.result,
            Some(serde_json::json!({ "partial": true }))
        );
    }

    #[test]
    fn cancellation_is_a_distinct_terminal_state() {
        let mut operation = OperationTask::start(
            "op-fixed-3".to_string(),
            OperationKind::Agent,
            Some("session-1".to_string()),
            None,
            "300".to_string(),
        );

        operation.cancel("301".to_string());

        assert_eq!(operation.status, OperationStatus::Cancelled);
        assert_eq!(operation.updated_at, "301");
        assert!(operation.error.is_none());
    }

    #[test]
    fn cli_is_a_serialized_operation_kind() {
        assert_eq!(
            serde_json::to_value(OperationKind::Cli).expect("serialize"),
            serde_json::json!("cli")
        );
        // The five existing kinds keep their wire values: a rename here silently breaks every
        // frontend consumer that switches on `kind`.
        for (kind, wire) in [
            (OperationKind::Sdk, "sdk"),
            (OperationKind::Mcp, "mcp"),
            (OperationKind::Agent, "agent"),
            (OperationKind::Workspace, "workspace"),
            (OperationKind::Extension, "extension"),
        ] {
            assert_eq!(
                serde_json::to_value(kind).expect("serialize"),
                serde_json::json!(wire)
            );
        }
    }

    #[test]
    fn operations_without_progress_omit_the_optional_fields_entirely() {
        let operation = OperationTask::start(
            "op-fixed-5".to_string(),
            OperationKind::Sdk,
            None,
            None,
            "500".to_string(),
        );

        let json = serde_json::to_value(&operation).expect("serialize");
        let object = json.as_object().expect("object");
        // Absent rather than null: an existing non-CLI consumer must see the exact payload it saw
        // before these fields existed.
        for field in ["phase", "completedUnits", "totalUnits", "cancellable"] {
            assert!(!object.contains_key(field), "{field} must be omitted");
        }
    }

    #[test]
    fn a_payload_written_before_progress_existed_still_deserializes() {
        let legacy = serde_json::json!({
            "id": "op-legacy",
            "kind": "sdk",
            "status": "running",
            "relatedEntityId": null,
            "message": null,
            "logs": [],
            "result": null,
            "error": null,
            "createdAt": "600",
            "updatedAt": "600"
        });

        let operation: OperationTask = serde_json::from_value(legacy).expect("deserialize");

        assert_eq!(operation.phase, None);
        assert_eq!(operation.completed_units, None);
        assert_eq!(operation.total_units, None);
        assert_eq!(operation.cancellable, None);
        assert_eq!(operation.status, OperationStatus::Running);
    }

    #[test]
    fn progress_reports_move_only_the_dimensions_they_carry() {
        let mut operation = OperationTask::start(
            "op-fixed-6".to_string(),
            OperationKind::Cli,
            Some("claude-code".to_string()),
            None,
            "700".to_string(),
        );

        operation.report_progress(
            OperationProgress::phase("querying-catalog").with_cancellable(true),
            "701".to_string(),
        );
        assert_eq!(operation.phase.as_deref(), Some("querying-catalog"));
        assert_eq!(operation.cancellable, Some(true));

        // A later report that only carries units must not erase the phase the previous one set.
        operation.report_progress(
            OperationProgress::default().with_units(1, 3),
            "702".to_string(),
        );
        assert_eq!(operation.phase.as_deref(), Some("querying-catalog"));
        assert_eq!(operation.cancellable, Some(true));
        assert_eq!(operation.completed_units, Some(1));
        assert_eq!(operation.total_units, Some(3));

        // Status stays authoritative; a phase is descriptive only.
        assert_eq!(operation.status, OperationStatus::Running);
        assert_eq!(operation.updated_at, "702");
    }

    #[test]
    fn execution_correlation_does_not_replace_operation_identity() {
        let mut operation = OperationTask::start(
            "op-fixed-4".to_string(),
            OperationKind::Agent,
            Some("session-1".to_string()),
            None,
            "400".to_string(),
        );

        operation.correlate_execution(
            "018f0f17-4d6a-7e20-b41d-66c5271a28d0".to_string(),
            "4bf92f3577b34da6a3ce929d0e0e4736".to_string(),
        );

        assert_eq!(operation.id, "op-fixed-4");
        assert_eq!(
            operation.execution_run_id.as_deref(),
            Some("018f0f17-4d6a-7e20-b41d-66c5271a28d0")
        );
        assert_eq!(
            operation.trace_id.as_deref(),
            Some("4bf92f3577b34da6a3ce929d0e0e4736")
        );
    }
}
