use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum McpTransportType {
    Stdio,
    Sse,
    StreamableHttp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum McpScope {
    User,
    Project,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum McpConnectionStatus {
    Connected,
    Disconnected,
    Error,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpServerConfig {
    pub(crate) name: String,
    pub(crate) transport_type: McpTransportType,
    pub(crate) command: Option<String>,
    pub(crate) args: Option<Vec<String>>,
    pub(crate) env: Option<BTreeMap<String, String>>,
    pub(crate) url: Option<String>,
    pub(crate) headers: Option<BTreeMap<String, String>>,
    pub(crate) description: Option<String>,
    pub(crate) active: bool,
    pub(crate) scope: McpScope,
    pub(crate) project_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PartialMcpServerConfig {
    pub(crate) name: Option<String>,
    pub(crate) transport_type: Option<McpTransportType>,
    pub(crate) command: Option<Option<String>>,
    pub(crate) args: Option<Option<Vec<String>>>,
    pub(crate) env: Option<Option<BTreeMap<String, String>>>,
    pub(crate) url: Option<Option<String>>,
    pub(crate) headers: Option<Option<BTreeMap<String, String>>>,
    pub(crate) description: Option<Option<String>>,
    pub(crate) active: Option<bool>,
    pub(crate) scope: Option<McpScope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpToolInfo {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) input_schema: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpServerStatus {
    pub(crate) name: String,
    pub(crate) connection_status: McpConnectionStatus,
    pub(crate) tools: Vec<McpToolInfo>,
    pub(crate) last_connected: Option<String>,
    pub(crate) error: Option<String>,
    pub(crate) duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpImportResult {
    pub(crate) imported: Vec<String>,
    pub(crate) skipped: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpImportExport {
    pub(crate) mcp_servers: BTreeMap<String, McpImportServerEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpImportServerEntry {
    pub(crate) command: Option<String>,
    pub(crate) args: Option<Vec<String>>,
    pub(crate) env: Option<BTreeMap<String, String>>,
    pub(crate) url: Option<String>,
    pub(crate) headers: Option<BTreeMap<String, String>>,
}
