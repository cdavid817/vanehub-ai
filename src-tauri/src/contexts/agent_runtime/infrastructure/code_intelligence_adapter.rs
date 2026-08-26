use crate::contexts::agent_runtime::application::{
    AgentCallHierarchyInput, AgentCodeCallRelation, AgentCodeDiagnostic, AgentCodeHover,
    AgentCodeIntelligenceContext, AgentCodeIntelligenceMetadata, AgentCodeIntelligenceOutcome,
    AgentCodeIntelligencePending, AgentCodeIntelligencePort, AgentCodeIntelligenceResponderPort,
    AgentCodeIntelligenceStatus, AgentCodeLocation, AgentCodeSymbol, AgentDocumentInput,
    AgentDocumentPositionInput, AgentWorkspaceSymbolInput,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEFAULT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);
const RESPONSE_POLL_INTERVAL: Duration = Duration::from_millis(20);

pub(crate) struct RuntimeAgentCodeIntelligenceAdapter {
    responder: Arc<dyn AgentCodeIntelligenceResponderPort>,
    response_timeout: Duration,
}

impl RuntimeAgentCodeIntelligenceAdapter {
    pub(crate) fn new(responder: Arc<dyn AgentCodeIntelligenceResponderPort>) -> Self {
        Self {
            responder,
            response_timeout: DEFAULT_RESPONSE_TIMEOUT,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_timeout(
        responder: Arc<dyn AgentCodeIntelligenceResponderPort>,
        response_timeout: Duration,
    ) -> Self {
        Self {
            responder,
            response_timeout,
        }
    }

    fn wait<T>(
        &self,
        pending: AgentCodeIntelligencePending<T>,
        cancelled: &AtomicBool,
    ) -> AgentCodeIntelligenceOutcome<T> {
        let deadline = Instant::now() + self.response_timeout;
        loop {
            if cancelled.load(Ordering::Acquire) {
                (pending.cancel)();
                return degraded(AgentCodeIntelligenceStatus::Failed, "generation_cancelled");
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                (pending.cancel)();
                return degraded(AgentCodeIntelligenceStatus::Timeout, "request_timeout");
            }
            match pending
                .response
                .recv_timeout(remaining.min(RESPONSE_POLL_INTERVAL))
            {
                Ok(outcome) => return outcome,
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    return degraded(AgentCodeIntelligenceStatus::Failed, "responder_closed")
                }
            }
        }
    }
}

impl AgentCodeIntelligencePort for RuntimeAgentCodeIntelligenceAdapter {
    fn is_available(&self, context: &AgentCodeIntelligenceContext) -> bool {
        self.responder.is_available(context)
    }

    fn find_definition(
        &self,
        context: &AgentCodeIntelligenceContext,
        input: &AgentDocumentPositionInput,
        cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeLocation>> {
        self.wait(
            self.responder
                .start_find_definition(context.clone(), input.clone()),
            &cancelled,
        )
    }

    fn find_references(
        &self,
        context: &AgentCodeIntelligenceContext,
        input: &AgentDocumentPositionInput,
        cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeLocation>> {
        self.wait(
            self.responder
                .start_find_references(context.clone(), input.clone()),
            &cancelled,
        )
    }

    fn get_hover(
        &self,
        context: &AgentCodeIntelligenceContext,
        input: &AgentDocumentPositionInput,
        cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Option<AgentCodeHover>> {
        self.wait(
            self.responder
                .start_get_hover(context.clone(), input.clone()),
            &cancelled,
        )
    }

    fn get_diagnostics(
        &self,
        context: &AgentCodeIntelligenceContext,
        input: &AgentDocumentInput,
        cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeDiagnostic>> {
        self.wait(
            self.responder
                .start_get_diagnostics(context.clone(), input.clone()),
            &cancelled,
        )
    }

    fn find_type_definition(
        &self,
        context: &AgentCodeIntelligenceContext,
        input: &AgentDocumentPositionInput,
        cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeLocation>> {
        self.wait(
            self.responder
                .start_find_type_definition(context.clone(), input.clone()),
            &cancelled,
        )
    }

    fn find_implementations(
        &self,
        context: &AgentCodeIntelligenceContext,
        input: &AgentDocumentPositionInput,
        cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeLocation>> {
        self.wait(
            self.responder
                .start_find_implementations(context.clone(), input.clone()),
            &cancelled,
        )
    }

    fn find_workspace_symbols(
        &self,
        context: &AgentCodeIntelligenceContext,
        input: &AgentWorkspaceSymbolInput,
        cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeSymbol>> {
        self.wait(
            self.responder
                .start_find_workspace_symbols(context.clone(), input.clone()),
            &cancelled,
        )
    }

    fn get_document_symbols(
        &self,
        context: &AgentCodeIntelligenceContext,
        input: &AgentDocumentInput,
        cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeSymbol>> {
        self.wait(
            self.responder
                .start_get_document_symbols(context.clone(), input.clone()),
            &cancelled,
        )
    }

    fn find_call_hierarchy(
        &self,
        context: &AgentCodeIntelligenceContext,
        input: &AgentCallHierarchyInput,
        cancelled: Arc<AtomicBool>,
    ) -> AgentCodeIntelligenceOutcome<Vec<AgentCodeCallRelation>> {
        self.wait(
            self.responder
                .start_find_call_hierarchy(context.clone(), input.clone()),
            &cancelled,
        )
    }
}

#[derive(Default)]
pub(crate) struct UnavailableAgentCodeIntelligenceResponder;

impl AgentCodeIntelligenceResponderPort for UnavailableAgentCodeIntelligenceResponder {
    fn is_available(&self, _: &AgentCodeIntelligenceContext) -> bool {
        false
    }

    fn start_find_definition(
        &self,
        _: AgentCodeIntelligenceContext,
        _: AgentDocumentPositionInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeLocation>> {
        unavailable_pending()
    }

    fn start_find_references(
        &self,
        _: AgentCodeIntelligenceContext,
        _: AgentDocumentPositionInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeLocation>> {
        unavailable_pending()
    }

    fn start_get_hover(
        &self,
        _: AgentCodeIntelligenceContext,
        _: AgentDocumentPositionInput,
    ) -> AgentCodeIntelligencePending<Option<AgentCodeHover>> {
        unavailable_pending()
    }

    fn start_get_diagnostics(
        &self,
        _: AgentCodeIntelligenceContext,
        _: AgentDocumentInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeDiagnostic>> {
        unavailable_pending()
    }

    fn start_find_type_definition(
        &self,
        _: AgentCodeIntelligenceContext,
        _: AgentDocumentPositionInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeLocation>> {
        unavailable_pending()
    }

    fn start_find_implementations(
        &self,
        _: AgentCodeIntelligenceContext,
        _: AgentDocumentPositionInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeLocation>> {
        unavailable_pending()
    }

    fn start_find_workspace_symbols(
        &self,
        _: AgentCodeIntelligenceContext,
        _: AgentWorkspaceSymbolInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeSymbol>> {
        unavailable_pending()
    }

    fn start_get_document_symbols(
        &self,
        _: AgentCodeIntelligenceContext,
        _: AgentDocumentInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeSymbol>> {
        unavailable_pending()
    }

    fn start_find_call_hierarchy(
        &self,
        _: AgentCodeIntelligenceContext,
        _: AgentCallHierarchyInput,
    ) -> AgentCodeIntelligencePending<Vec<AgentCodeCallRelation>> {
        unavailable_pending()
    }
}

fn unavailable_pending<T>() -> AgentCodeIntelligencePending<T> {
    let (send, response) = mpsc::channel();
    let _ = send.send(degraded(
        AgentCodeIntelligenceStatus::Unavailable,
        "not_configured",
    ));
    AgentCodeIntelligencePending {
        response,
        cancel: Arc::new(|| {}),
    }
}

fn degraded<T>(
    status: AgentCodeIntelligenceStatus,
    reason_code: &str,
) -> AgentCodeIntelligenceOutcome<T> {
    AgentCodeIntelligenceOutcome {
        metadata: AgentCodeIntelligenceMetadata {
            status,
            server: None,
            language: None,
            document_version: None,
            stale: false,
            returned_count: 0,
            total: 0,
            truncated: false,
            filtered_count: 0,
            reason_code: Some(reason_code.to_owned()),
        },
        value: None,
    }
}
