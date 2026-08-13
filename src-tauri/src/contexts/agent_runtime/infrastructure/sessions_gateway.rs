use crate::contexts::agent_runtime::application::{
    AgentChatConfiguration, AgentFileReference, AgentInvocationUsage, AgentMessage,
    AgentRuntimeApplicationError, AgentSession, AgentSessionGateway, AgentUsageAccountingKind,
    AgentUsageOverlap, AgentUsageRecord, CompleteAgentMessage, ConversationHistoryPort,
    DurableAgentGenerationMessages, DurableAgentGenerationStart, LoopChildRecoveryDecision,
    LoopChildRecoveryProjection, LoopRoleSessionPort, LoopRoleSessionRequest,
    LoopSessionRecoveryPort, MessageTokenUsage, NewAgentMessage, ToolUseBlock,
};
use crate::contexts::agent_runtime::domain::{AgentLifecycle, InteractionMode};
use crate::contexts::sessions::api::{
    AccountingUnit, ChatConfigurationValues, CompleteMessageRequest, CompletedInvocationAccounting,
    CreateMessageRequest, DurableGenerationStartRequest, DurableGenerationTerminalRequest,
    FailMessageRequest, FileReferenceInput, GenerationTerminalStatus,
    LoopRoleSessionRequest as SessionLoopRoleRequest, LoopSessionRole, MeasurementKind,
    MeasurementQuality, MessageTokenUsage as SessionMessageTokenUsage, MessageUsageRecord,
    NewModelInvocation, NewUsageObservation, RuntimeMessageSnapshot, SessionChatConfiguration,
    SessionLifecycle, SessionUsageAccountingKind, SessionUsageUnit, SessionsApi, SessionsError,
    TokenDimensions, TokenOverlap, UsageInteractionKind, UsagePurpose, UsageStatus,
};
use crate::contexts::sessions::domain::recovery::RecoveryDecision;
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

impl LoopSessionRecoveryPort for SessionsAgentRuntimeAdapter {
    fn recovery_projection(
        &self,
        session_id: &str,
    ) -> Result<LoopChildRecoveryProjection, AgentRuntimeApplicationError> {
        let projection = self
            .sessions
            .recovery_projection(session_id, None)
            .map_err(session_error)?;
        let decision = match projection.decision {
            Some(RecoveryDecision::Completed) => LoopChildRecoveryDecision::Completed,
            Some(RecoveryDecision::Failed | RecoveryDecision::InterruptedWithoutToolAmbiguity) => {
                LoopChildRecoveryDecision::Failed
            }
            Some(RecoveryDecision::Cancelled) => LoopChildRecoveryDecision::Cancelled,
            Some(
                RecoveryDecision::ActionRequired
                | RecoveryDecision::Quarantined
                | RecoveryDecision::RetryLater
                | RecoveryDecision::Acknowledged,
            )
            | None => LoopChildRecoveryDecision::Ambiguous,
        };
        Ok(LoopChildRecoveryProjection {
            session_id: projection.session_id,
            execution_run_id: projection.execution_run_id,
            recovery_revision: projection.recovery_revision,
            decision,
        })
    }
}

impl LoopRoleSessionPort for SessionsAgentRuntimeAdapter {
    fn create_worker_session(
        &self,
        request: LoopRoleSessionRequest,
    ) -> Result<String, AgentRuntimeApplicationError> {
        self.sessions
            .create_loop_role_session(SessionLoopRoleRequest {
                run_id: request.run_id,
                iteration_id: request.iteration_id,
                role: LoopSessionRole::Worker,
                agent_id: request.agent_id,
                interaction_mode: request.interaction_mode.as_str().to_string(),
                project_path: request.project_path,
                worktree_path: request.worktree_path,
                worktree_name: request.worktree_name,
                worktree_branch: request.worktree_branch,
            })
            .map(|session| session.id().to_string())
            .map_err(session_error)
    }

    fn create_verifier_session(
        &self,
        request: LoopRoleSessionRequest,
    ) -> Result<String, AgentRuntimeApplicationError> {
        self.sessions
            .create_loop_role_session(SessionLoopRoleRequest {
                run_id: request.run_id,
                iteration_id: request.iteration_id,
                role: LoopSessionRole::Verifier,
                agent_id: request.agent_id,
                interaction_mode: request.interaction_mode.as_str().to_string(),
                project_path: request.project_path,
                worktree_path: request.worktree_path,
                worktree_name: request.worktree_name,
                worktree_branch: request.worktree_branch,
            })
            .map(|session| session.id().to_string())
            .map_err(session_error)
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
        let loop_ownership = session.loop_ownership.map(|ownership| {
            crate::contexts::agent_runtime::application::LoopRoleGenerationOwnership {
                run_id: ownership.run_id,
                iteration_id: ownership.iteration_id,
                role: ownership.role.as_str().to_string(),
            }
        });
        let read_only = loop_ownership
            .as_ref()
            .is_some_and(|ownership| ownership.role == "verifier");
        Ok(Some(AgentSession {
            id: session.id,
            agent_id: session.agent_id,
            seats: session
                .seats
                .into_iter()
                .map(
                    |seat| crate::contexts::agent_runtime::application::AgentSessionSeat {
                        seat_id: seat.seat_id,
                        agent_id: seat.agent_id,
                        role_id: seat.role_id,
                        left_at: seat.left_at,
                    },
                )
                .collect(),
            interaction_mode,
            lifecycle: AgentLifecycle::from_storage_lossy(&session.lifecycle),
            folder: session.folder,
            runtime_session_id: session.runtime_session_id,
            archived: session.archived,
            read_only,
            loop_ownership,
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
                    execution_mode: configuration.execution_mode,
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
            execution_mode: validated.values.execution_mode,
            provider_id: validated.values.provider_id,
            model_id: validated.values.model_id,
            reasoning_depth: validated.values.reasoning_depth,
            streaming: validated.values.streaming,
            thinking: validated.values.thinking,
            long_context: validated.values.long_context,
        })
    }

    fn validate_seat_configuration(
        &self,
        session: &AgentSession,
        configuration: AgentChatConfiguration,
    ) -> Result<AgentChatConfiguration, AgentRuntimeApplicationError> {
        let validated = self
            .sessions
            .validate_seat_chat_configuration(SessionChatConfiguration {
                session_id: session.id.clone(),
                agent_id: configuration.agent_id,
                interaction_mode: configuration.interaction_mode.as_str().to_string(),
                values: ChatConfigurationValues {
                    execution_mode: configuration.execution_mode,
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
            execution_mode: validated.values.execution_mode,
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
                speaker_seat_id: message.speaker_seat_id,
                seat_index: message.seat_index,
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

    fn start_generation(
        &self,
        request: DurableAgentGenerationStart,
    ) -> Result<DurableAgentGenerationMessages, AgentRuntimeApplicationError> {
        self.sessions
            .start_generation(DurableGenerationStartRequest {
                session_id: request.session_id,
                execution_run_id: request.execution_run_id,
                seat_round_id: request.seat_round_id,
                parent_execution_run_id: request.parent_execution_run_id,
                user_message: request.user_message.map(create_message_request),
                assistant_message: create_message_request(request.assistant_message),
            })
            .map(|started| DurableAgentGenerationMessages {
                user_message: started
                    .user_message
                    .as_ref()
                    .map(RuntimeMessageSnapshot::from_record)
                    .map(agent_message),
                assistant_message: agent_message(RuntimeMessageSnapshot::from_record(
                    &started.assistant_message,
                )),
            })
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
        let current = self
            .sessions
            .runtime_message(&message.message_id)
            .map_err(session_error)?;
        if let Some(execution_run_id) = current
            .as_ref()
            .and_then(|record| record.execution_run_id.clone())
        {
            return self
                .sessions
                .terminalize_generation(DurableGenerationTerminalRequest {
                    session_id: message.session_id,
                    message_id: message.message_id,
                    execution_run_id,
                    terminal_status: GenerationTerminalStatus::Completed,
                    content: message.content,
                    thinking_content: message.thinking_content,
                    tool_use,
                    rich_blocks,
                    token_usage: message.token_usage.map(session_token_usage),
                    usage: message.usage.map(session_usage),
                    invocation_usage: message.invocation_usage.map(session_invocation_usage),
                    error: None,
                })
                .map(|result| agent_message(RuntimeMessageSnapshot::from_record(&result.message)))
                .map_err(session_error);
        }
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
                invocation_usage: message.invocation_usage.map(session_invocation_usage),
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
        if let Some(current) = self
            .sessions
            .runtime_message(message_id)
            .map_err(session_error)?
        {
            if let Some(execution_run_id) = current.execution_run_id {
                return self
                    .sessions
                    .terminalize_generation(DurableGenerationTerminalRequest {
                        session_id: session_id.to_string(),
                        message_id: message_id.to_string(),
                        execution_run_id,
                        terminal_status: GenerationTerminalStatus::Failed,
                        content: current.content,
                        thinking_content: current.thinking_content,
                        tool_use: (!current.tool_use.is_empty()).then_some(current.tool_use),
                        rich_blocks: (!current.rich_blocks.is_empty())
                            .then_some(current.rich_blocks),
                        token_usage: current.token_usage,
                        usage: None,
                        invocation_usage: None,
                        error: Some(error.to_string()),
                    })
                    .map(|result| {
                        agent_message(RuntimeMessageSnapshot::from_record(&result.message))
                    })
                    .map_err(session_error);
            }
        }
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
        let messages = self
            .sessions
            .list_messages(session_id, Some(200), None)
            .map_err(session_error)?;
        if let Some(current) = messages.iter().rev().find(|record| {
            record.message.status().as_str() == "streaming"
                && record.message.execution_run_id().is_some()
        }) {
            let Some(execution_run_id) = current.message.execution_run_id().map(str::to_string)
            else {
                return Err(AgentRuntimeApplicationError::Generation(
                    "correlated streaming message lost its execution run".to_string(),
                ));
            };
            let message_id = current.message.id().as_str().to_string();
            self.sessions
                .terminalize_generation(DurableGenerationTerminalRequest {
                    session_id: session_id.to_string(),
                    message_id: message_id.clone(),
                    execution_run_id,
                    terminal_status: GenerationTerminalStatus::Cancelled,
                    content: current.content.clone(),
                    thinking_content: current.thinking_content.clone(),
                    tool_use: current.tool_use.clone(),
                    rich_blocks: current.rich_blocks.clone(),
                    token_usage: current.token_usage.clone(),
                    usage: None,
                    invocation_usage: None,
                    error: None,
                })
                .map_err(session_error)?;
            return Ok(vec![message_id]);
        }
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

impl ConversationHistoryPort for SessionsAgentRuntimeAdapter {
    fn recent_messages(
        &self,
        session_id: &str,
        limit: i64,
    ) -> Result<Vec<AgentMessage>, AgentRuntimeApplicationError> {
        self.sessions
            .list_messages(session_id, Some(limit), None)
            .map_err(session_error)
            .map(|records| {
                records
                    .iter()
                    .map(RuntimeMessageSnapshot::from_record)
                    .map(agent_message)
                    .collect()
            })
    }
}

fn agent_message(message: RuntimeMessageSnapshot) -> AgentMessage {
    AgentMessage {
        id: message.id,
        session_id: message.session_id,
        speaker_seat_id: message.speaker_seat_id,
        seat_index: message.seat_index,
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
        session_sequence: message.session_sequence,
        execution_run_id: message.execution_run_id,
    }
}

fn create_message_request(message: NewAgentMessage) -> CreateMessageRequest {
    CreateMessageRequest {
        session_id: message.session_id,
        speaker_seat_id: message.speaker_seat_id,
        seat_index: message.seat_index,
        role: message.role,
        status: message.status,
        content: message.content,
        file_references: message
            .file_references
            .iter()
            .map(file_reference_input)
            .collect(),
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
    let (accounting_kind, unit) = match usage.accounting_kind {
        AgentUsageAccountingKind::Reported => (
            SessionUsageAccountingKind::Reported,
            SessionUsageUnit::Tokens,
        ),
        AgentUsageAccountingKind::Estimated => (
            SessionUsageAccountingKind::Estimated,
            SessionUsageUnit::Characters,
        ),
    };
    MessageUsageRecord {
        message_id: usage.message_id,
        session_id: usage.session_id,
        agent_id: usage.agent_id,
        provider_id: usage.provider_id,
        model_id: usage.model_id,
        accounting_kind,
        unit,
        input_count: usage.input_count,
        output_count: usage.output_count,
        cache_read_count: usage.cache_read_count,
        cache_creation_count: usage.cache_creation_count,
        source: usage.source,
        occurred_at: usage.occurred_at,
    }
}

fn session_invocation_usage(usage: AgentInvocationUsage) -> CompletedInvocationAccounting {
    let quality = match usage.usage.accounting_kind {
        AgentUsageAccountingKind::Reported => MeasurementQuality::Reported,
        AgentUsageAccountingKind::Estimated => MeasurementQuality::Estimated,
    };
    let unit = if quality == MeasurementQuality::Estimated {
        AccountingUnit::Characters
    } else {
        AccountingUnit::Tokens
    };
    let overlap = |value| match value {
        AgentUsageOverlap::Subset => TokenOverlap::Subset,
        AgentUsageOverlap::Exclusive => TokenOverlap::Exclusive,
        AgentUsageOverlap::Unknown => TokenOverlap::Unknown,
    };
    let invocation_id = usage.invocation_id;
    let message_id = usage.usage.message_id;
    let observed_at = usage.usage.occurred_at;
    let source_key = usage.source_identity.as_ref().map_or_else(
        || format!("managed-cli:message:{message_id}"),
        |identity| match usage.source_revision.as_deref() {
            Some(revision) => format!("managed-cli:step:{identity}:revision:{revision}"),
            None => format!("managed-cli:step:{identity}"),
        },
    );
    CompletedInvocationAccounting {
        invocation: NewModelInvocation {
            id: invocation_id.clone(),
            generation_id: Some(usage.generation_id),
            run_id: Some(usage.run_id),
            operation_id: Some(usage.operation_id),
            session_id: usage.usage.session_id,
            message_id: Some(message_id.clone()),
            agent_id: usage.usage.agent_id,
            provider_id: usage.usage.provider_id,
            profile_id: None,
            endpoint_id: None,
            model_id: usage.usage.model_id,
            interaction_kind: UsageInteractionKind::ManagedCli,
            purpose: UsagePurpose::AssistantInitial,
            request_sequence: 0,
            attempt: 0,
            started_at: observed_at.clone(),
        },
        observation: NewUsageObservation {
            id: usage.observation_id,
            invocation_id,
            quality,
            unit,
            measurement_kind: MeasurementKind::Interval,
            dimensions: TokenDimensions {
                input: usage.usage.input_count,
                output: usage.usage.output_count,
                cached_input: usage.usage.cache_read_count,
                cache_write_input: usage.usage.cache_creation_count,
                reasoning_output: usage.usage.reasoning_output_count,
                provider_total: usage.usage.provider_total_count,
            },
            cache_overlap: overlap(usage.usage.cache_overlap),
            reasoning_overlap: overlap(usage.usage.reasoning_overlap),
            normalization_version: usage.usage.normalization_version,
            source: usage.usage.source,
            source_key,
            source_revision: usage.source_revision,
            supersedes_observation_id: None,
            event_at: Some(observed_at.clone()),
            observed_at: observed_at.clone(),
            provenance_hash: None,
        },
        status: UsageStatus::Succeeded,
        completed_at: observed_at,
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
        SessionsError::SessionRevisionConflict(session_id) => {
            AgentRuntimeApplicationError::Session(format!(
                "Session participants changed since they were loaded: {session_id}"
            ))
        }
        error @ (SessionsError::RecoveryRevisionConflict { .. }
        | SessionsError::RecoveryActionNotAllowed { .. }) => {
            AgentRuntimeApplicationError::Session(error.to_string())
        }
        SessionsError::Repository(message)
        | SessionsError::RetryableStorage(message)
        | SessionsError::StructuralRecoveryEvidence(message)
        | SessionsError::Transaction(message)
        | SessionsError::FileContent(message)
        | SessionsError::Operation(message)
        | SessionsError::Logging(message)
        | SessionsError::Serialization(message)
        | SessionsError::Workspace(message)
        | SessionsError::Runtime(message) => AgentRuntimeApplicationError::Session(message),
    }
}
