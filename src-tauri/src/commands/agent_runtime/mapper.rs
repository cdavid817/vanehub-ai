use super::dto;
use crate::contexts::agent_runtime::api::{
    AgentAvailability, AgentChatConfiguration, AgentFileReference, AgentLifecycle, AgentMessage,
    AgentSessionDetails, AgentTerminalInputRequest, AgentTerminalSession,
    AgentTerminalSize as ApiAgentTerminalSize, AgentView,
    CoordinationAttempt as ApiCoordinationAttempt,
    CoordinationAttemptStatus as ApiCoordinationAttemptStatus,
    CoordinationCandidateRole as ApiCoordinationCandidateRole,
    CoordinationFailureKind as ApiCoordinationFailureKind,
    CoordinationNodeInput as ApiCoordinationNodeInput,
    CoordinationNodeStatus as ApiCoordinationNodeStatus,
    CoordinationOutput as ApiCoordinationOutput, CoordinationRun as ApiCoordinationRun,
    CoordinationRunStatus as ApiCoordinationRunStatus, InteractionMode, LaunchWorkflowResult,
    OpenAgentTerminalRequest, ReadinessView, ResizeAgentTerminalRequest, SendMessageRequest,
    StopAgentTerminalRequest, WorkflowView,
};
use crate::contexts::agent_runtime::application::{
    AgentTerminalCapability as ApiAgentTerminalCapability,
    AgentTerminalState as ApiAgentTerminalState,
};

pub(super) fn agents_to_dto(agents: Vec<AgentView>) -> Vec<dto::AgentRegistryEntry> {
    agents.into_iter().map(agent_to_dto).collect()
}

pub(super) fn start_coordination_request(
    input: dto::StartCoordinationInput,
) -> crate::contexts::agent_runtime::application::StartCoordinationRequest {
    crate::contexts::agent_runtime::application::StartCoordinationRequest {
        name: input.name,
        project_path: input.project_path,
        nodes: input
            .nodes
            .into_iter()
            .map(|node| ApiCoordinationNodeInput {
                id: node.id,
                primary_agent_id: node.primary_agent_id,
                fallback_agent_ids: node.fallback_agent_ids,
                instruction: node.instruction,
                depends_on: node.depends_on,
            })
            .collect(),
    }
}

pub(super) fn coordination_run_to_dto(run: ApiCoordinationRun) -> dto::CoordinationRun {
    dto::CoordinationRun {
        id: run.id,
        operation_id: run.operation_id,
        name: run.plan.name,
        project_path: run.plan.project_path,
        status: coordination_run_status_to_dto(run.status),
        nodes: run
            .nodes
            .into_iter()
            .map(coordination_node_to_dto)
            .collect(),
        simulated: false,
        cancel_requested: run.cancel_requested,
        created_at: run.created_at,
        started_at: run.started_at,
        updated_at: run.updated_at,
        completed_at: run.completed_at,
    }
}

fn coordination_node_to_dto(
    node: crate::contexts::agent_runtime::api::CoordinationNodeRun,
) -> dto::CoordinationNodeRun {
    dto::CoordinationNodeRun {
        id: node.definition.id,
        primary_agent_id: node.definition.primary_agent_id,
        fallback_agent_ids: node.definition.fallback_agent_ids,
        instruction: node.definition.instruction,
        depends_on: node.definition.depends_on,
        status: coordination_node_status_to_dto(node.status),
        actual_agent_id: node.actual_agent_id,
        output: node.output.map(coordination_output_to_dto),
        attempts: node
            .attempts
            .into_iter()
            .map(coordination_attempt_to_dto)
            .collect(),
        error: node.error,
        started_at: node.started_at,
        completed_at: node.completed_at,
    }
}

fn coordination_output_to_dto(output: ApiCoordinationOutput) -> dto::CoordinationOutput {
    dto::CoordinationOutput {
        source_node_id: output.source_node_id,
        agent_id: output.agent_id,
        attempt: output.attempt,
        content: output.content,
        byte_count: output.byte_count,
        truncated: output.truncated,
    }
}

fn coordination_attempt_to_dto(attempt: ApiCoordinationAttempt) -> dto::CoordinationAttempt {
    dto::CoordinationAttempt {
        attempt: attempt.attempt,
        agent_id: attempt.agent_id,
        candidate_role: match attempt.candidate_role {
            ApiCoordinationCandidateRole::Primary => dto::CoordinationCandidateRole::Primary,
            ApiCoordinationCandidateRole::Fallback => dto::CoordinationCandidateRole::Fallback,
        },
        status: match attempt.status {
            ApiCoordinationAttemptStatus::Running => dto::CoordinationAttemptStatus::Running,
            ApiCoordinationAttemptStatus::Succeeded => dto::CoordinationAttemptStatus::Succeeded,
            ApiCoordinationAttemptStatus::Failed => dto::CoordinationAttemptStatus::Failed,
            ApiCoordinationAttemptStatus::Cancelled => dto::CoordinationAttemptStatus::Cancelled,
        },
        failure_kind: attempt.failure_kind.map(|kind| match kind {
            ApiCoordinationFailureKind::Retryable => dto::CoordinationFailureKind::Retryable,
            ApiCoordinationFailureKind::NonRetryable => dto::CoordinationFailureKind::NonRetryable,
            ApiCoordinationFailureKind::Cancelled => dto::CoordinationFailureKind::Cancelled,
        }),
        error: attempt.error,
        started_at: attempt.started_at,
        completed_at: attempt.completed_at,
    }
}

fn coordination_run_status_to_dto(status: ApiCoordinationRunStatus) -> dto::CoordinationRunStatus {
    match status {
        ApiCoordinationRunStatus::Queued => dto::CoordinationRunStatus::Queued,
        ApiCoordinationRunStatus::Running => dto::CoordinationRunStatus::Running,
        ApiCoordinationRunStatus::Succeeded => dto::CoordinationRunStatus::Succeeded,
        ApiCoordinationRunStatus::Failed => dto::CoordinationRunStatus::Failed,
        ApiCoordinationRunStatus::Cancelled => dto::CoordinationRunStatus::Cancelled,
    }
}

fn coordination_node_status_to_dto(
    status: ApiCoordinationNodeStatus,
) -> dto::CoordinationNodeStatus {
    match status {
        ApiCoordinationNodeStatus::Blocked => dto::CoordinationNodeStatus::Blocked,
        ApiCoordinationNodeStatus::Queued => dto::CoordinationNodeStatus::Queued,
        ApiCoordinationNodeStatus::Running => dto::CoordinationNodeStatus::Running,
        ApiCoordinationNodeStatus::Succeeded => dto::CoordinationNodeStatus::Succeeded,
        ApiCoordinationNodeStatus::Failed => dto::CoordinationNodeStatus::Failed,
        ApiCoordinationNodeStatus::Skipped => dto::CoordinationNodeStatus::Skipped,
        ApiCoordinationNodeStatus::Cancelled => dto::CoordinationNodeStatus::Cancelled,
    }
}

pub(super) fn agent_to_dto(agent: AgentView) -> dto::AgentRegistryEntry {
    dto::AgentRegistryEntry {
        id: agent.id,
        display_name: agent.display_name,
        provider: agent.provider,
        managed_sdk_dependency_id: agent.managed_sdk_dependency_id,
        launch: dto::LaunchMetadata {
            kind: agent.launch.kind,
            command: agent.launch.command,
            url: agent.launch.url,
            executable_name: agent.launch.executable_name,
        },
        supported_interaction_modes: agent
            .supported_interaction_modes
            .into_iter()
            .map(interaction_mode_to_dto)
            .collect(),
        availability_state: availability_to_dto(agent.availability),
        unavailable_reason: agent.unavailable_reason,
        capability_tags: agent.capability_tags,
    }
}

pub(super) fn workflow_to_dto(workflow: WorkflowView) -> dto::WorkflowState {
    dto::WorkflowState {
        active_agent_id: workflow.active_agent_id,
        active_interaction_mode: workflow
            .active_interaction_mode
            .map(interaction_mode_to_dto),
        lifecycle_state: lifecycle_to_dto(workflow.lifecycle),
        intent: workflow.intent,
    }
}

pub(super) fn readiness_to_dto(readiness: ReadinessView) -> dto::ReadinessStatus {
    dto::ReadinessStatus {
        ready: readiness.ready,
        reason: readiness.reason,
        requires_authentication: readiness.requires_authentication,
    }
}

pub(super) fn launch_to_dto(launch: LaunchWorkflowResult) -> dto::LaunchResult {
    dto::LaunchResult {
        operation_id: Some(launch.operation_id),
        workflow: workflow_to_dto(launch.workflow),
        message: launch.message,
    }
}

pub(super) fn session_details_to_dto(details: AgentSessionDetails) -> dto::SessionDetails {
    dto::SessionDetails {
        agent_id: details.workflow.active_agent_id.clone(),
        interaction_mode: details
            .workflow
            .active_interaction_mode
            .map(interaction_mode_to_dto),
        lifecycle_state: lifecycle_to_dto(details.workflow.lifecycle),
        adapter: details.adapter,
        details: details.details,
    }
}

pub(super) fn send_message_request(
    session_id: String,
    content: String,
    configuration: dto::ChatConfig,
    file_references: Option<Vec<dto::ChatFileReference>>,
) -> SendMessageRequest {
    SendMessageRequest {
        source: crate::contexts::agent_runtime::application::AgentMessageSource::Desktop,
        session_id,
        content,
        configuration: AgentChatConfiguration {
            agent_id: configuration.agent_id,
            interaction_mode: interaction_mode_from_dto(configuration.interaction_mode),
            permission_mode: configuration.permission_mode,
            provider_id: configuration.provider_id,
            model_id: configuration.model_id,
            reasoning_depth: configuration.reasoning_depth,
            streaming: configuration.streaming,
            thinking: configuration.thinking,
            long_context: configuration.long_context,
        },
        file_references: file_references
            .unwrap_or_default()
            .into_iter()
            .map(|reference| AgentFileReference {
                id: reference.id,
                path: reference.path,
                name: reference.name,
                size_bytes: reference.size_bytes,
                content_hash: reference.content_hash,
            })
            .collect(),
    }
}

pub(super) fn message_to_dto(message: AgentMessage) -> dto::ChatMessage {
    let tool_use = (!message.tool_use.is_empty()).then(|| {
        message
            .tool_use
            .into_iter()
            .map(|tool_use| dto::ToolUseBlock {
                id: tool_use.id,
                name: tool_use.name,
                input: tool_use.input,
                output: tool_use.output,
                status: tool_use.status,
            })
            .collect()
    });
    let rich_blocks = (!message.rich_blocks.is_empty()).then_some(message.rich_blocks);
    let file_references = (!message.file_references.is_empty()).then(|| {
        message
            .file_references
            .into_iter()
            .map(|reference| dto::ChatFileReference {
                id: reference.id,
                path: reference.path,
                name: reference.name,
                size_bytes: reference.size_bytes,
                content_hash: reference.content_hash,
            })
            .collect()
    });
    dto::ChatMessage {
        id: message.id,
        session_id: message.session_id,
        role: message.role,
        content: message.content,
        status: message.status,
        tool_use,
        thinking_content: message.thinking_content,
        rich_blocks,
        token_usage: message.token_usage.map(|usage| dto::TokenUsage {
            input: usage.input,
            output: usage.output,
        }),
        file_references,
        error: message.error,
        created_at: message.created_at,
        updated_at: message.updated_at,
    }
}

pub(super) fn open_agent_terminal_request(
    session_id: String,
    size: dto::AgentTerminalSize,
) -> OpenAgentTerminalRequest {
    OpenAgentTerminalRequest {
        session_id,
        size: terminal_size_from_dto(size),
    }
}

pub(super) fn terminal_input_request(
    terminal_id: String,
    content: String,
) -> AgentTerminalInputRequest {
    AgentTerminalInputRequest {
        terminal_id,
        content,
    }
}

pub(super) fn resize_terminal_request(
    terminal_id: String,
    size: dto::AgentTerminalSize,
) -> ResizeAgentTerminalRequest {
    ResizeAgentTerminalRequest {
        terminal_id,
        size: terminal_size_from_dto(size),
    }
}

pub(super) fn stop_terminal_request(terminal_id: String) -> StopAgentTerminalRequest {
    StopAgentTerminalRequest { terminal_id }
}

pub(super) fn terminal_session_to_dto(session: AgentTerminalSession) -> dto::AgentTerminalSession {
    dto::AgentTerminalSession {
        terminal_id: session.terminal_id,
        session_id: session.session_id,
        agent_id: session.agent_id,
        state: terminal_state_to_dto(session.state),
        capability: terminal_capability_to_dto(session.capability),
        size: terminal_size_to_dto(session.size),
        runtime_session_id: session.runtime_session_id,
        retained: session.retained,
    }
}

pub(super) fn interaction_mode_from_dto(mode: dto::InteractionMode) -> InteractionMode {
    match mode {
        dto::InteractionMode::Browser => InteractionMode::Browser,
        dto::InteractionMode::NativeDesktop => InteractionMode::NativeDesktop,
        dto::InteractionMode::Cli => InteractionMode::Cli,
    }
}

fn interaction_mode_to_dto(mode: InteractionMode) -> dto::InteractionMode {
    match mode {
        InteractionMode::Browser => dto::InteractionMode::Browser,
        InteractionMode::NativeDesktop => dto::InteractionMode::NativeDesktop,
        InteractionMode::Cli => dto::InteractionMode::Cli,
    }
}

fn availability_to_dto(availability: AgentAvailability) -> dto::AvailabilityState {
    match availability {
        AgentAvailability::Available => dto::AvailabilityState::Available,
        AgentAvailability::Unavailable => dto::AvailabilityState::Unavailable,
        AgentAvailability::NeedsAuthentication => dto::AvailabilityState::NeedsAuth,
        AgentAvailability::Unknown => dto::AvailabilityState::Unknown,
    }
}

fn lifecycle_to_dto(lifecycle: AgentLifecycle) -> dto::SessionLifecycleState {
    match lifecycle {
        AgentLifecycle::Idle => dto::SessionLifecycleState::Idle,
        AgentLifecycle::Starting => dto::SessionLifecycleState::Starting,
        AgentLifecycle::Running => dto::SessionLifecycleState::Running,
        AgentLifecycle::Failed => dto::SessionLifecycleState::Failed,
        AgentLifecycle::Stopped => dto::SessionLifecycleState::Stopped,
    }
}

fn terminal_size_from_dto(size: dto::AgentTerminalSize) -> ApiAgentTerminalSize {
    ApiAgentTerminalSize {
        rows: size.rows,
        cols: size.cols,
    }
}

fn terminal_size_to_dto(size: ApiAgentTerminalSize) -> dto::AgentTerminalSize {
    dto::AgentTerminalSize {
        rows: size.rows,
        cols: size.cols,
    }
}

fn terminal_state_to_dto(state: ApiAgentTerminalState) -> dto::AgentTerminalState {
    match state {
        ApiAgentTerminalState::Starting => dto::AgentTerminalState::Starting,
        ApiAgentTerminalState::Running => dto::AgentTerminalState::Running,
        ApiAgentTerminalState::Stopped => dto::AgentTerminalState::Stopped,
        ApiAgentTerminalState::Failed => dto::AgentTerminalState::Failed,
    }
}

fn terminal_capability_to_dto(
    capability: ApiAgentTerminalCapability,
) -> dto::AgentTerminalCapability {
    match capability {
        ApiAgentTerminalCapability::Native => dto::AgentTerminalCapability::Native,
        ApiAgentTerminalCapability::Simulated => dto::AgentTerminalCapability::Simulated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::api::{
        AgentLaunchView, LaunchWorkflowResult, MessageTokenUsage, ReadinessView, WorkflowView,
    };
    use std::collections::BTreeMap;

    #[test]
    fn agent_mapping_preserves_the_existing_camel_case_and_enum_contract() {
        let value = serde_json::to_value(agent_to_dto(AgentView {
            id: "codex-cli".to_string(),
            display_name: "Codex CLI".to_string(),
            provider: "OpenAI".to_string(),
            managed_sdk_dependency_id: Some("codex-sdk".to_string()),
            launch: AgentLaunchView {
                kind: "cli".to_string(),
                command: Some("codex".to_string()),
                url: None,
                executable_name: Some("codex".to_string()),
            },
            supported_interaction_modes: vec![InteractionMode::Cli],
            availability: AgentAvailability::NeedsAuthentication,
            unavailable_reason: Some("authentication required".to_string()),
            capability_tags: vec!["coding".to_string()],
        }))
        .expect("serialize agent");

        assert_eq!(value["id"], "codex-cli");
        assert_eq!(value["managedSdkDependencyId"], "codex-sdk");
        assert_eq!(value["supportedInteractionModes"][0], "cli");
        assert_eq!(value["availabilityState"], "needs-auth");
        assert_eq!(value["launch"]["executableName"], "codex");
        assert!(value.get("availability_state").is_none());
    }

    #[test]
    fn message_mapping_keeps_optional_collections_absent_when_empty() {
        let value = serde_json::to_value(message_to_dto(AgentMessage {
            id: "message-1".to_string(),
            session_id: "session-1".to_string(),
            role: "assistant".to_string(),
            content: "done".to_string(),
            status: "completed".to_string(),
            tool_use: Vec::new(),
            thinking_content: None,
            rich_blocks: Vec::new(),
            token_usage: Some(MessageTokenUsage {
                input: 3,
                output: 5,
            }),
            file_references: Vec::new(),
            error: None,
            created_at: "100".to_string(),
            updated_at: "101".to_string(),
        }))
        .expect("serialize message");

        assert!(value["toolUse"].is_null());
        assert!(value["richBlocks"].is_null());
        assert!(value["fileReferences"].is_null());
        assert_eq!(value["tokenUsage"]["output"], 5);
        assert!(value.get("session_id").is_none());
    }

    #[test]
    fn workflow_readiness_launch_and_details_keep_legacy_transport_shapes() {
        let workflow = WorkflowView {
            active_agent_id: Some("codex-cli".to_string()),
            active_interaction_mode: Some(InteractionMode::Cli),
            lifecycle: AgentLifecycle::Running,
            intent: "coding".to_string(),
        };
        let launch = serde_json::to_value(launch_to_dto(LaunchWorkflowResult {
            operation_id: "operation-1".to_string(),
            workflow: workflow.clone(),
            message: "launched".to_string(),
        }))
        .expect("serialize launch");
        let readiness = serde_json::to_value(readiness_to_dto(ReadinessView {
            ready: true,
            reason: None,
            requires_authentication: true,
        }))
        .expect("serialize readiness");
        let details = serde_json::to_value(session_details_to_dto(AgentSessionDetails {
            workflow,
            adapter: "cli".to_string(),
            details: BTreeMap::from([("runtime".to_string(), "tauri".to_string())]),
        }))
        .expect("serialize details");

        assert_eq!(launch["operationId"], "operation-1");
        assert_eq!(launch["workflow"]["activeInteractionMode"], "cli");
        assert_eq!(launch["workflow"]["lifecycleState"], "running");
        assert_eq!(readiness["requiresAuthentication"], true);
        assert_eq!(details["agentId"], "codex-cli");
        assert_eq!(details["interactionMode"], "cli");
        assert_eq!(details["lifecycleState"], "running");
        assert_eq!(details["details"]["runtime"], "tauri");
        assert!(details.get("lifecycle_state").is_none());
    }

    #[test]
    fn terminal_session_mapping_keeps_camel_case_contract() {
        let value = serde_json::to_value(terminal_session_to_dto(AgentTerminalSession {
            terminal_id: "terminal-1".to_string(),
            session_id: "session-1".to_string(),
            agent_id: "codex-cli".to_string(),
            state: ApiAgentTerminalState::Running,
            capability: ApiAgentTerminalCapability::Native,
            size: ApiAgentTerminalSize { rows: 24, cols: 80 },
            runtime_session_id: Some("runtime-1".to_string()),
            retained: true,
        }))
        .expect("serialize terminal session");

        assert_eq!(value["terminalId"], "terminal-1");
        assert_eq!(value["sessionId"], "session-1");
        assert_eq!(value["agentId"], "codex-cli");
        assert_eq!(value["runtimeSessionId"], "runtime-1");
        assert_eq!(value["state"], "running");
        assert_eq!(value["capability"], "native");
        assert!(value.get("terminal_id").is_none());
    }

    #[test]
    fn coordination_contract_preserves_graph_attempt_and_output_shapes() {
        use crate::contexts::agent_runtime::domain::{
            CoordinationAttempt, CoordinationAttemptStatus, CoordinationCandidateRole,
            CoordinationNodeStatus, CoordinationOutput, CoordinationPlan, CoordinationPlanInput,
            CoordinationRun,
        };
        use std::collections::BTreeSet;

        let input = serde_json::from_value::<dto::StartCoordinationInput>(serde_json::json!({
            "name": "pipeline",
            "projectPath": "D:\\project",
            "nodes": [{
                "id": "implement",
                "primaryAgentId": "codex-cli",
                "fallbackAgentIds": ["claude-code"],
                "instruction": "implement",
                "dependsOn": []
            }]
        }))
        .expect("coordination input");
        let request = start_coordination_request(input);
        assert_eq!(request.nodes[0].fallback_agent_ids, vec!["claude-code"]);

        let plan = CoordinationPlan::new(
            CoordinationPlanInput {
                name: request.name,
                project_path: request.project_path,
                nodes: request.nodes,
            },
            &BTreeSet::from(["claude-code".to_string(), "codex-cli".to_string()]),
        )
        .expect("plan");
        let mut run = CoordinationRun::new(
            "coordination-run-1".to_string(),
            "operation-1".to_string(),
            plan,
            "2026-07-23T00:00:00Z".to_string(),
        )
        .expect("run");
        let node = run.node_mut("implement").expect("node");
        node.status = CoordinationNodeStatus::Succeeded;
        node.actual_agent_id = Some("claude-code".to_string());
        node.output = Some(CoordinationOutput::bounded(
            "implement".to_string(),
            "claude-code".to_string(),
            2,
            "done".to_string(),
        ));
        node.attempts.push(CoordinationAttempt {
            attempt: 2,
            agent_id: "claude-code".to_string(),
            candidate_role: CoordinationCandidateRole::Fallback,
            status: CoordinationAttemptStatus::Succeeded,
            failure_kind: None,
            error: None,
            started_at: "2026-07-23T00:00:01Z".to_string(),
            completed_at: Some("2026-07-23T00:00:02Z".to_string()),
        });

        let value = serde_json::to_value(coordination_run_to_dto(run)).expect("serialize run");
        assert_eq!(value["operationId"], "operation-1");
        assert_eq!(value["nodes"][0]["fallbackAgentIds"][0], "claude-code");
        assert_eq!(
            value["nodes"][0]["attempts"][0]["candidateRole"],
            "fallback"
        );
        assert_eq!(value["nodes"][0]["output"]["sourceNodeId"], "implement");
        assert_eq!(value["simulated"], false);
        assert!(value.get("operation_id").is_none());
    }
}
