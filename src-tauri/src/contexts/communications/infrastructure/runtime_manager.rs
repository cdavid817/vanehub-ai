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
use tokio::sync::{mpsc, oneshot, watch, Mutex as AsyncMutex, RwLock, Semaphore};
use tokio::task::JoinHandle;

const INBOUND_BUFFER: usize = 256;
#[cfg(not(test))]
const STOP_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(test)]
const STOP_TIMEOUT: Duration = Duration::from_millis(50);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);
pub(crate) const MAX_TOTAL_PENDING_IM_MESSAGES: usize = 64;
pub(crate) const MAX_ACTIVE_IM_GENERATIONS: usize = 8;

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

pub(crate) trait ConnectorLifecycleEventPort: Send + Sync {
    fn publish(&self, health: ConnectorHealth);
}

#[cfg(test)]
struct NoopConnectorLifecycleEvents;

#[cfg(test)]
impl ConnectorLifecycleEventPort for NoopConnectorLifecycleEvents {
    fn publish(&self, _health: ConnectorHealth) {}
}

struct ChatLane {
    sender: mpsc::Sender<LaneJob>,
    queued: AtomicUsize,
}

struct LaneJob {
    adapter: Arc<dyn ConnectorAdapter>,
    inbound: NormalizedInbound,
    reservation: QueueReservation,
}

struct PendingBudget {
    current: AtomicUsize,
    limit: usize,
}

impl PendingBudget {
    fn reserve(self: &Arc<Self>) -> Option<GlobalPendingReservation> {
        let mut current = self.current.load(Ordering::Acquire);
        loop {
            if current >= self.limit {
                return None;
            }
            match self.current.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Some(GlobalPendingReservation {
                        budget: Arc::clone(self),
                    })
                }
                Err(observed) => current = observed,
            }
        }
    }
}

struct GlobalPendingReservation {
    budget: Arc<PendingBudget>,
}

impl Drop for GlobalPendingReservation {
    fn drop(&mut self) {
        self.budget.current.fetch_sub(1, Ordering::AcqRel);
    }
}

struct QueueReservation {
    lane: Arc<ChatLane>,
    _global: GlobalPendingReservation,
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
    pending: Arc<PendingBudget>,
    active_generations: Arc<Semaphore>,
    #[cfg(test)]
    active_generation_limit: usize,
    lane_workers: AtomicUsize,
    lifecycle_events: Arc<dyn ConnectorLifecycleEventPort>,
    accepting: AtomicBool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuntimeCapacitySnapshot {
    pub(crate) total_pending: usize,
    pub(crate) active_generations: usize,
    pub(crate) lane_workers: usize,
    pub(crate) retained_lanes: usize,
}

impl ConnectorRuntimeManager {
    #[cfg(test)]
    pub fn new(handler: Arc<dyn InboundAgent>) -> Arc<Self> {
        Self::new_with_events(handler, Arc::new(NoopConnectorLifecycleEvents))
    }

    pub(crate) fn new_with_events(
        handler: Arc<dyn InboundAgent>,
        lifecycle_events: Arc<dyn ConnectorLifecycleEventPort>,
    ) -> Arc<Self> {
        Self::with_limits_and_events(
            handler,
            lifecycle_events,
            MAX_TOTAL_PENDING_IM_MESSAGES,
            MAX_ACTIVE_IM_GENERATIONS,
        )
    }

    fn with_limits_and_events(
        handler: Arc<dyn InboundAgent>,
        lifecycle_events: Arc<dyn ConnectorLifecycleEventPort>,
        total_pending_limit: usize,
        active_generation_limit: usize,
    ) -> Arc<Self> {
        let total_pending_limit = total_pending_limit.max(1);
        let active_generation_limit = active_generation_limit.max(1);
        Arc::new(Self {
            handler,
            connectors: RwLock::new(HashMap::new()),
            lanes: Mutex::new(HashMap::new()),
            pending: Arc::new(PendingBudget {
                current: AtomicUsize::new(0),
                limit: total_pending_limit,
            }),
            active_generations: Arc::new(Semaphore::new(active_generation_limit)),
            #[cfg(test)]
            active_generation_limit,
            lane_workers: AtomicUsize::new(0),
            lifecycle_events,
            accepting: AtomicBool::new(true),
        })
    }

    #[cfg(test)]
    fn with_limits(
        handler: Arc<dyn InboundAgent>,
        total_pending_limit: usize,
        active_generation_limit: usize,
    ) -> Arc<Self> {
        Self::with_limits_and_events(
            handler,
            Arc::new(NoopConnectorLifecycleEvents),
            total_pending_limit,
            active_generation_limit,
        )
    }

    #[cfg(test)]
    pub(crate) fn capacity_snapshot(&self) -> RuntimeCapacitySnapshot {
        RuntimeCapacitySnapshot {
            total_pending: self.pending.current.load(Ordering::Acquire),
            active_generations: self
                .active_generation_limit
                .saturating_sub(self.active_generations.available_permits()),
            lane_workers: self.lane_workers.load(Ordering::Acquire),
            retained_lanes: self
                .lanes
                .lock()
                .map(|lanes| lanes.len())
                .unwrap_or_default(),
        }
    }

    #[cfg(test)]
    pub async fn register(&self, adapter: Arc<dyn ConnectorAdapter>) {
        self.connectors
            .write()
            .await
            .insert(adapter.kind(), Arc::new(ManagedConnector::new(adapter)));
    }

    pub async fn replace_and_start(
        self: &Arc<Self>,
        adapter: Arc<dyn ConnectorAdapter>,
    ) -> Result<(), ConnectorRuntimeError> {
        let kind = adapter.kind();
        let previous = self.connectors.read().await.get(&kind).cloned();
        let previous_was_running = if let Some(previous) = &previous {
            let state = previous.state.lock().await;
            matches!(
                state
                    .status
                    .health(kind, state.updated_at.clone())
                    .lifecycle,
                ConnectorLifecycle::Connecting
                    | ConnectorLifecycle::Connected
                    | ConnectorLifecycle::Reconnecting
            )
        } else {
            false
        };
        if previous.is_some() {
            self.stop(kind).await?;
        }
        self.connectors
            .write()
            .await
            .insert(kind, Arc::new(ManagedConnector::new(adapter)));
        if let Err(primary) = self.start(kind).await {
            if let Some(previous) = previous {
                self.connectors.write().await.insert(kind, previous);
                if previous_was_running {
                    if let Err(rollback) = self.start(kind).await {
                        self.diagnostic(
                            kind,
                            DiagnosticLevel::Error,
                            "rollback-start",
                            &rollback.safe_code,
                            0,
                        );
                    }
                }
            } else {
                self.connectors.write().await.remove(&kind);
            }
            return Err(primary);
        }
        Ok(())
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

    async fn publish_health(&self, kind: ConnectorKind, managed: &ManagedConnector) {
        let state = managed.state.lock().await;
        self.lifecycle_events
            .publish(state.status.health(kind, state.updated_at.clone()));
    }

    pub async fn start(self: &Arc<Self>, kind: ConnectorKind) -> Result<(), ConnectorRuntimeError> {
        let managed = self.connector(kind).await?;
        self.stop(kind).await?;

        let (sender, receiver) = mpsc::channel(INBOUND_BUFFER);
        let (shutdown_sender, shutdown_receiver) = watch::channel(false);
        let (startup_sender, startup_receiver) = oneshot::channel();
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
            let mut startup_sender = Some(startup_sender);
            runtime.diagnostic(kind, DiagnosticLevel::Info, "start", "connecting", 0);
            let mut retry_count = 0_u32;
            let result = loop {
                if *shutdown_receiver.borrow() {
                    if let Some(startup_sender) = startup_sender.take() {
                        let _ = startup_sender
                            .send(Err(ConnectorRuntimeError::new("runtime-shutting-down")));
                    }
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
                                if let Some(startup_sender) = startup_sender.take() {
                                    let _ = startup_sender.send(Ok(()));
                                }
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
                    Err(error) => {
                        if let Some(startup_sender) = startup_sender.take() {
                            let _ = startup_sender.send(Err(error.clone()));
                        }
                        break Err(error);
                    }
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
            drop(state);
            runtime.publish_health(kind, &worker).await;
        });

        managed.state.lock().await.task = Some(task);
        self.publish_health(kind, &managed).await;
        match tokio::time::timeout(STARTUP_TIMEOUT, startup_receiver).await {
            Ok(Ok(Ok(()))) => Ok(()),
            Ok(Ok(Err(error))) => Err(error),
            Ok(Err(_)) => Err(ConnectorRuntimeError::new("connector-startup-closed")),
            Err(_) => {
                let _ = self.stop(kind).await;
                Err(ConnectorRuntimeError::new("connector-startup-timeout"))
            }
        }
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
        drop(state);
        self.publish_health(kind, &managed).await;
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
        let mut first_error = None;
        for kind in kinds {
            if let Err(error) = self.stop(kind).await {
                self.diagnostic(
                    kind,
                    DiagnosticLevel::Error,
                    "shutdown",
                    &error.safe_code,
                    0,
                );
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
        first_error.map_or(Ok(()), Err)
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
        drop(state);
        self.publish_health(managed.adapter.kind(), managed).await;
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

    pub(crate) fn record_protocol_diagnostic(
        &self,
        connector: ConnectorKind,
        safe_code: &'static str,
    ) {
        self.diagnostic(connector, DiagnosticLevel::Warn, "receive", safe_code, 0);
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
        let Some(global_reservation) = self.pending.reserve() else {
            self.send_busy(Arc::clone(&adapter), inbound, "global-queue-full")
                .await?;
            return Ok(());
        };
        let lane_key = (inbound.connector, inbound.chat_id.clone());
        let lane = {
            let mut lanes = self
                .lanes
                .lock()
                .map_err(|_| ConnectorRuntimeError::new("queue-lock-failed"))?;
            if let Some(lane) = lanes.get(&lane_key) {
                Arc::clone(lane)
            } else {
                let (sender, receiver) =
                    mpsc::channel(crate::contexts::communications::domain::MAX_PENDING_PER_CHAT);
                let lane = Arc::new(ChatLane {
                    sender,
                    queued: AtomicUsize::new(0),
                });
                lanes.insert(lane_key.clone(), Arc::clone(&lane));
                self.start_lane_worker(lane_key.clone(), receiver);
                lane
            }
        };
        let queued = lane.queued.fetch_add(1, Ordering::AcqRel);
        if pending_delivery_admission(queued) == DeliveryAdmission::Busy {
            lane.queued.fetch_sub(1, Ordering::AcqRel);
            drop(global_reservation);
            self.send_busy(adapter, inbound, "chat-queue-full").await?;
            self.cleanup_lane(&lane_key, &lane)?;
            return Ok(());
        }
        let reservation = QueueReservation {
            lane: Arc::clone(&lane),
            _global: global_reservation,
        };
        lane.sender
            .send(LaneJob {
                adapter,
                inbound,
                reservation,
            })
            .await
            .map_err(|_| ConnectorRuntimeError::new("lane-worker-unavailable"))?;
        Ok(())
    }

    fn start_lane_worker(
        self: &Arc<Self>,
        lane_key: (ConnectorKind, String),
        mut receiver: mpsc::Receiver<LaneJob>,
    ) {
        self.lane_workers.fetch_add(1, Ordering::AcqRel);
        let runtime = Arc::clone(self);
        tokio::spawn(async move {
            while let Some(job) = receiver.recv().await {
                let job_lane = Arc::clone(&job.reservation.lane);
                if let Err(error) = runtime.dispatch_claimed(job.adapter, job.inbound).await {
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
                drop(job.reservation);
                let _ = runtime.cleanup_lane(&lane_key, &job_lane);
            }
            runtime.lane_workers.fetch_sub(1, Ordering::AcqRel);
        });
    }

    fn cleanup_lane(
        &self,
        lane_key: &(ConnectorKind, String),
        lane: &Arc<ChatLane>,
    ) -> Result<(), ConnectorRuntimeError> {
        let mut lanes = self
            .lanes
            .lock()
            .map_err(|_| ConnectorRuntimeError::new("queue-lock-failed"))?;
        let remove = lane.queued.load(Ordering::Acquire) == 0
            && lanes
                .get(lane_key)
                .is_some_and(|current| Arc::ptr_eq(current, lane));
        if remove {
            lanes.remove(lane_key);
        }
        Ok(())
    }

    async fn send_busy(
        &self,
        adapter: Arc<dyn ConnectorAdapter>,
        inbound: NormalizedInbound,
        safe_code: &'static str,
    ) -> Result<(), ConnectorRuntimeError> {
        let connector = inbound.connector;
        adapter
            .send_text(OutboundText {
                chat_id: inbound.chat_id,
                text: self.handler.busy_message(),
                reply_context: inbound.reply_context,
            })
            .await?;
        self.diagnostic(
            connector,
            DiagnosticLevel::Warn,
            "queue-inbound",
            safe_code,
            0,
        );
        Ok(())
    }

    async fn dispatch_claimed(
        &self,
        adapter: Arc<dyn ConnectorAdapter>,
        inbound: NormalizedInbound,
    ) -> Result<(), DispatchFailure> {
        let connector = inbound.connector;
        let chat_id = inbound.chat_id.clone();
        let reply_context = inbound.reply_context.clone();
        let active = Arc::clone(&self.active_generations)
            .acquire_owned()
            .await
            .map_err(|_| DispatchFailure {
                connector,
                error: ConnectorRuntimeError::new("generation-capacity-closed"),
                session_id: None,
                message_id: None,
            })?;
        let handled = self.handler.handle(inbound).await;
        drop(active);
        let (response, session_id, message_id) = match handled {
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

    struct OrderedAgent {
        order: AsyncMutex<Vec<String>>,
    }

    struct FailingSendAdapter {
        sends: AtomicUsize,
    }

    struct TrackingRuntimeAdapter {
        kind: ConnectorKind,
        starts: AtomicUsize,
        active: AtomicUsize,
        max_active: AtomicUsize,
        fail_start: bool,
    }

    struct StubbornRuntimeAdapter;

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
    impl InboundAgent for OrderedAgent {
        async fn handle(
            &self,
            inbound: NormalizedInbound,
        ) -> Result<InboundOutcome, ConnectorRuntimeError> {
            self.order.lock().await.push(inbound.event_id.clone());
            tokio::task::yield_now().await;
            Ok(InboundOutcome::Reply {
                text: inbound.event_id.clone(),
                session_id: format!("session-{}", inbound.chat_id),
                message_id: format!("message-{}", inbound.event_id),
            })
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

    #[async_trait]
    impl ConnectorAdapter for TrackingRuntimeAdapter {
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
            _inbound: mpsc::Sender<InboundDelivery>,
            mut shutdown: watch::Receiver<bool>,
            ready: oneshot::Sender<()>,
        ) -> Result<(), ConnectorRuntimeError> {
            self.starts.fetch_add(1, Ordering::AcqRel);
            if self.fail_start {
                return Err(ConnectorRuntimeError::new("telegram-api-401"));
            }
            let active = self.active.fetch_add(1, Ordering::AcqRel) + 1;
            self.max_active.fetch_max(active, Ordering::AcqRel);
            let _ = ready.send(());
            let _ = shutdown.changed().await;
            self.active.fetch_sub(1, Ordering::AcqRel);
            Ok(())
        }

        async fn send_text(&self, _outbound: OutboundText) -> Result<(), ConnectorRuntimeError> {
            Ok(())
        }
    }

    #[async_trait]
    impl ConnectorAdapter for StubbornRuntimeAdapter {
        fn kind(&self) -> ConnectorKind {
            ConnectorKind::Feishu
        }

        fn max_outbound_chars(&self) -> usize {
            20_000
        }

        async fn test_connection(&self) -> Result<(), ConnectorRuntimeError> {
            Ok(())
        }

        async fn run(
            &self,
            _inbound: mpsc::Sender<InboundDelivery>,
            _shutdown: watch::Receiver<bool>,
            ready: oneshot::Sender<()>,
        ) -> Result<(), ConnectorRuntimeError> {
            let _ = ready.send(());
            std::future::pending().await
        }

        async fn send_text(&self, _outbound: OutboundText) -> Result<(), ConnectorRuntimeError> {
            Ok(())
        }
    }

    fn inbound(event_id: &str) -> NormalizedInbound {
        inbound_chat(event_id, "same-chat")
    }

    fn inbound_chat(event_id: &str, chat_id: &str) -> NormalizedInbound {
        NormalizedInbound {
            connector: ConnectorKind::Telegram,
            event_id: event_id.to_string(),
            chat_id: chat_id.to_string(),
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
        assert_eq!(
            runtime
                .start(ConnectorKind::Telegram)
                .await
                .expect_err("authentication failure")
                .safe_code,
            "telegram-api-401"
        );
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
        assert_eq!(
            runtime
                .start(ConnectorKind::WeChat)
                .await
                .expect_err("authorization failure")
                .safe_code,
            "wechat-authorization-expired"
        );

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
    async fn coordinated_replace_never_orphans_workers_and_restores_previous_on_start_failure() {
        let agent = Arc::new(FakeAgent {
            seen: AsyncMutex::new(HashSet::new()),
            bindings: AsyncMutex::new(HashMap::new()),
            diagnostics: Mutex::new(Vec::new()),
        });
        let runtime = ConnectorRuntimeManager::new(agent);
        let original = Arc::new(TrackingRuntimeAdapter {
            kind: ConnectorKind::Telegram,
            starts: AtomicUsize::new(0),
            active: AtomicUsize::new(0),
            max_active: AtomicUsize::new(0),
            fail_start: false,
        });
        runtime
            .replace_and_start(original.clone())
            .await
            .expect("start original");
        let replacement = Arc::new(TrackingRuntimeAdapter {
            kind: ConnectorKind::Telegram,
            starts: AtomicUsize::new(0),
            active: AtomicUsize::new(0),
            max_active: AtomicUsize::new(0),
            fail_start: false,
        });
        runtime
            .replace_and_start(replacement.clone())
            .await
            .expect("replace");
        assert_eq!(original.active.load(Ordering::Acquire), 0);
        assert_eq!(replacement.active.load(Ordering::Acquire), 1);
        assert_eq!(original.max_active.load(Ordering::Acquire), 1);
        assert_eq!(replacement.max_active.load(Ordering::Acquire), 1);

        let failing = Arc::new(TrackingRuntimeAdapter {
            kind: ConnectorKind::Telegram,
            starts: AtomicUsize::new(0),
            active: AtomicUsize::new(0),
            max_active: AtomicUsize::new(0),
            fail_start: true,
        });
        assert_eq!(
            runtime
                .replace_and_start(failing.clone())
                .await
                .expect_err("replacement failure")
                .safe_code,
            "telegram-api-401"
        );
        assert_eq!(failing.active.load(Ordering::Acquire), 0);
        assert_eq!(replacement.active.load(Ordering::Acquire), 1);
        assert_eq!(replacement.starts.load(Ordering::Acquire), 2);
        runtime.shutdown().await.expect("shutdown");
    }

    #[tokio::test]
    async fn shutdown_attempts_every_connector_after_one_times_out() {
        let agent = Arc::new(FakeAgent {
            seen: AsyncMutex::new(HashSet::new()),
            bindings: AsyncMutex::new(HashMap::new()),
            diagnostics: Mutex::new(Vec::new()),
        });
        let runtime = ConnectorRuntimeManager::new(agent);
        let responsive = Arc::new(TrackingRuntimeAdapter {
            kind: ConnectorKind::Telegram,
            starts: AtomicUsize::new(0),
            active: AtomicUsize::new(0),
            max_active: AtomicUsize::new(0),
            fail_start: false,
        });
        runtime
            .replace_and_start(Arc::new(StubbornRuntimeAdapter))
            .await
            .expect("start stubborn");
        runtime
            .replace_and_start(responsive.clone())
            .await
            .expect("start responsive");

        assert_eq!(
            runtime.shutdown().await.expect_err("one timeout").safe_code,
            "shutdown-timeout"
        );
        assert_eq!(responsive.active.load(Ordering::Acquire), 0);
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
    async fn lane_worker_preserves_fifo_and_releases_all_capacity_when_idle() {
        let agent = Arc::new(OrderedAgent {
            order: AsyncMutex::new(Vec::new()),
        });
        let sent = Arc::new(AsyncMutex::new(Vec::new()));
        let adapter = Arc::new(FakeAdapter {
            kind: ConnectorKind::Telegram,
            sent,
        });
        let runtime = ConnectorRuntimeManager::with_limits(agent.clone(), 8, 2);

        for event_id in ["first", "second", "third"] {
            runtime
                .accept_inbound(adapter.clone(), inbound(event_id))
                .await
                .expect("admit");
        }
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let snapshot = runtime.capacity_snapshot();
                if snapshot.total_pending == 0
                    && snapshot.lane_workers == 0
                    && snapshot.retained_lanes == 0
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("drain");

        assert_eq!(
            agent.order.lock().await.as_slice(),
            ["first", "second", "third"]
        );
        assert_eq!(
            runtime.capacity_snapshot(),
            RuntimeCapacitySnapshot {
                total_pending: 0,
                active_generations: 0,
                lane_workers: 0,
                retained_lanes: 0,
            }
        );
    }

    #[tokio::test]
    async fn global_pending_and_active_generation_limits_bound_distinct_chat_stress() {
        let agent = Arc::new(BlockingAgent {
            diagnostics: Mutex::new(Vec::new()),
        });
        let sent = Arc::new(AsyncMutex::new(Vec::new()));
        let adapter = Arc::new(FakeAdapter {
            kind: ConnectorKind::Telegram,
            sent: Arc::clone(&sent),
        });
        let runtime = ConnectorRuntimeManager::with_limits(agent.clone(), 6, 2);

        for index in 0..40 {
            runtime
                .accept_inbound(
                    adapter.clone(),
                    inbound_chat(&format!("event-{index}"), &format!("chat-{index}")),
                )
                .await
                .expect("bounded admission");
        }
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if runtime.capacity_snapshot().active_generations == 2 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("active bound reached");

        assert_eq!(
            runtime.capacity_snapshot(),
            RuntimeCapacitySnapshot {
                total_pending: 6,
                active_generations: 2,
                lane_workers: 6,
                retained_lanes: 6,
            }
        );
        assert_eq!(sent.lock().await.len(), 34);
        assert!(agent.diagnostics.lock().unwrap().iter().all(|event| {
            event.operation == "queue-inbound" && event.safe_code == "global-queue-full"
        }));
    }

    #[tokio::test]
    async fn stale_lane_cleanup_cannot_remove_a_reused_lane_generation() {
        let agent = Arc::new(OrderedAgent {
            order: AsyncMutex::new(Vec::new()),
        });
        let runtime = ConnectorRuntimeManager::with_limits(agent, 2, 1);
        let key = (ConnectorKind::Telegram, "reused-chat".to_string());
        let (old_sender, _old_receiver) = mpsc::channel(1);
        let old = Arc::new(ChatLane {
            sender: old_sender,
            queued: AtomicUsize::new(0),
        });
        let (new_sender, _new_receiver) = mpsc::channel(1);
        let replacement = Arc::new(ChatLane {
            sender: new_sender,
            queued: AtomicUsize::new(1),
        });
        runtime
            .lanes
            .lock()
            .expect("lanes")
            .insert(key.clone(), Arc::clone(&replacement));

        runtime.cleanup_lane(&key, &old).expect("cleanup stale");

        let lanes = runtime.lanes.lock().expect("lanes");
        assert!(lanes
            .get(&key)
            .is_some_and(|current| Arc::ptr_eq(current, &replacement)));
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
