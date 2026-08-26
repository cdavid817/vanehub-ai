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
use super::semantic_results::{NormalizedLocations, SemanticResultNormalizer};
use crate::contexts::code_intelligence::domain::models::{
    Language, NormalizedDiagnostic, NormalizedHover, NormalizedLocation, QueryOutcome, QueryStatus,
    SemanticMethod,
};
use lsp_types::{Hover, Location};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
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

/// What every position-based query needs from its caller. Named so the helper below stays inside
/// clippy's argument budget, and so adding a method does not re-thread six parameters by hand.
struct PositionRequest<'a> {
    launch: LspProcessLaunch,
    language: Language,
    relative_path: &'a str,
    line: u32,
    column: u32,
    cancelled: Arc<AtomicBool>,
}

struct QueryFailure {
    status: QueryStatus,
    reason: &'static str,
    language: Option<Language>,
    document_version: Option<crate::contexts::code_intelligence::domain::models::DocumentVersion>,
}

impl QueryFailure {
    fn outcome<T>(self) -> QueryOutcome<T> {
        QueryOutcome::degraded_with_identity(
            self.status,
            self.reason,
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
        language: Language,
        relative_path: &str,
        line: u32,
        column: u32,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Vec<NormalizedLocation>> {
        let request = PositionRequest {
            launch,
            language,
            relative_path,
            line,
            column,
            cancelled,
        };
        self.located_query(
            request,
            SemanticMethod::Definition,
            |normalizer, response| normalizer.definitions(response),
        )
        .await
    }

    pub(crate) async fn find_references(
        &self,
        launch: LspProcessLaunch,
        language: Language,
        relative_path: &str,
        line: u32,
        column: u32,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Vec<NormalizedLocation>> {
        let request = PositionRequest {
            launch,
            language,
            relative_path,
            line,
            column,
            cancelled,
        };
        self.located_query(
            request,
            SemanticMethod::References,
            |normalizer, response: Option<Vec<Location>>| {
                normalizer.references(response.unwrap_or_default())
            },
        )
        .await
    }

    pub(crate) async fn get_hover(
        &self,
        launch: LspProcessLaunch,
        language: Language,
        relative_path: &str,
        line: u32,
        column: u32,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Option<NormalizedHover>> {
        let request = PositionRequest {
            launch,
            language,
            relative_path,
            line,
            column,
            cancelled,
        };
        let (prepared, response) = match self
            .position_query::<Option<Hover>>(request, SemanticMethod::Hover)
            .await
        {
            Ok(result) => result,
            Err(failure) => return failure.outcome(),
        };
        let normalizer = match normalizer_for(&prepared, language) {
            Ok(normalizer) => normalizer,
            Err(failure) => return failure.outcome(),
        };
        let hover = normalizer.hover(prepared.document.text(), response);
        let truncated = hover.as_ref().is_some_and(|value| value.truncated);
        let count = usize::from(hover.is_some());
        QueryOutcome::ready_with_metadata(
            hover,
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
        language: Language,
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
            language,
            prepared.document.version(),
            result.stale(),
            returned_count,
            result.total(),
            result.truncated(),
            result.filtered_count(),
        )
    }

    /// The whole shape of a method that answers with locations. Only the endpoint and the
    /// normalization differ between them, and both are supplied by the caller.
    async fn located_query<R, F>(
        &self,
        request: PositionRequest<'_>,
        method: SemanticMethod,
        normalize: F,
    ) -> QueryOutcome<Vec<NormalizedLocation>>
    where
        R: DeserializeOwned,
        F: FnOnce(&SemanticResultNormalizer, R) -> NormalizedLocations,
    {
        let language = request.language;
        let (prepared, response) = match self.position_query::<R>(request, method).await {
            Ok(result) => result,
            Err(failure) => return failure.outcome(),
        };
        match normalizer_for(&prepared, language) {
            Ok(normalizer) => {
                located_outcome(language, &prepared, normalize(&normalizer, response))
            }
            Err(failure) => failure.outcome(),
        }
    }

    /// Prepare, resolve the position, send one request, and hand back both halves. Every failure
    /// path has already released the request slot by the time this returns `Err`.
    async fn position_query<R: DeserializeOwned>(
        &self,
        request: PositionRequest<'_>,
        method: SemanticMethod,
    ) -> Result<(PreparedQuery, R), QueryFailure> {
        let prepared = self
            .prepare(
                request.launch,
                request.language,
                request.relative_path,
                method,
            )
            .await?;
        let position = self
            .position(&prepared, request.line, request.column)
            .await?;
        let (lsp_method, mut params) = wire_request(method);
        params["textDocument"] = json!({"uri": prepared.document.uri()});
        params["position"] = json!(position);
        match self
            .request(&prepared, lsp_method, params, request.cancelled)
            .await
        {
            Ok(response) => Ok((prepared, response)),
            Err(error) => Err(request_failure(&prepared, request.language, error)),
        }
    }

    /// Owns the record-then-release bookkeeping. A method added later cannot forget to release the
    /// request slot, which leaks silently: nothing fails, the server just stops admitting work.
    async fn request<R: DeserializeOwned>(
        &self,
        prepared: &PreparedQuery,
        lsp_method: &str,
        params: Value,
        cancelled: Arc<AtomicBool>,
    ) -> Result<R, JsonRpcError> {
        let started = Instant::now();
        let response: Result<R, JsonRpcError> = prepared
            .handle
            .client()
            .request_with_control(
                lsp_method,
                params,
                JsonRpcRequestControl::standard(cancelled),
            )
            .await;
        match response.as_ref() {
            Ok(_) => self.processes.record_response(prepared.handle.key()).await,
            Err(error) => {
                self.processes
                    .record_request_failure(prepared.handle.key(), *error, started.elapsed())
                    .await;
            }
        }
        self.processes.release_request(prepared.handle.key()).await;
        response
    }

    async fn prepare(
        &self,
        launch: LspProcessLaunch,
        language: Language,
        relative_path: &str,
        method: SemanticMethod,
    ) -> Result<PreparedQuery, QueryFailure> {
        let key = launch.key.clone();
        let acquisition = self
            .processes
            .acquire(launch, ActivationReason::ToolRequest, true)
            .await;
        let handle = match acquisition {
            LspProcessAcquisition::Ready(handle) => handle,
            LspProcessAcquisition::Warming => {
                self.processes.release_request(&key).await;
                return Err(failure(QueryStatus::Warming, "server_starting", language));
            }
            LspProcessAcquisition::Unavailable => {
                self.processes.release_request(&key).await;
                return Err(failure(
                    QueryStatus::Unavailable,
                    "server_unavailable",
                    language,
                ));
            }
            LspProcessAcquisition::Failed => {
                self.processes.release_request(&key).await;
                return Err(failure(QueryStatus::Failed, "server_failed", language));
            }
        };
        if !handle.capabilities().supports(method) {
            self.processes.release_request(handle.key()).await;
            return Err(failure(
                QueryStatus::Unavailable,
                "method_unsupported",
                language,
            ));
        }
        self.apply_invalidations(handle.key().session_root_ref())
            .await;
        let resources = match self.manager(&handle).await {
            Ok(resources) => resources,
            Err(reason) => {
                self.processes.release_request(handle.key()).await;
                return Err(failure(QueryStatus::Failed, reason, language));
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
                    prepared.handle.key().language(),
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

fn request_failure(
    prepared: &PreparedQuery,
    language: Language,
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

/// The wire method and the parameters beyond the document position, derived from the semantic
/// method so no call site can pair a method with another method's endpoint. Exhaustive on purpose:
/// a variant added without an endpoint fails to compile here.
fn wire_request(method: SemanticMethod) -> (&'static str, Value) {
    match method {
        SemanticMethod::Definition => ("textDocument/definition", json!({})),
        SemanticMethod::References => (
            "textDocument/references",
            json!({"context": {"includeDeclaration": true}}),
        ),
        SemanticMethod::Hover => ("textDocument/hover", json!({})),
        // Diagnostics arrive as a server notification and never route through a request, so this
        // arm exists only to keep the match exhaustive.
        SemanticMethod::Diagnostics => ("textDocument/publishDiagnostics", json!({})),
    }
}

fn normalizer_for(
    prepared: &PreparedQuery,
    language: Language,
) -> Result<SemanticResultNormalizer, QueryFailure> {
    SemanticResultNormalizer::new(
        prepared.handle.key().session_root_ref(),
        prepared.handle.capabilities().position_encoding,
    )
    .map_err(|_| prepared_failure(prepared, language, "workspace_unavailable"))
}

fn located_outcome(
    language: Language,
    prepared: &PreparedQuery,
    normalized: NormalizedLocations,
) -> QueryOutcome<Vec<NormalizedLocation>> {
    // The normalizer already truncated to the method's own cap, so the returned count is the
    // vector's length. Taking the cap as an argument would only be a chance to pass the wrong one.
    let returned = normalized.locations.len();
    QueryOutcome::ready_with_metadata(
        normalized.locations,
        language,
        prepared.document.version(),
        returned,
        normalized.total,
        normalized.truncated,
        normalized.filtered_count,
    )
}

fn prepared_failure(
    prepared: &PreparedQuery,
    language: Language,
    reason: &'static str,
) -> QueryFailure {
    QueryFailure {
        status: QueryStatus::Failed,
        reason,
        language: Some(language),
        document_version: Some(prepared.document.version()),
    }
}

const fn failure(status: QueryStatus, reason: &'static str, language: Language) -> QueryFailure {
    QueryFailure {
        status,
        reason,
        language: Some(language),
        document_version: None,
    }
}
