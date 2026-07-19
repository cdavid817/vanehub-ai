use crate::contexts::agent_runtime::application::{
    AgentEvent, AgentEventPort, AgentRuntimeApplicationError, AgentTerminalEvent,
    AgentTerminalEventPort, AgentTerminalState, MessageTokenUsage, ToolUseBlock,
};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter};

#[derive(Clone)]
pub(crate) struct TauriAgentRuntimeEventAdapter {
    app: AppHandle,
}

impl TauriAgentRuntimeEventAdapter {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl AgentEventPort for TauriAgentRuntimeEventAdapter {
    fn publish(&self, event: AgentEvent) -> Result<(), AgentRuntimeApplicationError> {
        let Some(event) = chat_event(event) else {
            return Ok(());
        };
        self.app
            .emit("chat:event", event)
            .map_err(|error| AgentRuntimeApplicationError::Event(error.to_string()))
    }
}

impl AgentTerminalEventPort for TauriAgentRuntimeEventAdapter {
    fn publish_terminal(
        &self,
        event: AgentTerminalEvent,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.app
            .emit("agent-terminal:event", terminal_event(event))
            .map_err(|error| AgentRuntimeApplicationError::Event(error.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
enum ChatStreamEvent {
    #[serde(rename_all = "camelCase")]
    Started {
        session_id: String,
        message_id: String,
    },
    #[serde(rename_all = "camelCase")]
    Token {
        session_id: String,
        message_id: String,
        content_delta: String,
    },
    #[serde(rename_all = "camelCase")]
    Thinking {
        session_id: String,
        message_id: String,
        content_delta: String,
    },
    #[serde(rename_all = "camelCase")]
    ToolUse {
        session_id: String,
        message_id: String,
        tool_use: SerializableToolUse,
    },
    #[serde(rename_all = "camelCase")]
    RichBlock {
        session_id: String,
        message_id: String,
        block: Value,
    },
    #[serde(rename_all = "camelCase")]
    Completed {
        session_id: String,
        message_id: String,
        token_usage: Option<SerializableTokenUsage>,
    },
    #[serde(rename_all = "camelCase")]
    Failed {
        session_id: String,
        message_id: String,
        error: String,
    },
    #[serde(rename_all = "camelCase")]
    Cancelled {
        session_id: String,
        message_id: String,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "type")]
enum SerializableAgentTerminalEvent {
    #[serde(rename_all = "camelCase")]
    Output {
        terminal_id: String,
        session_id: String,
        content: String,
    },
    #[serde(rename_all = "camelCase")]
    State {
        terminal_id: String,
        session_id: String,
        state: &'static str,
        error: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    RuntimeSessionId {
        terminal_id: String,
        session_id: String,
        runtime_session_id: String,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct SerializableToolUse {
    id: String,
    name: String,
    input: Option<Value>,
    output: Option<Value>,
    status: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct SerializableTokenUsage {
    input: i64,
    output: i64,
}

fn chat_event(event: AgentEvent) -> Option<ChatStreamEvent> {
    match event {
        AgentEvent::WorkflowChanged(_) => None,
        AgentEvent::MessageStarted {
            session_id,
            message_id,
        } => Some(ChatStreamEvent::Started {
            session_id,
            message_id,
        }),
        AgentEvent::MessageToken {
            session_id,
            message_id,
            content_delta,
        } => Some(ChatStreamEvent::Token {
            session_id,
            message_id,
            content_delta,
        }),
        AgentEvent::MessageThinking {
            session_id,
            message_id,
            content_delta,
        } => Some(ChatStreamEvent::Thinking {
            session_id,
            message_id,
            content_delta,
        }),
        AgentEvent::MessageToolUse {
            session_id,
            message_id,
            tool_use,
        } => Some(ChatStreamEvent::ToolUse {
            session_id,
            message_id,
            tool_use: serializable_tool_use(tool_use),
        }),
        AgentEvent::MessageRichBlock {
            session_id,
            message_id,
            block,
        } => Some(ChatStreamEvent::RichBlock {
            session_id,
            message_id,
            block,
        }),
        AgentEvent::MessageCompleted {
            session_id,
            message_id,
            token_usage,
        } => Some(ChatStreamEvent::Completed {
            session_id,
            message_id,
            token_usage: token_usage.map(serializable_token_usage),
        }),
        AgentEvent::MessageFailed {
            session_id,
            message_id,
            error,
        } => Some(ChatStreamEvent::Failed {
            session_id,
            message_id,
            error,
        }),
        AgentEvent::MessageCancelled {
            session_id,
            message_id,
        } => Some(ChatStreamEvent::Cancelled {
            session_id,
            message_id,
        }),
    }
}

fn terminal_event(event: AgentTerminalEvent) -> SerializableAgentTerminalEvent {
    match event {
        AgentTerminalEvent::Output {
            terminal_id,
            session_id,
            content,
        } => SerializableAgentTerminalEvent::Output {
            terminal_id,
            session_id,
            content,
        },
        AgentTerminalEvent::State {
            terminal_id,
            session_id,
            state,
            error,
        } => SerializableAgentTerminalEvent::State {
            terminal_id,
            session_id,
            state: terminal_state_label(state),
            error,
        },
        AgentTerminalEvent::RuntimeSessionId {
            terminal_id,
            session_id,
            runtime_session_id,
        } => SerializableAgentTerminalEvent::RuntimeSessionId {
            terminal_id,
            session_id,
            runtime_session_id,
        },
    }
}

fn terminal_state_label(state: AgentTerminalState) -> &'static str {
    match state {
        AgentTerminalState::Starting => "starting",
        AgentTerminalState::Running => "running",
        AgentTerminalState::Stopped => "stopped",
        AgentTerminalState::Failed => "failed",
    }
}

fn serializable_tool_use(tool_use: ToolUseBlock) -> SerializableToolUse {
    SerializableToolUse {
        id: tool_use.id,
        name: tool_use.name,
        input: tool_use.input,
        output: tool_use.output,
        status: tool_use.status,
    }
}

fn serializable_token_usage(usage: MessageTokenUsage) -> SerializableTokenUsage {
    SerializableTokenUsage {
        input: usage.input,
        output: usage.output,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_events_keep_the_existing_tag_and_camel_case_payload_contract() {
        let value = serde_json::to_value(chat_event(AgentEvent::MessageCompleted {
            session_id: "session-1".to_string(),
            message_id: "message-1".to_string(),
            token_usage: Some(MessageTokenUsage {
                input: 10,
                output: 20,
            }),
        }))
        .expect("serialize chat event");

        assert_eq!(value["type"], "completed");
        assert_eq!(value["sessionId"], "session-1");
        assert_eq!(value["messageId"], "message-1");
        assert_eq!(value["tokenUsage"]["input"], 10);
        assert!(value.get("session_id").is_none());
    }

    #[test]
    fn terminal_events_use_dedicated_tag_and_camel_case_payload_contract() {
        let value = serde_json::to_value(terminal_event(AgentTerminalEvent::RuntimeSessionId {
            terminal_id: "terminal-1".to_string(),
            session_id: "session-1".to_string(),
            runtime_session_id: "runtime-1".to_string(),
        }))
        .expect("serialize terminal event");

        assert_eq!(value["type"], "runtime_session_id");
        assert_eq!(value["terminalId"], "terminal-1");
        assert_eq!(value["sessionId"], "session-1");
        assert_eq!(value["runtimeSessionId"], "runtime-1");
        assert!(value.get("runtime_session_id").is_none());
    }
}
