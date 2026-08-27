use super::ports::{
    AgentCallHierarchyInput, AgentCodeCallRelation, AgentCodeDiagnostic, AgentCodeHover,
    AgentCodeIntelligenceContext, AgentCodeIntelligenceMetadata, AgentCodeIntelligenceOutcome,
    AgentCodeIntelligencePort, AgentCodeIntelligenceStatus, AgentCodeLocation, AgentCodeSymbol,
    AgentDocumentInput, AgentDocumentPositionInput, AgentWorkspaceMutation,
    AgentWorkspaceMutationPort, AgentWorkspaceSymbolInput,
};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

#[test]
fn model_facing_query_inputs_have_no_workspace_or_server_selector() {
    let position = AgentDocumentPositionInput {
        relative_path: "src/main.rs".to_owned(),
        line: 4,
        column: 7,
    };
    let AgentDocumentPositionInput {
        relative_path,
        line,
        column,
    } = position;
    assert_eq!(
        (relative_path.as_str(), line, column),
        ("src/main.rs", 4, 7)
    );

    let document = AgentDocumentInput {
        relative_path: "src/main.rs".to_owned(),
    };
    let AgentDocumentInput { relative_path } = document;
    assert_eq!(relative_path, "src/main.rs");

    assert_eq!(
        [
            AgentCodeIntelligenceStatus::Ready,
            AgentCodeIntelligenceStatus::Warming,
            AgentCodeIntelligenceStatus::Timeout,
            AgentCodeIntelligenceStatus::Unavailable,
            AgentCodeIntelligenceStatus::Failed,
        ]
        .len(),
        5
    );
}

#[test]
fn session_context_is_separate_from_queries_and_port_methods_are_cancellable() {
    let port = CapturingCodeIntelligence::default();
    let context = AgentCodeIntelligenceContext::from_session_workspace("C:/workspace");
    let position = AgentDocumentPositionInput {
        relative_path: "src/main.rs".to_owned(),
        line: 1,
        column: 1,
    };
    let cancellation = Arc::new(AtomicBool::new(false));

    assert!(port.is_available(&context));
    let _ = port.find_definition(&context, &position, cancellation.clone());
    let _ = port.find_references(&context, &position, cancellation.clone());
    let _ = port.get_hover(&context, &position, cancellation.clone());
    let _ = port.get_diagnostics(
        &context,
        &AgentDocumentInput {
            relative_path: "src/main.rs".to_owned(),
        },
        cancellation,
    );

    assert_eq!(
        port.workspaces.lock().expect("workspaces").as_slice(),
        ["C:/workspace"; 5]
    );
}

#[test]
fn mutation_contract_contains_only_canonical_workspace_and_normalized_path() {
    let port = CapturingMutationPort::default();
    let mutation = AgentWorkspaceMutation {
        canonical_workspace: PathBuf::from("C:/workspace"),
        relative_path: "src/main.rs".to_owned(),
    };

    port.publish(mutation);

    let captured = port.mutations.lock().expect("mutations");
    let AgentWorkspaceMutation {
        canonical_workspace,
        relative_path,
    } = &captured[0];
    assert_eq!(canonical_workspace, &PathBuf::from("C:/workspace"));
    assert_eq!(relative_path, "src/main.rs");
}

#[derive(Default)]
struct CapturingCodeIntelligence {
    workspaces: Mutex<Vec<String>>,
}

impl CapturingCodeIntelligence {
    fn capture(&self, context: &AgentCodeIntelligenceContext) {
        self.workspaces
            .lock()
            .expect("workspaces")
            .push(context.session_workspace().to_owned());
    }

    fn outcome<T>(&self, value: T) -> AgentCodeIntelligenceOutcome<T> {
        AgentCodeIntelligenceOutcome {
            metadata: AgentCodeIntelligenceMetadata {
                status: AgentCodeIntelligenceStatus::Ready,
                server: None,
                language: None,
                document_version: Some(1),
                stale: false,
                returned_count: 0,
                total: 0,
                truncated: false,
                filtered_count: 0,
                reason_code: None,
            },
            value: Some(value),
        }
    }
}

impl AgentCodeIntelligencePort for CapturingCodeIntelligence {
    fn is_available(&self, context: &AgentCodeIntelligenceContext) -> bool {
        self.capture(context);
        true
    }

    fn find_definition(
        &self,
        context: &AgentCodeIntelligenceContext,
        _input: &AgentDocumentPositionInput,
        _cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeLocation>> {
        self.capture(context);
        self.outcome(Vec::new())
    }

    fn find_references(
        &self,
        context: &AgentCodeIntelligenceContext,
        _input: &AgentDocumentPositionInput,
        _cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeLocation>> {
        self.capture(context);
        self.outcome(Vec::new())
    }

    fn get_hover(
        &self,
        context: &AgentCodeIntelligenceContext,
        _input: &AgentDocumentPositionInput,
        _cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Option<AgentCodeHover>> {
        self.capture(context);
        self.outcome(None)
    }

    fn get_diagnostics(
        &self,
        context: &AgentCodeIntelligenceContext,
        _input: &AgentDocumentInput,
        _cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeDiagnostic>> {
        self.capture(context);
        self.outcome(Vec::new())
    }

    fn find_type_definition(
        &self,
        context: &AgentCodeIntelligenceContext,
        _input: &AgentDocumentPositionInput,
        _cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeLocation>> {
        self.capture(context);
        self.outcome(Vec::new())
    }

    fn find_implementations(
        &self,
        context: &AgentCodeIntelligenceContext,
        _input: &AgentDocumentPositionInput,
        _cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeLocation>> {
        self.capture(context);
        self.outcome(Vec::new())
    }

    fn find_workspace_symbols(
        &self,
        context: &AgentCodeIntelligenceContext,
        _input: &AgentWorkspaceSymbolInput,
        _cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeSymbol>> {
        self.capture(context);
        self.outcome(Vec::new())
    }

    fn get_document_symbols(
        &self,
        context: &AgentCodeIntelligenceContext,
        _input: &AgentDocumentInput,
        _cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeSymbol>> {
        self.capture(context);
        self.outcome(Vec::new())
    }

    fn find_call_hierarchy(
        &self,
        context: &AgentCodeIntelligenceContext,
        _input: &AgentCallHierarchyInput,
        _cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeCallRelation>> {
        self.capture(context);
        self.outcome(Vec::new())
    }
}

#[derive(Default)]
struct CapturingMutationPort {
    mutations: Mutex<Vec<AgentWorkspaceMutation>>,
}

impl AgentWorkspaceMutationPort for CapturingMutationPort {
    fn publish(&self, mutation: AgentWorkspaceMutation) {
        self.mutations.lock().expect("mutations").push(mutation);
    }
}
