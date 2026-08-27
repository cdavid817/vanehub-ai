use crate::contexts::agent_runtime::application::{
    AgentCallDirection, AgentCallHierarchyInput, AgentCodeCallRelation, AgentCodeDiagnostic,
    AgentCodeHover, AgentCodeIntelligenceContext, AgentCodeIntelligenceMetadata,
    AgentCodeIntelligenceOutcome, AgentCodeIntelligencePending, AgentCodeIntelligenceResponderPort,
    AgentCodeIntelligenceStatus, AgentCodeLocation, AgentCodeRange, AgentCodeSymbol,
    AgentDocumentInput, AgentDocumentPositionInput, AgentWorkspaceMutation,
    AgentWorkspaceMutationPort, AgentWorkspaceSymbolInput,
};
use crate::contexts::code_intelligence::api::{
    CallDirection, CodeIntelligenceApi, DiagnosticSeverity, NormalizedCallRelation,
    NormalizedDiagnostic, NormalizedHover, NormalizedLocation, NormalizedRange, NormalizedSymbol,
    QueryOutcome, QueryStatus,
};
use crate::contexts::operations::api::DiagnosticLogPort;
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::retrieval::api::CodeIndexApi;
use crate::platform::database::NativeDatabase;
use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::OnceLock;

pub(crate) fn assemble_code_intelligence_api(
    database: NativeDatabase,
    fallback_log_directory: PathBuf,
) -> CodeIntelligenceApi {
    let logging: Arc<dyn DiagnosticLogPort> =
        Arc::new(UnifiedLoggingAdapter::active(fallback_log_directory));
    CodeIntelligenceApi::from_database(database, logging)
}

pub(crate) struct NativeCodeIntelligenceResponder {
    api: CodeIntelligenceApi,
}

impl NativeCodeIntelligenceResponder {
    pub(crate) fn new(api: CodeIntelligenceApi) -> Self {
        Self { api }
    }

    fn pending<T, F>(future: F, cancelled: Arc<AtomicBool>) -> AgentCodeIntelligencePending<T>
    where
        T: Send + 'static,
        F: Future<Output = AgentCodeIntelligenceOutcome<T>> + Send + 'static,
    {
        let (send, response) = mpsc::channel();
        tauri::async_runtime::spawn(async move {
            let _ = send.send(future.await);
        });
        let cancel_flag = cancelled.clone();
        AgentCodeIntelligencePending {
            response,
            cancel: Arc::new(move || cancel_flag.store(true, Ordering::Release)),
        }
    }
}

impl AgentCodeIntelligenceResponderPort for NativeCodeIntelligenceResponder {
    fn is_available(&self, context: &AgentCodeIntelligenceContext) -> bool {
        let workspace = std::path::Path::new(context.session_workspace());
        let available = self.api.is_agent_workspace_available(workspace);
        if available {
            self.api.prewarm_workspace(workspace);
        }
        available
    }

    fn start_find_definition(
        &self,
        context: AgentCodeIntelligenceContext,
        input: AgentDocumentPositionInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeLocation>> {
        let api = self.api.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let request_cancelled = cancelled.clone();
        Self::pending(
            async move {
                map_locations(
                    api.find_definition(
                        std::path::Path::new(context.session_workspace()),
                        &input.relative_path,
                        input.line,
                        input.column,
                        request_cancelled,
                    )
                    .await,
                )
            },
            cancelled,
        )
    }

    fn start_find_references(
        &self,
        context: AgentCodeIntelligenceContext,
        input: AgentDocumentPositionInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeLocation>> {
        let api = self.api.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let request_cancelled = cancelled.clone();
        Self::pending(
            async move {
                map_locations(
                    api.find_references(
                        std::path::Path::new(context.session_workspace()),
                        &input.relative_path,
                        input.line,
                        input.column,
                        request_cancelled,
                    )
                    .await,
                )
            },
            cancelled,
        )
    }

    fn start_get_hover(
        &self,
        context: AgentCodeIntelligenceContext,
        input: AgentDocumentPositionInput,
    ) -> AgentCodeIntelligencePending<Option<AgentCodeHover>> {
        let api = self.api.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let request_cancelled = cancelled.clone();
        Self::pending(
            async move {
                map_hover(
                    api.get_hover(
                        std::path::Path::new(context.session_workspace()),
                        &input.relative_path,
                        input.line,
                        input.column,
                        request_cancelled,
                    )
                    .await,
                )
            },
            cancelled,
        )
    }

    fn start_get_diagnostics(
        &self,
        context: AgentCodeIntelligenceContext,
        input: AgentDocumentInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeDiagnostic>> {
        let api = self.api.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let request_cancelled = cancelled.clone();
        Self::pending(
            async move {
                let relative_path = input.relative_path;
                map_diagnostics(
                    api.get_diagnostics(
                        std::path::Path::new(context.session_workspace()),
                        &relative_path,
                        request_cancelled,
                    )
                    .await,
                    &relative_path,
                )
            },
            cancelled,
        )
    }

    fn start_find_type_definition(
        &self,
        context: AgentCodeIntelligenceContext,
        input: AgentDocumentPositionInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeLocation>> {
        let api = self.api.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let request_cancelled = cancelled.clone();
        Self::pending(
            async move {
                map_locations(
                    api.find_type_definition(
                        std::path::Path::new(context.session_workspace()),
                        &input.relative_path,
                        input.line,
                        input.column,
                        request_cancelled,
                    )
                    .await,
                )
            },
            cancelled,
        )
    }

    fn start_find_implementations(
        &self,
        context: AgentCodeIntelligenceContext,
        input: AgentDocumentPositionInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeLocation>> {
        let api = self.api.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let request_cancelled = cancelled.clone();
        Self::pending(
            async move {
                map_locations(
                    api.find_implementations(
                        std::path::Path::new(context.session_workspace()),
                        &input.relative_path,
                        input.line,
                        input.column,
                        request_cancelled,
                    )
                    .await,
                )
            },
            cancelled,
        )
    }

    fn start_find_workspace_symbols(
        &self,
        context: AgentCodeIntelligenceContext,
        input: AgentWorkspaceSymbolInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeSymbol>> {
        let api = self.api.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let request_cancelled = cancelled.clone();
        Self::pending(
            async move {
                map_symbols(
                    api.find_workspace_symbols(
                        std::path::Path::new(context.session_workspace()),
                        &input.relative_path,
                        &input.query,
                        request_cancelled,
                    )
                    .await,
                )
            },
            cancelled,
        )
    }

    fn start_get_document_symbols(
        &self,
        context: AgentCodeIntelligenceContext,
        input: AgentDocumentInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeSymbol>> {
        let api = self.api.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let request_cancelled = cancelled.clone();
        Self::pending(
            async move {
                map_symbols(
                    api.get_document_symbols(
                        std::path::Path::new(context.session_workspace()),
                        &input.relative_path,
                        request_cancelled,
                    )
                    .await,
                )
            },
            cancelled,
        )
    }

    fn start_find_call_hierarchy(
        &self,
        context: AgentCodeIntelligenceContext,
        input: AgentCallHierarchyInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeCallRelation>> {
        let api = self.api.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let request_cancelled = cancelled.clone();
        let direction = match input.direction {
            AgentCallDirection::Incoming => CallDirection::Incoming,
            AgentCallDirection::Outgoing => CallDirection::Outgoing,
        };
        Self::pending(
            async move {
                map_call_relations(
                    api.find_call_hierarchy(
                        std::path::Path::new(context.session_workspace()),
                        &input.position.relative_path,
                        input.position.line,
                        input.position.column,
                        direction,
                        request_cancelled,
                    )
                    .await,
                )
            },
            cancelled,
        )
    }
}

fn map_locations(
    outcome: QueryOutcome<Vec<NormalizedLocation>>,
) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeLocation>> {
    map_outcome(outcome, |locations| {
        locations.into_iter().map(map_location).collect()
    })
}

fn map_symbols(
    outcome: QueryOutcome<Vec<NormalizedSymbol>>,
) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeSymbol>> {
    map_outcome(outcome, |symbols| {
        symbols.into_iter().map(map_symbol).collect()
    })
}

fn map_call_relations(
    outcome: QueryOutcome<Vec<NormalizedCallRelation>>,
) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeCallRelation>> {
    map_outcome(outcome, |relations| {
        relations
            .into_iter()
            .map(|relation| AgentCodeCallRelation {
                symbol: map_symbol(relation.symbol),
                call_sites: relation.call_sites.into_iter().map(map_range).collect(),
            })
            .collect()
    })
}

fn map_symbol(symbol: NormalizedSymbol) -> AgentCodeSymbol {
    AgentCodeSymbol {
        name: symbol.name,
        kind: symbol.kind.to_owned(),
        container: symbol.container,
        file: symbol.location.file().to_owned(),
        range: map_range(symbol.location.range),
        preview: symbol.location.preview,
    }
}

fn map_hover(
    outcome: QueryOutcome<Option<NormalizedHover>>,
) -> AgentCodeIntelligenceOutcome<Option<AgentCodeHover>> {
    map_outcome(outcome, |hover| {
        hover.map(|hover| AgentCodeHover {
            signature: hover.signature,
            documentation: hover.documentation,
            range: hover.range.map(map_range),
        })
    })
}

fn map_diagnostics(
    outcome: QueryOutcome<Vec<NormalizedDiagnostic>>,
    relative_path: &str,
) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeDiagnostic>> {
    map_outcome(outcome, |diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| AgentCodeDiagnostic {
                file: relative_path.to_owned(),
                range: map_range(diagnostic.range),
                severity: diagnostic.severity.map(|severity| {
                    match severity {
                        DiagnosticSeverity::Error => "error",
                        DiagnosticSeverity::Warning => "warning",
                        DiagnosticSeverity::Information => "information",
                        DiagnosticSeverity::Hint => "hint",
                    }
                    .to_owned()
                }),
                message: diagnostic.message,
                source: diagnostic.source,
                code: diagnostic.code,
            })
            .collect()
    })
}

fn map_outcome<T, U>(
    outcome: QueryOutcome<T>,
    map: impl FnOnce(T) -> U,
) -> AgentCodeIntelligenceOutcome<U> {
    let metadata = AgentCodeIntelligenceMetadata {
        status: match outcome.status() {
            QueryStatus::Ready => AgentCodeIntelligenceStatus::Ready,
            QueryStatus::Warming => AgentCodeIntelligenceStatus::Warming,
            QueryStatus::Timeout => AgentCodeIntelligenceStatus::Timeout,
            QueryStatus::Unavailable => AgentCodeIntelligenceStatus::Unavailable,
            QueryStatus::Failed => AgentCodeIntelligenceStatus::Failed,
        },
        server: outcome
            .language
            .map(|language| language.server_id.to_owned()),
        language: outcome.language.map(|language| language.id.to_owned()),
        document_version: outcome.document_version().map(|version| version.value()),
        stale: outcome.stale,
        returned_count: outcome.returned_count,
        total: outcome.total,
        truncated: outcome.truncated,
        filtered_count: outcome.filtered_count,
        reason_code: outcome.reason_code().map(str::to_owned),
    };
    AgentCodeIntelligenceOutcome {
        metadata,
        value: outcome.into_value().map(map),
    }
}

fn map_location(location: NormalizedLocation) -> AgentCodeLocation {
    AgentCodeLocation {
        file: location.file().to_owned(),
        range: map_range(location.range),
        preview: location.preview,
    }
}

fn map_range(range: NormalizedRange) -> AgentCodeRange {
    AgentCodeRange {
        start_line: range.start_line,
        start_column: range.start_column,
        end_line: range.end_line,
        end_column: range.end_column,
    }
}

pub(crate) struct WorkspaceMutationFanout {
    code_intelligence: CodeIntelligenceApi,
    code_index: OnceLock<CodeIndexApi>,
}

impl WorkspaceMutationFanout {
    pub(crate) fn new(code_intelligence: CodeIntelligenceApi) -> Self {
        Self {
            code_intelligence,
            code_index: OnceLock::new(),
        }
    }

    pub(crate) fn bind_code_index(&self, code_index: CodeIndexApi) -> Result<(), String> {
        self.code_index
            .set(code_index)
            .map_err(|_| "workspace mutation code-index target is already bound".to_string())
    }
}

impl AgentWorkspaceMutationPort for WorkspaceMutationFanout {
    fn publish(&self, mutation: AgentWorkspaceMutation) {
        self.code_intelligence
            .invalidate_document_lease(&mutation.canonical_workspace, &mutation.relative_path);
        if let Some(code_index) = self.code_index.get() {
            code_index
                .notify_targeted_change(&mutation.canonical_workspace, &mutation.relative_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::code_intelligence::api::{LspConfiguration, LspLanguageId};
    use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
    use crate::test_support::TempDirectory;

    #[test]
    fn native_responder_requires_configuration_trust_and_a_discovered_server() {
        let directory = TempDirectory::new("native-code-intelligence-responder");
        let database = NativeDatabase::new(directory.path().join("data")).expect("test database");
        let api = CodeIntelligenceApi::from_database(
            database,
            Arc::new(UnifiedLoggingAdapter::new(directory.path().join("logs"))),
        );
        let responder = NativeCodeIntelligenceResponder::new(api.clone());
        let context = AgentCodeIntelligenceContext::from_session_workspace(
            directory.path().to_string_lossy().into_owned(),
        );
        assert!(!responder.is_available(&context));

        let mut configuration = LspConfiguration {
            enabled: true,
            ..LspConfiguration::default()
        };
        let rust = configuration
            .languages
            .get_mut(&LspLanguageId::new("rust").expect("rust language id"))
            .expect("Rust configuration");
        rust.enabled = true;
        rust.executable_override = Some(
            std::env::current_exe()
                .expect("test executable")
                .to_string_lossy()
                .into_owned(),
        );
        api.save_configuration(&configuration)
            .expect("save configuration");
        assert!(!responder.is_available(&context));

        api.update_workspace_trust(directory.path(), true)
            .expect("trust workspace");
        assert!(responder.is_available(&context));
    }
}
