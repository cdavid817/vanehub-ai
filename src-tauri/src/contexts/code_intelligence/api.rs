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
pub(crate) use super::domain::models::{
    ConfigurationFingerprint, DiagnosticSeverity, DocumentSyncMode, LanguageFamily,
    NegotiatedCapabilities, NormalizedDiagnostic, NormalizedHover, NormalizedLocation,
    NormalizedRange, PositionEncoding, ProcessState, QueryOutcome, QueryStatus, ServerKind,
    WorkspaceTrust,
};
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
    #[error("code-intelligence storage operation failed")]
    Storage,
    #[error("language-server shutdown did not complete")]
    ShutdownFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiscoveredServer {
    pub(crate) language: LanguageFamily,
    pub(crate) server: ServerKind,
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
    pub(crate) language: LanguageFamily,
    pub(crate) server: ServerKind,
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
        [LanguageFamily::Rust, LanguageFamily::TypeScriptJavaScript]
            .into_iter()
            .map(|language| {
                let language_configuration = configuration
                    .languages
                    .get(&language)
                    .ok_or(CodeIntelligenceApiError::InvalidConfiguration)?;
                let discovery = self.discovery.discover(
                    language.server_kind(),
                    language_configuration
                        .executable_override
                        .as_deref()
                        .map(Path::new),
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
            && configuration.languages.iter().any(|(language, settings)| {
                settings.enabled
                    && self
                        .discovery
                        .discover(
                            language.server_kind(),
                            settings.executable_override.as_deref().map(Path::new),
                        )
                        .availability()
                        == DiscoveryAvailability::Available
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
        let Some(language) = language_for_path(Path::new(relative_path)) else {
            return unavailable_query("unsupported_language", None);
        };
        let Ok(launch) = self.process_launch(workspace_root, relative_path) else {
            return unavailable_query("not_configured", Some(language));
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
        let Some(language) = language_for_path(Path::new(relative_path)) else {
            return unavailable_query("unsupported_language", None);
        };
        let Ok(launch) = self.process_launch(workspace_root, relative_path) else {
            return unavailable_query("not_configured", Some(language));
        };
        self.semantic_queries
            .find_references(launch, language, relative_path, line, column, cancelled)
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
        let Some(language) = language_for_path(Path::new(relative_path)) else {
            return unavailable_query("unsupported_language", None);
        };
        let Ok(launch) = self.process_launch(workspace_root, relative_path) else {
            return unavailable_query("not_configured", Some(language));
        };
        self.semantic_queries
            .get_hover(launch, language, relative_path, line, column, cancelled)
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
        if cancelled.load(Ordering::Acquire) {
            return QueryOutcome::degraded_with_identity(
                QueryStatus::Failed,
                "generation_cancelled",
                Some(language.server_kind()),
                Some(language),
                None,
            );
        }
        let Ok(launch) = self.process_launch(workspace_root, relative_path) else {
            return unavailable_query("not_configured", Some(language));
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
        language: LanguageFamily,
    ) -> Result<IsolatedServerTestResult, CodeIntelligenceApiError> {
        let configuration = self.configuration()?;
        let language_configuration = configuration
            .languages
            .get(&language)
            .ok_or(CodeIntelligenceApiError::InvalidConfiguration)?;
        let discovery = self.discovery.discover(
            language.server_kind(),
            language_configuration
                .executable_override
                .as_deref()
                .map(Path::new),
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
                let language = match snapshot.key.server_kind() {
                    ServerKind::RustAnalyzer => LanguageFamily::Rust,
                    ServerKind::TypeScriptLanguageServer => LanguageFamily::TypeScriptJavaScript,
                };
                ServerStatus {
                    language,
                    server: snapshot.key.server_kind(),
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
            .languages
            .get(&language)
            .filter(|settings| settings.enabled)
            .ok_or(CodeIntelligenceApiError::ConfigurationUnavailable)?;
        let discovery = self.discovery.discover(
            language.server_kind(),
            settings.executable_override.as_deref().map(Path::new),
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
        .map_err(|_| CodeIntelligenceApiError::InvalidWorkspace)?;
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
            language.server_kind(),
            fingerprint,
        )
        .map_err(|_| CodeIntelligenceApiError::InvalidWorkspace)?;
        Ok(super::infrastructure::LspProcessLaunch {
            key,
            executable: executable.to_string_lossy().into_owned(),
            arguments: discovery
                .arguments()
                .iter()
                .map(|argument| (*argument).to_string())
                .collect(),
            initialization_options: settings.initialization_options.clone(),
        })
    }
}

fn unavailable_query<T>(reason: &'static str, language: Option<LanguageFamily>) -> QueryOutcome<T> {
    QueryOutcome::degraded_with_identity(
        QueryStatus::Unavailable,
        reason,
        language.map(LanguageFamily::server_kind),
        language,
        None,
    )
}

fn language_for_path(path: &Path) -> Option<LanguageFamily> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "rs" => Some(LanguageFamily::Rust),
        "ts" | "tsx" | "js" | "jsx" | "mts" | "cts" | "mjs" | "cjs" => {
            Some(LanguageFamily::TypeScriptJavaScript)
        }
        _ => None,
    }
}

fn configuration_fingerprint(
    language: LanguageFamily,
    executable: &Path,
    arguments: &[&str],
    initialization_options: &serde_json::Value,
    trust_revision: u64,
) -> Result<ConfigurationFingerprint, CodeIntelligenceApiError> {
    let mut digest = Sha256::new();
    digest.update(language.as_id().as_bytes());
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
    language: LanguageFamily,
    discovery: super::infrastructure::ServerDiscoveryResult,
) -> DiscoveredServer {
    DiscoveredServer {
        language,
        server: discovery.server_kind(),
        availability: discovery.availability(),
        executable_path: discovery
            .executable()
            .map(|path| path.to_string_lossy().into_owned()),
        arguments: discovery
            .arguments()
            .iter()
            .map(|argument| (*argument).to_string())
            .collect(),
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
