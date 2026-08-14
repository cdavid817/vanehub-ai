use super::{
    AgentExecutionRequest, AgentExecutionResult, CommunicationsApplicationError, CommunicationsLog,
    CommunicationsOperation, ConnectorCredential, ConnectorRuntimeDefinition,
};
use crate::contexts::communications::domain::{
    BindingState, ChatBindingKey, CheckpointKey, ConnectorCheckpoint, ConnectorConfig,
    ConnectorHealth, ConnectorKind, InboundEventIdentity, PairingIntent, RoutingSettings,
    SessionBinding,
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

    fn delete_configuration(
        &self,
        kind: ConnectorKind,
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

    fn cleanup_dedup_before(
        &self,
        cutoff: &str,
        limit: usize,
    ) -> Result<usize, CommunicationsApplicationError>;

    fn load_checkpoint(
        &self,
        key: &CheckpointKey,
    ) -> Result<Option<String>, CommunicationsApplicationError>;

    fn save_checkpoint(
        &self,
        checkpoint: &ConnectorCheckpoint,
        updated_at: &str,
    ) -> Result<(), CommunicationsApplicationError>;

    fn binding_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionBinding>, CommunicationsApplicationError>;

    fn binding_for_chat(
        &self,
        key: &ChatBindingKey,
    ) -> Result<Option<SessionBinding>, CommunicationsApplicationError>;

    fn save_pairing_intent(
        &self,
        intent: &PairingIntent,
    ) -> Result<(), CommunicationsApplicationError>;

    fn pairing_intents(
        &self,
        connector: ConnectorKind,
        now: &str,
    ) -> Result<Vec<PairingIntent>, CommunicationsApplicationError>;

    fn consume_pairing_intent(
        &self,
        intent_id: &str,
        key: &ChatBindingKey,
        now: &str,
        replace: bool,
        delivery_credential_ref: &str,
    ) -> Result<SessionBinding, CommunicationsApplicationError>;

    fn binding_delivery_reference(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, CommunicationsApplicationError>;

    fn replacement_delivery_references(
        &self,
        session_id: &str,
        key: &ChatBindingKey,
    ) -> Result<Vec<String>, CommunicationsApplicationError>;

    fn cancel_pairing(
        &self,
        session_id: &str,
        connector: ConnectorKind,
    ) -> Result<bool, CommunicationsApplicationError>;

    fn set_binding_state(
        &self,
        session_id: &str,
        state: BindingState,
        updated_at: &str,
    ) -> Result<SessionBinding, CommunicationsApplicationError>;

    fn set_completion_notifications(
        &self,
        session_id: &str,
        enabled: bool,
        updated_at: &str,
    ) -> Result<SessionBinding, CommunicationsApplicationError>;

    fn remove_session_binding(
        &self,
        session_id: &str,
    ) -> Result<bool, CommunicationsApplicationError>;

    fn claim_notification_delivery(
        &self,
        message_id: &str,
        session_id: &str,
        connector: ConnectorKind,
        delivered_at: &str,
    ) -> Result<bool, CommunicationsApplicationError>;

    fn release_notification_delivery(
        &self,
        message_id: &str,
        session_id: &str,
        connector: ConnectorKind,
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

    fn store_delivery_handle(
        &self,
        kind: ConnectorKind,
        binding_id: &str,
        handle: &str,
    ) -> Result<String, CommunicationsApplicationError>;

    fn load_delivery_handle(
        &self,
        reference: &str,
    ) -> Result<Option<zeroize::Zeroizing<String>>, CommunicationsApplicationError>;

    fn delete_delivery_handle(&self, reference: &str)
        -> Result<(), CommunicationsApplicationError>;
}

#[async_trait]
pub(crate) trait CommunicationsTransportPort: Send + Sync {
    async fn health(&self) -> Vec<ConnectorHealth>;

    async fn replace_and_start(
        &self,
        definition: ConnectorRuntimeDefinition,
    ) -> Result<(), CommunicationsApplicationError>;

    async fn stop(&self, kind: ConnectorKind) -> Result<(), CommunicationsApplicationError>;

    async fn clear_connector_data(
        &self,
        kind: ConnectorKind,
    ) -> Result<(), CommunicationsApplicationError>;

    async fn test(
        &self,
        definition: ConnectorRuntimeDefinition,
    ) -> Result<(), CommunicationsApplicationError>;

    async fn shutdown(&self) -> Result<(), CommunicationsApplicationError>;

    async fn send_notification(
        &self,
        kind: ConnectorKind,
        chat_id: &str,
        text: &str,
    ) -> Result<(), CommunicationsApplicationError>;
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
    fn reset(&self, kind: Option<ConnectorKind>) -> Result<(), CommunicationsApplicationError>;

    fn exists(&self, session_id: &str) -> Result<bool, CommunicationsApplicationError>;
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
