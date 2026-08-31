use super::initialize_negotiation::{build_initialize_params, initialize_and_notify};
use super::json_rpc_actor::JsonRpcActorLimits;
use super::lsp_framing::FrameLimits;
use super::lsp_server_requests::{LspClientRequestLimits, LspServerRequestHandler};
use super::lsp_stdio_child::{LspShutdownDisposition, ManagedLspStdio};
use super::server_discovery::ServerDiscoveryResult;
use crate::contexts::code_intelligence::domain::models::{Language, NegotiatedCapabilities};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use url::Url;

const STDERR_LIMIT: usize = 64 * 1024;
const MIN_TEST_TIMEOUT: Duration = Duration::from_millis(100);

/// The least time cleanup gets, whatever the caller's own deadline has left.
///
/// The deadline a caller supplies bounds the work it asked for. Cleanup is not that work — it is
/// what this code owes the machine afterwards — and giving it the remainder means a slow spawn
/// leaves it nothing. What follows is not a slow cleanup but a false one: `start_kill` is issued,
/// the wait is skipped because there is no time to wait in, and the phase reports failure for a
/// child that did in fact die. A caller told "cleanup failed" cannot tell that from a process tree
/// still running.
///
/// Two seconds because it has to cover the whole shutdown ladder — a graceful window, a terminate
/// signal on Unix, then a kill and the wait that observes it — on a machine loaded enough to have
/// spent the caller's budget in the first place. It is a ceiling, not a delay: cleanup that
/// finishes sooner returns sooner.
const CLEANUP_FLOOR: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub(crate) struct ServerTestCommand {
    language: Language,
    executable: Option<String>,
    arguments: Vec<String>,
    initialization_options: Value,
}

impl fmt::Debug for ServerTestCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ServerTestCommand")
            .field("language", &self.language)
            .field("available", &self.executable.is_some())
            .finish()
    }
}

impl ServerTestCommand {
    pub(crate) fn available(
        language: Language,
        executable: String,
        arguments: Vec<String>,
        initialization_options: Value,
    ) -> Self {
        Self {
            language,
            executable: Some(executable),
            arguments,
            initialization_options,
        }
    }

    pub(crate) const fn unavailable(language: Language) -> Self {
        Self {
            language,
            executable: None,
            arguments: Vec::new(),
            initialization_options: Value::Null,
        }
    }

    pub(crate) fn from_discovery(
        discovery: &ServerDiscoveryResult,
        initialization_options: Value,
    ) -> Self {
        let Some(executable) = discovery.executable() else {
            return Self::unavailable(discovery.language());
        };
        Self::available(
            discovery.language(),
            executable.to_string_lossy().into_owned(),
            discovery.arguments().to_vec(),
            initialization_options,
        )
    }

    pub(crate) const fn language(&self) -> Language {
        self.language
    }

    pub(crate) fn arguments(&self) -> &[String] {
        &self.arguments
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ServerTestPhase {
    Discovery,
    Spawn,
    Initialize,
    Cleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ServerTestPhaseStatus {
    Succeeded,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ServerTestReason {
    ExecutableUnavailable,
    MinimalProjectFailed,
    SpawnFailed,
    InitializeFailed,
    InitializeTimedOut,
    ForcedTermination,
    CleanupFailed,
    InvalidDeadline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ServerTestPhaseResult {
    pub(crate) phase: ServerTestPhase,
    pub(crate) status: ServerTestPhaseStatus,
    pub(crate) reason: Option<ServerTestReason>,
}

#[derive(Debug)]
pub(crate) struct IsolatedServerTestResult {
    phases: Vec<ServerTestPhaseResult>,
    negotiated_capabilities: Option<NegotiatedCapabilities>,
}

impl IsolatedServerTestResult {
    pub(crate) fn phases(&self) -> &[ServerTestPhaseResult] {
        &self.phases
    }

    pub(crate) fn phase(&self, phase: ServerTestPhase) -> Option<&ServerTestPhaseResult> {
        self.phases.iter().find(|result| result.phase == phase)
    }

    pub(crate) fn negotiated_capabilities(&self) -> Option<&NegotiatedCapabilities> {
        self.negotiated_capabilities.as_ref()
    }

    pub(crate) fn cleaned_up(&self) -> bool {
        self.phase(ServerTestPhase::Cleanup)
            .is_some_and(|phase| phase.status == ServerTestPhaseStatus::Succeeded)
    }
}

pub(crate) struct IsolatedServerTester;

impl IsolatedServerTester {
    pub(crate) async fn run(
        command: ServerTestCommand,
        timeout: Duration,
    ) -> IsolatedServerTestResult {
        let mut result = empty_result();
        if timeout < MIN_TEST_TIMEOUT {
            fail_phase(
                &mut result,
                ServerTestPhase::Discovery,
                ServerTestReason::InvalidDeadline,
            );
            return result;
        }
        let Some(executable) = command.executable.as_deref() else {
            fail_phase(
                &mut result,
                ServerTestPhase::Discovery,
                ServerTestReason::ExecutableUnavailable,
            );
            return result;
        };
        succeed_phase(&mut result, ServerTestPhase::Discovery, None);
        let deadline = Instant::now() + timeout;
        let project = match MinimalProject::create(command.language) {
            Ok(project) => project,
            Err(()) => {
                fail_phase(
                    &mut result,
                    ServerTestPhase::Spawn,
                    ServerTestReason::MinimalProjectFailed,
                );
                return result;
            }
        };
        let Some((actor_limits, handler)) = protocol_configuration() else {
            fail_phase(
                &mut result,
                ServerTestPhase::Spawn,
                ServerTestReason::SpawnFailed,
            );
            return result;
        };
        let spawn = ManagedLspStdio::spawn(
            executable,
            &command.arguments,
            &BTreeMap::new(),
            FrameLimits::default(),
            STDERR_LIMIT,
            actor_limits,
            handler,
        );
        let (client, _events, mut process) = match spawn {
            Ok(spawned) => spawned,
            Err(_) => {
                fail_phase(
                    &mut result,
                    ServerTestPhase::Spawn,
                    ServerTestReason::SpawnFailed,
                );
                return result;
            }
        };
        succeed_phase(&mut result, ServerTestPhase::Spawn, None);

        let initialize_budget = initialization_budget(deadline);
        let initialize = tokio::time::timeout(
            initialize_budget,
            initialize_and_notify(
                &client,
                build_initialize_params(
                    project.root_uri(),
                    command.initialization_options,
                    Some(std::process::id()),
                ),
            ),
        )
        .await;
        match initialize {
            Ok(Ok(capabilities)) => {
                result.negotiated_capabilities = Some(capabilities);
                succeed_phase(&mut result, ServerTestPhase::Initialize, None);
            }
            Ok(Err(_)) => fail_phase(
                &mut result,
                ServerTestPhase::Initialize,
                ServerTestReason::InitializeFailed,
            ),
            Err(_) => fail_phase(
                &mut result,
                ServerTestPhase::Initialize,
                ServerTestReason::InitializeTimedOut,
            ),
        }

        // The later of the caller's deadline and the floor. `max` rather than "the floor when the
        // deadline has passed": a deadline with 50 ms left is as unable to observe a kill as one
        // with none, and the boundary between them is not a distinction worth encoding.
        let cleanup_deadline = deadline.max(Instant::now() + CLEANUP_FLOOR);
        match process.shutdown_protocol(&client, cleanup_deadline).await {
            Ok(outcome) => {
                let _exit_observed = outcome.exit.status;
                let reason = (outcome.disposition == LspShutdownDisposition::Forced)
                    .then_some(ServerTestReason::ForcedTermination);
                succeed_phase(&mut result, ServerTestPhase::Cleanup, reason);
            }
            Err(_) => fail_phase(
                &mut result,
                ServerTestPhase::Cleanup,
                ServerTestReason::CleanupFailed,
            ),
        }
        result
    }
}

struct MinimalProject {
    _directory: TempDir,
    root_uri: String,
}

impl MinimalProject {
    fn create(language: Language) -> Result<Self, ()> {
        let directory = tempfile::tempdir().map_err(|_| ())?;
        for (relative, contents) in language.fixture_files {
            write_file(directory.path(), relative, contents.as_bytes())?;
        }
        let root_uri = Url::from_directory_path(directory.path())
            .map_err(|_| ())?
            .to_string();
        Ok(Self {
            _directory: directory,
            root_uri,
        })
    }

    fn root_uri(&self) -> &str {
        &self.root_uri
    }
}

fn write_file(root: &Path, relative: &str, contents: &[u8]) -> Result<(), ()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| ())?;
    }
    std::fs::write(path, contents).map_err(|_| ())
}

fn protocol_configuration() -> Option<(JsonRpcActorLimits, Arc<LspServerRequestHandler>)> {
    let actor = JsonRpcActorLimits::new(16, 16, 16, 16, 16, 8).ok()?;
    let handler_limits = LspClientRequestLimits::new(16, 32, 16, 32 * 1024).ok()?;
    let handler = LspServerRequestHandler::new(BTreeMap::new(), handler_limits).ok()?;
    Some((actor, Arc::new(handler)))
}

fn initialization_budget(deadline: Instant) -> Duration {
    deadline
        .saturating_duration_since(Instant::now())
        .saturating_sub(Duration::from_millis(300))
}

fn empty_result() -> IsolatedServerTestResult {
    IsolatedServerTestResult {
        phases: [
            ServerTestPhase::Discovery,
            ServerTestPhase::Spawn,
            ServerTestPhase::Initialize,
            ServerTestPhase::Cleanup,
        ]
        .into_iter()
        .map(|phase| ServerTestPhaseResult {
            phase,
            status: ServerTestPhaseStatus::Skipped,
            reason: None,
        })
        .collect(),
        negotiated_capabilities: None,
    }
}

fn succeed_phase(
    result: &mut IsolatedServerTestResult,
    phase: ServerTestPhase,
    reason: Option<ServerTestReason>,
) {
    set_phase(result, phase, ServerTestPhaseStatus::Succeeded, reason);
}

fn fail_phase(
    result: &mut IsolatedServerTestResult,
    phase: ServerTestPhase,
    reason: ServerTestReason,
) {
    set_phase(result, phase, ServerTestPhaseStatus::Failed, Some(reason));
}

fn set_phase(
    result: &mut IsolatedServerTestResult,
    phase: ServerTestPhase,
    status: ServerTestPhaseStatus,
    reason: Option<ServerTestReason>,
) {
    if let Some(current) = result
        .phases
        .iter_mut()
        .find(|current| current.phase == phase)
    {
        current.status = status;
        current.reason = reason;
    }
}
