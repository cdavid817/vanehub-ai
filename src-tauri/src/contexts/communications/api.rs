use super::application::CommunicationsApplicationService;
use super::domain::{
    ConnectorConfig, ConnectorKind, NormalizedInbound, RoutingSettings, SessionBinding,
    SessionConnectorAccess,
};
use super::infrastructure::WeChatAuthorizationService;
use std::sync::Arc;

pub(crate) use super::application::{
    CommunicationsApplicationError, ConnectorStartupResult, ConnectorSummary, InboundRouteOutcome,
    PairingStartResult, SaveConnectorRequest, SessionBindingSnapshot,
};

#[derive(Clone)]
pub(crate) struct CommunicationsApi {
    service: CommunicationsApplicationService,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WeChatAuthorizationResult {
    pub(crate) status: String,
    pub(crate) image_data_url: Option<String>,
    pub(crate) expires_at: Option<String>,
    pub(crate) safe_error_code: Option<String>,
}

#[derive(Clone)]
pub(crate) struct WeChatAuthorizationApi {
    service: Arc<WeChatAuthorizationService>,
}

impl WeChatAuthorizationApi {
    pub(crate) fn new(communications: CommunicationsApi) -> Self {
        Self {
            service: Arc::new(WeChatAuthorizationService::new(communications)),
        }
    }

    pub(crate) async fn begin(
        &self,
    ) -> Result<WeChatAuthorizationResult, CommunicationsApplicationError> {
        self.service.begin().await
    }

    pub(crate) async fn poll(
        &self,
    ) -> Result<WeChatAuthorizationResult, CommunicationsApplicationError> {
        self.service.poll().await
    }

    pub(crate) async fn cancel(&self) -> Result<(), CommunicationsApplicationError> {
        self.service.cancel().await
    }
}

impl CommunicationsApi {
    pub(crate) fn new(service: CommunicationsApplicationService) -> Self {
        Self { service }
    }

    pub(crate) async fn list_connectors(
        &self,
    ) -> Result<Vec<ConnectorSummary>, CommunicationsApplicationError> {
        // `connector_snapshot` runs synchronous rusqlite + credential I/O; running it inline
        // blocks the async executor for every concurrent command sharing this worker.
        // Capture the snapshot on the blocking pool, await transport health here, then fold
        // them together (allocation-only) back on the pool.
        let snapshot_service = self.service.clone();
        let snapshot =
            tauri::async_runtime::spawn_blocking(move || snapshot_service.connector_snapshot())
                .await
                .map_err(|_| {
                    CommunicationsApplicationError::failure("connector-snapshot-task-failed")
                })??;
        let health = self
            .service
            .transport_health()
            .await
            .into_iter()
            .map(|health| (health.kind, health))
            .collect::<std::collections::HashMap<_, _>>();
        let assemble_service = self.service.clone();
        let summaries = tauri::async_runtime::spawn_blocking(move || {
            assemble_service.assemble_connectors(snapshot, health)
        })
        .await
        .map_err(|_| CommunicationsApplicationError::failure("connector-assemble-task-failed"))?;
        Ok(summaries)
    }

    pub(crate) fn routing(
        &self,
    ) -> Result<Option<RoutingSettings>, CommunicationsApplicationError> {
        self.service.routing()
    }

    pub(crate) fn save_routing(
        &self,
        routing: &RoutingSettings,
    ) -> Result<RoutingSettings, CommunicationsApplicationError> {
        self.service.save_routing(routing)
    }

    pub(crate) async fn save_connector(
        &self,
        request: SaveConnectorRequest,
    ) -> Result<ConnectorConfig, CommunicationsApplicationError> {
        self.service.save_connector(request).await
    }

    pub(crate) async fn set_connector_enabled(
        &self,
        kind: ConnectorKind,
        enabled: bool,
    ) -> Result<(), CommunicationsApplicationError> {
        self.service.set_connector_enabled(kind, enabled).await
    }

    pub(crate) async fn clear_connector(
        &self,
        kind: ConnectorKind,
    ) -> Result<(), CommunicationsApplicationError> {
        self.service.clear_connector(kind).await
    }

    pub(crate) async fn test_connector(
        &self,
        kind: ConnectorKind,
    ) -> Result<(), CommunicationsApplicationError> {
        self.service.test_connector(kind).await
    }

    pub(crate) async fn restart_connector(
        &self,
        kind: ConnectorKind,
    ) -> Result<(), CommunicationsApplicationError> {
        self.service.restart_connector(kind).await
    }

    pub(crate) async fn start_saved_connectors(
        &self,
    ) -> Result<Vec<ConnectorStartupResult>, CommunicationsApplicationError> {
        self.service.start_saved_connectors().await
    }

    pub(crate) async fn shutdown(&self) -> Result<(), CommunicationsApplicationError> {
        self.service.shutdown().await
    }

    pub(crate) fn claim_inbound(
        &self,
        connector: ConnectorKind,
        event_id: &str,
    ) -> Result<bool, CommunicationsApplicationError> {
        self.service.claim_inbound(connector, event_id)
    }

    pub(crate) fn maintain_deduplication(&self) -> Result<usize, CommunicationsApplicationError> {
        self.service.maintain_deduplication()
    }

    pub(crate) fn route_inbound(
        &self,
        inbound: NormalizedInbound,
    ) -> Result<InboundRouteOutcome, CommunicationsApplicationError> {
        self.service.route_inbound(inbound)
    }

    pub(crate) async fn begin_pairing(
        &self,
        session_id: &str,
        connector: ConnectorKind,
        replace_existing: bool,
    ) -> Result<PairingStartResult, CommunicationsApplicationError> {
        self.service
            .begin_pairing(session_id, connector, replace_existing)
            .await
    }

    pub(crate) fn cancel_pairing(
        &self,
        session_id: &str,
        connector: ConnectorKind,
    ) -> Result<bool, CommunicationsApplicationError> {
        self.service.cancel_pairing(session_id, connector)
    }

    pub(crate) fn session_binding(
        &self,
        session_id: &str,
        connector: ConnectorKind,
    ) -> Result<SessionBindingSnapshot, CommunicationsApplicationError> {
        self.service.session_binding(session_id, connector)
    }

    pub(crate) fn set_binding_paused(
        &self,
        session_id: &str,
        paused: bool,
    ) -> Result<SessionBinding, CommunicationsApplicationError> {
        self.service.set_binding_paused(session_id, paused)
    }

    pub(crate) fn set_session_access(
        &self,
        session_id: &str,
        connector: ConnectorKind,
        enabled: bool,
    ) -> Result<SessionConnectorAccess, CommunicationsApplicationError> {
        self.service
            .set_session_access(session_id, connector, enabled)
    }

    pub(crate) fn set_completion_notifications(
        &self,
        session_id: &str,
        enabled: bool,
    ) -> Result<SessionBinding, CommunicationsApplicationError> {
        self.service
            .set_completion_notifications(session_id, enabled)
    }

    pub(crate) fn remove_binding(
        &self,
        session_id: &str,
    ) -> Result<bool, CommunicationsApplicationError> {
        self.service.remove_binding(session_id)
    }

    pub(crate) async fn notify_session_completion(
        &self,
        session_id: &str,
        message_id: &str,
        originated_from_im: bool,
    ) -> Result<bool, CommunicationsApplicationError> {
        self.service
            .notify_session_completion(session_id, message_id, originated_from_im)
            .await
    }

    pub(crate) fn reset_bindings(
        &self,
        kind: Option<ConnectorKind>,
    ) -> Result<(), CommunicationsApplicationError> {
        self.service.reset_bindings(kind)
    }
}
