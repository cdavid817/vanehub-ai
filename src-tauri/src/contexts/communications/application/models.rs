use crate::contexts::communications::domain::{
    ConnectorConfig, ConnectorDescriptor, ConnectorHealth, ConnectorKind, SessionBinding,
    SessionConnectorAccess,
};
use std::collections::BTreeMap;
use zeroize::Zeroizing;

#[derive(Clone)]
pub(crate) struct SaveConnectorRequest {
    pub(crate) kind: ConnectorKind,
    pub(crate) enabled: bool,
    pub(crate) display_name: Option<String>,
    pub(crate) public_config: serde_json::Value,
    pub(crate) credential_patch: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectorSummary {
    pub(crate) descriptor: ConnectorDescriptor,
    pub(crate) configuration: ConnectorConfig,
    pub(crate) health: ConnectorHealth,
    pub(crate) has_credentials: bool,
}

#[derive(Clone)]
pub(crate) struct ConnectorCredential {
    pub(crate) reference: String,
    pub(crate) secret: Zeroizing<String>,
}

#[derive(Clone)]
pub(crate) struct ConnectorRuntimeDefinition {
    pub(crate) configuration: ConnectorConfig,
    pub(crate) secret: Zeroizing<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommunicationsOperation {
    pub(crate) id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommunicationsLogLevel {
    Info,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommunicationsLog {
    pub(crate) level: CommunicationsLogLevel,
    pub(crate) event: &'static str,
    pub(crate) message: String,
    pub(crate) connector: Option<ConnectorKind>,
    pub(crate) safe_code: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) timestamp: String,
}

#[derive(Clone)]
pub(crate) struct AgentExecutionRequest {
    pub(crate) connector: ConnectorKind,
    pub(crate) session_id: String,
    pub(crate) text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentExecutionResult {
    pub(crate) reply: String,
    pub(crate) message_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InboundRouteOutcome {
    Reply {
        text: String,
        session_id: String,
        message_id: String,
    },
    SystemReply {
        text: String,
    },
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PairingStartResult {
    pub(crate) connector: ConnectorKind,
    pub(crate) session_id: String,
    pub(crate) code: String,
    pub(crate) expires_at: String,
    pub(crate) replace_existing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionBindingSnapshot {
    pub(crate) binding: Option<SessionBinding>,
    pub(crate) pending_connector: Option<ConnectorKind>,
    pub(crate) access: SessionConnectorAccess,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentExecutionOutcome {
    Reply(AgentExecutionResult),
    InvalidSeat { valid_mentions: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectorStartupResult {
    pub(crate) kind: ConnectorKind,
    pub(crate) safe_error_code: Option<String>,
}
