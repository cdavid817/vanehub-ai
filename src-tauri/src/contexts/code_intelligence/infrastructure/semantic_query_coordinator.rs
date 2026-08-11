use super::diagnostics_cache::{DiagnosticsCache, DiagnosticsReadiness};
use super::document_invalidation::LspDocumentInvalidationQueue;
use super::document_lease::{DocumentLeaseManager, DocumentNotificationSink, PreparedDocument};
use super::document_snapshot::DocumentAdmission;
use super::json_rpc_actor::{JsonRpcError, JsonRpcRequestControl};
use super::position_conversion::{AgentPosition, PositionConverter};
use super::process_registry::ActivationReason;
use super::project_root::ProcessKey;
use super::runtime_process_coordinator::{
    LspProcessAcquisition, LspProcessHandle, LspProcessLaunch, RuntimeProcessCoordinator,
};
use super::semantic_results::SemanticResultNormalizer;
use crate::contexts::code_intelligence::domain::models::{
    LanguageFamily, NormalizedDiagnostic, NormalizedHover, NormalizedLocation, QueryOutcome,
    QueryStatus, SemanticMethod,
};
use lsp_types::{GotoDefinitionResponse, Hover, Location};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

struct LeaseEntry {
    process_id: u64,
    manager: Arc<Mutex<DocumentLeaseManager>>,
    diagnostics: Arc<DiagnosticsCache>,
}

#[derive(Clone)]
struct LeaseResources {
    manager: Arc<Mutex<DocumentLeaseManager>>,
    diagnostics: Arc<DiagnosticsCache>,
}

struct PreparedQuery {
    handle: LspProcessHandle,
    document: PreparedDocument,
    diagnostics: Arc<DiagnosticsCache>,
}

struct QueryFailure {
    status: QueryStatus,
    reason: &'static str,
    server: Option<crate::contexts::code_intelligence::domain::models::ServerKind>,
    language: Option<LanguageFamily>,
    document_version: Option<crate::contexts::code_intelligence::domain::models::DocumentVersion>,
}

impl QueryFailure {
    fn outcome<T>(self) -> QueryOutcome<T> {
        QueryOutcome::degraded_with_identity(
            self.status,
            self.reason,
            self.server,
            self.language,
            self.document_version,
        )
    }
}

#[derive(Clone)]
pub(crate) struct SemanticQueryCoordinator {
    processes: RuntimeProcessCoordinator,
    invalidations: LspDocumentInvalidationQueue,
    leases: Arc<Mutex<HashMap<ProcessKey, LeaseEntry>>>,
    epoch: Instant,
}

impl SemanticQueryCoordinator {
    pub(crate) fn new(
        processes: RuntimeProcessCoordinator,
        invalidations: LspDocumentInvalidationQueue,
    ) -> Self {
        Self {
            processes,
            invalidations,
            leases: Arc::new(Mutex::new(HashMap::new())),
            epoch: Instant::now(),
        }
    }

    pub(crate) async fn find_definition(
        &self,
        launch: LspProcessLaunch,
        language: LanguageFamily,
        relative_path: &str,
        line: u32,
        column: u32,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Vec<NormalizedLocation>> {
        let prepared = match self
            .prepare(launch, language, relative_path, SemanticMethod::Definition)
            .await
        {
            Ok(prepared) => prepared,
            Err(failure) => return failure.outcome(),
        };
        let position = match self.position(&prepared, line, column).await {
            Ok(position) => position,
            Err(failure) => return failure.outcome(),
        };
        let request_started = Instant::now();
        let response: Result<Option<GotoDefinitionResponse>, JsonRpcError> = prepared
            .handle
            .client()
            .request_with_control(
                "textDocument/definition",
                json!({"textDocument": {"uri": prepared.document.uri()}, "position": position}),
                request_control(cancelled),
            )
            .await;
        if response.is_ok() {
            self.processes.record_response(prepared.handle.key()).await;
        } else if let Err(error) = response.as_ref() {
            self.processes
                .record_request_failure(prepared.handle.key(), *error, request_started.elapsed())
                .await;
        }
        self.processes.release_request(prepared.handle.key()).await;
        let response = match response {
            Ok(response) => response,
            Err(error) => return request_failure(&prepared, language, error).outcome(),
        };
        let normalizer = match SemanticResultNormalizer::new(
            prepared.handle.key().session_root_ref(),
            prepared.handle.capabilities().position_encoding,
        ) {
            Ok(normalizer) => normalizer,
            Err(_) => {
                return prepared_failure(&prepared, language, "workspace_unavailable").outcome()
            }
        };
        let normalized = normalizer.definitions(response);
        QueryOutcome::ready_with_metadata(
            normalized.locations,
            prepared.handle.key().server_kind(),
            language,
            prepared.document.version(),
            normalized
                .total
                .min(super::semantic_results::MAX_DEFINITIONS),
            normalized.total,
            normalized.truncated,
            normalized.filtered_count,
        )
    }

    pub(crate) async fn find_references(
        &self,
        launch: LspProcessLaunch,
        language: LanguageFamily,
        relative_path: &str,
        line: u32,
        column: u32,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Vec<NormalizedLocation>> {
        let prepared = match self
            .prepare(launch, language, relative_path, SemanticMethod::References)
            .await
        {
            Ok(prepared) => prepared,
            Err(failure) => return failure.outcome(),
        };
        let position = match self.position(&prepared, line, column).await {
            Ok(position) => position,
            Err(failure) => return failure.outcome(),
        };
        let request_started = Instant::now();
        let response: Result<Option<Vec<Location>>, JsonRpcError> = prepared
            .handle
            .client()
            .request_with_control(
                "textDocument/references",
                json!({
                    "textDocument": {"uri": prepared.document.uri()},
                    "position": position,
                    "context": {"includeDeclaration": true}
                }),
                request_control(cancelled),
            )
            .await;
        if response.is_ok() {
            self.processes.record_response(prepared.handle.key()).await;
        } else if let Err(error) = response.as_ref() {
            self.processes
                .record_request_failure(prepared.handle.key(), *error, request_started.elapsed())
                .await;
        }
        self.processes.release_request(prepared.handle.key()).await;
        let response = match response {
            Ok(response) => response.unwrap_or_default(),
            Err(error) => return request_failure(&prepared, language, error).outcome(),
        };
        let normalizer = match SemanticResultNormalizer::new(
            prepared.handle.key().session_root_ref(),
            prepared.handle.capabilities().position_encoding,
        ) {
            Ok(normalizer) => normalizer,
            Err(_) => {
                return prepared_failure(&prepared, language, "workspace_unavailable").outcome()
            }
        };
        let normalized = normalizer.references(response);
        QueryOutcome::ready_with_metadata(
            normalized.locations,
            prepared.handle.key().server_kind(),
            language,
            prepared.document.version(),
            normalized
                .total
                .min(super::semantic_results::MAX_REFERENCES),
            normalized.total,
            normalized.truncated,
            normalized.filtered_count,
        )
    }

    pub(crate) async fn get_hover(
        &self,
        launch: LspProcessLaunch,
        language: LanguageFamily,
        relative_path: &str,
        line: u32,
        column: u32,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Option<NormalizedHover>> {
        let prepared = match self
            .prepare(launch, language, relative_path, SemanticMethod::Hover)
            .await
        {
            Ok(prepared) => prepared,
            Err(failure) => return failure.outcome(),
        };
        let position = match self.position(&prepared, line, column).await {
            Ok(position) => position,
            Err(failure) => return failure.outcome(),
        };
        let request_started = Instant::now();
        let response: Result<Option<Hover>, JsonRpcError> = prepared
            .handle
            .client()
            .request_with_control(
                "textDocument/hover",
                json!({"textDocument": {"uri": prepared.document.uri()}, "position": position}),
                request_control(cancelled),
            )
            .await;
        if response.is_ok() {
            self.processes.record_response(prepared.handle.key()).await;
        } else if let Err(error) = response.as_ref() {
            self.processes
                .record_request_failure(prepared.handle.key(), *error, request_started.elapsed())
                .await;
        }
        self.processes.release_request(prepared.handle.key()).await;
        let response = match response {
            Ok(response) => response,
            Err(error) => return request_failure(&prepared, language, error).outcome(),
        };
        let normalizer = match SemanticResultNormalizer::new(
            prepared.handle.key().session_root_ref(),
            prepared.handle.capabilities().position_encoding,
        ) {
            Ok(normalizer) => normalizer,
            Err(_) => {
                return prepared_failure(&prepared, language, "workspace_unavailable").outcome()
            }
        };
        let hover = normalizer.hover(prepared.document.text(), response);
        let truncated = hover.as_ref().is_some_and(|value| value.truncated);
        let count = usize::from(hover.is_some());
        QueryOutcome::ready_with_metadata(
            hover,
            prepared.handle.key().server_kind(),
            language,
            prepared.document.version(),
            count,
            count,
            truncated,
            0,
        )
    }

    pub(crate) async fn get_diagnostics(
        &self,
        launch: LspProcessLaunch,
        language: LanguageFamily,
        relative_path: &str,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Vec<NormalizedDiagnostic>> {
        let prepared = match self
            .prepare(launch, language, relative_path, SemanticMethod::Diagnostics)
            .await
        {
            Ok(prepared) => prepared,
            Err(failure) => return failure.outcome(),
        };
        let wait_started = Instant::now();
        let waiting = prepared.diagnostics.wait_for_current(
            prepared.document.uri(),
            prepared.document.version(),
            DiagnosticsReadiness::Ready,
            Duration::from_secs(9),
        );
        tokio::pin!(waiting);
        let result = tokio::select! {
            result = &mut waiting => Some(result),
            () = wait_for_cancellation(cancelled) => None,
        };
        self.processes.release_request(prepared.handle.key()).await;
        let Some(result) = result else {
            self.processes
                .record_diagnostics_wait(prepared.handle.key(), true, wait_started.elapsed())
                .await;
            return QueryOutcome::degraded_with_identity(
                QueryStatus::Failed,
                "generation_cancelled",
                Some(prepared.handle.key().server_kind()),
                Some(language),
                Some(prepared.document.version()),
            );
        };
        if result.status() == QueryStatus::Timeout {
            self.processes
                .record_diagnostics_wait(prepared.handle.key(), false, wait_started.elapsed())
                .await;
        }
        let diagnostics = result
            .snapshot()
            .map(|snapshot| snapshot.diagnostics().to_vec());
        let returned_count = diagnostics.as_ref().map_or(0, Vec::len);
        let reason = match result.status() {
            QueryStatus::Ready => None,
            QueryStatus::Timeout if diagnostics.is_some() => Some("diagnostics_stale"),
            QueryStatus::Timeout => Some("diagnostics_timeout"),
            _ => Some("diagnostics_unavailable"),
        };
        QueryOutcome::status_with_value(
            result.status(),
            diagnostics,
            reason,
            prepared.handle.key().server_kind(),
            language,
            prepared.document.version(),
            result.stale(),
            returned_count,
            result.total(),
            result.truncated(),
            result.filtered_count(),
        )
    }

    async fn prepare(
        &self,
        launch: LspProcessLaunch,
        language: LanguageFamily,
        relative_path: &str,
        method: SemanticMethod,
    ) -> Result<PreparedQuery, QueryFailure> {
        let key = launch.key.clone();
        let server = Some(key.server_kind());
        let acquisition = self
            .processes
            .acquire(launch, ActivationReason::ToolRequest, true)
            .await;
        let handle = match acquisition {
            LspProcessAcquisition::Ready(handle) => handle,
            LspProcessAcquisition::Warming => {
                self.processes.release_request(&key).await;
                return Err(failure(
                    QueryStatus::Warming,
                    "server_starting",
                    server,
                    language,
                ));
            }
            LspProcessAcquisition::Unavailable => {
                self.processes.release_request(&key).await;
                return Err(failure(
                    QueryStatus::Unavailable,
                    "server_unavailable",
                    server,
                    language,
                ));
            }
            LspProcessAcquisition::Failed => {
                self.processes.release_request(&key).await;
                return Err(failure(
                    QueryStatus::Failed,
                    "server_failed",
                    server,
                    language,
                ));
            }
        };
        if !handle.capabilities().supports(method) {
            self.processes.release_request(handle.key()).await;
            return Err(failure(
                QueryStatus::Unavailable,
                "method_unsupported",
                server,
                language,
            ));
        }
        self.apply_invalidations(handle.key().session_root_ref())
            .await;
        let resources = match self.manager(&handle).await {
            Ok(resources) => resources,
            Err(reason) => {
                self.processes.release_request(handle.key()).await;
                return Err(failure(QueryStatus::Failed, reason, server, language));
            }
        };
        let mut manager = resources.manager.lock().await;
        let document = manager.prepare(relative_path, self.epoch.elapsed()).await;
        let count = manager.active_count();
        drop(manager);
        self.processes
            .set_document_leases(handle.key(), count)
            .await;
        match document {
            Ok(document) => Ok(PreparedQuery {
                handle,
                document,
                diagnostics: resources.diagnostics,
            }),
            Err(_) => {
                self.processes.release_request(handle.key()).await;
                Err(failure(
                    QueryStatus::Failed,
                    "document_unavailable",
                    server,
                    language,
                ))
            }
        }
    }

    async fn manager(&self, handle: &LspProcessHandle) -> Result<LeaseResources, &'static str> {
        let mut leases = self.leases.lock().await;
        if let Some(entry) = leases.get(handle.key()) {
            if entry.process_id == handle.id() {
                return Ok(LeaseResources {
                    manager: entry.manager.clone(),
                    diagnostics: entry.diagnostics.clone(),
                });
            }
        }
        let admission = DocumentAdmission::new(handle.key().session_root_ref())
            .map_err(|_| "workspace_unavailable")?;
        let sink: Arc<dyn DocumentNotificationSink> = Arc::new(handle.client());
        let manager = Arc::new(Mutex::new(DocumentLeaseManager::new(
            admission,
            handle.capabilities().document_sync,
            handle.capabilities().position_encoding,
            sink,
        )));
        let diagnostics = Arc::new(
            DiagnosticsCache::new(
                handle.key().session_root_ref(),
                handle.capabilities().position_encoding,
            )
            .map_err(|_| "workspace_unavailable")?,
        );
        self.processes.notifications().register_diagnostics(
            handle.id(),
            manager.clone(),
            diagnostics.clone(),
        );
        leases.insert(
            handle.key().clone(),
            LeaseEntry {
                process_id: handle.id(),
                manager: manager.clone(),
                diagnostics: diagnostics.clone(),
            },
        );
        Ok(LeaseResources {
            manager,
            diagnostics,
        })
    }

    async fn apply_invalidations(&self, workspace: &std::path::Path) {
        let invalidations = self.invalidations.drain_workspace(workspace);
        if invalidations.is_empty() {
            return;
        }
        let managers = self
            .leases
            .lock()
            .await
            .iter()
            .filter(|(key, _)| key.session_root_ref() == workspace)
            .map(|(_, entry)| entry.manager.clone())
            .collect::<Vec<_>>();
        for manager in managers {
            let mut manager = manager.lock().await;
            for relative_path in &invalidations {
                manager.invalidate(relative_path);
            }
        }
    }

    async fn position(
        &self,
        prepared: &PreparedQuery,
        line: u32,
        column: u32,
    ) -> Result<lsp_types::Position, QueryFailure> {
        match PositionConverter::new(
            prepared.document.text(),
            prepared.handle.capabilities().position_encoding,
        )
        .agent_to_lsp(AgentPosition::new(line, column))
        {
            Ok(position) => Ok(position),
            Err(_) => {
                self.processes.release_request(prepared.handle.key()).await;
                Err(prepared_failure(
                    prepared,
                    language_for(prepared),
                    "invalid_position",
                ))
            }
        }
    }
}

async fn wait_for_cancellation(cancelled: Arc<AtomicBool>) {
    while !cancelled.load(Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn language_for(prepared: &PreparedQuery) -> LanguageFamily {
    match prepared.handle.key().server_kind() {
        crate::contexts::code_intelligence::domain::models::ServerKind::RustAnalyzer => {
            LanguageFamily::Rust
        }
        crate::contexts::code_intelligence::domain::models::ServerKind::TypeScriptLanguageServer => {
            LanguageFamily::TypeScriptJavaScript
        }
    }
}

fn request_control(cancelled: Arc<AtomicBool>) -> JsonRpcRequestControl {
    JsonRpcRequestControl::standard(cancelled)
}

fn request_failure(
    prepared: &PreparedQuery,
    language: LanguageFamily,
    error: JsonRpcError,
) -> QueryFailure {
    let (status, reason) = match error {
        JsonRpcError::Timeout => (QueryStatus::Timeout, "request_timeout"),
        JsonRpcError::Cancelled => (QueryStatus::Failed, "generation_cancelled"),
        JsonRpcError::ActorStopped => (QueryStatus::Unavailable, "server_unavailable"),
        _ => (QueryStatus::Failed, "request_failed"),
    };
    let mut failure = prepared_failure(prepared, language, reason);
    failure.status = status;
    failure
}

fn prepared_failure(
    prepared: &PreparedQuery,
    language: LanguageFamily,
    reason: &'static str,
) -> QueryFailure {
    QueryFailure {
        status: QueryStatus::Failed,
        reason,
        server: Some(prepared.handle.key().server_kind()),
        language: Some(language),
        document_version: Some(prepared.document.version()),
    }
}

fn failure(
    status: QueryStatus,
    reason: &'static str,
    server: Option<crate::contexts::code_intelligence::domain::models::ServerKind>,
    language: LanguageFamily,
) -> QueryFailure {
    QueryFailure {
        status,
        reason,
        server,
        language: Some(language),
        document_version: None,
    }
}
