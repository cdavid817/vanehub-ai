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
    CallDirection, DocumentVersion, Language, NormalizedCallRelation, NormalizedDiagnostic,
    NormalizedHover, NormalizedLocation, NormalizedSymbol, QueryOutcome, QueryStatus,
    SemanticMethod,
};
use lsp_types::{
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall,
    DocumentSymbolResponse, Hover, Location, WorkspaceSymbolResponse,
};
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

/// The whole exchange's budget. One deadline covers preparation and the direction walk together:
/// two steps at the per-request deadline would let a slow server take twice as long as any other
/// tool while every individual request still looked healthy.
const CALL_HIERARCHY_BUDGET: Duration = Duration::from_secs(10);
const CALL_HIERARCHY_CLEANUP_RESERVE: Duration = Duration::from_millis(250);

#[derive(Default)]
struct CallHierarchyExchange {
    incoming: Option<Vec<CallHierarchyIncomingCall>>,
    outgoing: Option<Vec<CallHierarchyOutgoingCall>>,
    /// Prepared items past the first. Following all of them would multiply the request count by an
    /// amount the server chooses, so the rest are reported rather than walked.
    unfollowed: usize,
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

    /// Takes an `AgentPosition` rather than a line and a column: with the direction added, the
    /// separate pair would put this one argument over clippy's limit.
    pub(crate) async fn find_call_hierarchy(
        &self,
        launch: LspProcessLaunch,
        language: Language,
        relative_path: &str,
        position: AgentPosition,
        direction: CallDirection,
        cancelled: Arc<AtomicBool>,
    ) -> QueryOutcome<Vec<NormalizedCallRelation>> {
        let request = QueryRequest {
            launch,
            language,
            relative_path,
            cancelled: cancelled.clone(),
        };
        let prepared = match self.prepared(request, SemanticMethod::CallHierarchy).await {
            Ok(prepared) => prepared,
            Err(failure) => return failure.outcome(),
        };
        let exchange = match self.position(&prepared, position) {
            Ok(position) => {
                self.call_hierarchy_exchange(&prepared, position, direction, cancelled)
                    .await
            }
            Err(failure) => Err(failure),
        };
        // One release for the whole exchange. The slot is held across both requests, so the
        // per-request helper is not the one that can give it back.
        self.processes.release_request(prepared.handle.key()).await;
        let version = Some(prepared.document.version());
        let exchange = match exchange {
            Ok(exchange) => exchange,
            Err(failure) => return failure.outcome(),
        };
        let normalizer = match normalizer_for(&prepared.handle, language, version) {
            Ok(normalizer) => normalizer,
            Err(failure) => return failure.outcome(),
        };
        let normalized = normalizer.call_relations(direction, exchange.incoming, exchange.outgoing);
        QueryOutcome::status_with_value(
            QueryStatus::Ready,
            Some(normalized.items),
            // A ready answer that is not the whole answer. Silence here would let the Agent read a
            // partial hierarchy as a complete one.
            (exchange.unfollowed > 0).then_some("call_hierarchy_items_not_followed"),
            language,
            prepared.document.version(),
            false,
            normalized
                .total
                .min(super::semantic_results::MAX_CALL_RELATIONS),
            normalized.total,
            normalized.truncated || exchange.unfollowed > 0,
            normalized.filtered_count,
        )
    }

    async fn call_hierarchy_exchange(
        &self,
        prepared: &PreparedQuery,
        position: lsp_types::Position,
        direction: CallDirection,
        cancelled: Arc<AtomicBool>,
    ) -> Result<CallHierarchyExchange, QueryFailure> {
        let language = prepared.handle.key().language();
        let version = Some(prepared.document.version());
        let started = Instant::now();
        let items: Option<Vec<CallHierarchyItem>> = self
            .send(
                &prepared.handle,
                "textDocument/prepareCallHierarchy",
                json!({"textDocument": {"uri": prepared.document.uri()}, "position": position}),
                remaining_budget(started, cancelled.clone(), language, version)?,
            )
            .await
            .map_err(|error| request_failure(error, language, version))?;
        let mut items = items.unwrap_or_default();
        if items.is_empty() {
            // Nothing at that position is callable. That is an answer; asking for the calls of
            // nothing is not, so no second request is sent.
            return Ok(CallHierarchyExchange::default());
        }
        let unfollowed = items.len() - 1;
        let item = items.swap_remove(0);
        let lsp_method = match direction {
            CallDirection::Incoming => "callHierarchy/incomingCalls",
            CallDirection::Outgoing => "callHierarchy/outgoingCalls",
        };
        let control = remaining_budget(started, cancelled, language, version)?;
        let params = json!({"item": item});
        match direction {
            CallDirection::Incoming => {
                let incoming = self
                    .send(&prepared.handle, lsp_method, params, control)
                    .await
                    .map_err(|error| request_failure(error, language, version))?;
                Ok(CallHierarchyExchange {
                    incoming,
                    outgoing: None,
                    unfollowed,
                })
            }
            CallDirection::Outgoing => {
                let outgoing = self
                    .send(&prepared.handle, lsp_method, params, control)
                    .await
                    .map_err(|error| request_failure(error, language, version))?;
                Ok(CallHierarchyExchange {
                    incoming: None,
                    outgoing,
                    unfollowed,
                })
            }
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
        let position = match self.position(&prepared, position) {
            Ok(position) => position,
            Err(failure) => {
                // `prepare` succeeded, so the slot is held and nothing else will give it back.
                self.processes.release_request(prepared.handle.key()).await;
                return Err(failure);
            }
        };
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

    /// One request that finishes the query, releasing the slot on every path. A method added
    /// later cannot forget to release it, which leaks silently: nothing fails, the server just
    /// stops admitting work.
    async fn request<R: DeserializeOwned>(
        &self,
        handle: &LspProcessHandle,
        lsp_method: &str,
        params: Value,
        cancelled: Arc<AtomicBool>,
    ) -> Result<R, JsonRpcError> {
        let response = self
            .send(
                handle,
                lsp_method,
                params,
                JsonRpcRequestControl::standard(cancelled),
            )
            .await;
        self.processes.release_request(handle.key()).await;
        response
    }

    /// One request with its response or failure recorded, and the slot left held. A query that
    /// sends more than one request holds it across all of them.
    async fn send<R: DeserializeOwned>(
        &self,
        handle: &LspProcessHandle,
        lsp_method: &str,
        params: Value,
        control: JsonRpcRequestControl,
    ) -> Result<R, JsonRpcError> {
        let started = Instant::now();
        let response: Result<R, JsonRpcError> = handle
            .client()
            .request_with_control(lsp_method, params, control)
            .await;
        match response.as_ref() {
            Ok(_) => self.processes.record_response(handle.key()).await,
            Err(error) => {
                self.processes
                    .record_request_failure(handle.key(), *error, started.elapsed())
                    .await;
            }
        }
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

    /// Does not release the request slot. Which caller releases it depends on how many requests
    /// the query sends, so the decision stays with the caller rather than being made here twice.
    fn position(
        &self,
        prepared: &PreparedQuery,
        position: AgentPosition,
    ) -> Result<lsp_types::Position, QueryFailure> {
        PositionConverter::new(
            prepared.document.text(),
            prepared.handle.capabilities().position_encoding,
        )
        .agent_to_lsp(position)
        .map_err(|_| {
            prepared_failure(
                prepared,
                prepared.handle.key().language(),
                "invalid_position",
            )
        })
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
        // Three endpoints chosen by direction inside the exchange rather than one read from here.
        // The arm names the first of them and keeps the match exhaustive.
        SemanticMethod::CallHierarchy => ("textDocument/prepareCallHierarchy", json!({})),
    }
}

/// What is left of the exchange's single budget. Running it out is a timeout, which is what the
/// caller would have seen had one request spent the whole thing.
fn remaining_budget(
    started: Instant,
    cancelled: Arc<AtomicBool>,
    language: Language,
    document_version: Option<DocumentVersion>,
) -> Result<JsonRpcRequestControl, QueryFailure> {
    JsonRpcRequestControl::new(
        CALL_HIERARCHY_BUDGET.saturating_sub(started.elapsed()),
        CALL_HIERARCHY_CLEANUP_RESERVE,
        cancelled,
    )
    .map_err(|_| QueryFailure {
        status: QueryStatus::Timeout,
        reason: "request_timeout",
        language: Some(language),
        document_version,
    })
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
    let returned = normalized.items.len();
    match document_version {
        Some(version) => QueryOutcome::ready_with_metadata(
            normalized.items,
            language,
            version,
            returned,
            normalized.total,
            normalized.truncated,
            normalized.filtered_count,
        ),
        None => QueryOutcome::ready_without_document(
            normalized.items,
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
    let returned = normalized.items.len();
    QueryOutcome::ready_with_metadata(
        normalized.items,
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
