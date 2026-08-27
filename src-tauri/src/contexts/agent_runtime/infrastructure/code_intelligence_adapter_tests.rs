use super::code_intelligence_adapter::RuntimeAgentCodeIntelligenceAdapter;
use crate::contexts::agent_runtime::application::{
    AgentCallHierarchyInput, AgentCodeCallRelation, AgentCodeDiagnostic, AgentCodeHover,
    AgentCodeIntelligenceContext, AgentCodeIntelligenceMetadata, AgentCodeIntelligenceOutcome,
    AgentCodeIntelligencePending, AgentCodeIntelligencePort, AgentCodeIntelligenceResponderPort,
    AgentCodeIntelligenceStatus, AgentCodeLocation, AgentCodeSymbol, AgentDocumentInput,
    AgentDocumentPositionInput, AgentWorkspaceSymbolInput,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

struct FakeResponder {
    available: bool,
    delayed: bool,
    cancel_called: Arc<AtomicBool>,
}

impl AgentCodeIntelligenceResponderPort for FakeResponder {
    fn is_available(&self, _: &AgentCodeIntelligenceContext) -> bool {
        self.available
    }

    fn start_find_definition(
        &self,
        _: AgentCodeIntelligenceContext,
        _: AgentDocumentPositionInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeLocation>> {
        if self.delayed {
            return delayed_pending(self.cancel_called.clone());
        }
        immediate_pending(ready(Vec::new()))
    }

    fn start_find_references(
        &self,
        _: AgentCodeIntelligenceContext,
        _: AgentDocumentPositionInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeLocation>> {
        immediate_pending(ready(Vec::new()))
    }

    fn start_get_hover(
        &self,
        _: AgentCodeIntelligenceContext,
        _: AgentDocumentPositionInput,
    ) -> AgentCodeIntelligencePending<Option<AgentCodeHover>> {
        immediate_pending(ready(None))
    }

    fn start_get_diagnostics(
        &self,
        _: AgentCodeIntelligenceContext,
        _: AgentDocumentInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeDiagnostic>> {
        immediate_pending(ready(Vec::new()))
    }

    fn start_find_type_definition(
        &self,
        _: AgentCodeIntelligenceContext,
        _: AgentDocumentPositionInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeLocation>> {
        immediate_pending(ready(Vec::new()))
    }

    fn start_find_implementations(
        &self,
        _: AgentCodeIntelligenceContext,
        _: AgentDocumentPositionInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeLocation>> {
        immediate_pending(ready(Vec::new()))
    }

    fn start_find_workspace_symbols(
        &self,
        _: AgentCodeIntelligenceContext,
        _: AgentWorkspaceSymbolInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeSymbol>> {
        immediate_pending(ready(Vec::new()))
    }

    fn start_get_document_symbols(
        &self,
        _: AgentCodeIntelligenceContext,
        _: AgentDocumentInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeSymbol>> {
        immediate_pending(ready(Vec::new()))
    }

    fn start_find_call_hierarchy(
        &self,
        _: AgentCodeIntelligenceContext,
        _: AgentCallHierarchyInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeCallRelation>> {
        immediate_pending(ready(Vec::new()))
    }
}

#[test]
fn synchronous_adapter_forwards_availability_and_async_response() {
    let adapter = adapter(false, Duration::from_secs(1));
    let context = context();

    assert!(adapter.is_available(&context));
    let outcome = adapter.find_definition(&context, &position(), Arc::new(AtomicBool::new(false)));

    assert_eq!(outcome.metadata.status, AgentCodeIntelligenceStatus::Ready);
    assert_eq!(outcome.value, Some(Vec::new()));
}

#[test]
fn generation_cancellation_stops_waiting_and_calls_actor_cancel_hook() {
    let cancel_called = Arc::new(AtomicBool::new(false));
    let responder = Arc::new(FakeResponder {
        available: true,
        delayed: true,
        cancel_called: cancel_called.clone(),
    });
    let adapter =
        RuntimeAgentCodeIntelligenceAdapter::with_timeout(responder, Duration::from_secs(1));
    let cancelled = Arc::new(AtomicBool::new(true));

    let outcome = adapter.find_definition(&context(), &position(), cancelled);

    assert!(cancel_called.load(Ordering::Acquire));
    assert_eq!(outcome.metadata.status, AgentCodeIntelligenceStatus::Failed);
    assert_eq!(
        outcome.metadata.reason_code.as_deref(),
        Some("generation_cancelled")
    );
}

#[test]
fn response_deadline_returns_fail_soft_timeout_and_cancels_actor_request() {
    let cancel_called = Arc::new(AtomicBool::new(false));
    let responder = Arc::new(FakeResponder {
        available: true,
        delayed: true,
        cancel_called: cancel_called.clone(),
    });
    let adapter =
        RuntimeAgentCodeIntelligenceAdapter::with_timeout(responder, Duration::from_millis(25));

    let outcome =
        adapter.find_definition(&context(), &position(), Arc::new(AtomicBool::new(false)));

    assert!(cancel_called.load(Ordering::Acquire));
    assert_eq!(
        outcome.metadata.status,
        AgentCodeIntelligenceStatus::Timeout
    );
    assert_eq!(
        outcome.metadata.reason_code.as_deref(),
        Some("request_timeout")
    );
}

fn adapter(delayed: bool, timeout: Duration) -> RuntimeAgentCodeIntelligenceAdapter {
    RuntimeAgentCodeIntelligenceAdapter::with_timeout(
        Arc::new(FakeResponder {
            available: true,
            delayed,
            cancel_called: Arc::new(AtomicBool::new(false)),
        }),
        timeout,
    )
}

fn context() -> AgentCodeIntelligenceContext {
    AgentCodeIntelligenceContext::from_session_workspace("workspace")
}

fn position() -> AgentDocumentPositionInput {
    AgentDocumentPositionInput {
        relative_path: "src/lib.rs".to_owned(),
        line: 1,
        column: 1,
    }
}

fn immediate_pending<T: Send + 'static>(
    outcome: AgentCodeIntelligenceOutcome<T>,
) -> AgentCodeIntelligencePending<T> {
    let (send, response) = mpsc::channel();
    send.send(outcome).expect("send response");
    AgentCodeIntelligencePending {
        response,
        cancel: Arc::new(|| {}),
    }
}

fn delayed_pending<T: Send + 'static>(
    cancel_called: Arc<AtomicBool>,
) -> AgentCodeIntelligencePending<T> {
    let (send, response) = mpsc::channel();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        drop(send);
    });
    AgentCodeIntelligencePending {
        response,
        cancel: Arc::new(move || cancel_called.store(true, Ordering::Release)),
    }
}

fn ready<T>(value: T) -> AgentCodeIntelligenceOutcome<T> {
    AgentCodeIntelligenceOutcome {
        metadata: AgentCodeIntelligenceMetadata {
            status: AgentCodeIntelligenceStatus::Ready,
            server: Some("fixture".to_owned()),
            language: Some("rust".to_owned()),
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
