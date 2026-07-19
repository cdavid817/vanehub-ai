use super::application::CommunicationsApplicationService;
use super::domain::{ConnectorConfig, ConnectorKind, NormalizedInbound, RoutingSettings};
use super::infrastructure::WeChatAuthorizationService;
use std::sync::Arc;

pub(crate) use super::application::{
    CommunicationsApplicationError, ConnectorStartupResult, ConnectorSummary, InboundRouteOutcome,
    SaveConnectorRequest,
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

    pub(crate) fn cancel(&self) -> Result<(), CommunicationsApplicationError> {
        self.service.cancel()
    }
}

impl CommunicationsApi {
    pub(crate) fn new(service: CommunicationsApplicationService) -> Self {
        Self { service }
    }

    pub(crate) async fn list_connectors(
        &self,
    ) -> Result<Vec<ConnectorSummary>, CommunicationsApplicationError> {
        self.service.list_connectors().await
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

    pub(crate) fn route_inbound(
        &self,
        inbound: NormalizedInbound,
    ) -> Result<InboundRouteOutcome, CommunicationsApplicationError> {
        self.service.route_inbound(inbound)
    }

    pub(crate) fn reset_bindings(
        &self,
        kind: Option<ConnectorKind>,
    ) -> Result<(), CommunicationsApplicationError> {
        self.service.reset_bindings(kind)
    }
}
