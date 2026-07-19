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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OperationStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationLogEntry {
    pub operation_id: String,
    pub line: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OperationTask {
    pub id: String,
    pub kind: OperationKind,
    pub status: OperationStatus,
    pub related_entity_id: Option<String>,
    pub message: Option<String>,
    pub logs: Vec<OperationLogEntry>,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
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
            kind,
            status: OperationStatus::Running,
            related_entity_id,
            message,
            logs: Vec::new(),
            result: None,
            error: None,
            created_at: now.clone(),
            updated_at: now,
        }
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
}
