use crate::contexts::communications::domain::{
    classify_safe_code, ConnectorErrorClass, ConnectorKind, NormalizedInbound, OutboundText,
};
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch};

const IMMEDIATE_EMPTY_POLL_THRESHOLD: std::time::Duration = std::time::Duration::from_millis(100);
const EMPTY_POLL_BACKOFF: std::time::Duration = std::time::Duration::from_millis(200);

pub(crate) type SafeDiagnosticSink =
    Arc<dyn Fn(ConnectorKind, &'static str) + Send + Sync + 'static>;

pub(crate) struct MalformedEventReporter {
    kind: ConnectorKind,
    emitted: AtomicBool,
    sink: Option<SafeDiagnosticSink>,
}

impl MalformedEventReporter {
    pub(crate) fn new(kind: ConnectorKind) -> Self {
        Self {
            kind,
            emitted: AtomicBool::new(false),
            sink: None,
        }
    }

    pub(crate) fn with_sink(mut self, sink: SafeDiagnosticSink) -> Self {
        self.sink = Some(sink);
        self
    }

    pub(crate) fn report(&self) {
        if self.emitted.swap(true, Ordering::AcqRel) {
            return;
        }
        if let Some(sink) = &self.sink {
            sink(self.kind, "malformed-event");
        }
    }
}

pub(super) async fn pace_immediate_empty_poll(
    started: std::time::Instant,
    empty: bool,
    shutdown: &mut watch::Receiver<bool>,
) -> bool {
    if !empty || started.elapsed() >= IMMEDIATE_EMPTY_POLL_THRESHOLD {
        return false;
    }
    tokio::select! {
        _ = tokio::time::sleep(EMPTY_POLL_BACKOFF) => false,
        _ = shutdown.changed() => true,
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[tokio::test]
    async fn immediate_empty_poll_is_paced_and_shutdown_interrupts_the_delay() {
        let (_stop, mut shutdown) = watch::channel(false);
        let started = std::time::Instant::now();
        assert!(!pace_immediate_empty_poll(started, false, &mut shutdown).await);

        let (stop, mut shutdown) = watch::channel(false);
        stop.send(true).expect("shutdown");
        assert!(pace_immediate_empty_poll(std::time::Instant::now(), true, &mut shutdown).await);
    }

    #[test]
    fn malformed_diagnostics_are_redacted_and_bounded_per_adapter() {
        let calls = Arc::new(AtomicUsize::new(0));
        let captured = Arc::clone(&calls);
        let reporter = MalformedEventReporter::new(ConnectorKind::Telegram).with_sink(Arc::new(
            move |kind, safe_code| {
                assert_eq!(kind, ConnectorKind::Telegram);
                assert_eq!(safe_code, "malformed-event");
                captured.fetch_add(1, Ordering::AcqRel);
            },
        ));
        reporter.report();
        reporter.report();
        assert_eq!(calls.load(Ordering::Acquire), 1);
    }
}
