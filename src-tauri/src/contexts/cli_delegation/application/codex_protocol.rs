use super::DelegationProviderEvent;
use serde_json::Value;

const MAX_FRAME_BYTES: usize = 1024 * 1024;
const MAX_FINAL_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodexProtocolError {
    FrameLimit,
    MalformedJson,
    EventBeforeInitialization,
    DuplicateInitialization,
    DuplicateTerminal,
    MissingTerminal,
    ProviderFailure,
    ExitMismatch,
    InvalidFinalOutput,
}

pub(crate) struct CodexDelegationAdapter {
    initialized: bool,
    terminal: bool,
}

impl CodexDelegationAdapter {
    pub(crate) fn new() -> Self {
        Self {
            initialized: false,
            terminal: false,
        }
    }

    pub(crate) fn decode_stdout_line(
        &mut self,
        bytes: &[u8],
    ) -> Result<Vec<DelegationProviderEvent>, CodexProtocolError> {
        if bytes.len() > MAX_FRAME_BYTES {
            return Err(CodexProtocolError::FrameLimit);
        }
        let value: Value =
            serde_json::from_slice(bytes).map_err(|_| CodexProtocolError::MalformedJson)?;
        let kind = value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match kind {
            "thread.started" => {
                if self.initialized {
                    return Err(CodexProtocolError::DuplicateInitialization);
                }
                self.initialized = true;
                Ok(vec![DelegationProviderEvent::Initialized])
            }
            _ if !self.initialized => Err(CodexProtocolError::EventBeforeInitialization),
            "turn.completed" => {
                if self.terminal {
                    return Err(CodexProtocolError::DuplicateTerminal);
                }
                self.terminal = true;
                Ok(vec![
                    DelegationProviderEvent::UsageUpdated,
                    DelegationProviderEvent::FinalCandidate,
                ])
            }
            "turn.failed" | "error" => Err(CodexProtocolError::ProviderFailure),
            "item.started" => Ok(vec![DelegationProviderEvent::ActionStarted {
                id: item_id(&value),
            }]),
            "item.updated" => Ok(vec![DelegationProviderEvent::ActionUpdated {
                id: item_id(&value),
            }]),
            "item.completed" => Ok(vec![DelegationProviderEvent::ActionCompleted {
                id: item_id(&value),
            }]),
            "turn.started" => Ok(vec![DelegationProviderEvent::Progress]),
            _ => Ok(vec![DelegationProviderEvent::Progress]),
        }
    }

    pub(crate) fn finalize(
        self,
        exit_code: Option<i32>,
        final_output: &[u8],
    ) -> Result<Value, CodexProtocolError> {
        if !self.terminal {
            return Err(CodexProtocolError::MissingTerminal);
        }
        if exit_code != Some(0) {
            return Err(CodexProtocolError::ExitMismatch);
        }
        decode_final(final_output)
    }
}

fn item_id(value: &Value) -> String {
    value
        .pointer("/item/id")
        .or_else(|| value.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .chars()
        .take(256)
        .collect()
}

fn decode_final(bytes: &[u8]) -> Result<Value, CodexProtocolError> {
    if bytes.len() as u64 > MAX_FINAL_BYTES {
        return Err(CodexProtocolError::InvalidFinalOutput);
    }
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| CodexProtocolError::InvalidFinalOutput)?;
    if !value.is_object() {
        return Err(CodexProtocolError::InvalidFinalOutput);
    }
    Ok(value)
}

#[cfg(test)]
#[path = "codex_protocol_tests.rs"]
mod tests;
