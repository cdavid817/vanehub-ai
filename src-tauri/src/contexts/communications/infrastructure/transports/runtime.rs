use crate::contexts::communications::domain::{
    classify_safe_code, ConnectorErrorClass, ConnectorKind, NormalizedInbound, OutboundText,
};
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot, watch};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorRuntimeError {
    pub safe_code: String,
    pub user_message: Option<String>,
    pub class: ConnectorErrorClass,
}

impl ConnectorRuntimeError {
    pub fn new(safe_code: impl Into<String>) -> Self {
        let safe_code = safe_code.into();
        Self {
            class: classify_safe_code(&safe_code),
            safe_code,
            user_message: None,
        }
    }

    pub fn user_visible(safe_code: impl Into<String>, message: impl Into<String>) -> Self {
        let safe_code = safe_code.into();
        Self {
            class: classify_safe_code(&safe_code),
            safe_code,
            user_message: Some(message.into()),
        }
    }
}

pub struct InboundDelivery {
    pub message: NormalizedInbound,
    pub(crate) acceptance: oneshot::Sender<Result<(), ConnectorRuntimeError>>,
}

impl InboundDelivery {
    #[cfg(test)]
    pub fn accept(self) -> NormalizedInbound {
        let _ = self.acceptance.send(Ok(()));
        self.message
    }
}

pub async fn submit_inbound(
    inbound: &mpsc::Sender<InboundDelivery>,
    message: NormalizedInbound,
) -> Result<(), ConnectorRuntimeError> {
    let (acceptance, accepted) = oneshot::channel();
    inbound
        .send(InboundDelivery {
            message,
            acceptance,
        })
        .await
        .map_err(|_| ConnectorRuntimeError::new("inbound-closed"))?;
    accepted
        .await
        .map_err(|_| ConnectorRuntimeError::new("inbound-acceptance-closed"))?
}

#[async_trait]
pub trait ConnectorAdapter: Send + Sync {
    fn kind(&self) -> ConnectorKind;
    fn max_outbound_chars(&self) -> usize;
    async fn test_connection(&self) -> Result<(), ConnectorRuntimeError>;
    async fn run(
        &self,
        inbound: mpsc::Sender<InboundDelivery>,
        shutdown: watch::Receiver<bool>,
        ready: oneshot::Sender<()>,
    ) -> Result<(), ConnectorRuntimeError>;
    async fn send_text(&self, outbound: OutboundText) -> Result<(), ConnectorRuntimeError>;
}
