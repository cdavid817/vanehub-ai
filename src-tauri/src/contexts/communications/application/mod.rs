mod error;
mod lifecycle_coordinator;
mod models;
mod ports;
mod service;

pub(crate) use error::CommunicationsApplicationError;
pub(crate) use models::{
    AgentExecutionOutcome, AgentExecutionRequest, AgentExecutionResult, CommunicationsLog,
    CommunicationsLogLevel, CommunicationsOperation, ConnectorCredential,
    ConnectorRuntimeDefinition, ConnectorStartupResult, ConnectorSummary, InboundRouteOutcome,
    PairingStartResult, SaveConnectorRequest, SessionBindingSnapshot,
};
pub(crate) use ports::{
    CommunicationsAgentExecutionPort, CommunicationsClockPort, CommunicationsCredentialPort,
    CommunicationsLoggingPort, CommunicationsOperationPort, CommunicationsRepository,
    CommunicationsSessionBindingPort, CommunicationsTransportPort,
};
pub(crate) use service::{
    CommunicationsApplicationPorts, CommunicationsApplicationService, CommunicationsCopy,
    CommunicationsCopyProvider,
};

#[cfg(test)]
mod tests;
