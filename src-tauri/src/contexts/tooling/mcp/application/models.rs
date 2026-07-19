use crate::contexts::tooling::mcp::domain::{
    ConnectionOutcome, Scope, ServerConfiguration, ToolDescriptor, TransportType,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ServerPatch {
    pub(crate) name: Option<String>,
    pub(crate) transport_type: Option<TransportType>,
    pub(crate) command: Option<Option<String>>,
    pub(crate) args: Option<Option<Vec<String>>>,
    pub(crate) env: Option<Option<BTreeMap<String, String>>>,
    pub(crate) url: Option<Option<String>>,
    pub(crate) headers: Option<Option<BTreeMap<String, String>>>,
    pub(crate) description: Option<Option<String>>,
    pub(crate) active: Option<bool>,
    pub(crate) scope: Option<Scope>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ImportEntry {
    pub(crate) command: Option<String>,
    pub(crate) args: Option<Vec<String>>,
    pub(crate) env: Option<BTreeMap<String, String>>,
    pub(crate) url: Option<String>,
    pub(crate) headers: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ImportBundle {
    pub(crate) servers: BTreeMap<String, ImportEntry>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ImportResult {
    pub(crate) imported: Vec<String>,
    pub(crate) skipped: Vec<String>,
}

pub(crate) type ExportBundle = ImportBundle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartedOperation {
    pub(crate) id: String,
    pub(crate) related_entity_id: Option<String>,
    pub(crate) message: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedConnectionTest {
    pub(crate) operation: StartedOperation,
    pub(super) server: ServerConfiguration,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ConnectionTestResult {
    pub(crate) success: bool,
    pub(crate) operation_id: String,
    pub(crate) tools: Vec<ToolDescriptor>,
    pub(crate) error: Option<String>,
    pub(crate) duration_ms: u64,
}

impl ConnectionTestResult {
    pub(super) fn from_outcome(operation_id: String, outcome: &ConnectionOutcome) -> Self {
        Self {
            success: outcome.is_success(),
            operation_id,
            tools: outcome.tools().to_vec(),
            error: outcome.error().map(str::to_string),
            duration_ms: outcome.duration_ms(),
        }
    }
}
