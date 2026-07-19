use crate::contexts::agent_runtime::application::{
    AgentChatConfiguration, AgentFileReference, AgentMessage, AgentRuntimeApplicationError,
    AgentSession, AgentSessionGateway, AgentUsageRecord, CompleteAgentMessage, MessageTokenUsage,
    NewAgentMessage, ToolUseBlock,
};
use crate::contexts::agent_runtime::domain::{AgentLifecycle, InteractionMode};
use crate::contexts::sessions::api::{
    ChatConfigurationValues, CompleteMessageRequest, CreateMessageRequest, FailMessageRequest,
    FileReferenceInput, MessageTokenUsage as SessionMessageTokenUsage, MessageUsageRecord,
    RuntimeMessageSnapshot, SessionChatConfiguration, SessionLifecycle, SessionUsageAccountingKind,
    SessionUsageUnit, SessionsApi, SessionsError,
};
use serde_json::{json, Value};

#[derive(Clone)]
pub(crate) struct SessionsAgentRuntimeAdapter {
    sessions: SessionsApi,
}

impl SessionsAgentRuntimeAdapter {
    pub(crate) fn new(sessions: SessionsApi) -> Self {
        Self { sessions }
    }
}

impl AgentSessionGateway for SessionsAgentRuntimeAdapter {
    fn find_session(
        &self,
        session_id: &str,
    ) -> Result<Option<AgentSession>, AgentRuntimeApplicationError> {
        let Some(session) = self
            .sessions
            .runtime_session(session_id)
            .map_err(session_error)?
        else {
            return Ok(None);
        };
        let interaction_mode = InteractionMode::parse(&session.interaction_mode)?;
        Ok(Some(AgentSession {
            id: session.id,
            agent_id: session.agent_id,
            interaction_mode,
            lifecycle: AgentLifecycle::from_storage_lossy(&session.lifecycle),
            folder: session.folder,
            runtime_session_id: session.runtime_session_id,
            archived: session.archived,
        }))
    }

    fn validate_configuration(
        &self,
        session: &AgentSession,
        configuration: AgentChatConfiguration,
    ) -> Result<AgentChatConfiguration, AgentRuntimeApplicationError> {
        let validated = self
            .sessions
            .validate_chat_configuration(SessionChatConfiguration {
                session_id: session.id.clone(),
                agent_id: configuration.agent_id,
                interaction_mode: configuration.interaction_mode.as_str().to_string(),
                values: ChatConfigurationValues {
                    permission_mode: configuration.permission_mode,
                    provider_id: configuration.provider_id,
                    model_id: configuration.model_id,
                    reasoning_depth: configuration.reasoning_depth,
                    streaming: configuration.streaming,
                    thinking: configuration.thinking,
                    long_context: configuration.long_context,
                },
            })
            .map_err(session_error)?;
        Ok(AgentChatConfiguration {
            agent_id: validated.agent_id,
            interaction_mode: InteractionMode::parse(&validated.interaction_mode)?,
            permission_mode: validated.values.permission_mode,
            provider_id: validated.values.provider_id,
            model_id: validated.values.model_id,
            reasoning_depth: validated.values.reasoning_depth,
            streaming: validated.values.streaming,
            thinking: validated.values.thinking,
            long_context: validated.values.long_context,
        })
    }

    fn compose_prompt(
        &self,
        session_id: &str,
        content: &str,
        file_references: &[AgentFileReference],
    ) -> Result<String, AgentRuntimeApplicationError> {
        self.sessions
            .compose_prompt(
                session_id,
                content,
                file_references.iter().map(file_reference_input).collect(),
            )
            .map_err(session_error)
    }

    fn create_message(
        &self,
        message: NewAgentMessage,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        self.sessions
            .create_message(CreateMessageRequest {
                session_id: message.session_id,
                role: message.role,
                status: message.status,
                content: message.content,
                file_references: message
                    .file_references
                    .iter()
                    .map(file_reference_input)
                    .collect(),
            })
            .map(|record| agent_message(RuntimeMessageSnapshot::from_record(&record)))
            .map_err(session_error)
    }

    fn find_message(
        &self,
        message_id: &str,
    ) -> Result<Option<AgentMessage>, AgentRuntimeApplicationError> {
        self.sessions
            .runtime_message(message_id)
            .map(|message| message.map(agent_message))
            .map_err(session_error)
    }

    fn append_content(
        &self,
        message_id: &str,
        content_delta: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.sessions
            .append_message_content(message_id, content_delta)
            .map_err(session_error)
    }

    fn append_thinking(
        &self,
        message_id: &str,
        content_delta: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.sessions
            .append_message_thinking(message_id, content_delta)
            .map_err(session_error)
    }

    fn append_tool_use(
        &self,
        message_id: &str,
        tool_use: ToolUseBlock,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.sessions
            .append_message_tool_use(message_id, tool_use_value(&tool_use))
            .map_err(session_error)
    }

    fn append_rich_block(
        &self,
        message_id: &str,
        block: Value,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.sessions
            .append_message_rich_block(message_id, block)
            .map_err(session_error)
    }

    fn complete_message(
        &self,
        message: CompleteAgentMessage,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        let tool_use = (!message.tool_use.is_empty())
            .then(|| message.tool_use.iter().map(tool_use_value).collect());
        let rich_blocks = (!message.rich_blocks.is_empty()).then_some(message.rich_blocks);
        self.sessions
            .complete_message(CompleteMessageRequest {
                message_id: message.message_id,
                session_id: message.session_id,
                content: message.content,
                thinking_content: message.thinking_content,
                tool_use,
                rich_blocks,
                token_usage: message.token_usage.map(session_token_usage),
                usage: message.usage.map(session_usage),
            })
            .map(|record| agent_message(RuntimeMessageSnapshot::from_record(&record)))
            .map_err(session_error)
    }

    fn fail_message(
        &self,
        message_id: &str,
        session_id: &str,
        error: &str,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        self.sessions
            .fail_message(FailMessageRequest {
                message_id: message_id.to_string(),
                session_id: session_id.to_string(),
                error: error.to_string(),
            })
            .map(|record| agent_message(RuntimeMessageSnapshot::from_record(&record)))
            .map_err(session_error)
    }

    fn cancel_streaming_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<String>, AgentRuntimeApplicationError> {
        self.sessions
            .cancel_streaming_messages(session_id)
            .map_err(session_error)
    }

    fn update_lifecycle(
        &self,
        session_id: &str,
        lifecycle: AgentLifecycle,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.sessions
            .update_runtime_lifecycle(session_id, session_lifecycle(lifecycle))
            .map_err(session_error)
    }

    fn update_runtime_session_id(
        &self,
        session_id: &str,
        runtime_session_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.sessions
            .update_runtime_session_id(session_id, runtime_session_id)
            .map_err(session_error)
    }
}

fn file_reference_input(reference: &AgentFileReference) -> FileReferenceInput {
    FileReferenceInput {
        id: reference.id.clone(),
        path: reference.path.clone(),
        name: reference.name.clone(),
        size_bytes: reference.size_bytes,
        content_hash: reference.content_hash.clone(),
    }
}

fn agent_message(message: RuntimeMessageSnapshot) -> AgentMessage {
    AgentMessage {
        id: message.id,
        session_id: message.session_id,
        role: message.role,
        content: message.content,
        status: message.status,
        tool_use: message
            .tool_use
            .into_iter()
            .filter_map(tool_use_from_value)
            .collect(),
        thinking_content: message.thinking_content,
        rich_blocks: message.rich_blocks,
        token_usage: message.token_usage.map(|usage| MessageTokenUsage {
            input: usage.input,
            output: usage.output,
        }),
        file_references: message
            .file_references
            .into_iter()
            .map(|reference| AgentFileReference {
                id: reference.id,
                path: reference.path,
                name: reference.name,
                size_bytes: reference.size_bytes,
                content_hash: reference.content_hash,
            })
            .collect(),
        error: message.error,
        created_at: message.created_at,
        updated_at: message.updated_at,
    }
}

fn tool_use_value(tool_use: &ToolUseBlock) -> Value {
    json!({
        "id": tool_use.id,
        "name": tool_use.name,
        "input": tool_use.input,
        "output": tool_use.output,
        "status": tool_use.status,
    })
}

fn tool_use_from_value(value: Value) -> Option<ToolUseBlock> {
    Some(ToolUseBlock {
        id: value.get("id")?.as_str()?.to_string(),
        name: value.get("name")?.as_str()?.to_string(),
        input: value.get("input").filter(|value| !value.is_null()).cloned(),
        output: value
            .get("output")
            .filter(|value| !value.is_null())
            .cloned(),
        status: value.get("status")?.as_str()?.to_string(),
    })
}

fn session_token_usage(usage: MessageTokenUsage) -> SessionMessageTokenUsage {
    SessionMessageTokenUsage {
        input: usage.input,
        output: usage.output,
    }
}

fn session_usage(usage: AgentUsageRecord) -> MessageUsageRecord {
    MessageUsageRecord {
        message_id: usage.message_id,
        session_id: usage.session_id,
        agent_id: usage.agent_id,
        provider_id: usage.provider_id,
        model_id: usage.model_id,
        accounting_kind: SessionUsageAccountingKind::Estimated,
        unit: SessionUsageUnit::Characters,
        input_count: usage.input_count,
        output_count: usage.output_count,
        cache_read_count: 0,
        cache_creation_count: 0,
        source: usage.source,
        occurred_at: usage.occurred_at,
    }
}

fn session_lifecycle(lifecycle: AgentLifecycle) -> SessionLifecycle {
    match lifecycle {
        AgentLifecycle::Idle => SessionLifecycle::Idle,
        AgentLifecycle::Starting => SessionLifecycle::Starting,
        AgentLifecycle::Running => SessionLifecycle::Running,
        AgentLifecycle::Failed => SessionLifecycle::Failed,
        AgentLifecycle::Stopped => SessionLifecycle::Stopped,
    }
}

fn session_error(error: SessionsError) -> AgentRuntimeApplicationError {
    match error {
        SessionsError::Domain(error) => AgentRuntimeApplicationError::Validation(error.to_string()),
        SessionsError::Validation(message) => AgentRuntimeApplicationError::Validation(message),
        SessionsError::AgentNotFound(agent_id) => {
            AgentRuntimeApplicationError::AgentNotFound(agent_id)
        }
        SessionsError::UnsupportedInteractionMode(mode) => {
            AgentRuntimeApplicationError::UnsupportedInteractionMode(mode)
        }
        SessionsError::SessionNotFound(session_id) => {
            AgentRuntimeApplicationError::SessionNotFound(session_id)
        }
        SessionsError::MessageNotFound(message_id) => {
            AgentRuntimeApplicationError::MessageNotFound(message_id)
        }
        SessionsError::WorkspaceLaunch(message) | SessionsError::RuntimeLaunch(message) => {
            AgentRuntimeApplicationError::Process(message)
        }
        SessionsError::CategoryNotFound(category_id) => {
            AgentRuntimeApplicationError::Session(format!("Category not found: {category_id}"))
        }
        SessionsError::CategoryNameConflict(_) => {
            AgentRuntimeApplicationError::Session("Category name already exists.".to_string())
        }
        SessionsError::Repository(message)
        | SessionsError::Transaction(message)
        | SessionsError::FileContent(message)
        | SessionsError::Operation(message)
        | SessionsError::Logging(message)
        | SessionsError::Serialization(message)
        | SessionsError::Workspace(message)
        | SessionsError::Runtime(message) => AgentRuntimeApplicationError::Session(message),
    }
}
