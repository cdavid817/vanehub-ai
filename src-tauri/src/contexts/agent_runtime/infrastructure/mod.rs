mod availability;
mod cli_profile;
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
mod process_adapter;
mod prompt_gateway;
pub(crate) mod providers;
mod runtime_support;
mod schema;
mod sessions_gateway;
mod sqlite_repository;
mod terminal_process;
mod terminal_wrapper;

pub(crate) use availability::RuntimeAgentAvailabilityAdapter;
pub(crate) use cli_profile::RuntimeAgentCliProfileAdapter;
pub(crate) use events::TauriAgentRuntimeEventAdapter;
pub(crate) use generation_coordinator::InMemoryGenerationCoordinator;
pub(crate) use loop_execution_coordinator::InMemoryLoopExecutionCoordinator;
pub(crate) use loop_generation_completions::InMemoryLoopRoleGenerationCompletions;
pub(crate) use loop_project::WorkspaceLoopProjectAdapter;
pub(crate) use loop_repository::SqliteLoopRepository;
pub(crate) use loop_scheduler::NativeLoopScheduler;
pub(crate) use loop_schema::apply_loop_schema;
pub(crate) use loop_verification_process::StructuredLoopVerificationProcess;
pub(crate) use process_adapter::{
    ManagedMcpRelayPort, PreparedMcpRelay, RuntimeAgentProcessAdapter,
};
pub(crate) use prompt_gateway::RuntimeEffectivePromptAdapter;
pub(crate) use runtime_support::{
    AgentRuntimeLoggingAdapter, AgentRuntimeOperationAdapter, SystemAgentRuntimeClock,
};
pub(crate) use schema::seed_registry;
pub(crate) use sessions_gateway::SessionsAgentRuntimeAdapter;
pub(crate) use sqlite_repository::SqliteAgentRuntimeRepository;
pub(crate) use terminal_process::PortablePtyAgentTerminalRuntime;

#[cfg(test)]
mod tests;
