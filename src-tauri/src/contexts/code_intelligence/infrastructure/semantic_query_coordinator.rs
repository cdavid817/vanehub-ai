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
use super::semantic_results::{NormalizedLocations, NormalizedSymbols, SemanticResultNormalizer};
use crate::contexts::code_intelligence::domain::models::{
    DocumentVersion, Language, NormalizedDiagnostic, NormalizedHover, NormalizedLocation,
    NormalizedSymbol, QueryOutcome, QueryStatus, SemanticMethod,
};
use lsp_types::{DocumentSymbolResponse, Hover, Location, WorkspaceSymbolResponse};
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

/// What every document-scoped query needs from its caller. Named so the helpers below stay inside
/// clippy's argument budget, and so adding a method does not re-thread four parameters by hand.
/// The position, where there is one, travels separately: not every method has one.
struct QueryRequest<'a> {
    launch: LspProcessLaunch,
    language: Language,
    relative_path: &'a str,
    cancelled: Arc<AtomicBool>,
}

struct QueryFailure {
    status: QueryStatus,
    reason: &'static str,
    language: Option<Language>,
    document_version: Option<DocumentVersion>,
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
        let request = QueryRequest {
            launch,
            language,
            relative_path,
            cancelled,
        };
        let position = AgentPosition::new(line, column);
        self.located_query(
            request,
            position,
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
        let request = QueryRequest {
            launch,
            language,
            relative_path,
            cancelled,
        };
        let position = AgentPosition::new(line, column);
        self.located_query(
            request,
            position,
            SemanticMethod::References,
            |normalizer, response: Option<Vec<Location>>| {
                normalizer.references(response.unwrap_or_default())
            },
        )
        .await
    }

    // Reached from tests until the tool catalog wires it up. `expect` rather than `allow` so the
    // attribute fails the build once it is wired, instead of outliving its reason in silence.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "tool catalog wiring lands with the Agent surface")
    )]
    pub(crate) async fn find_type_definition(
        &self,
        launch: LspProcessLaunch,
        language: Language,
        relative_path: &str,
        line: u32,
        column: u32,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Vec<NormalizedLocation>> {
        let request = QueryRequest {
            launch,
            language,
            relative_path,
            cancelled,
        };
        let position = AgentPosition::new(line, column);
        // `textDocument/typeDefinition` answers in the same three shapes as `definition`, so it
        // reuses that normalization -- and with it the cap of 20 and the truncation metadata.
        self.located_query(
            request,
            position,
            SemanticMethod::TypeDefinition,
            |normalizer, response| normalizer.definitions(response),
        )
        .await
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "tool catalog wiring lands with the Agent surface")
    )]
    pub(crate) async fn find_implementations(
        &self,
        launch: LspProcessLaunch,
        language: Language,
        relative_path: &str,
        line: u32,
        column: u32,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Vec<NormalizedLocation>> {
        let request = QueryRequest {
            launch,
            language,
            relative_path,
            cancelled,
        };
        let position = AgentPosition::new(line, column);
        self.located_query(
            request,
            position,
            SemanticMethod::Implementation,
            |normalizer, response| normalizer.definitions(response),
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
        let request = QueryRequest {
            launch,
            language,
            relative_path,
            cancelled,
        };
        let position = AgentPosition::new(line, column);
        let (prepared, response) = match self
            .position_query::<Option<Hover>>(request, position, SemanticMethod::Hover)
            .await
        {
            Ok(result) => result,
            Err(failure) => return failure.outcome(),
        };
        let version = Some(prepared.document.version());
        let normalizer = match normalizer_for(&prepared.handle, language, version) {
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

    /// The one query with no document: it names a project through its launch, not a file, so it
    /// skips admission and the document lease entirely.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "tool catalog wiring lands with the Agent surface")
    )]
    pub(crate) async fn find_workspace_symbols(
        &self,
        launch: LspProcessLaunch,
        language: Language,
        query: &str,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Vec<NormalizedSymbol>> {
        // Refused here rather than sent on: servers differ on what an empty query means, and the
        // ones that answer it answer with the whole index.
        if query.trim().is_empty() {
            return QueryOutcome::degraded_with_identity(
                QueryStatus::Failed,
                "invalid_query",
                Some(language),
                None,
            );
        }
        let handle = match self
            .admit(launch, language, SemanticMethod::WorkspaceSymbols)
            .await
        {
            Ok(handle) => handle,
            Err(failure) => return failure.outcome(),
        };
        let response: Result<Option<WorkspaceSymbolResponse>, JsonRpcError> = self
            .request(
                &handle,
                "workspace/symbol",
                json!({"query": query}),
                cancelled,
            )
            .await;
        let response = match response {
            Ok(response) => response,
            Err(error) => return request_failure(error, language, None).outcome(),
        };
        match normalizer_for(&handle, language, None) {
            Ok(normalizer) => {
                symbol_outcome(language, None, normalizer.workspace_symbols(response))
            }
            Err(failure) => failure.outcome(),
        }
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "tool catalog wiring lands with the Agent surface")
    )]
    pub(crate) async fn get_document_symbols(
        &self,
        launch: LspProcessLaunch,
        language: Language,
        relative_path: &str,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Vec<NormalizedSymbol>> {
        let request = QueryRequest {
            launch,
            language,
            relative_path,
            cancelled,
        };
        let (prepared, response) = match self
            .document_query::<Option<DocumentSymbolResponse>>(
                request,
                SemanticMethod::DocumentSymbols,
            )
            .await
        {
            Ok(result) => result,
            Err(failure) => return failure.outcome(),
        };
        let version = Some(prepared.document.version());
        match normalizer_for(&prepared.handle, language, version) {
            Ok(normalizer) => symbol_outcome(
                language,
                version,
                normalizer.document_symbols(relative_path, response),
            ),
            Err(failure) => failure.outcome(),
        }
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
        request: QueryRequest<'_>,
        position: AgentPosition,
        method: SemanticMethod,
        normalize: F,
    ) -> QueryOutcome<Vec<NormalizedLocation>>
    where
        R: DeserializeOwned,
        F: FnOnce(&SemanticResultNormalizer, R) -> NormalizedLocations,
    {
        let language = request.language;
        let (prepared, response) = match self.position_query::<R>(request, position, method).await {
            Ok(result) => result,
            Err(failure) => return failure.outcome(),
        };
        let version = Some(prepared.document.version());
        match normalizer_for(&prepared.handle, language, version) {
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
        request: QueryRequest<'_>,
        position: AgentPosition,
        method: SemanticMethod,
    ) -> Result<(PreparedQuery, R), QueryFailure> {
        let language = request.language;
        let cancelled = request.cancelled.clone();
        let prepared = self.prepared(request, method).await?;
        let position = self.position(&prepared, position).await?;
        let (lsp_method, mut params) = wire_request(method);
        params["textDocument"] = json!({"uri": prepared.document.uri()});
        params["position"] = json!(position);
        self.finish(prepared, lsp_method, params, cancelled, language)
            .await
    }

    /// The same shape for a method that names a document but no position inside it.
    async fn document_query<R: DeserializeOwned>(
        &self,
        request: QueryRequest<'_>,
        method: SemanticMethod,
    ) -> Result<(PreparedQuery, R), QueryFailure> {
        let language = request.language;
        let cancelled = request.cancelled.clone();
        let prepared = self.prepared(request, method).await?;
        let (lsp_method, mut params) = wire_request(method);
        params["textDocument"] = json!({"uri": prepared.document.uri()});
        self.finish(prepared, lsp_method, params, cancelled, language)
            .await
    }

    async fn prepared(
        &self,
        request: QueryRequest<'_>,
        method: SemanticMethod,
    ) -> Result<PreparedQuery, QueryFailure> {
        self.prepare(
            request.launch,
            request.language,
            request.relative_path,
            method,
        )
        .await
    }

    async fn finish<R: DeserializeOwned>(
        &self,
        prepared: PreparedQuery,
        lsp_method: &str,
        params: Value,
        cancelled: Arc<AtomicBool>,
        language: Language,
    ) -> Result<(PreparedQuery, R), QueryFailure> {
        let version = Some(prepared.document.version());
        match self
            .request(&prepared.handle, lsp_method, params, cancelled)
            .await
        {
            Ok(response) => Ok((prepared, response)),
            Err(error) => Err(request_failure(error, language, version)),
        }
    }

    /// Owns the record-then-release bookkeeping. A method added later cannot forget to release the
    /// request slot, which leaks silently: nothing fails, the server just stops admitting work.
    async fn request<R: DeserializeOwned>(
        &self,
        handle: &LspProcessHandle,
        lsp_method: &str,
        params: Value,
        cancelled: Arc<AtomicBool>,
    ) -> Result<R, JsonRpcError> {
        let started = Instant::now();
        let response: Result<R, JsonRpcError> = handle
            .client()
            .request_with_control(
                lsp_method,
                params,
                JsonRpcRequestControl::standard(cancelled),
            )
            .await;
        match response.as_ref() {
            Ok(_) => self.processes.record_response(handle.key()).await,
            Err(error) => {
                self.processes
                    .record_request_failure(handle.key(), *error, started.elapsed())
                    .await;
            }
        }
        self.processes.release_request(handle.key()).await;
        response
    }

    async fn prepare(
        &self,
        launch: LspProcessLaunch,
        language: Language,
        relative_path: &str,
        method: SemanticMethod,
    ) -> Result<PreparedQuery, QueryFailure> {
        let handle = self.admit(launch, language, method).await?;
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

    /// A ready process that advertises the method, or the failure to report instead. Split out of
    /// `prepare` because the workspace-wide query needs a server but no document.
    async fn admit(
        &self,
        launch: LspProcessLaunch,
        language: Language,
        method: SemanticMethod,
    ) -> Result<LspProcessHandle, QueryFailure> {
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
        Ok(handle)
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
        position: AgentPosition,
    ) -> Result<lsp_types::Position, QueryFailure> {
        match PositionConverter::new(
            prepared.document.text(),
            prepared.handle.capabilities().position_encoding,
        )
        .agent_to_lsp(position)
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
    error: JsonRpcError,
    language: Language,
    document_version: Option<DocumentVersion>,
) -> QueryFailure {
    let (status, reason) = match error {
        JsonRpcError::Timeout => (QueryStatus::Timeout, "request_timeout"),
        JsonRpcError::Cancelled => (QueryStatus::Failed, "generation_cancelled"),
        JsonRpcError::ActorStopped => (QueryStatus::Unavailable, "server_unavailable"),
        _ => (QueryStatus::Failed, "request_failed"),
    };
    QueryFailure {
        status,
        reason,
        language: Some(language),
        document_version,
    }
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
        SemanticMethod::TypeDefinition => ("textDocument/typeDefinition", json!({})),
        SemanticMethod::Implementation => ("textDocument/implementation", json!({})),
        SemanticMethod::DocumentSymbols => ("textDocument/documentSymbol", json!({})),
        // Sent without a document, so it never goes through the helpers that read this table. The
        // arm keeps the match exhaustive and names the endpoint in the same place as the others.
        SemanticMethod::WorkspaceSymbols => ("workspace/symbol", json!({})),
    }
}

fn normalizer_for(
    handle: &LspProcessHandle,
    language: Language,
    document_version: Option<DocumentVersion>,
) -> Result<SemanticResultNormalizer, QueryFailure> {
    SemanticResultNormalizer::new(
        handle.key().session_root_ref(),
        handle.capabilities().position_encoding,
    )
    .map_err(|_| QueryFailure {
        status: QueryStatus::Failed,
        reason: "workspace_unavailable",
        language: Some(language),
        document_version,
    })
}

fn symbol_outcome(
    language: Language,
    document_version: Option<DocumentVersion>,
    normalized: NormalizedSymbols,
) -> QueryOutcome<Vec<NormalizedSymbol>> {
    let returned = normalized.symbols.len();
    match document_version {
        Some(version) => QueryOutcome::ready_with_metadata(
            normalized.symbols,
            language,
            version,
            returned,
            normalized.total,
            normalized.truncated,
            normalized.filtered_count,
        ),
        None => QueryOutcome::ready_without_document(
            normalized.symbols,
            language,
            returned,
            normalized.total,
            normalized.truncated,
            normalized.filtered_count,
        ),
    }
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
