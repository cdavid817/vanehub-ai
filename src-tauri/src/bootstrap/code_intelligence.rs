use crate::contexts::agent_runtime::application::{
    AgentCodeDiagnostic, AgentCodeHover, AgentCodeIntelligenceContext,
    AgentCodeIntelligenceMetadata, AgentCodeIntelligenceOutcome, AgentCodeIntelligencePending,
    AgentCodeIntelligenceResponderPort, AgentCodeIntelligenceStatus, AgentCodeLocation,
    AgentCodeRange, AgentDocumentInput, AgentDocumentPositionInput, AgentWorkspaceMutation,
    AgentWorkspaceMutationPort,
};
use crate::contexts::code_intelligence::api::{
    CodeIntelligenceApi, DiagnosticSeverity, LanguageFamily, NormalizedDiagnostic, NormalizedHover,
    NormalizedLocation, NormalizedRange, QueryOutcome, QueryStatus,
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
}

fn map_locations(
    outcome: QueryOutcome<Vec<NormalizedLocation>>,
) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeLocation>> {
    map_outcome(outcome, |locations| {
        locations.into_iter().map(map_location).collect()
    })
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
        server: outcome.server.map(|server| server.as_id().to_owned()),
        language: outcome
            .language
            .map(LanguageFamily::as_id)
            .map(str::to_owned),
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
    evidence: Arc<dyn crate::contexts::workspaces::api::WorkspaceEvidencePort>,
}

impl WorkspaceMutationFanout {
    pub(crate) fn new(code_intelligence: CodeIntelligenceApi) -> Self {
        Self {
            code_intelligence,
            code_index: OnceLock::new(),
            evidence: Arc::new(crate::contexts::workspaces::application::NoWorkspaceEvidence),
        }
    }

    /// Bootstrap swaps in the real publisher. The fanout is already the single point every
    /// successful mutation passes through, so evidence joins the existing targets rather than
    /// adding a second observation path that could disagree with them.
    pub(crate) fn with_evidence(
        mut self,
        evidence: Arc<dyn crate::contexts::workspaces::api::WorkspaceEvidencePort>,
    ) -> Self {
        self.evidence = evidence;
        self
    }

    pub(crate) fn bind_code_index(&self, code_index: CodeIndexApi) -> Result<(), String> {
        self.code_index
            .set(code_index)
            .map_err(|_| "workspace mutation code-index target is already bound".to_string())
    }
}

impl AgentWorkspaceMutationPort for WorkspaceMutationFanout {
    /// Reached only after a mutation succeeded: the tool handlers publish on the success branch
    /// and nowhere else, so a rejected or failed write produces no evidence at all.
    fn publish(&self, mutation: AgentWorkspaceMutation) {
        self.code_intelligence
            .invalidate_document_lease(&mutation.canonical_workspace, &mutation.relative_path);
        if let Some(code_index) = self.code_index.get() {
            code_index
                .notify_targeted_change(&mutation.canonical_workspace, &mutation.relative_path);
        }
        let Some(basename) = mutation
            .relative_path
            .rsplit('/')
            .next()
            .filter(|value| !value.is_empty())
        else {
            return;
        };
        self.evidence.try_publish(
            crate::contexts::workspaces::api::WorkspaceEvidenceSignal::FileMutationObserved {
                session_id: mutation.session_id.clone(),
                // The file's own name. The directory it sits in stays here: a workspace path says
                // where someone works, which is not what "this file changed" needs to say.
                basename: basename.to_string(),
                path_fingerprint: path_fingerprint(&mutation.relative_path),
                change_kind: match mutation.change_kind {
                    crate::contexts::agent_runtime::application::AgentWorkspaceChangeKind::Created => {
                        crate::contexts::workspaces::api::WorkspaceFileChangeKind::Created
                    }
                    crate::contexts::agent_runtime::application::AgentWorkspaceChangeKind::Modified => {
                        crate::contexts::workspaces::api::WorkspaceFileChangeKind::Modified
                    }
                },
                // The runtime performed this write itself, so the witness is the write: there is
                // no earlier snapshot to compare against, and inventing one would imply a
                // comparison nobody made.
                witness_fingerprint: mutation_witness(&mutation),
                observed_directly: true,
                occurred_at: chrono::Utc::now().to_rfc3339(),
            },
        );
    }
}

/// A stable digest of the workspace-relative path.
///
/// Groups two changes to one file without the path ever being stored. Truncated to the identifier
/// bound the journal enforces, which is far more entropy than a workspace has files.
fn path_fingerprint(relative_path: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(relative_path.as_bytes());
    digest
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// What this observation was made against. A direct write witnesses itself, so the witness is the
/// path digest and the change kind together: two writes to one file are two observations, and a
/// witness that ignored the kind would collapse a create and a later modify into one.
fn mutation_witness(mutation: &AgentWorkspaceMutation) -> String {
    format!(
        "{}:{}",
        path_fingerprint(&mutation.relative_path),
        match mutation.change_kind {
            crate::contexts::agent_runtime::application::AgentWorkspaceChangeKind::Created =>
                "created",
            crate::contexts::agent_runtime::application::AgentWorkspaceChangeKind::Modified =>
                "modified",
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::code_intelligence::api::{LanguageFamily, LspConfiguration};
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
            .get_mut(&LanguageFamily::Rust)
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
