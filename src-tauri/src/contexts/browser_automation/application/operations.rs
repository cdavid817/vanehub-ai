use super::{
    BrowserContextPolicy, BrowserHandoffError, BrowserHandoffManager, BrowserOwnership,
    BrowserSessionError, BrowserSessionManager,
};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowserAction {
    Navigate,
    GoBack,
    GoForward,
    Inspect,
    Click,
    Fill,
    Extract,
    Screenshot,
    Evaluate,
}

impl BrowserAction {
    fn method(self) -> &'static str {
        match self {
            Self::Navigate => "page.navigate",
            Self::GoBack => "page.go_back",
            Self::GoForward => "page.go_forward",
            Self::Inspect => "page.inspect",
            Self::Click => "page.click",
            Self::Fill => "page.fill",
            Self::Extract => "page.extract",
            Self::Screenshot => "page.screenshot",
            Self::Evaluate => "page.evaluate",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BrowserOperationRequest {
    pub(crate) ownership: BrowserOwnership,
    pub(crate) action: BrowserAction,
    pub(crate) page_id: Option<String>,
    pub(crate) input: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BrowserOperationResult {
    pub(crate) contract_version: u16,
    pub(crate) action: BrowserAction,
    pub(crate) page_id: String,
    pub(crate) frame_id: Option<String>,
    pub(crate) url: Option<String>,
    pub(crate) payload: Value,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BrowserOperationError {
    InvalidInput,
    Session(BrowserSessionError),
    ProtocolFailure,
    UnsafeResult,
    Handoff(BrowserHandoffError),
}

pub(crate) struct BrowserOperationService {
    sessions: BrowserSessionManager,
    policy: BrowserContextPolicy,
    handoffs: Arc<BrowserHandoffManager>,
}

impl BrowserOperationService {
    pub(crate) fn new(sessions: BrowserSessionManager, policy: BrowserContextPolicy) -> Self {
        Self {
            sessions,
            policy,
            handoffs: Arc::new(BrowserHandoffManager::default()),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn with_handoff_manager(
        sessions: BrowserSessionManager,
        policy: BrowserContextPolicy,
        handoffs: Arc<BrowserHandoffManager>,
    ) -> Self {
        Self {
            sessions,
            policy,
            handoffs,
        }
    }

    pub(crate) fn start(&self, ownership: BrowserOwnership) -> Result<(), BrowserOperationError> {
        self.sessions
            .ensure_session(ownership, self.policy)
            .map_err(BrowserOperationError::Session)
    }

    pub(crate) fn begin_handoff(
        &self,
        ownership: BrowserOwnership,
        page_id: String,
        duration: Duration,
    ) -> Result<super::BrowserHandoff, BrowserOperationError> {
        let handoff = self
            .handoffs
            .begin(ownership.clone(), page_id.clone(), Instant::now(), duration)
            .map_err(BrowserOperationError::Handoff)?;
        let sidecar = self
            .sessions
            .with_session(ownership.clone(), self.policy, |session| {
                session
                    .request("page.handoff", json!({"page_id": page_id, "input": {}}))
                    .map_err(BrowserSessionError::ProtocolFailure)
            });
        if let Err(error) = sidecar {
            let _ = self.handoffs.close(&ownership);
            return Err(BrowserOperationError::Session(error));
        }
        Ok(handoff)
    }

    #[allow(dead_code)]
    pub(crate) fn resume_handoff(
        &self,
        ownership: &BrowserOwnership,
        page_id: &str,
        handoff_id: &str,
        explicit_user_action: bool,
    ) -> Result<u64, BrowserOperationError> {
        let revision = self
            .handoffs
            .resume(ownership, handoff_id, Instant::now(), explicit_user_action)
            .map_err(BrowserOperationError::Handoff)?;
        self.sessions
            .with_session(ownership.clone(), self.policy, |session| {
                session
                    .request("page.resume", json!({"page_id": page_id, "input": {}}))
                    .map_err(BrowserSessionError::ProtocolFailure)
                    .map(|_| ())
            })
            .map_err(BrowserOperationError::Session)?;
        Ok(revision)
    }

    pub(crate) fn close(&self, ownership: &BrowserOwnership) -> Result<(), BrowserOperationError> {
        self.handoffs
            .close(ownership)
            .map_err(BrowserOperationError::Handoff)?;
        self.sessions
            .close_generation(ownership)
            .map_err(BrowserOperationError::Session)
    }

    pub(crate) fn execute(
        &self,
        request: BrowserOperationRequest,
    ) -> Result<BrowserOperationResult, BrowserOperationError> {
        validate_request(&request)?;
        self.handoffs
            .ensure_automation_allowed(&request.ownership, request.action, Instant::now())
            .map_err(BrowserOperationError::Handoff)?;
        let action = request.action;
        let ownership = request.ownership.clone();
        let page_id = request.page_id.clone();
        let input = request.input;
        let result = self
            .sessions
            .with_session(request.ownership, self.policy, |session| {
                let response = session
                    .request(action.method(), json!({"page_id": page_id, "input": input}))
                    .map_err(BrowserSessionError::ProtocolFailure)?;
                if !response.ok {
                    return Err(BrowserSessionError::UnsafeContext);
                }
                project_result(action, response.result)
                    .map_err(|_| BrowserSessionError::UnsafeContext)
            })
            .map_err(|error| match error {
                BrowserSessionError::UnsafeContext => BrowserOperationError::UnsafeResult,
                other => BrowserOperationError::Session(other),
            })?;
        self.handoffs
            .record_completed(&ownership, action)
            .map_err(BrowserOperationError::Handoff)?;
        Ok(result)
    }
}

fn validate_request(request: &BrowserOperationRequest) -> Result<(), BrowserOperationError> {
    if !request.input.is_object() {
        return Err(BrowserOperationError::InvalidInput);
    }
    if let Some(page_id) = &request.page_id {
        if page_id.is_empty() || page_id.len() > 128 {
            return Err(BrowserOperationError::InvalidInput);
        }
    }
    let encoded =
        serde_json::to_vec(&request.input).map_err(|_| BrowserOperationError::InvalidInput)?;
    if encoded.len() > 64 * 1024 {
        return Err(BrowserOperationError::InvalidInput);
    }
    match request.action {
        BrowserAction::Navigate => string_field(&request.input, "url", 4096)?,
        BrowserAction::Click | BrowserAction::Extract => {
            string_field(&request.input, "selector", 1024)?
        }
        BrowserAction::Fill => {
            string_field(&request.input, "selector", 1024)?;
            string_field(&request.input, "text", 16_384)?;
        }
        BrowserAction::Evaluate => string_field(&request.input, "expression", 16_384)?,
        _ => {}
    }
    Ok(())
}

fn string_field(input: &Value, name: &str, max_chars: usize) -> Result<(), BrowserOperationError> {
    let value = input
        .get(name)
        .and_then(Value::as_str)
        .ok_or(BrowserOperationError::InvalidInput)?;
    if value.is_empty() || value.chars().count() > max_chars {
        return Err(BrowserOperationError::InvalidInput);
    }
    Ok(())
}

fn project_result(
    action: BrowserAction,
    result: Option<Value>,
) -> Result<BrowserOperationResult, BrowserOperationError> {
    let result = result.ok_or(BrowserOperationError::ProtocolFailure)?;
    let page_id = result
        .get("page_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .ok_or(BrowserOperationError::UnsafeResult)?
        .to_string();
    let payload = result.get("payload").cloned().unwrap_or(Value::Null);
    if serde_json::to_vec(&payload)
        .map_err(|_| BrowserOperationError::UnsafeResult)?
        .len()
        > 256 * 1024
    {
        return Err(BrowserOperationError::UnsafeResult);
    }
    Ok(BrowserOperationResult {
        contract_version: 1,
        action,
        page_id,
        frame_id: result
            .get("frame_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        url: result
            .get("url")
            .and_then(Value::as_str)
            .map(str::to_string),
        payload,
        truncated: result
            .get("truncated")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

#[cfg(test)]
#[path = "operations_tests.rs"]
mod tests;
