use super::initialize_negotiation::{
    build_initialize_params, initialize_and_notify, InitializeNegotiationError,
};
use super::json_rpc_actor::{JsonRpcActorLimits, JsonRpcClient, JsonRpcError, JsonRpcEvents};
use super::lsp_diagnostics::{
    LspCrashReason, LspDiagnosticEvent, LspDiagnosticIdentity, LspDiagnosticKind,
    LspDiagnosticLogger, LspMethodCategory, LspPrivateDiagnosticData,
};
use super::lsp_framing::{FrameLimits, LspFrameError};
use super::lsp_server_requests::{LspClientRequestLimits, LspServerRequestHandler};
use super::lsp_stdio_child::{LspShutdownDisposition, LspStdioError, ManagedLspStdio};
use super::process_registry::{
    ActivationReason, LifecycleAction, LifecyclePolicy, ProcessRegistry, ProcessStatusSnapshot,
};
use super::project_root::ProcessKey;
use super::runtime_notifications::RuntimeNotificationRouter;
use super::shutdown_coordinator::{
    ActiveLspProcess, ActiveLspProcessWait, LspShutdownCoordinator, LspShutdownSummary,
};
use crate::contexts::code_intelligence::domain::models::Language;
use crate::contexts::code_intelligence::domain::models::{NegotiatedCapabilities, ProcessState};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::Mutex;
use url::Url;

const INITIALIZE_TIMEOUT: Duration = Duration::from_secs(10);
const PROCESS_STOP_TIMEOUT: Duration = Duration::from_secs(3);
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(250);
const STDERR_LIMIT: usize = 64 * 1024;
const MAX_SERVER_STATUS_SNAPSHOTS: usize = 64;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeProcessError {
    #[error("language-server process is unavailable")]
    Unavailable,
    #[error("language-server process could not start")]
    Spawn,
    #[error("language-server initialization failed")]
    Initialize,
}

#[derive(Clone)]
pub(crate) struct LspProcessLaunch {
    pub(crate) key: ProcessKey,
    pub(crate) executable: String,
    pub(crate) arguments: Vec<String>,
    pub(crate) initialization_options: Value,
}

#[derive(Clone)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct LspProcessHandle {
    id: u64,
    key: ProcessKey,
    client: JsonRpcClient,
    capabilities: NegotiatedCapabilities,
}

#[cfg_attr(not(test), allow(dead_code))]
impl LspProcessHandle {
    pub(crate) const fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn key(&self) -> &ProcessKey {
        &self.key
    }

    pub(crate) fn client(&self) -> JsonRpcClient {
        self.client.clone()
    }

    pub(crate) fn capabilities(&self) -> &NegotiatedCapabilities {
        &self.capabilities
    }
}

pub(crate) enum LspProcessAcquisition {
    Ready(LspProcessHandle),
    Warming,
    Unavailable,
    Failed,
}

struct RunningProcess {
    id: u64,
    handle: LspProcessHandle,
    active: ActiveLspProcess,
}

struct RuntimeProcessState {
    registry: ProcessRegistry,
    launches: HashMap<ProcessKey, LspProcessLaunch>,
    starting: HashMap<ProcessKey, (u64, ActiveLspProcess)>,
    processes: HashMap<ProcessKey, RunningProcess>,
    telemetry: HashMap<ProcessKey, ProcessTelemetry>,
}

#[derive(Default)]
struct ProcessTelemetry {
    process_id: Option<u64>,
    last_response_at: Option<String>,
    diagnostic_count: usize,
    capabilities: Option<NegotiatedCapabilities>,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeServerStatusSnapshot {
    pub(crate) key: ProcessKey,
    pub(crate) process: ProcessStatusSnapshot,
    pub(crate) last_response_at: Option<String>,
    pub(crate) diagnostic_count: usize,
    pub(crate) capabilities: Option<NegotiatedCapabilities>,
}

struct RuntimeProcessInner {
    state: Mutex<RuntimeProcessState>,
    shutdown: LspShutdownCoordinator,
    epoch: Instant,
    ids: AtomicU64,
    notifications: RuntimeNotificationRouter,
    diagnostics: LspDiagnosticLogger,
}

#[derive(Clone)]
pub(crate) struct RuntimeProcessCoordinator {
    inner: Arc<RuntimeProcessInner>,
}

impl RuntimeProcessCoordinator {
    pub(crate) fn new(
        shutdown: LspShutdownCoordinator,
        policy: LifecyclePolicy,
        diagnostics: LspDiagnosticLogger,
    ) -> Self {
        Self {
            inner: Arc::new(RuntimeProcessInner {
                state: Mutex::new(RuntimeProcessState {
                    registry: ProcessRegistry::new(policy),
                    launches: HashMap::new(),
                    starting: HashMap::new(),
                    processes: HashMap::new(),
                    telemetry: HashMap::new(),
                }),
                shutdown,
                epoch: Instant::now(),
                ids: AtomicU64::new(0),
                notifications: RuntimeNotificationRouter::default(),
                diagnostics,
            }),
        }
    }

    pub(crate) async fn acquire(
        &self,
        launch: LspProcessLaunch,
        reason: ActivationReason,
        authorized: bool,
    ) -> LspProcessAcquisition {
        if !self.inner.shutdown.is_accepting() {
            return LspProcessAcquisition::Unavailable;
        }
        let key = launch.key.clone();
        let (actions, previous_state) = {
            let mut state = self.inner.state.lock().await;
            let previous_state = state.registry.status(&key).map(|status| status.state);
            state.launches.insert(key.clone(), launch);
            (
                state
                    .registry
                    .acquire(key.clone(), reason, self.now(), authorized),
                previous_state,
            )
        };
        if actions
            .iter()
            .any(|action| matches!(action, LifecycleAction::Reject(_)))
        {
            return LspProcessAcquisition::Unavailable;
        }
        for action in actions {
            if let LifecycleAction::Start(start_key) = action {
                self.record_lifecycle(
                    &start_key,
                    previous_state.unwrap_or(ProcessState::Absent),
                    ProcessState::Starting,
                );
                if self.start(start_key.clone()).await.is_err() {
                    self.record_exit(&start_key).await;
                }
            }
        }
        let state = self.inner.state.lock().await;
        if let Some(process) = state.processes.get(&key) {
            return LspProcessAcquisition::Ready(process.handle.clone());
        }
        match state.registry.status(&key).map(|status| status.state) {
            Some(ProcessState::Starting | ProcessState::Initializing | ProcessState::Backoff) => {
                LspProcessAcquisition::Warming
            }
            Some(ProcessState::Failed) => LspProcessAcquisition::Failed,
            _ => LspProcessAcquisition::Unavailable,
        }
    }

    pub(crate) async fn release_request(&self, key: &ProcessKey) {
        self.inner
            .state
            .lock()
            .await
            .registry
            .release_request(key, self.now());
    }

    pub(crate) async fn set_document_leases(&self, key: &ProcessKey, count: usize) {
        self.inner
            .state
            .lock()
            .await
            .registry
            .set_document_leases(key, count, self.now());
    }

    pub(crate) fn notifications(&self) -> RuntimeNotificationRouter {
        self.inner.notifications.clone()
    }

    pub(crate) async fn record_response(&self, key: &ProcessKey) {
        if let Some(telemetry) = self.inner.state.lock().await.telemetry.get_mut(key) {
            telemetry.last_response_at = Some(now_rfc3339());
        }
    }

    pub(crate) async fn record_request_failure(
        &self,
        key: &ProcessKey,
        error: JsonRpcError,
        duration: Duration,
    ) {
        let server_state = self
            .status(key)
            .await
            .map_or(ProcessState::Absent, |status| status.state);
        let kind = match error {
            JsonRpcError::Timeout => LspDiagnosticKind::Timeout {
                method: LspMethodCategory::SemanticQuery,
                duration_ms: duration_millis(duration),
                server_state,
            },
            JsonRpcError::Cancelled => LspDiagnosticKind::Cancellation {
                method: LspMethodCategory::SemanticQuery,
                duration_ms: duration_millis(duration),
                server_state,
            },
            _ => return,
        };
        self.record_diagnostic(key, kind);
    }

    pub(crate) async fn record_diagnostics_wait(
        &self,
        key: &ProcessKey,
        cancelled: bool,
        duration: Duration,
    ) {
        let error = if cancelled {
            JsonRpcError::Cancelled
        } else {
            JsonRpcError::Timeout
        };
        self.record_request_failure(key, error, duration).await;
    }

    pub(crate) async fn shutdown_all(&self, deadline: Instant) -> LspShutdownSummary {
        let started = Instant::now();
        let keys = self.inner.state.lock().await.registry.keys();
        for key in &keys {
            let from = self
                .status(key)
                .await
                .map_or(ProcessState::Absent, |status| status.state);
            self.record_lifecycle(key, from, ProcessState::Stopping);
        }
        let summary = self.inner.shutdown.shutdown_all(deadline).await;
        let process_ids = {
            let mut state = self.inner.state.lock().await;
            let process_ids = state
                .telemetry
                .values()
                .filter_map(|telemetry| telemetry.process_id)
                .collect::<Vec<_>>();
            for key in &keys {
                state.registry.remove(key);
            }
            state.launches.clear();
            state.starting.clear();
            state.processes.clear();
            state.telemetry.clear();
            process_ids
        };
        for process_id in process_ids {
            self.inner.notifications.process_exited(process_id);
        }
        for key in &keys {
            self.record_diagnostic(
                key,
                LspDiagnosticKind::Shutdown {
                    forced: summary.forced > 0 || summary.failed > 0,
                    process_count: summary.total,
                    duration_ms: duration_millis(started.elapsed()),
                },
            );
            self.record_lifecycle(key, ProcessState::Stopping, ProcessState::Absent);
        }
        summary
    }

    pub(crate) async fn status_snapshots(&self) -> Vec<RuntimeServerStatusSnapshot> {
        let state = self.inner.state.lock().await;
        let mut snapshots = state
            .registry
            .keys()
            .into_iter()
            .filter_map(|key| {
                let process = state.registry.status(&key)?;
                let telemetry = state.telemetry.get(&key);
                Some(RuntimeServerStatusSnapshot {
                    key,
                    process,
                    last_response_at: telemetry.and_then(|value| value.last_response_at.clone()),
                    diagnostic_count: telemetry.map_or(0, |value| value.diagnostic_count),
                    capabilities: telemetry.and_then(|value| value.capabilities.clone()),
                })
            })
            .collect::<Vec<_>>();
        snapshots.sort_by(|left, right| {
            (
                left.key.session_root_ref(),
                left.key.project_root_ref(),
                left.key.language().server_id,
            )
                .cmp(&(
                    right.key.session_root_ref(),
                    right.key.project_root_ref(),
                    right.key.language().server_id,
                ))
        });
        snapshots.truncate(MAX_SERVER_STATUS_SNAPSHOTS);
        snapshots
    }

    pub(crate) async fn revoke_workspace(&self, workspace: &std::path::Path) {
        let actions = self
            .inner
            .state
            .lock()
            .await
            .registry
            .revoke_session(workspace);
        self.apply_stop_actions(actions).await;
    }

    /// Stops every process serving one language.
    ///
    /// Used before removing that language's managed install: on Windows a directory a process
    /// still holds open simply will not delete, so this is ordering rather than politeness.
    pub(crate) async fn stop_language(&self, language: Language) {
        let keys = self
            .inner
            .state
            .lock()
            .await
            .registry
            .keys()
            .into_iter()
            .filter(|key| key.language() == language)
            .collect::<Vec<_>>();
        for key in keys {
            self.stop(&key).await;
        }
    }

    pub(crate) async fn configuration_replaced(&self) {
        let keys = self.inner.state.lock().await.registry.keys();
        for key in keys {
            self.stop(&key).await;
        }
    }

    pub(crate) async fn tick(&self) {
        if !self.inner.shutdown.is_accepting() {
            return;
        }
        let (actions, previous_states) = {
            let mut state = self.inner.state.lock().await;
            let previous_states = state
                .registry
                .keys()
                .into_iter()
                .filter_map(|key| {
                    state
                        .registry
                        .status(&key)
                        .map(|status| (key, status.state))
                })
                .collect::<HashMap<_, _>>();
            (state.registry.tick(self.now()), previous_states)
        };
        for action in actions {
            match action {
                LifecycleAction::Start(key) => {
                    let previous = previous_states
                        .get(&key)
                        .copied()
                        .unwrap_or(ProcessState::Absent);
                    self.record_lifecycle(&key, previous, ProcessState::Starting);
                    if matches!(previous, ProcessState::Backoff | ProcessState::Failed) {
                        let restart_attempt = self
                            .status(&key)
                            .await
                            .map_or(0, |status| status.restart_count);
                        self.record_diagnostic(
                            &key,
                            LspDiagnosticKind::Restart { restart_attempt },
                        );
                    }
                    if self.start(key.clone()).await.is_err() {
                        self.record_exit(&key).await;
                    }
                }
                LifecycleAction::Stop(key) => self.stop(&key).await,
                LifecycleAction::FailPending(_) | LifecycleAction::Reject(_) => {}
            }
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) async fn status(&self, key: &ProcessKey) -> Option<ProcessStatusSnapshot> {
        self.inner.state.lock().await.registry.status(key)
    }

    async fn start(&self, key: ProcessKey) -> Result<(), RuntimeProcessError> {
        let launch = self
            .inner
            .state
            .lock()
            .await
            .launches
            .get(&key)
            .cloned()
            .ok_or(RuntimeProcessError::Unavailable)?;
        let root_uri = Url::from_directory_path(key.project_root_ref())
            .map_err(|_| RuntimeProcessError::Initialize)?;
        let (actor_limits, handler) = protocol_configuration()?;
        let (client, events, process) = ManagedLspStdio::spawn(
            &launch.executable,
            &launch.arguments,
            &BTreeMap::new(),
            FrameLimits::default(),
            STDERR_LIMIT,
            actor_limits,
            handler,
        )
        .map_err(|_| RuntimeProcessError::Spawn)?;
        let id = self.inner.ids.fetch_add(1, Ordering::Relaxed);
        {
            let mut state = self.inner.state.lock().await;
            let telemetry = state.telemetry.entry(key.clone()).or_default();
            telemetry.process_id = Some(id);
            telemetry.diagnostic_count = 0;
        }
        self.spawn_event_monitor(key.clone(), id, events);
        let active = ActiveLspProcess::new(client, process);
        if let Err(rejected) = self.inner.shutdown.register(active.clone()).await {
            let _ = rejected
                .force_shutdown(Instant::now() + PROCESS_STOP_TIMEOUT)
                .await;
            return Err(RuntimeProcessError::Unavailable);
        }
        {
            let mut state = self.inner.state.lock().await;
            state.registry.mark_initializing(&key, self.now());
            state.starting.insert(key.clone(), (id, active.clone()));
        }
        self.record_lifecycle(&key, ProcessState::Starting, ProcessState::Initializing);
        let initialize_started = Instant::now();
        let initialize = tokio::time::timeout(
            INITIALIZE_TIMEOUT,
            initialize_and_notify(
                &active.client(),
                build_initialize_params(
                    root_uri.as_str(),
                    launch.initialization_options,
                    Some(std::process::id()),
                ),
            ),
        )
        .await;
        let capabilities = match initialize {
            Ok(Ok(capabilities)) => capabilities,
            Err(_) => {
                self.record_diagnostic(
                    &key,
                    LspDiagnosticKind::Timeout {
                        method: LspMethodCategory::Initialize,
                        duration_ms: duration_millis(initialize_started.elapsed()),
                        server_state: ProcessState::Initializing,
                    },
                );
                self.remove_starting(&key, id).await;
                let _ = active
                    .force_shutdown(Instant::now() + PROCESS_STOP_TIMEOUT)
                    .await;
                return Err(RuntimeProcessError::Initialize);
            }
            Ok(Err(InitializeNegotiationError::Transport(_))) | Ok(Err(_)) => {
                self.remove_starting(&key, id).await;
                let _ = active
                    .force_shutdown(Instant::now() + PROCESS_STOP_TIMEOUT)
                    .await;
                return Err(RuntimeProcessError::Initialize);
            }
        };
        let mut state = self.inner.state.lock().await;
        let current = state
            .starting
            .get(&key)
            .is_some_and(|(current_id, _)| *current_id == id)
            && state.launches.contains_key(&key)
            && self.inner.shutdown.is_accepting();
        if !current {
            drop(state);
            let _ = active
                .force_shutdown(Instant::now() + PROCESS_STOP_TIMEOUT)
                .await;
            return Err(RuntimeProcessError::Unavailable);
        }
        let handle = LspProcessHandle {
            id,
            key: key.clone(),
            client: active.client(),
            capabilities: capabilities.clone(),
        };
        state.starting.remove(&key);
        state.registry.mark_ready(&key, self.now());
        if let Some(telemetry) = state.telemetry.get_mut(&key) {
            telemetry.last_response_at = Some(now_rfc3339());
            telemetry.capabilities = Some(capabilities.clone());
        }
        state.processes.insert(
            key.clone(),
            RunningProcess {
                id,
                handle,
                active: active.clone(),
            },
        );
        drop(state);
        self.record_lifecycle(&key, ProcessState::Initializing, ProcessState::Ready);
        self.spawn_exit_monitor(key, id, active);
        Ok(())
    }

    fn spawn_exit_monitor(&self, key: ProcessKey, id: u64, active: ActiveLspProcess) {
        let coordinator = self.clone();
        tokio::spawn(async move {
            loop {
                let deadline = Instant::now() + PROCESS_POLL_INTERVAL;
                match active.wait_until(deadline).await {
                    Ok(ActiveLspProcessWait::Exited(exit)) => {
                        coordinator
                            .handle_unexpected_exit(&key, id, exit.status.code(), None)
                            .await;
                        return;
                    }
                    Ok(ActiveLspProcessWait::Gone) => return,
                    Err(error) => {
                        coordinator
                            .handle_unexpected_exit(&key, id, None, Some(error))
                            .await;
                        return;
                    }
                    Ok(ActiveLspProcessWait::Pending) => tokio::task::yield_now().await,
                }
            }
        });
    }

    fn spawn_event_monitor(&self, key: ProcessKey, id: u64, events: JsonRpcEvents) {
        let coordinator = self.clone();
        tokio::spawn(async move {
            let (mut notifications, mut protocol_events) = events.into_receivers();
            let mut notifications_open = true;
            let mut protocol_events_open = true;
            while notifications_open || protocol_events_open {
                tokio::select! {
                    notification = notifications.recv(), if notifications_open => {
                        match notification {
                            Some(notification) => {
                                if let Some(observation) = coordinator
                                    .inner
                                    .notifications
                                    .handle(id, notification)
                                    .await
                                {
                                    coordinator
                                        .observe_notification(&key, id, observation.diagnostic_count)
                                        .await;
                                }
                            }
                            None => notifications_open = false,
                        }
                    }
                    event = protocol_events.recv(), if protocol_events_open => {
                        match event {
                            Some(_) => coordinator.record_response(&key).await,
                            None => protocol_events_open = false,
                        }
                    }
                }
            }
        });
    }

    async fn observe_notification(&self, key: &ProcessKey, id: u64, diagnostic_count: usize) {
        let mut state = self.inner.state.lock().await;
        let Some(telemetry) = state.telemetry.get_mut(key) else {
            return;
        };
        if telemetry.process_id == Some(id) {
            let changed = telemetry.diagnostic_count != diagnostic_count;
            telemetry.last_response_at = Some(now_rfc3339());
            telemetry.diagnostic_count = diagnostic_count;
            drop(state);
            if changed {
                self.record_diagnostic(
                    key,
                    LspDiagnosticKind::DiagnosticsCount {
                        count: diagnostic_count,
                    },
                );
            }
        }
    }

    async fn handle_unexpected_exit(
        &self,
        key: &ProcessKey,
        id: u64,
        exit_code: Option<i32>,
        protocol_error: Option<LspStdioError>,
    ) {
        let mut state = self.inner.state.lock().await;
        if state
            .processes
            .get(key)
            .is_none_or(|process| process.id != id)
        {
            return;
        }
        state.processes.remove(key);
        if let Some(telemetry) = state.telemetry.get_mut(key) {
            telemetry.process_id = None;
            telemetry.diagnostic_count = 0;
        }
        self.inner.notifications.process_exited(id);
        let previous = state
            .registry
            .status(key)
            .map_or(ProcessState::Ready, |status| status.state);
        let next = if self.inner.shutdown.is_accepting() {
            state.registry.unexpected_exit(key, self.now());
            state.registry.status(key).map(|status| status.state)
        } else {
            state.registry.remove(key);
            state.launches.remove(key);
            Some(ProcessState::Absent)
        };
        let restart_attempt = state
            .registry
            .status(key)
            .map_or(0, |status| status.restart_count);
        drop(state);
        if protocol_error.as_ref().is_some_and(is_protocol_limit) {
            self.record_diagnostic(
                key,
                LspDiagnosticKind::ProtocolLimit {
                    method: LspMethodCategory::Transport,
                    duration_ms: 0,
                    observed_bytes: 0,
                },
            );
        }
        self.record_diagnostic(
            key,
            LspDiagnosticKind::Crash {
                exit_code,
                restart_attempt,
                reason: if protocol_error.is_some() {
                    LspCrashReason::ProtocolFailure
                } else {
                    LspCrashReason::UnexpectedExit
                },
            },
        );
        if let Some(next) = next {
            self.record_lifecycle(key, previous, next);
        }
    }

    async fn record_exit(&self, key: &ProcessKey) {
        let mut state = self.inner.state.lock().await;
        let previous = state
            .registry
            .status(key)
            .map_or(ProcessState::Starting, |status| status.state);
        if self.inner.shutdown.is_accepting() {
            state.registry.unexpected_exit(key, self.now());
        } else {
            state.registry.remove(key);
            state.launches.remove(key);
        }
        let next = state
            .registry
            .status(key)
            .map_or(ProcessState::Absent, |status| status.state);
        drop(state);
        self.record_lifecycle(key, previous, next);
    }

    async fn remove_starting(&self, key: &ProcessKey, id: u64) {
        let mut state = self.inner.state.lock().await;
        if state
            .starting
            .get(key)
            .is_some_and(|(current_id, _)| *current_id == id)
        {
            state.starting.remove(key);
        }
    }

    async fn apply_stop_actions(&self, actions: Vec<LifecycleAction>) {
        for action in actions {
            if let LifecycleAction::Stop(key) = action {
                self.stop(&key).await;
            }
        }
    }

    async fn stop(&self, key: &ProcessKey) {
        let (active, process_id, from_state) = {
            let mut state = self.inner.state.lock().await;
            let from_state = state
                .registry
                .status(key)
                .map_or(ProcessState::Absent, |status| status.state);
            let active = state
                .processes
                .remove(key)
                .map(|process| process.active)
                .or_else(|| state.starting.remove(key).map(|(_, active)| active));
            let process_id = state.telemetry.get(key).and_then(|value| value.process_id);
            (active, process_id, from_state)
        };
        self.record_lifecycle(key, from_state, ProcessState::Stopping);
        if let Some(process_id) = process_id {
            self.inner.notifications.process_exited(process_id);
        }
        let process_count = usize::from(active.is_some());
        let started = Instant::now();
        let outcome = if let Some(active) = active {
            Some(active.shutdown(Instant::now() + PROCESS_STOP_TIMEOUT).await)
        } else {
            None
        };
        let forced = match outcome.as_ref() {
            Some(Ok(Some(outcome))) => outcome.disposition == LspShutdownDisposition::Forced,
            Some(Err(_)) => true,
            _ => false,
        };
        self.record_diagnostic(
            key,
            LspDiagnosticKind::Shutdown {
                forced,
                process_count,
                duration_ms: duration_millis(started.elapsed()),
            },
        );
        let mut state = self.inner.state.lock().await;
        state.registry.remove(key);
        state.launches.remove(key);
        state.telemetry.remove(key);
        drop(state);
        self.record_lifecycle(key, ProcessState::Stopping, ProcessState::Absent);
    }

    fn record_lifecycle(&self, key: &ProcessKey, from: ProcessState, to: ProcessState) {
        if from == to {
            return;
        }
        self.record_diagnostic(key, LspDiagnosticKind::Lifecycle { from, to });
    }

    fn record_diagnostic(&self, key: &ProcessKey, kind: LspDiagnosticKind) {
        self.inner.diagnostics.record(LspDiagnosticEvent {
            identity: diagnostic_identity(key),
            kind,
            private: LspPrivateDiagnosticData::default(),
        });
    }

    fn now(&self) -> Duration {
        self.inner.epoch.elapsed()
    }
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn diagnostic_identity(key: &ProcessKey) -> LspDiagnosticIdentity {
    let language = key.language();
    let mut digest = Sha256::new();
    digest.update(key.session_root_ref().to_string_lossy().as_bytes());
    let workspace_id = digest
        .finalize()
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    LspDiagnosticIdentity {
        language,
        workspace_id: Some(format!("workspace-{workspace_id}")),
        correlation_id: None,
    }
}

fn duration_millis(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn is_protocol_limit(error: &LspStdioError) -> bool {
    matches!(
        error,
        LspStdioError::Protocol(LspFrameError::HeaderTooLarge | LspFrameError::PayloadTooLarge)
    )
}

fn protocol_configuration(
) -> Result<(JsonRpcActorLimits, Arc<LspServerRequestHandler>), RuntimeProcessError> {
    let actor =
        JsonRpcActorLimits::new(32, 32, 32, 32, 32, 16).map_err(|_| RuntimeProcessError::Spawn)?;
    let limits = LspClientRequestLimits::new(16, 32, 16, 32 * 1024)
        .map_err(|_| RuntimeProcessError::Spawn)?;
    let handler = LspServerRequestHandler::new(BTreeMap::new(), limits)
        .map_err(|_| RuntimeProcessError::Spawn)?;
    Ok((actor, Arc::new(handler)))
}
