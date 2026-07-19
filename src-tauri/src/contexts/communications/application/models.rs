use crate::contexts::communications::domain::{
    ConnectorConfig, ConnectorDescriptor, ConnectorHealth, ConnectorKind, RoutingSettings,
};
use zeroize::Zeroizing;

#[derive(Clone)]
pub(crate) struct SaveConnectorRequest {
    pub(crate) kind: ConnectorKind,
    pub(crate) enabled: bool,
    pub(crate) display_name: Option<String>,
    pub(crate) public_config: serde_json::Value,
    pub(crate) replacement_secret: Option<String>,
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
    pub(crate) session_id: String,
    pub(crate) text: String,
    pub(crate) routing: RoutingSettings,
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
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectorStartupResult {
    pub(crate) kind: ConnectorKind,
    pub(crate) safe_error_code: Option<String>,
}
