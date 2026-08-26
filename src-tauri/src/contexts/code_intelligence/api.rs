//! Public contracts exposed to the rest of the application.

use super::application::ports::{LspConfigurationRepository, WorkspaceTrustRepository};
use crate::contexts::operations::api::DiagnosticLogPort;
use crate::platform::database::{DatabaseError, NativeDatabase};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;

pub(crate) use super::domain::configuration::{LanguageConfiguration, LspConfiguration};
// Published for callers that hold a language id without a registry lookup; only tests need it in
// this build, and gating the lint keeps that from reading as a dead export.
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use super::domain::language_id::LspLanguageId;
pub(crate) use super::domain::models::{
    resolve_language, CallDirection, ConfigurationFingerprint, DiagnosticSeverity,
    DocumentSyncMode, Language, NegotiatedCapabilities, NormalizedCallRelation,
    NormalizedDiagnostic, NormalizedHover, NormalizedLocation, NormalizedRange, NormalizedSymbol,
    PositionEncoding, ProcessState, QueryOutcome, QueryStatus, WorkspaceTrust,
};
// Published so a command-layer test can build a negotiated record from the client's own method
// list rather than restating it. Only tests need it in this build.
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use super::domain::models::SemanticMethod;
pub(crate) use super::domain::registry::{definition_for_extension, LANGUAGE_DEFINITIONS};
pub(crate) use super::infrastructure::{
    DiscoveryAvailability, DiscoveryReason, IsolatedServerTestResult, ServerTestPhase,
    ServerTestPhaseStatus, ServerTestReason,
};

const SERVER_TEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodeIntelligenceApiError {
    #[error("code-intelligence configuration is unavailable")]
    ConfigurationUnavailable,
    #[error("invalid LSP configuration")]
    InvalidConfiguration,
    #[error("invalid workspace root")]
    InvalidWorkspace,
    /// Distinct from `InvalidWorkspace` so the boundary can say the language's required project
    /// marker is missing. Collapsing them sends a user to the settings page when the thing to fix
    /// is their build system.
    #[error("the language's required project marker is missing from the workspace")]
    MissingProjectMarker,
    #[error("code-intelligence storage operation failed")]
    Storage,
    #[error("language-server shutdown did not complete")]
    ShutdownFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiscoveredServer {
    pub(crate) language: Language,
    pub(crate) availability: DiscoveryAvailability,
    pub(crate) executable_path: Option<String>,
    pub(crate) arguments: Vec<String>,
    pub(crate) reason: Option<DiscoveryReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum ServerStatusReason {
    RestartExhausted,
    ProtocolLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ServerStatus {
    pub(crate) language: Language,
    pub(crate) relative_project_root: String,
    pub(crate) state: ProcessState,
    pub(crate) restart_count: u32,
    pub(crate) last_response_at: Option<String>,
    pub(crate) diagnostic_count: usize,
    pub(crate) reason: Option<ServerStatusReason>,
    pub(crate) negotiated_capabilities: Option<NegotiatedCapabilities>,
}

#[derive(Clone)]
pub(crate) struct CodeIntelligenceApi {
    repository: Option<super::infrastructure::SqliteCodeIntelligenceRepository>,
    discovery: super::infrastructure::ServerDiscovery,
    processes: super::infrastructure::RuntimeProcessCoordinator,
    semantic_queries: super::infrastructure::SemanticQueryCoordinator,
    document_invalidations: super::infrastructure::LspDocumentInvalidationQueue,
    maintenance_started: Arc<AtomicBool>,
}

impl CodeIntelligenceApi {
    pub(crate) fn from_database(
        database: NativeDatabase,
        logging: Arc<dyn DiagnosticLogPort>,
    ) -> Self {
        let shutdown = super::infrastructure::LspShutdownCoordinator::default();
        let diagnostics = super::infrastructure::LspDiagnosticLogger::new(logging);
        let processes = super::infrastructure::RuntimeProcessCoordinator::new(
            shutdown.clone(),
            super::infrastructure::LifecyclePolicy::default(),
            diagnostics,
        );
        let document_invalidations = super::infrastructure::LspDocumentInvalidationQueue::default();
        Self {
            repository: Some(
                super::infrastructure::SqliteCodeIntelligenceRepository::new(database),
            ),
            discovery: super::infrastructure::ServerDiscovery::new(Arc::new(
                super::infrastructure::SystemNativeExecutableLocator,
            )),
            semantic_queries: super::infrastructure::SemanticQueryCoordinator::new(
                processes.clone(),
                document_invalidations.clone(),
            ),
            processes,
            document_invalidations,
            maintenance_started: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn configuration(&self) -> Result<LspConfiguration, CodeIntelligenceApiError> {
        self.repository()?
            .load_configuration()
            .map_err(map_domain_error)
    }

    pub(crate) fn save_configuration(
        &self,
        configuration: &LspConfiguration,
    ) -> Result<(), CodeIntelligenceApiError> {
        let repository = self.repository()?;
        let previous = repository.load_configuration().ok();
        repository
            .save_configuration(configuration)
            .map_err(map_domain_error)?;
        if previous.as_ref() != Some(configuration) {
            let processes = self.processes.clone();
            tauri::async_runtime::spawn(async move {
                processes.configuration_replaced().await;
            });
        }
        Ok(())
    }

    pub(crate) fn list_workspace_trust(
        &self,
    ) -> Result<Vec<WorkspaceTrust>, CodeIntelligenceApiError> {
        self.repository()?
            .list_workspace_trust()
            .map_err(map_domain_error)
    }

    pub(crate) fn update_workspace_trust(
        &self,
        workspace_root: &Path,
        trusted: bool,
    ) -> Result<WorkspaceTrust, CodeIntelligenceApiError> {
        let trust = self
            .repository()?
            .set_workspace_trust(workspace_root, trusted)
            .map_err(map_domain_error)?;
        if !trust.is_trusted() {
            let processes = self.processes.clone();
            let canonical_root = std::path::PathBuf::from(trust.canonical_root());
            tauri::async_runtime::spawn(async move {
                processes.revoke_workspace(&canonical_root).await;
            });
        }
        Ok(trust)
    }

    pub(crate) fn discover_servers(
        &self,
    ) -> Result<Vec<DiscoveredServer>, CodeIntelligenceApiError> {
        let configuration = self.configuration()?;
        LANGUAGE_DEFINITIONS
            .iter()
            .map(|language| {
                // A language the stored configuration has never seen -- one this build added --
                // is discovered against its defaults rather than refused.
                let defaults = LanguageConfiguration::default();
                let language_configuration = configuration
                    .language(&language.language_id())
                    .unwrap_or(&defaults);
                let discovery = self.discovery.discover(
                    language,
                    language_configuration
                        .executable_override
                        .as_deref()
                        .map(Path::new),
                    language_configuration.startup_arguments.as_ref(),
                );
                Ok(discovery_view(language, discovery))
            })
            .collect()
    }

    pub(crate) fn is_agent_workspace_available(&self, workspace_root: &Path) -> bool {
        let Ok(canonical_root) = std::fs::canonicalize(workspace_root) else {
            return false;
        };
        let Ok(configuration) = self.configuration() else {
            return false;
        };
        if !configuration.enabled {
            return false;
        }
        let trusted = self.list_workspace_trust().is_ok_and(|workspaces| {
            workspaces.iter().any(|workspace| {
                workspace.is_trusted()
                    && Path::new(workspace.canonical_root()) == canonical_root.as_path()
            })
        });
        trusted
            && configuration
                .languages
                .iter()
                .any(|(language_id, settings)| {
                    settings.enabled
                        && resolve_language(language_id.as_str()).is_some_and(|language| {
                            self.discovery
                                .discover(
                                    language,
                                    settings.executable_override.as_deref().map(Path::new),
                                    settings.startup_arguments.as_ref(),
                                )
                                .availability()
                                == DiscoveryAvailability::Available
                        })
                })
    }

    pub(crate) fn prewarm_workspace(&self, workspace_root: &Path) {
        let api = self.clone();
        let workspace_root = workspace_root.to_path_buf();
        tauri::async_runtime::spawn(async move {
            for relative_path in prewarm_candidates(&workspace_root) {
                if let Ok(launch) = api.process_launch(&workspace_root, &relative_path) {
                    let _ = api
                        .processes
                        .acquire(
                            launch,
                            super::infrastructure::ActivationReason::Prewarm {
                                inventory: true,
                                manifest: false,
                            },
                            true,
                        )
                        .await;
                }
            }
        });
    }

    pub(crate) async fn find_definition(
        &self,
        workspace_root: &Path,
        relative_path: &str,
        line: u32,
        column: u32,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Vec<NormalizedLocation>> {
        let (language, launch) = match self.resolve_query(workspace_root, relative_path) {
            Ok(resolved) => resolved,
            Err(outcome) => return outcome,
        };
        self.semantic_queries
            .find_definition(launch, language, relative_path, line, column, cancelled)
            .await
    }

    pub(crate) async fn find_references(
        &self,
        workspace_root: &Path,
        relative_path: &str,
        line: u32,
        column: u32,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Vec<NormalizedLocation>> {
        let (language, launch) = match self.resolve_query(workspace_root, relative_path) {
            Ok(resolved) => resolved,
            Err(outcome) => return outcome,
        };
        self.semantic_queries
            .find_references(launch, language, relative_path, line, column, cancelled)
            .await
    }

    // No caller until the tool catalog wires it up. `expect` rather than `allow` so the attribute
    // fails the build once it is wired, instead of outliving its reason in silence.
    #[expect(dead_code, reason = "tool catalog wiring lands with the Agent surface")]
    pub(crate) async fn find_type_definition(
        &self,
        workspace_root: &Path,
        relative_path: &str,
        line: u32,
        column: u32,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Vec<NormalizedLocation>> {
        let (language, launch) = match self.resolve_query(workspace_root, relative_path) {
            Ok(resolved) => resolved,
            Err(outcome) => return outcome,
        };
        self.semantic_queries
            .find_type_definition(launch, language, relative_path, line, column, cancelled)
            .await
    }

    #[expect(dead_code, reason = "tool catalog wiring lands with the Agent surface")]
    pub(crate) async fn find_implementations(
        &self,
        workspace_root: &Path,
        relative_path: &str,
        line: u32,
        column: u32,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Vec<NormalizedLocation>> {
        let (language, launch) = match self.resolve_query(workspace_root, relative_path) {
            Ok(resolved) => resolved,
            Err(outcome) => return outcome,
        };
        self.semantic_queries
            .find_implementations(launch, language, relative_path, line, column, cancelled)
            .await
    }

    pub(crate) async fn get_hover(
        &self,
        workspace_root: &Path,
        relative_path: &str,
        line: u32,
        column: u32,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Option<NormalizedHover>> {
        let (language, launch) = match self.resolve_query(workspace_root, relative_path) {
            Ok(resolved) => resolved,
            Err(outcome) => return outcome,
        };
        self.semantic_queries
            .get_hover(launch, language, relative_path, line, column, cancelled)
            .await
    }

    /// `relative_path` anchors the search rather than scoping it. LSP has no notion of "the
    /// repository": a server indexes one project root, and a repository can hold several, so the
    /// file the Agent is working in is what says which index to search.
    #[expect(dead_code, reason = "tool catalog wiring lands with the Agent surface")]
    pub(crate) async fn find_workspace_symbols(
        &self,
        workspace_root: &Path,
        relative_path: &str,
        query: &str,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Vec<NormalizedSymbol>> {
        let (language, launch) = match self.resolve_query(workspace_root, relative_path) {
            Ok(resolved) => resolved,
            Err(outcome) => return outcome,
        };
        self.semantic_queries
            .find_workspace_symbols(launch, language, query, cancelled)
            .await
    }

    #[expect(dead_code, reason = "tool catalog wiring lands with the Agent surface")]
    pub(crate) async fn find_call_hierarchy(
        &self,
        workspace_root: &Path,
        relative_path: &str,
        line: u32,
        column: u32,
        direction: CallDirection,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Vec<NormalizedCallRelation>> {
        let (language, launch) = match self.resolve_query(workspace_root, relative_path) {
            Ok(resolved) => resolved,
            Err(outcome) => return outcome,
        };
        self.semantic_queries
            .find_call_hierarchy(
                launch,
                language,
                relative_path,
                super::infrastructure::AgentPosition::new(line, column),
                direction,
                cancelled,
            )
            .await
    }

    #[expect(dead_code, reason = "tool catalog wiring lands with the Agent surface")]
    pub(crate) async fn get_document_symbols(
        &self,
        workspace_root: &Path,
        relative_path: &str,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Vec<NormalizedSymbol>> {
        let (language, launch) = match self.resolve_query(workspace_root, relative_path) {
            Ok(resolved) => resolved,
            Err(outcome) => return outcome,
        };
        self.semantic_queries
            .get_document_symbols(launch, language, relative_path, cancelled)
            .await
    }

    pub(crate) async fn get_diagnostics(
        &self,
        workspace_root: &Path,
        relative_path: &str,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Vec<NormalizedDiagnostic>> {
        let Some(language) = language_for_path(Path::new(relative_path)) else {
            return unavailable_query("unsupported_language", None);
        };
        // Checked before the launch so a generation cancelled while queued never spawns a server.
        if cancelled.load(Ordering::Acquire) {
            return QueryOutcome::degraded_with_identity(
                QueryStatus::Failed,
                "generation_cancelled",
                Some(language),
                None,
            );
        }
        let launch = match self.process_launch(workspace_root, relative_path) {
            Ok(launch) => launch,
            Err(error) => return unavailable_query(launch_reason(error), Some(language)),
        };
        self.semantic_queries
            .get_diagnostics(launch, language, relative_path, cancelled)
            .await
    }

    pub(crate) fn start_maintenance(&self) {
        if self.maintenance_started.swap(true, Ordering::AcqRel) {
            return;
        }
        let processes = self.processes.clone();
        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                processes.tick().await;
            }
        });
    }

    pub(crate) async fn test_server(
        &self,
        language: Language,
    ) -> Result<IsolatedServerTestResult, CodeIntelligenceApiError> {
        let configuration = self.configuration()?;
        let defaults = LanguageConfiguration::default();
        let language_configuration = configuration
            .language(&language.language_id())
            .unwrap_or(&defaults);
        let discovery = self.discovery.discover(
            language,
            language_configuration
                .executable_override
                .as_deref()
                .map(Path::new),
            language_configuration.startup_arguments.as_ref(),
        );
        let command = super::infrastructure::ServerTestCommand::from_discovery(
            &discovery,
            language_configuration.initialization_options.clone(),
        );
        Ok(super::infrastructure::IsolatedServerTester::run(command, SERVER_TEST_TIMEOUT).await)
    }

    pub(crate) async fn server_statuses(
        &self,
    ) -> Result<Vec<ServerStatus>, CodeIntelligenceApiError> {
        self.repository()?;
        Ok(self
            .processes
            .status_snapshots()
            .await
            .into_iter()
            .map(|snapshot| {
                let relative_project_root = snapshot
                    .key
                    .project_root_ref()
                    .strip_prefix(snapshot.key.session_root_ref())
                    .ok()
                    .filter(|path| !path.as_os_str().is_empty())
                    .map(|path| path.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_else(|| ".".to_owned());
                ServerStatus {
                    language: snapshot.key.language(),
                    relative_project_root,
                    state: snapshot.process.state,
                    restart_count: snapshot.process.restart_count,
                    last_response_at: snapshot.last_response_at,
                    diagnostic_count: snapshot.diagnostic_count,
                    reason: (snapshot.process.state == ProcessState::Failed)
                        .then_some(ServerStatusReason::RestartExhausted),
                    negotiated_capabilities: snapshot.capabilities,
                }
            })
            .collect())
    }

    pub(crate) async fn shutdown(&self, deadline: Instant) -> Result<(), CodeIntelligenceApiError> {
        let summary = self.processes.shutdown_all(deadline).await;
        if summary.failed == 0 {
            Ok(())
        } else {
            Err(CodeIntelligenceApiError::ShutdownFailed)
        }
    }

    pub(crate) fn invalidate_document_lease(&self, workspace: &Path, relative_path: &str) {
        self.document_invalidations
            .publish(workspace, relative_path);
    }

    fn repository(
        &self,
    ) -> Result<&super::infrastructure::SqliteCodeIntelligenceRepository, CodeIntelligenceApiError>
    {
        self.repository
            .as_ref()
            .ok_or(CodeIntelligenceApiError::ConfigurationUnavailable)
    }

    /// The language and the launch every semantic entry point needs, or the outcome to return
    /// instead. Sharing it means a new entry point cannot reach a server picked for the wrong
    /// language, which is the failure that looks like the server misbehaving.
    fn resolve_query<T>(
        &self,
        workspace_root: &Path,
        relative_path: &str,
    ) -> Result<(Language, super::infrastructure::LspProcessLaunch), QueryOutcome<T>> {
        let Some(language) = language_for_path(Path::new(relative_path)) else {
            return Err(unavailable_query("unsupported_language", None));
        };
        match self.process_launch(workspace_root, relative_path) {
            Ok(launch) => Ok((language, launch)),
            Err(error) => Err(unavailable_query(launch_reason(error), Some(language))),
        }
    }

    fn process_launch(
        &self,
        workspace_root: &Path,
        relative_path: &str,
    ) -> Result<super::infrastructure::LspProcessLaunch, CodeIntelligenceApiError> {
        let canonical_root = workspace_root
            .canonicalize()
            .map_err(|_| CodeIntelligenceApiError::InvalidWorkspace)?;
        let trust = self
            .list_workspace_trust()?
            .into_iter()
            .find(|trust| Path::new(trust.canonical_root()) == canonical_root)
            .filter(WorkspaceTrust::is_trusted)
            .ok_or(CodeIntelligenceApiError::InvalidWorkspace)?;
        let language = language_for_path(Path::new(relative_path))
            .ok_or(CodeIntelligenceApiError::InvalidWorkspace)?;
        let configuration = self.configuration()?;
        if !configuration.enabled {
            return Err(CodeIntelligenceApiError::ConfigurationUnavailable);
        }
        let settings = configuration
            .language(&language.language_id())
            .filter(|settings| settings.enabled)
            .ok_or(CodeIntelligenceApiError::ConfigurationUnavailable)?;
        let discovery = self.discovery.discover(
            language,
            settings.executable_override.as_deref().map(Path::new),
            settings.startup_arguments.as_ref(),
        );
        let executable = discovery
            .executable()
            .ok_or(CodeIntelligenceApiError::ConfigurationUnavailable)?;
        let document = canonical_root.join(relative_path);
        let project_root = super::infrastructure::ProjectRootResolver::resolve(
            &canonical_root,
            &document,
            language,
        )
        .map_err(|error| match error {
            super::infrastructure::ProjectRootError::RequiredMarkerMissing => {
                CodeIntelligenceApiError::MissingProjectMarker
            }
            _ => CodeIntelligenceApiError::InvalidWorkspace,
        })?;
        let fingerprint = configuration_fingerprint(
            language,
            executable,
            discovery.arguments(),
            &settings.initialization_options,
            trust.revision(),
        )?;
        let key = super::infrastructure::ProcessKey::new(
            &canonical_root,
            &project_root,
            language,
            fingerprint,
        )
        .map_err(|_| CodeIntelligenceApiError::InvalidWorkspace)?;
        Ok(super::infrastructure::LspProcessLaunch {
            key,
            executable: executable.to_string_lossy().into_owned(),
            arguments: discovery.arguments().to_vec(),
            initialization_options: settings.initialization_options.clone(),
        })
    }
}

/// A launch that never happened still has to say why. Every failure used to read as
/// `not_configured`, which is actively misleading for a workspace whose configuration is fine and
/// whose build system has simply not produced the marker the language needs.
const fn launch_reason(error: CodeIntelligenceApiError) -> &'static str {
    match error {
        CodeIntelligenceApiError::MissingProjectMarker => "missing_project_marker",
        _ => "not_configured",
    }
}

fn unavailable_query<T>(reason: &'static str, language: Option<Language>) -> QueryOutcome<T> {
    QueryOutcome::degraded_with_identity(QueryStatus::Unavailable, reason, language, None)
}

/// Resolves through the same registry mapping document admission uses. The two lists used to be
/// written separately and had drifted: this one accepted `.mts` and `.cts`, which admission then
/// refused, so such a file passed the gate only to fail one step later.
fn language_for_path(path: &Path) -> Option<Language> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    definition_for_extension(&extension).map(|(language, _)| language)
}

pub(super) fn configuration_fingerprint(
    language: Language,
    executable: &Path,
    arguments: &[String],
    initialization_options: &serde_json::Value,
    trust_revision: u64,
) -> Result<ConfigurationFingerprint, CodeIntelligenceApiError> {
    let mut digest = Sha256::new();
    digest.update(language.id.as_bytes());
    digest.update(executable.to_string_lossy().as_bytes());
    for argument in arguments {
        digest.update(argument.as_bytes());
        digest.update([0]);
    }
    digest.update(
        serde_json::to_vec(initialization_options)
            .map_err(|_| CodeIntelligenceApiError::InvalidConfiguration)?,
    );
    digest.update(trust_revision.to_le_bytes());
    let digest = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    ConfigurationFingerprint::new(format!("sha256:{digest}")).map_err(map_domain_error)
}

fn prewarm_candidates(workspace_root: &Path) -> Vec<String> {
    const MAX_ENTRIES: usize = 512;
    const MAX_DEPTH: usize = 4;
    let mut stack = vec![(workspace_root.to_path_buf(), 0_usize)];
    let mut candidates = std::collections::BTreeMap::new();
    let mut inspected = 0_usize;
    while let Some((directory, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            if inspected >= MAX_ENTRIES || candidates.len() == 2 {
                break;
            }
            inspected += 1;
            let path = entry.path();
            let name = entry.file_name();
            if name.to_string_lossy().starts_with('.') {
                continue;
            }
            if path.is_dir() && depth < MAX_DEPTH {
                stack.push((path, depth + 1));
            } else if path.is_file() {
                if let Some(language) = language_for_path(&path) {
                    if let Ok(relative) = path.strip_prefix(workspace_root) {
                        candidates
                            .entry(language)
                            .or_insert_with(|| relative.to_string_lossy().replace('\\', "/"));
                    }
                }
            }
        }
    }
    candidates.into_values().collect()
}

fn discovery_view(
    language: Language,
    discovery: super::infrastructure::ServerDiscoveryResult,
) -> DiscoveredServer {
    DiscoveredServer {
        language,
        availability: discovery.availability(),
        executable_path: discovery
            .executable()
            .map(|path| path.to_string_lossy().into_owned()),
        arguments: discovery.arguments().to_vec(),
        reason: discovery.reason(),
    }
}

fn map_domain_error(error: super::domain::models::DomainModelError) -> CodeIntelligenceApiError {
    use super::domain::models::DomainModelError;
    match error {
        DomainModelError::InvalidWorkspaceRoot => CodeIntelligenceApiError::InvalidWorkspace,
        DomainModelError::Storage => CodeIntelligenceApiError::Storage,
        _ => CodeIntelligenceApiError::InvalidConfiguration,
    }
}

pub(crate) fn apply_schema(connection: &Connection) -> Result<(), DatabaseError> {
    super::infrastructure::apply_schema(connection)
}

pub(crate) fn apply_language_registry_schema(connection: &Connection) -> Result<(), DatabaseError> {
    super::infrastructure::apply_language_registry_schema(connection)
}
