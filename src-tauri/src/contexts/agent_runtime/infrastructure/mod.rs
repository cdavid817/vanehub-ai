mod anthropic_provider;
mod api_credentials;
mod api_process_adapter;
mod availability;
mod cli_profile;
mod composite_process_gateway;
mod coordination_executor;
mod coordination_repository;
mod coordination_scheduler;
mod coordination_schema;
mod core_instructions;
mod credential_aware_registry;
mod events;
mod generation_coordinator;
mod loop_execution_coordinator;
mod loop_generation_completions;
mod loop_project;
mod loop_repository;
#[cfg(test)]
mod loop_repository_control_tests;
mod loop_repository_views;
mod loop_scheduler;
mod loop_schema;
mod loop_verification_process;
mod mcp_tool_gateway;
mod memory_repository;
mod memory_schema;
mod message_terminal_completions;
mod onepiece_model_discovery;
mod openai_compatible_provider;
mod process_adapter;
mod prompt_gateway;
pub(crate) mod providers;
mod runtime_support;
mod schema;
mod sessions_gateway;
mod skill_gateway;
mod sqlite_repository;
mod terminal_observability;
mod terminal_process;
mod terminal_usage_ingestion;
mod terminal_wrapper;
mod tool_call_accumulator;
mod tools;

pub(crate) use api_credentials::OsApiCredentialAdapter;
pub(crate) use api_process_adapter::RuntimeAgentApiAdapter;
pub(crate) use availability::RuntimeAgentAvailabilityAdapter;
pub(crate) use cli_profile::RuntimeAgentCliProfileAdapter;
pub(crate) use composite_process_gateway::CompositeAgentProcessGateway;
pub(crate) use coordination_executor::NativeCoordinationNodeExecutor;
pub(crate) use coordination_repository::SqliteCoordinationRepository;
pub(crate) use coordination_scheduler::NativeCoordinationScheduler;
pub(crate) use coordination_schema::apply_coordination_schema;
pub(crate) use core_instructions::NativeAgentCoreInstructionsAdapter;
pub(crate) use credential_aware_registry::CredentialAwareAgentRegistry;
pub(crate) use events::TauriAgentRuntimeEventAdapter;
pub(crate) use generation_coordinator::InMemoryGenerationCoordinator;
pub(crate) use loop_execution_coordinator::InMemoryLoopExecutionCoordinator;
pub(crate) use loop_generation_completions::InMemoryLoopRoleGenerationCompletions;
pub(crate) use loop_project::WorkspaceLoopProjectAdapter;
pub(crate) use loop_repository::SqliteLoopRepository;
pub(crate) use loop_scheduler::NativeLoopScheduler;
pub(crate) use loop_schema::apply_loop_schema;
pub(crate) use loop_verification_process::StructuredLoopVerificationProcess;
pub(crate) use mcp_tool_gateway::RuntimeAgentMcpToolAdapter;
pub(crate) use memory_repository::SqliteAgentMemoryRepository;
pub(crate) use memory_schema::apply_memory_schema;
pub(crate) use message_terminal_completions::InMemoryAgentMessageTerminalCompletions;
pub(crate) use onepiece_model_discovery::HttpOnePieceModelDiscoveryAdapter;
pub(crate) use process_adapter::{
    ManagedMcpRelayPort, PreparedMcpRelay, RuntimeAgentProcessAdapter,
};
pub(crate) use prompt_gateway::RuntimeEffectivePromptAdapter;
pub(crate) use runtime_support::{
    AgentRuntimeLoggingAdapter, AgentRuntimeOperationAdapter, SystemAgentRuntimeClock,
    UuidCoordinationIds,
};
pub(crate) use schema::{
    apply_agent_origin_schema, apply_agent_tool_trust_schema, apply_api_agent_schema,
    apply_onepiece_provider_catalog_schema, apply_onepiece_provider_endpoint_schema,
    apply_onepiece_provider_profiles_schema, apply_openai_compatible_schema, seed_registry,
};
pub(crate) use sessions_gateway::SessionsAgentRuntimeAdapter;
pub(crate) use skill_gateway::RuntimeAgentSkillAdapter;
pub(crate) use sqlite_repository::SqliteAgentRuntimeRepository;
pub(crate) use terminal_observability::TerminalExecutionObservability;
pub(crate) use terminal_process::PortablePtyAgentTerminalRuntime;

#[cfg(test)]
mod tests;
