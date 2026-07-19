use super::{
    AgentExecutionRequest, AgentExecutionResult, CommunicationsApplicationError, CommunicationsLog,
    CommunicationsOperation, ConnectorCredential, ConnectorRuntimeDefinition,
};
use crate::contexts::communications::domain::{
    ChatBindingKey, CheckpointKey, ConnectorCheckpoint, ConnectorConfig, ConnectorHealth,
    ConnectorKind, InboundEventIdentity, RoutingSettings,
};
use async_trait::async_trait;

pub(crate) trait CommunicationsRepository: Send + Sync {
    fn list_configurations(&self) -> Result<Vec<ConnectorConfig>, CommunicationsApplicationError>;

    fn find_configuration(
        &self,
        kind: ConnectorKind,
    ) -> Result<Option<ConnectorConfig>, CommunicationsApplicationError>;

    fn save_configuration(
        &self,
        configuration: &ConnectorConfig,
        updated_at: &str,
    ) -> Result<(), CommunicationsApplicationError>;

    fn load_routing(&self) -> Result<Option<RoutingSettings>, CommunicationsApplicationError>;

    fn save_routing(
        &self,
        routing: &RoutingSettings,
        updated_at: &str,
    ) -> Result<(), CommunicationsApplicationError>;

    fn claim_event(
        &self,
        event: &InboundEventIdentity,
        received_at: &str,
    ) -> Result<bool, CommunicationsApplicationError>;

    fn cleanup_dedup_before(&self, cutoff: &str) -> Result<usize, CommunicationsApplicationError>;

    fn load_checkpoint(
        &self,
        key: &CheckpointKey,
    ) -> Result<Option<String>, CommunicationsApplicationError>;

    fn save_checkpoint(
        &self,
        checkpoint: &ConnectorCheckpoint,
        updated_at: &str,
    ) -> Result<(), CommunicationsApplicationError>;
}

pub(crate) trait CommunicationsCredentialPort: Send + Sync {
    fn load(
        &self,
        kind: ConnectorKind,
    ) -> Result<Option<ConnectorCredential>, CommunicationsApplicationError>;

    fn store(
        &self,
        kind: ConnectorKind,
        secret: &str,
    ) -> Result<ConnectorCredential, CommunicationsApplicationError>;

    fn delete(&self, kind: ConnectorKind) -> Result<(), CommunicationsApplicationError>;
}

#[async_trait]
pub(crate) trait CommunicationsTransportPort: Send + Sync {
    async fn health(&self) -> Vec<ConnectorHealth>;

    async fn start(
        &self,
        definition: ConnectorRuntimeDefinition,
    ) -> Result<(), CommunicationsApplicationError>;

    async fn stop(&self, kind: ConnectorKind) -> Result<(), CommunicationsApplicationError>;

    async fn test(
        &self,
        definition: ConnectorRuntimeDefinition,
    ) -> Result<(), CommunicationsApplicationError>;

    async fn shutdown(&self) -> Result<(), CommunicationsApplicationError>;
}

pub(crate) trait CommunicationsAgentExecutionPort: Send + Sync {
    fn validate_routing(
        &self,
        routing: &RoutingSettings,
    ) -> Result<RoutingSettings, CommunicationsApplicationError>;

    fn execute(
        &self,
        request: AgentExecutionRequest,
    ) -> Result<AgentExecutionResult, CommunicationsApplicationError>;
}

pub(crate) trait CommunicationsSessionBindingPort: Send + Sync {
    fn resolve_or_create(
        &self,
        key: &ChatBindingKey,
        routing: &RoutingSettings,
    ) -> Result<String, CommunicationsApplicationError>;

    fn reset(&self, kind: Option<ConnectorKind>) -> Result<(), CommunicationsApplicationError>;
}

pub(crate) trait CommunicationsOperationPort: Send + Sync {
    fn start(
        &self,
        kind: ConnectorKind,
        action: &'static str,
    ) -> Result<CommunicationsOperation, CommunicationsApplicationError>;

    fn complete(&self, operation_id: &str) -> Result<(), CommunicationsApplicationError>;

    fn fail(
        &self,
        operation_id: &str,
        safe_code: &str,
    ) -> Result<(), CommunicationsApplicationError>;
}

pub(crate) trait CommunicationsClockPort: Send + Sync {
    fn now_rfc3339(&self) -> String;

    fn days_ago_rfc3339(&self, days: u32) -> String;
}

pub(crate) trait CommunicationsLoggingPort: Send + Sync {
    fn record(&self, log: CommunicationsLog) -> Result<(), CommunicationsApplicationError>;
}
