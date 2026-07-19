use super::application_adapters::CommunicationsLoggingAdapter;
use super::runtime_manager::{
    ConnectorDiagnostic, ConnectorRuntimeError, InboundAgent, InboundOutcome,
};
use crate::contexts::communications::api::{CommunicationsApi, InboundRouteOutcome};
use crate::contexts::communications::application::CommunicationsApplicationError;
use crate::contexts::communications::domain::NormalizedInbound;
use async_trait::async_trait;
use std::sync::{Arc, OnceLock};

pub(crate) type BusyMessageProvider = Arc<dyn Fn() -> String + Send + Sync>;

pub(crate) struct CommunicationsInboundBridge {
    api: OnceLock<CommunicationsApi>,
    logging: Arc<CommunicationsLoggingAdapter>,
    busy_message: BusyMessageProvider,
}

impl CommunicationsInboundBridge {
    pub(crate) fn new(
        logging: Arc<CommunicationsLoggingAdapter>,
        busy_message: BusyMessageProvider,
    ) -> Self {
        Self {
            api: OnceLock::new(),
            logging,
            busy_message,
        }
    }

    pub(crate) fn attach(
        &self,
        api: CommunicationsApi,
    ) -> Result<(), CommunicationsApplicationError> {
        self.api
            .set(api)
            .map_err(|_| CommunicationsApplicationError::failure("communications-api-attached"))
    }

    fn api(&self) -> Result<CommunicationsApi, ConnectorRuntimeError> {
        self.api
            .get()
            .cloned()
            .ok_or_else(|| ConnectorRuntimeError::new("communications-api-unavailable"))
    }
}

#[async_trait]
impl InboundAgent for CommunicationsInboundBridge {
    async fn claim(&self, inbound: &NormalizedInbound) -> Result<bool, ConnectorRuntimeError> {
        let api = self.api()?;
        let connector = inbound.connector;
        let event_id = inbound.event_id.clone();
        tokio::task::spawn_blocking(move || api.claim_inbound(connector, &event_id))
            .await
            .map_err(|_| ConnectorRuntimeError::new("dedup-task-join-failed"))?
            .map_err(application_error)
    }

    async fn handle(
        &self,
        inbound: NormalizedInbound,
    ) -> Result<InboundOutcome, ConnectorRuntimeError> {
        let api = self.api()?;
        tokio::task::spawn_blocking(move || api.route_inbound(inbound))
            .await
            .map_err(|_| ConnectorRuntimeError::new("agent-task-join-failed"))?
            .map(map_outcome)
            .map_err(application_error)
    }

    fn diagnostic(&self, event: ConnectorDiagnostic) {
        self.logging.record_runtime(event);
    }

    fn busy_message(&self) -> String {
        (self.busy_message)()
    }
}

fn map_outcome(outcome: InboundRouteOutcome) -> InboundOutcome {
    match outcome {
        InboundRouteOutcome::Reply {
            text,
            session_id,
            message_id,
        } => InboundOutcome::Reply {
            text,
            session_id,
            message_id,
        },
        InboundRouteOutcome::Ignored => InboundOutcome::Ignored,
    }
}

fn application_error(error: CommunicationsApplicationError) -> ConnectorRuntimeError {
    match error.user_message() {
        Some(message) => ConnectorRuntimeError::user_visible(error.safe_code(), message),
        None => ConnectorRuntimeError::new(error.safe_code()),
    }
}
