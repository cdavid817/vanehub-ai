#[cfg(test)]
use super::transports::submit_inbound;
pub(crate) use super::transports::{ConnectorAdapter, ConnectorRuntimeError, InboundDelivery};
use crate::contexts::communications::domain::{
    pending_delivery_admission, safe_platform_status_code, split_text, ConnectorErrorClass,
    ConnectorHealth, ConnectorKind, ConnectorLifecycle, ConnectorStatus, DeduplicationDecision,
    DeliveryAdmission, InboundDisposition, NormalizedInbound, OutboundText,
};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, watch, Mutex as AsyncMutex, RwLock};
use tokio::task::JoinHandle;

const INBOUND_BUFFER: usize = 256;
const STOP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorDiagnostic {
    pub level: DiagnosticLevel,
    pub connector: ConnectorKind,
    pub operation: &'static str,
    pub safe_code: String,
    pub retry_count: u32,
    pub internal_session_id: Option<String>,
    pub internal_message_id: Option<String>,
    pub platform_status_code: Option<String>,
    pub retry_classification: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboundOutcome {
    Reply {
        text: String,
        session_id: String,
        message_id: String,
    },
    Ignored,
}

#[async_trait]
pub trait InboundAgent: Send + Sync {
    async fn claim(&self, _inbound: &NormalizedInbound) -> Result<bool, ConnectorRuntimeError> {
        Ok(true)
    }

    async fn handle(
        &self,
        inbound: NormalizedInbound,
    ) -> Result<InboundOutcome, ConnectorRuntimeError>;

    fn diagnostic(&self, _event: ConnectorDiagnostic) {}

    fn busy_message(&self) -> String {
        "Too many pending messages. Please try again later.".to_string()
    }
}

struct ChatLane {
    serial: AsyncMutex<()>,
    queued: AtomicUsize,
}

impl Default for ChatLane {
    fn default() -> Self {
        Self {
            serial: AsyncMutex::new(()),
            queued: AtomicUsize::new(0),
        }
    }
}

struct QueueReservation {
    lane: Arc<ChatLane>,
}

impl Drop for QueueReservation {
    fn drop(&mut self) {
        self.lane.queued.fetch_sub(1, Ordering::AcqRel);
    }
}

struct WorkerState {
    status: ConnectorStatus,
    updated_at: String,
    shutdown: Option<watch::Sender<bool>>,
    task: Option<JoinHandle<()>>,
}

struct ManagedConnector {
    adapter: Arc<dyn ConnectorAdapter>,
    state: AsyncMutex<WorkerState>,
}

impl ManagedConnector {
    fn new(adapter: Arc<dyn ConnectorAdapter>) -> Self {
        Self {
            adapter,
            state: AsyncMutex::new(WorkerState {
                status: ConnectorStatus::disabled(),
                updated_at: Utc::now().to_rfc3339(),
                shutdown: None,
                task: None,
            }),
        }
    }
}

pub(crate) struct ConnectorRuntimeManager {
    handler: Arc<dyn InboundAgent>,
    connectors: RwLock<HashMap<ConnectorKind, Arc<ManagedConnector>>>,
    lanes: Mutex<HashMap<(ConnectorKind, String), Arc<ChatLane>>>,
    accepting: AtomicBool,
}

impl ConnectorRuntimeManager {
    pub fn new(handler: Arc<dyn InboundAgent>) -> Arc<Self> {
        Arc::new(Self {
            handler,
            connectors: RwLock::new(HashMap::new()),
            lanes: Mutex::new(HashMap::new()),
            accepting: AtomicBool::new(true),
        })
    }

    pub async fn register(&self, adapter: Arc<dyn ConnectorAdapter>) {
        self.connectors
            .write()
            .await
            .insert(adapter.kind(), Arc::new(ManagedConnector::new(adapter)));
    }

    pub async fn health(&self) -> Vec<ConnectorHealth> {
        let connectors = self.connectors.read().await;
        let mut result = Vec::with_capacity(connectors.len());
        for (kind, managed) in connectors.iter() {
            let state = managed.state.lock().await;
            result.push(state.status.health(*kind, state.updated_at.clone()));
        }
        result.sort_by_key(|health| health.kind.as_str());
        result
    }

    pub async fn test_connection(
        &self,
        kind: ConnectorKind,
        timeout: Duration,
    ) -> Result<(), ConnectorRuntimeError> {
        let managed = self.connector(kind).await?;
        tokio::time::timeout(timeout, managed.adapter.test_connection())
            .await
            .map_err(|_| ConnectorRuntimeError::new("connection-timeout"))?
    }

    pub async fn start(self: &Arc<Self>, kind: ConnectorKind) -> Result<(), ConnectorRuntimeError> {
        let managed = self.connector(kind).await?;
        self.stop(kind).await?;

        let (sender, receiver) = mpsc::channel(INBOUND_BUFFER);
        let (shutdown_sender, shutdown_receiver) = watch::channel(false);
        let generation = {
            let mut state = managed.state.lock().await;
            let generation = state
                .status
                .begin_start()
                .map_err(|_| ConnectorRuntimeError::new("connector-state-invalid"))?;
            state.updated_at = Utc::now().to_rfc3339();
            state.shutdown = Some(shutdown_sender);
            generation
        };

        let runtime = Arc::clone(self);
        let worker = Arc::clone(&managed);
        let adapter = Arc::clone(&managed.adapter);
        let processor = tokio::spawn(process_inbound(
            Arc::clone(&runtime),
            Arc::clone(&adapter),
            receiver,
        ));
        let task = tokio::spawn(async move {
            runtime.diagnostic(kind, DiagnosticLevel::Info, "start", "connecting", 0);
            let mut retry_count = 0_u32;
            let result = loop {
                if *shutdown_receiver.borrow() {
                    break Ok(());
                }
                let (ready_sender, ready_receiver) = oneshot::channel();
                let run = adapter.run(sender.clone(), shutdown_receiver.clone(), ready_sender);
                tokio::pin!(run);
                let run_result = tokio::select! {
                    result = &mut run => result,
                    ready = ready_receiver => {
                        match ready {
                            Ok(()) => {
                                retry_count = 0;
                                runtime
                                    .set_lifecycle(
                                        &worker,
                                        generation,
                                        ConnectorLifecycle::Connected,
                                        None,
                                    )
                                    .await;
                                runtime.diagnostic(
                                    kind,
                                    DiagnosticLevel::Info,
                                    "connect",
                                    "connected",
                                    0,
                                );
                                run.await
                            }
                            Err(_) => Err(ConnectorRuntimeError::new("connector-readiness-closed")),
                        }
                    }
                };

                match run_result {
                    Ok(()) => break Ok(()),
                    Err(error) if error.class == ConnectorErrorClass::Transient => {
                        retry_count = retry_count.saturating_add(1);
                        runtime
                            .set_lifecycle(
                                &worker,
                                generation,
                                ConnectorLifecycle::Reconnecting,
                                Some(error.safe_code.clone()),
                            )
                            .await;
                        runtime.diagnostic(
                            kind,
                            DiagnosticLevel::Warn,
                            "reconnect",
                            &error.safe_code,
                            retry_count,
                        );
                        if wait_for_retry(retry_count, shutdown_receiver.clone()).await {
                            break Ok(());
                        }
                    }
                    Err(error) => break Err(error),
                }
            };
            processor.abort();
            let mut state = worker.state.lock().await;
            if !state.status.is_generation(generation) {
                return;
            }
            state.shutdown = None;
            match result {
                Ok(()) => {
                    let _ = state.status.finish(generation);
                    state.updated_at = Utc::now().to_rfc3339();
                }
                Err(error) => {
                    let _ = state
                        .status
                        .fail(generation, error.class, error.safe_code.clone());
                    runtime.diagnostic(
                        kind,
                        DiagnosticLevel::Error,
                        "connect",
                        &error.safe_code,
                        retry_count,
                    );
                    state.updated_at = Utc::now().to_rfc3339();
                }
            }
        });

        managed.state.lock().await.task = Some(task);
        Ok(())
    }

    pub async fn stop(&self, kind: ConnectorKind) -> Result<(), ConnectorRuntimeError> {
        let managed = self.connector(kind).await?;
        let (shutdown, task) = {
            let mut state = managed.state.lock().await;
            let has_worker = state.shutdown.is_some() || state.task.is_some();
            if has_worker {
                state.updated_at = Utc::now().to_rfc3339();
            }
            (state.shutdown.take(), state.task.take())
        };
        if let Some(shutdown) = shutdown {
            let _ = shutdown.send(true);
        }
        if let Some(mut task) = task {
            if tokio::time::timeout(STOP_TIMEOUT, &mut task).await.is_err() {
                task.abort();
                let mut state = managed.state.lock().await;
                state.status.shutdown_timeout();
                state.updated_at = Utc::now().to_rfc3339();
                return Err(ConnectorRuntimeError::new("shutdown-timeout"));
            }
        }
        let mut state = managed.state.lock().await;
        state.status.disable();
        state.updated_at = Utc::now().to_rfc3339();
        self.diagnostic(kind, DiagnosticLevel::Info, "stop", "disabled", 0);
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), ConnectorRuntimeError> {
        self.accepting.store(false, Ordering::Release);
        let kinds = self
            .connectors
            .read()
            .await
            .keys()
            .copied()
            .collect::<Vec<_>>();
        for kind in kinds {
            self.stop(kind).await?;
        }
        Ok(())
    }

    async fn connector(
        &self,
        kind: ConnectorKind,
    ) -> Result<Arc<ManagedConnector>, ConnectorRuntimeError> {
        self.connectors
            .read()
            .await
            .get(&kind)
            .cloned()
            .ok_or_else(|| ConnectorRuntimeError::new("connector-not-registered"))
    }

    async fn set_lifecycle(
        &self,
        managed: &ManagedConnector,
        generation: u64,
        lifecycle: ConnectorLifecycle,
        safe_error_code: Option<String>,
    ) {
        let mut state = managed.state.lock().await;
        let transition = match lifecycle {
            ConnectorLifecycle::Connected => state.status.mark_connected(generation),
            ConnectorLifecycle::Reconnecting => state.status.mark_reconnecting(
                generation,
                safe_error_code.unwrap_or_else(|| "connector-reconnecting".to_string()),
            ),
            _ => return,
        };
        if matches!(transition, Ok(true)) {
            state.updated_at = Utc::now().to_rfc3339();
        }
    }

    fn diagnostic(
        &self,
        connector: ConnectorKind,
        level: DiagnosticLevel,
        operation: &'static str,
        safe_code: &str,
        retry_count: u32,
    ) {
        self.handler.diagnostic(ConnectorDiagnostic {
            level,
            connector,
            operation,
            safe_code: safe_code.to_string(),
            retry_count,
            internal_session_id: None,
            internal_message_id: None,
            platform_status_code: None,
            retry_classification: None,
        });
    }

    async fn accept_inbound(
        self: &Arc<Self>,
        adapter: Arc<dyn ConnectorAdapter>,
        inbound: NormalizedInbound,
    ) -> Result<(), ConnectorRuntimeError> {
        if !self.accepting.load(Ordering::Acquire) {
            return Err(ConnectorRuntimeError::new("runtime-shutting-down"));
        }
        match inbound.disposition() {
            InboundDisposition::Deliver => {}
            InboundDisposition::IgnoreGroupMessage => {
                self.diagnostic(
                    inbound.connector,
                    DiagnosticLevel::Debug,
                    "ignore-inbound",
                    "group-message",
                    0,
                );
                return Ok(());
            }
            InboundDisposition::IgnoreUnsupportedContent => {
                self.diagnostic(
                    inbound.connector,
                    DiagnosticLevel::Debug,
                    "ignore-inbound",
                    "unsupported-content",
                    0,
                );
                return Ok(());
            }
        }
        if DeduplicationDecision::from_claimed(self.handler.claim(&inbound).await?)
            == DeduplicationDecision::IgnoreDuplicate
        {
            self.diagnostic(
                inbound.connector,
                DiagnosticLevel::Debug,
                "ignore-inbound",
                "duplicate-event",
                0,
            );
            return Ok(());
        }
        let lane = {
            let mut lanes = self
                .lanes
                .lock()
                .map_err(|_| ConnectorRuntimeError::new("queue-lock-failed"))?;
            Arc::clone(
                lanes
                    .entry((inbound.connector, inbound.chat_id.clone()))
                    .or_insert_with(|| Arc::new(ChatLane::default())),
            )
        };
        let queued = lane.queued.fetch_add(1, Ordering::AcqRel);
        if pending_delivery_admission(queued) == DeliveryAdmission::Busy {
            lane.queued.fetch_sub(1, Ordering::AcqRel);
            adapter
                .send_text(OutboundText {
                    chat_id: inbound.chat_id,
                    text: self.handler.busy_message(),
                    reply_context: inbound.reply_context,
                })
                .await?;
            self.diagnostic(
                inbound.connector,
                DiagnosticLevel::Warn,
                "queue-inbound",
                "chat-queue-full",
                0,
            );
            return Ok(());
        }
        let reservation = QueueReservation {
            lane: Arc::clone(&lane),
        };
        let runtime = Arc::clone(self);
        tokio::spawn(async move {
            if let Err(error) = runtime
                .dispatch_claimed(adapter, inbound, lane, reservation)
                .await
            {
                runtime.handler.diagnostic(ConnectorDiagnostic {
                    connector: error.connector,
                    level: DiagnosticLevel::Error,
                    operation: "deliver-final",
                    safe_code: error.error.safe_code.clone(),
                    retry_count: 0,
                    internal_session_id: error.session_id,
                    internal_message_id: error.message_id,
                    platform_status_code: safe_platform_status_code(&error.error.safe_code),
                    retry_classification: Some(error.error.class.as_str().to_string()),
                });
            }
        });
        Ok(())
    }

    async fn dispatch_claimed(
        &self,
        adapter: Arc<dyn ConnectorAdapter>,
        inbound: NormalizedInbound,
        lane: Arc<ChatLane>,
        _reservation: QueueReservation,
    ) -> Result<(), DispatchFailure> {
        let connector = inbound.connector;
        let _serial = lane.serial.lock().await;
        let chat_id = inbound.chat_id.clone();
        let reply_context = inbound.reply_context.clone();
        let (response, session_id, message_id) = match self.handler.handle(inbound).await {
            Ok(InboundOutcome::Reply {
                text,
                session_id,
                message_id,
            }) => (text, Some(session_id), Some(message_id)),
            Ok(InboundOutcome::Ignored) => return Ok(()),
            Err(error) => match error.user_message {
                Some(message) => (message, None, None),
                None => {
                    return Err(DispatchFailure {
                        connector,
                        error,
                        session_id: None,
                        message_id: None,
                    })
                }
            },
        };
        for text in split_text(&response, adapter.max_outbound_chars()) {
            adapter
                .send_text(OutboundText {
                    chat_id: chat_id.clone(),
                    text,
                    reply_context: reply_context.clone(),
                })
                .await
                .map_err(|error| DispatchFailure {
                    connector,
                    error,
                    session_id: session_id.clone(),
                    message_id: message_id.clone(),
                })?;
        }
        Ok(())
    }
}

struct DispatchFailure {
    connector: ConnectorKind,
    error: ConnectorRuntimeError,
    session_id: Option<String>,
    message_id: Option<String>,
}

async fn process_inbound(
    runtime: Arc<ConnectorRuntimeManager>,
    adapter: Arc<dyn ConnectorAdapter>,
    mut receiver: mpsc::Receiver<InboundDelivery>,
) {
    while let Some(delivery) = receiver.recv().await {
        let connector = delivery.message.connector;
        let result = runtime
            .accept_inbound(Arc::clone(&adapter), delivery.message)
            .await;
        if let Err(error) = &result {
            runtime.diagnostic(
                connector,
                DiagnosticLevel::Error,
                "accept-inbound",
                &error.safe_code,
                0,
            );
        }
        let _ = delivery.acceptance.send(result);
    }
}

async fn wait_for_retry(attempt: u32, mut shutdown: watch::Receiver<bool>) -> bool {
    #[cfg(test)]
    let delay = {
        let _ = attempt;
        Duration::from_millis(10)
    };
    #[cfg(not(test))]
    let delay = {
        let base = 2_u64.saturating_pow(attempt.min(5)).min(60);
        let jitter = u64::from(rand::random::<u16>() % 751);
        Duration::from_secs(base) + Duration::from_millis(jitter)
    };
    tokio::select! {
        _ = tokio::time::sleep(delay) => false,
        _ = shutdown.changed() => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::AtomicUsize;

    struct FakeAgent {
        seen: AsyncMutex<HashSet<(ConnectorKind, String)>>,
        bindings: AsyncMutex<HashMap<(ConnectorKind, String), String>>,
        diagnostics: Mutex<Vec<ConnectorDiagnostic>>,
    }

    #[async_trait]
    impl InboundAgent for FakeAgent {
        async fn claim(&self, inbound: &NormalizedInbound) -> Result<bool, ConnectorRuntimeError> {
            Ok(self
                .seen
                .lock()
                .await
                .insert((inbound.connector, inbound.event_id.clone())))
        }

        async fn handle(
            &self,
            inbound: NormalizedInbound,
        ) -> Result<InboundOutcome, ConnectorRuntimeError> {
            let binding_key = (inbound.connector, inbound.chat_id.clone());
            self.bindings
                .lock()
                .await
                .entry(binding_key)
                .or_insert_with(|| format!("session-{}", inbound.connector.as_str()));
            Ok(InboundOutcome::Reply {
                text: format!("final:{}:{}", inbound.connector.as_str(), inbound.text),
                session_id: format!("session-{}", inbound.connector.as_str()),
                message_id: format!("message-{}", inbound.event_id),
            })
        }

        fn diagnostic(&self, event: ConnectorDiagnostic) {
            self.diagnostics.lock().unwrap().push(event);
        }
    }

    struct FakeAdapter {
        kind: ConnectorKind,
        sent: Arc<AsyncMutex<Vec<OutboundText>>>,
    }

    struct AuthenticationFailingAdapter {
        attempts: AtomicUsize,
    }

    struct RecoveringAdapter {
        attempts: AtomicUsize,
    }

    struct AuthorizationExpiredAdapter {
        attempts: AtomicUsize,
    }

    struct BlockingAgent {
        diagnostics: Mutex<Vec<ConnectorDiagnostic>>,
    }

    struct CountingAgent {
        handles: AtomicUsize,
        diagnostics: Mutex<Vec<ConnectorDiagnostic>>,
    }

    struct FailingSendAdapter {
        sends: AtomicUsize,
    }

    #[async_trait]
    impl InboundAgent for BlockingAgent {
        async fn handle(
            &self,
            _inbound: NormalizedInbound,
        ) -> Result<InboundOutcome, ConnectorRuntimeError> {
            std::future::pending().await
        }

        fn diagnostic(&self, event: ConnectorDiagnostic) {
            self.diagnostics.lock().unwrap().push(event);
        }

        fn busy_message(&self) -> String {
            "queue busy".to_string()
        }
    }

    #[async_trait]
    impl InboundAgent for CountingAgent {
        async fn handle(
            &self,
            _inbound: NormalizedInbound,
        ) -> Result<InboundOutcome, ConnectorRuntimeError> {
            self.handles.fetch_add(1, Ordering::AcqRel);
            Ok(InboundOutcome::Reply {
                text: "final".to_string(),
                session_id: "internal-session".to_string(),
                message_id: "internal-message".to_string(),
            })
        }

        fn diagnostic(&self, event: ConnectorDiagnostic) {
            self.diagnostics.lock().unwrap().push(event);
        }
    }

    #[async_trait]
    impl ConnectorAdapter for RecoveringAdapter {
        fn kind(&self) -> ConnectorKind {
            ConnectorKind::Telegram
        }

        fn max_outbound_chars(&self) -> usize {
            4_096
        }

        async fn test_connection(&self) -> Result<(), ConnectorRuntimeError> {
            Ok(())
        }

        async fn run(
            &self,
            _inbound: mpsc::Sender<InboundDelivery>,
            mut shutdown: watch::Receiver<bool>,
            ready: oneshot::Sender<()>,
        ) -> Result<(), ConnectorRuntimeError> {
            if self.attempts.fetch_add(1, Ordering::AcqRel) == 0 {
                return Err(ConnectorRuntimeError::new("telegram-http-503"));
            }
            let _ = ready.send(());
            let _ = shutdown.changed().await;
            Ok(())
        }

        async fn send_text(&self, _outbound: OutboundText) -> Result<(), ConnectorRuntimeError> {
            Ok(())
        }
    }

    #[async_trait]
    impl ConnectorAdapter for AuthorizationExpiredAdapter {
        fn kind(&self) -> ConnectorKind {
            ConnectorKind::WeChat
        }

        fn max_outbound_chars(&self) -> usize {
            2_000
        }

        async fn test_connection(&self) -> Result<(), ConnectorRuntimeError> {
            Err(ConnectorRuntimeError::new("wechat-authorization-expired"))
        }

        async fn run(
            &self,
            _inbound: mpsc::Sender<InboundDelivery>,
            _shutdown: watch::Receiver<bool>,
            _ready: oneshot::Sender<()>,
        ) -> Result<(), ConnectorRuntimeError> {
            self.attempts.fetch_add(1, Ordering::AcqRel);
            Err(ConnectorRuntimeError::new("wechat-authorization-expired"))
        }

        async fn send_text(&self, _outbound: OutboundText) -> Result<(), ConnectorRuntimeError> {
            Ok(())
        }
    }

    #[async_trait]
    impl ConnectorAdapter for FailingSendAdapter {
        fn kind(&self) -> ConnectorKind {
            ConnectorKind::Telegram
        }

        fn max_outbound_chars(&self) -> usize {
            4_096
        }

        async fn test_connection(&self) -> Result<(), ConnectorRuntimeError> {
            Ok(())
        }

        async fn run(
            &self,
            _inbound: mpsc::Sender<InboundDelivery>,
            _shutdown: watch::Receiver<bool>,
            _ready: oneshot::Sender<()>,
        ) -> Result<(), ConnectorRuntimeError> {
            Ok(())
        }

        async fn send_text(&self, _outbound: OutboundText) -> Result<(), ConnectorRuntimeError> {
            self.sends.fetch_add(1, Ordering::AcqRel);
            Err(ConnectorRuntimeError::new("telegram-api-429"))
        }
    }

    fn inbound(event_id: &str) -> NormalizedInbound {
        NormalizedInbound {
            connector: ConnectorKind::Telegram,
            event_id: event_id.to_string(),
            chat_id: "same-chat".to_string(),
            sender_id: "external-sender".to_string(),
            text: "status".to_string(),
            direct: true,
            reply_context: None,
        }
    }

    #[async_trait]
    impl ConnectorAdapter for AuthenticationFailingAdapter {
        fn kind(&self) -> ConnectorKind {
            ConnectorKind::Telegram
        }

        fn max_outbound_chars(&self) -> usize {
            4_096
        }

        async fn test_connection(&self) -> Result<(), ConnectorRuntimeError> {
            Err(ConnectorRuntimeError::new("telegram-api-401"))
        }

        async fn run(
            &self,
            _inbound: mpsc::Sender<InboundDelivery>,
            _shutdown: watch::Receiver<bool>,
            _ready: oneshot::Sender<()>,
        ) -> Result<(), ConnectorRuntimeError> {
            self.attempts.fetch_add(1, Ordering::AcqRel);
            Err(ConnectorRuntimeError::new("telegram-api-401"))
        }

        async fn send_text(&self, _outbound: OutboundText) -> Result<(), ConnectorRuntimeError> {
            Ok(())
        }
    }

    #[async_trait]
    impl ConnectorAdapter for FakeAdapter {
        fn kind(&self) -> ConnectorKind {
            self.kind
        }

        fn max_outbound_chars(&self) -> usize {
            2_000
        }

        async fn test_connection(&self) -> Result<(), ConnectorRuntimeError> {
            Ok(())
        }

        async fn run(
            &self,
            inbound: mpsc::Sender<InboundDelivery>,
            mut shutdown: watch::Receiver<bool>,
            ready: oneshot::Sender<()>,
        ) -> Result<(), ConnectorRuntimeError> {
            let _ = ready.send(());
            let event = NormalizedInbound {
                connector: self.kind,
                event_id: format!("event-{}", self.kind.as_str()),
                chat_id: format!("chat-{}", self.kind.as_str()),
                sender_id: "sender-redacted".to_string(),
                text: "status please".to_string(),
                direct: true,
                reply_context: Some("reply-context".to_string()),
            };
            submit_inbound(&inbound, event.clone()).await?;
            submit_inbound(&inbound, event).await?;
            while !*shutdown.borrow() {
                if shutdown.changed().await.is_err() {
                    break;
                }
            }
            Ok(())
        }

        async fn send_text(&self, outbound: OutboundText) -> Result<(), ConnectorRuntimeError> {
            self.sent.lock().await.push(outbound);
            Ok(())
        }
    }

    #[tokio::test]
    async fn runs_all_five_connectors_through_dedup_binding_and_final_delivery() {
        let agent = Arc::new(FakeAgent {
            seen: AsyncMutex::new(HashSet::new()),
            bindings: AsyncMutex::new(HashMap::new()),
            diagnostics: Mutex::new(Vec::new()),
        });
        let runtime = ConnectorRuntimeManager::new(agent.clone());
        let sent = Arc::new(AsyncMutex::new(Vec::new()));
        for kind in ConnectorKind::ALL {
            runtime
                .register(Arc::new(FakeAdapter {
                    kind,
                    sent: Arc::clone(&sent),
                }))
                .await;
            runtime.start(kind).await.unwrap();
        }

        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if sent.lock().await.len() == 5 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();

        assert_eq!(sent.lock().await.len(), 5);
        assert_eq!(agent.bindings.lock().await.len(), 5);
        assert!(agent
            .diagnostics
            .lock()
            .unwrap()
            .iter()
            .any(
                |event| event.operation == "ignore-inbound" && event.safe_code == "duplicate-event"
            ));
        assert!(runtime
            .health()
            .await
            .iter()
            .all(|health| health.lifecycle == ConnectorLifecycle::Connected));
        runtime.shutdown().await.unwrap();
        assert!(runtime
            .health()
            .await
            .iter()
            .all(|health| health.lifecycle == ConnectorLifecycle::Disabled));
        let error = runtime
            .accept_inbound(
                Arc::new(FakeAdapter {
                    kind: ConnectorKind::Telegram,
                    sent: Arc::clone(&sent),
                }),
                NormalizedInbound {
                    connector: ConnectorKind::Telegram,
                    event_id: "after-shutdown".to_string(),
                    chat_id: "chat-after-shutdown".to_string(),
                    sender_id: "sender-redacted".to_string(),
                    text: "should not run".to_string(),
                    direct: true,
                    reply_context: None,
                },
            )
            .await
            .unwrap_err();
        assert_eq!(error.safe_code, "runtime-shutting-down");
    }

    #[test]
    fn split_text_preserves_unicode_scalar_boundaries_and_order() {
        assert_eq!(split_text("ab你cd", 2), vec!["ab", "你c", "d"]);
    }

    #[test]
    fn classifies_authentication_and_authorization_errors_as_non_retryable() {
        assert_eq!(
            ConnectorRuntimeError::new("telegram-api-401").class,
            ConnectorErrorClass::Authentication
        );
        assert_eq!(
            ConnectorRuntimeError::new("wechat-authorization-expired").class,
            ConnectorErrorClass::AuthorizationExpired
        );
        assert_eq!(
            ConnectorRuntimeError::new("telegram-http-503").class,
            ConnectorErrorClass::Transient
        );
    }

    #[tokio::test]
    async fn authentication_failure_enters_error_without_retrying() {
        let agent = Arc::new(FakeAgent {
            seen: AsyncMutex::new(HashSet::new()),
            bindings: AsyncMutex::new(HashMap::new()),
            diagnostics: Mutex::new(Vec::new()),
        });
        let adapter = Arc::new(AuthenticationFailingAdapter {
            attempts: AtomicUsize::new(0),
        });
        let runtime = ConnectorRuntimeManager::new(agent);
        runtime.register(adapter.clone()).await;
        runtime.start(ConnectorKind::Telegram).await.unwrap();
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if runtime.health().await[0].lifecycle == ConnectorLifecycle::Error {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert_eq!(adapter.attempts.load(Ordering::Acquire), 1);
    }

    #[tokio::test]
    async fn transient_failure_reconnects_and_only_marks_connected_after_ready() {
        let agent = Arc::new(FakeAgent {
            seen: AsyncMutex::new(HashSet::new()),
            bindings: AsyncMutex::new(HashMap::new()),
            diagnostics: Mutex::new(Vec::new()),
        });
        let adapter = Arc::new(RecoveringAdapter {
            attempts: AtomicUsize::new(0),
        });
        let runtime = ConnectorRuntimeManager::new(agent.clone());
        runtime.register(adapter.clone()).await;
        runtime.start(ConnectorKind::Telegram).await.unwrap();

        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if runtime.health().await[0].lifecycle == ConnectorLifecycle::Connected {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();

        assert_eq!(adapter.attempts.load(Ordering::Acquire), 2);
        assert!(agent.diagnostics.lock().unwrap().iter().any(|event| {
            event.operation == "reconnect"
                && event.safe_code == "telegram-http-503"
                && event.retry_count == 1
        }));
        runtime.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn authorization_expiry_uses_dedicated_lifecycle_without_retrying() {
        let agent = Arc::new(FakeAgent {
            seen: AsyncMutex::new(HashSet::new()),
            bindings: AsyncMutex::new(HashMap::new()),
            diagnostics: Mutex::new(Vec::new()),
        });
        let adapter = Arc::new(AuthorizationExpiredAdapter {
            attempts: AtomicUsize::new(0),
        });
        let runtime = ConnectorRuntimeManager::new(agent);
        runtime.register(adapter.clone()).await;
        runtime.start(ConnectorKind::WeChat).await.unwrap();

        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if runtime.health().await[0].lifecycle == ConnectorLifecycle::AuthorizationExpired {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(adapter.attempts.load(Ordering::Acquire), 1);
    }

    #[tokio::test]
    async fn ninth_pending_message_hits_per_chat_capacity_limit() {
        let agent = Arc::new(BlockingAgent {
            diagnostics: Mutex::new(Vec::new()),
        });
        let sent = Arc::new(AsyncMutex::new(Vec::new()));
        let adapter = Arc::new(FakeAdapter {
            kind: ConnectorKind::Telegram,
            sent: Arc::clone(&sent),
        });
        let runtime = ConnectorRuntimeManager::new(agent.clone());

        for index in 0..=8 {
            runtime
                .accept_inbound(adapter.clone(), inbound(&format!("queue-{index}")))
                .await
                .unwrap();
        }

        assert_eq!(sent.lock().await.as_slice()[0].text, "queue busy");
        assert!(agent.diagnostics.lock().unwrap().iter().any(|event| {
            event.operation == "queue-inbound" && event.safe_code == "chat-queue-full"
        }));
    }

    #[tokio::test]
    async fn final_delivery_failure_is_logged_with_trace_ids_and_not_rerun() {
        let agent = Arc::new(CountingAgent {
            handles: AtomicUsize::new(0),
            diagnostics: Mutex::new(Vec::new()),
        });
        let adapter = Arc::new(FailingSendAdapter {
            sends: AtomicUsize::new(0),
        });
        let runtime = ConnectorRuntimeManager::new(agent.clone());
        runtime
            .accept_inbound(adapter.clone(), inbound("delivery-failure"))
            .await
            .unwrap();

        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if !agent.diagnostics.lock().unwrap().is_empty() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();

        assert_eq!(agent.handles.load(Ordering::Acquire), 1);
        assert_eq!(adapter.sends.load(Ordering::Acquire), 1);
        let diagnostics = agent.diagnostics.lock().unwrap();
        let event = diagnostics
            .iter()
            .find(|event| event.operation == "deliver-final")
            .unwrap();
        assert_eq!(
            event.internal_session_id.as_deref(),
            Some("internal-session")
        );
        assert_eq!(
            event.internal_message_id.as_deref(),
            Some("internal-message")
        );
        assert_eq!(event.platform_status_code.as_deref(), Some("429"));
        assert_eq!(event.retry_classification.as_deref(), Some("transient"));
    }
}
