use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const MAX_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DelegationProviderEvent {
    Initialized,
    Progress,
    ActionStarted {
        id: String,
    },
    ActionUpdated {
        id: String,
    },
    ActionCompleted {
        id: String,
    },
    UsageUpdated,
    ProviderRetry,
    FinalCandidate,
    ProviderError,
    UnknownEvent {
        event_type: String,
        hash: String,
        size: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClaudeProtocolError {
    FrameLimit,
    MalformedJson,
    EventBeforeInitialization,
    DuplicateInitialization,
    InvalidActionOrder,
    InvalidTerminal,
    DuplicateTerminal,
    MissingTerminal,
    ExitMismatch,
}

pub(crate) struct ClaudeDelegationAdapter {
    initialized: bool,
    terminal: bool,
    active_actions: BTreeSet<String>,
    structured_output: Option<Value>,
}

impl ClaudeDelegationAdapter {
    pub(crate) fn new() -> Self {
        Self {
            initialized: false,
            terminal: false,
            active_actions: BTreeSet::new(),
            structured_output: None,
        }
    }

    pub(crate) fn decode_stdout_line(
        &mut self,
        bytes: &[u8],
    ) -> Result<Vec<DelegationProviderEvent>, ClaudeProtocolError> {
        if bytes.len() > MAX_FRAME_BYTES {
            return Err(ClaudeProtocolError::FrameLimit);
        }
        let value: Value =
            serde_json::from_slice(bytes).map_err(|_| ClaudeProtocolError::MalformedJson)?;
        let event_type = value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if matches!(event_type, "system" | "session_init") {
            return self.initialize(&value);
        }
        if !self.initialized {
            return Err(ClaudeProtocolError::EventBeforeInitialization);
        }
        if matches!(event_type, "result" | "complete" | "completed") {
            return self.terminal(&value);
        }
        if event_type == "stream_event" {
            let event = value
                .get("event")
                .filter(|candidate| candidate.is_object())
                .ok_or(ClaudeProtocolError::MalformedJson)?;
            let nested_type = event
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            return self.non_terminal(nested_type, event);
        }
        self.non_terminal(event_type, &value)
    }

    pub(crate) fn finalize(self, exit_code: Option<i32>) -> Result<Value, ClaudeProtocolError> {
        if !self.terminal {
            return Err(ClaudeProtocolError::MissingTerminal);
        }
        if exit_code != Some(0) || !self.active_actions.is_empty() {
            return Err(ClaudeProtocolError::ExitMismatch);
        }
        self.structured_output
            .ok_or(ClaudeProtocolError::InvalidTerminal)
    }

    fn initialize(
        &mut self,
        value: &Value,
    ) -> Result<Vec<DelegationProviderEvent>, ClaudeProtocolError> {
        if self.initialized || self.terminal {
            return Err(ClaudeProtocolError::DuplicateInitialization);
        }
        let session = value
            .get("session_id")
            .or_else(|| value.get("sessionId"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        if session.is_empty() {
            return Err(ClaudeProtocolError::EventBeforeInitialization);
        }
        self.initialized = true;
        Ok(vec![DelegationProviderEvent::Initialized])
    }

    fn terminal(
        &mut self,
        value: &Value,
    ) -> Result<Vec<DelegationProviderEvent>, ClaudeProtocolError> {
        if self.terminal {
            return Err(ClaudeProtocolError::DuplicateTerminal);
        }
        self.terminal = true;
        if value.get("is_error").and_then(Value::as_bool) == Some(true)
            || value.get("subtype").and_then(Value::as_str) != Some("success")
        {
            return Ok(vec![DelegationProviderEvent::ProviderError]);
        }
        let output = value
            .get("structured_output")
            .filter(|candidate| candidate.is_object())
            .cloned()
            .ok_or(ClaudeProtocolError::InvalidTerminal)?;
        self.structured_output = Some(output);
        let mut events = Vec::new();
        if value.get("usage").is_some() {
            events.push(DelegationProviderEvent::UsageUpdated);
        }
        events.push(DelegationProviderEvent::FinalCandidate);
        Ok(events)
    }

    fn non_terminal(
        &mut self,
        event_type: &str,
        value: &Value,
    ) -> Result<Vec<DelegationProviderEvent>, ClaudeProtocolError> {
        if self.terminal {
            return Err(ClaudeProtocolError::DuplicateTerminal);
        }
        match event_type {
            "assistant" => self.message_actions(value, "tool_use", true),
            "user" => self.message_actions(value, "tool_result", false),
            "assistant_delta"
            | "content_block_delta"
            | "content_block_start"
            | "content_block_stop"
            | "message_start"
            | "message_delta"
            | "message_stop" => Ok(vec![DelegationProviderEvent::Progress]),
            "tool_use" => self.start_action(value),
            "tool_result" | "tool_error" | "tool_failure" => self.finish_action(value),
            "usage" | "usage_update" => Ok(vec![DelegationProviderEvent::UsageUpdated]),
            "retry" | "provider_retry" => Ok(vec![DelegationProviderEvent::ProviderRetry]),
            "error" | "failed" => Ok(vec![DelegationProviderEvent::ProviderError]),
            _ => Ok(vec![unknown(event_type, value)]),
        }
    }

    fn message_actions(
        &mut self,
        value: &Value,
        expected_type: &str,
        start: bool,
    ) -> Result<Vec<DelegationProviderEvent>, ClaudeProtocolError> {
        let blocks = value
            .pointer("/message/content")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|block| block.get("type").and_then(Value::as_str) == Some(expected_type));
        let mut events = Vec::new();
        for block in blocks {
            events.extend(if start {
                self.start_action(block)?
            } else {
                self.finish_action(block)?
            });
        }
        if events.is_empty() {
            events.push(DelegationProviderEvent::Progress);
        }
        Ok(events)
    }

    fn start_action(
        &mut self,
        value: &Value,
    ) -> Result<Vec<DelegationProviderEvent>, ClaudeProtocolError> {
        let Some(id) = action_id(value) else {
            return Ok(vec![DelegationProviderEvent::Progress]);
        };
        if !self.active_actions.insert(id.clone()) {
            return Err(ClaudeProtocolError::InvalidActionOrder);
        }
        Ok(vec![DelegationProviderEvent::ActionStarted { id }])
    }

    fn finish_action(
        &mut self,
        value: &Value,
    ) -> Result<Vec<DelegationProviderEvent>, ClaudeProtocolError> {
        let id = action_id(value).ok_or(ClaudeProtocolError::InvalidActionOrder)?;
        if !self.active_actions.remove(&id) {
            return Err(ClaudeProtocolError::InvalidActionOrder);
        }
        Ok(vec![DelegationProviderEvent::ActionCompleted { id }])
    }
}

fn action_id(value: &Value) -> Option<String> {
    value
        .get("id")
        .or_else(|| value.get("tool_use_id"))
        .or_else(|| value.pointer("/content_block/id"))
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && id.len() <= 256)
        .map(str::to_string)
}

fn unknown(event_type: &str, value: &Value) -> DelegationProviderEvent {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let digest = Sha256::digest(&bytes);
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    DelegationProviderEvent::UnknownEvent {
        event_type: event_type.chars().take(64).collect(),
        hash: format!("sha256:{hex}"),
        size: bytes.len(),
    }
}

#[cfg(test)]
#[path = "claude_protocol_tests.rs"]
mod tests;
