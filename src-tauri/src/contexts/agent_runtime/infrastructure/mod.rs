mod availability;
mod cli_profile;
mod events;
mod generation_coordinator;
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
pub(crate) use process_adapter::RuntimeAgentProcessAdapter;
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
