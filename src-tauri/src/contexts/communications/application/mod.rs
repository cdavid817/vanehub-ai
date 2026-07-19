mod error;
mod models;
mod ports;
mod service;

pub(crate) use error::CommunicationsApplicationError;
pub(crate) use models::{
    AgentExecutionRequest, AgentExecutionResult, CommunicationsLog, CommunicationsLogLevel,
    CommunicationsOperation, ConnectorCredential, ConnectorRuntimeDefinition,
    ConnectorStartupResult, ConnectorSummary, InboundRouteOutcome, SaveConnectorRequest,
};
pub(crate) use ports::{
    CommunicationsAgentExecutionPort, CommunicationsClockPort, CommunicationsCredentialPort,
    CommunicationsLoggingPort, CommunicationsOperationPort, CommunicationsRepository,
    CommunicationsSessionBindingPort, CommunicationsTransportPort,
};
pub(crate) use service::{CommunicationsApplicationPorts, CommunicationsApplicationService};

#[cfg(test)]
mod tests;
