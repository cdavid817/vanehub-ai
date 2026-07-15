use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OperationKind {
    Sdk,
    Mcp,
    Agent,
    Workspace,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
