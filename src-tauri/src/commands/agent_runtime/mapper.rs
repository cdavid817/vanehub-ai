use super::dto;
use crate::contexts::agent_runtime::api::{
    AgentAvailability, AgentChatConfiguration, AgentFileReference, AgentLifecycle, AgentMemory,
    AgentMessage, AgentSessionDetails, AgentTerminalInputRequest, AgentTerminalSession,
    AgentTerminalSize as ApiAgentTerminalSize, AgentView, ApiProviderConfig, InteractionMode,
    LaunchWorkflowResult, OnePieceProviderConfig, OpenAgentTerminalRequest, ReadinessView,
    RegisterApiAgentInput, ResizeAgentTerminalRequest, SaveOnePieceProviderConfigInput,
    SendMessageRequest, StopAgentTerminalRequest, UpdateApiAgentInput, WorkflowView,
};
use crate::contexts::agent_runtime::application::{
    AgentTerminalCapability as ApiAgentTerminalCapability,
    AgentTerminalState as ApiAgentTerminalState, MemorySource,
};

pub(super) fn agents_to_dto(agents: Vec<AgentView>) -> Vec<dto::AgentRegistryEntry> {
    agents.into_iter().map(agent_to_dto).collect()
}

pub(super) fn agent_memories_to_dto(memories: Vec<AgentMemory>) -> Vec<dto::AgentMemoryEntry> {
    memories.into_iter().map(agent_memory_to_dto).collect()
}

fn agent_memory_to_dto(memory: AgentMemory) -> dto::AgentMemoryEntry {
    dto::AgentMemoryEntry {
        id: memory.id,
        agent_id: memory.agent_id,
        folder: memory.folder,
        content: memory.content,
        source: match memory.source {
            MemorySource::Explicit => dto::AgentMemorySource::Explicit,
            MemorySource::Automatic => dto::AgentMemorySource::Automatic,
        },
        created_at: memory.created_at,
    }
}

pub(super) fn register_api_agent_request(
    input: dto::RegisterApiAgentInput,
) -> RegisterApiAgentInput {
    RegisterApiAgentInput {
        display_name: input.display_name,
        provider: input.provider,
        api_key: input.api_key,
        model_id: input.model_id,
        interface_format: input.interface_format,
        base_url: input.base_url,
    }
}

pub(super) fn api_agent_provider_config_to_dto(
    config: ApiProviderConfig,
) -> dto::ApiAgentProviderConfig {
    dto::ApiAgentProviderConfig {
        model_id: config.model_id,
        interface_format: config.interface_format,
        base_url: config.base_url,
        auto_approve_tools: config.auto_approve_tools,
    }
}

pub(super) fn update_api_agent_request(input: dto::UpdateApiAgentInput) -> UpdateApiAgentInput {
    UpdateApiAgentInput {
        display_name: input.display_name,
        model_id: input.model_id,
        base_url: input.base_url,
        new_api_key: input.new_api_key,
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
        agent_origin: match agent.origin {
            crate::contexts::agent_runtime::domain::AgentOrigin::Builtin => {
                dto::AgentOrigin::Builtin
            }
            crate::contexts::agent_runtime::domain::AgentOrigin::User => dto::AgentOrigin::User,
        },
    }
}

pub(super) fn onepiece_provider_config_to_dto(
    config: OnePieceProviderConfig,
) -> dto::OnePieceProviderConfig {
    dto::OnePieceProviderConfig {
        provider: config.provider,
        model_id: config.model_id,
        interface_format: config.interface_format,
        base_url: config.base_url,
        auto_approve_tools: config.auto_approve_tools,
        credential_present: config.credential_present,
    }
}

pub(super) fn save_onepiece_provider_config_request(
    input: dto::SaveOnePieceProviderConfigInput,
) -> SaveOnePieceProviderConfigInput {
    SaveOnePieceProviderConfigInput {
        provider: input.provider,
        model_id: input.model_id,
        interface_format: input.interface_format,
        base_url: input.base_url,
        api_key: input.api_key,
    }
}

pub(super) fn onepiece_provider_profiles_to_dto(
    overview: crate::contexts::agent_runtime::api::OnePieceProviderProfiles,
) -> dto::OnePieceProviderProfiles {
    dto::OnePieceProviderProfiles {
        profiles: overview
            .profiles
            .into_iter()
            .map(|profile| dto::OnePieceProviderProfile {
                id: profile.id,
                name: profile.name,
                source_provider_id: profile.source_provider_id,
                source_endpoint_type: profile.source_endpoint_type,
                source_preset_version: profile.source_preset_version,
                provider: profile.provider,
                model_id: profile.model_id,
                interface_format: profile.interface_format,
                base_url: profile.base_url,
                active: profile.active,
                credential_present: profile.credential_present,
            })
            .collect(),
        active_profile_id: overview.active_profile_id,
    }
}

pub(super) fn onepiece_provider_presets_to_dto(
    presets: Vec<crate::contexts::agent_runtime::api::OnePieceProviderPreset>,
) -> Vec<dto::OnePieceProviderPreset> {
    presets
        .into_iter()
        .map(|preset| dto::OnePieceProviderPreset {
            id: preset.id,
            catalog_version: preset.catalog_version,
            display_name: preset.display_name,
            category: preset.category,
            icon_key: preset.icon_key,
            provider: preset.provider,
            default_model_id: preset.default_model_id,
            fallback_models: preset.fallback_models,
            interface_format: preset.interface_format,
            base_url: preset.base_url,
            api_key_url: preset.api_key_url,
            docs_url: preset.docs_url,
            model_discovery: dto::OnePieceModelDiscoveryMetadata {
                strategy: preset.model_discovery_strategy,
            },
            default_endpoint_type: preset.default_endpoint_type,
            endpoints: preset
                .endpoints
                .into_iter()
                .map(|endpoint| dto::OnePieceProviderEndpoint {
                    endpoint_type: endpoint.endpoint_type,
                    base_url: endpoint.base_url,
                    interface_format: endpoint.interface_format,
                    auth_strategy: endpoint.auth_strategy,
                    source: endpoint.source,
                    model_discovery: dto::OnePieceEndpointDiscoveryMetadata {
                        strategy: endpoint.model_discovery_strategy,
                        url: endpoint.model_discovery_url,
                    },
                })
                .collect(),
        })
        .collect()
}

pub(super) fn save_onepiece_provider_profile_request(
    input: dto::SaveOnePieceProviderProfileInput,
) -> crate::contexts::agent_runtime::api::SaveOnePieceProviderProfileInput {
    crate::contexts::agent_runtime::api::SaveOnePieceProviderProfileInput {
        id: input.id,
        name: input.name,
        provider_id: input.provider_id,
        endpoint_type: input.endpoint_type,
        model_id: input.model_id,
        api_key: input.api_key,
    }
}

pub(super) fn discover_onepiece_provider_models_request(
    input: dto::DiscoverOnePieceProviderModelsInput,
) -> crate::contexts::agent_runtime::api::DiscoverOnePieceProviderModelsInput {
    crate::contexts::agent_runtime::api::DiscoverOnePieceProviderModelsInput {
        provider_id: input.provider_id,
        endpoint_type: input.endpoint_type,
        profile_id: input.profile_id,
        api_key: input.api_key,
    }
}

pub(super) fn validate_onepiece_provider_credential_request(
    input: dto::ValidateOnePieceProviderCredentialInput,
) -> crate::contexts::agent_runtime::api::ValidateOnePieceProviderCredentialInput {
    crate::contexts::agent_runtime::api::ValidateOnePieceProviderCredentialInput {
        provider_id: input.provider_id,
        endpoint_type: input.endpoint_type,
        model_id: input.model_id,
        profile_id: input.profile_id,
        api_key: input.api_key,
    }
}

pub(super) fn onepiece_provider_model_discovery_to_dto(
    result: crate::contexts::agent_runtime::api::OnePieceProviderModelDiscoveryResult,
) -> dto::OnePieceProviderModelDiscoveryResult {
    dto::OnePieceProviderModelDiscoveryResult {
        provider_id: result.provider_id,
        endpoint_type: result.endpoint_type,
        models: result
            .models
            .into_iter()
            .map(|model| dto::OnePieceProviderModelOption {
                id: model.id,
                display_name: model.display_name,
                source: model.source,
            })
            .collect(),
        source: result.source,
        warning: result.warning,
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
            execution_mode: configuration.execution_mode,
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
                start_line: reference.start_line,
                end_line: reference.end_line,
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
                start_line: reference.start_line,
                end_line: reference.end_line,
            })
            .collect()
    });
    dto::ChatMessage {
        id: message.id,
        session_id: message.session_id,
        seat_index: message.seat_index,
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
        session_sequence: message.session_sequence,
        execution_run_id: message.execution_run_id,
        feedback: None,
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
        dto::InteractionMode::Api => InteractionMode::Api,
    }
}

fn interaction_mode_to_dto(mode: InteractionMode) -> dto::InteractionMode {
    match mode {
        InteractionMode::Browser => dto::InteractionMode::Browser,
        InteractionMode::NativeDesktop => dto::InteractionMode::NativeDesktop,
        InteractionMode::Cli => dto::InteractionMode::Cli,
        InteractionMode::Api => dto::InteractionMode::Api,
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

pub(super) fn expert_role_to_dto(
    role: crate::contexts::agent_runtime::domain::ExpertRole,
) -> dto::ExpertRole {
    dto::ExpertRole {
        id: role.id,
        display_name: role.display_name,
        avatar: role.avatar,
        color: role.color,
        responsibility: role.responsibility,
        instruction: role.instruction,
        skill_ids: role.skill_ids,
        review_policy: dto::ExpertRoleReviewPolicy {
            peer_reviewer: role.review_policy.peer_reviewer,
            require_different_family: role.review_policy.require_different_family,
        },
        preferred_providers: role.preferred_providers,
        origin: role.origin.as_str().to_string(),
        created_at: role.created_at,
        updated_at: role.updated_at,
    }
}

pub(super) fn save_expert_role_request(
    input: dto::SaveExpertRoleInput,
) -> (
    Option<String>,
    crate::contexts::agent_runtime::domain::ExpertRoleInput,
) {
    use crate::contexts::agent_runtime::domain::{ExpertRoleInput, ExpertRoleReviewPolicy};
    (
        input.id,
        ExpertRoleInput {
            display_name: input.display_name,
            avatar: input.avatar,
            color: input.color,
            responsibility: input.responsibility,
            instruction: input.instruction,
            skill_ids: input.skill_ids,
            review_policy: ExpertRoleReviewPolicy {
                peer_reviewer: input.review_policy.peer_reviewer,
                require_different_family: input.review_policy.require_different_family,
            },
            preferred_providers: input.preferred_providers,
        },
    )
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
            origin: crate::contexts::agent_runtime::domain::AgentOrigin::Builtin,
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
    fn onepiece_configuration_mapping_preserves_non_secret_camel_case_contract() {
        let value = serde_json::to_value(onepiece_provider_config_to_dto(OnePieceProviderConfig {
            provider: "OpenAI Proxy".to_string(),
            model_id: Some("gpt-test".to_string()),
            interface_format: Some("openai-compatible".to_string()),
            base_url: Some("https://gateway.example.test/v1".to_string()),
            auto_approve_tools: false,
            credential_present: true,
        }))
        .expect("serialize OnePiece config");
        let request = save_onepiece_provider_config_request(
            serde_json::from_value(serde_json::json!({
                "provider": "Anthropic",
                "modelId": "claude-test",
                "interfaceFormat": "anthropic",
                "baseUrl": null,
                "apiKey": "sk-input-only"
            }))
            .expect("deserialize OnePiece input"),
        );

        assert_eq!(value["provider"], "OpenAI Proxy");
        assert_eq!(value["modelId"], "gpt-test");
        assert_eq!(value["interfaceFormat"], "openai-compatible");
        assert_eq!(value["credentialPresent"], true);
        assert!(value.get("apiKey").is_none());
        assert!(value.get("credential").is_none());
        assert_eq!(request.api_key.as_deref(), Some("sk-input-only"));
    }

    #[test]
    fn message_mapping_keeps_optional_collections_absent_when_empty() {
        let value = serde_json::to_value(message_to_dto(AgentMessage {
            id: "message-1".to_string(),
            session_id: "session-1".to_string(),
            speaker_seat_id: None,
            seat_index: None,
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
            session_sequence: 1,
            execution_run_id: None,
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
}
