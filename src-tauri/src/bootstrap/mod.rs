//! Native composition root and Tauri runtime bootstrap.

mod agent_run_controls;
mod agent_runtime;
mod cli;
mod cli_config;
mod cli_parameters;
mod code_intelligence;
mod communications;
mod desktop;
mod execution_observability;
mod extensions;
mod local_media;
mod managed_mcp_relay;
mod mcp;
mod operations;
mod permissions;
mod plugin_integrations;
mod prompt_hooks;
mod retrieval;
mod runtime;
mod scheduled_tasks;
mod sdk;
mod sessions;
mod skills;
mod ssh_connections;
mod workspaces;

pub(crate) use crate::contexts::sessions::infrastructure::scheduled_tasks::ScheduledTaskLogDirectory;
pub(crate) use agent_run_controls::AgentRunControlsApi;
pub(crate) use agent_runtime::{
    assemble_agent_runners, assemble_agent_runtime_api, assemble_shared_agent_registry,
    AgentRuntimeAssembly, AgentRuntimeDependencies,
};
pub(crate) use cli::{assemble_cli_api, start_initial_cli_refresh};
pub(crate) use cli_config::assemble_cli_config_api;
pub(crate) use cli_parameters::assemble_cli_parameter_apis;
pub(crate) use code_intelligence::{
    assemble_code_intelligence_api, NativeCodeIntelligenceResponder, WorkspaceMutationFanout,
};
pub(crate) use communications::{assemble_communications, CommunicationsDependencies};
pub(crate) use desktop::{
    assemble_desktop_lifecycle_api, assemble_desktop_settings_api, assemble_floating_assistant_api,
    initialize_desktop_runtime,
};
pub(crate) use execution_observability::{
    assemble_evaluation_api, assemble_execution_observability_api, relay_telemetry,
    start_execution_retention_job,
};
pub(crate) use extensions::assemble_extension_api;
pub(crate) use local_media::{assemble_local_media_api, worker_bridge_candidates};
pub(crate) use mcp::assemble_mcp_api;
#[cfg(test)]
pub(crate) use operations::assemble_agent_runs_api;
pub(crate) use operations::{assemble_agent_runs_api_with_recovery, assemble_operations_api};
pub(crate) use permissions::{assemble_permissions_api, start_permission_timeout_sweep_job};
pub(crate) use plugin_integrations::assemble_plugin_integration_api;
pub(crate) use prompt_hooks::assemble_prompt_hook_api;
pub(crate) use retrieval::{
    assemble_retrieval, start_retrieval_indexing_worker, DeferredAgentRetrieval, RetrievalAssembly,
};
pub(crate) use runtime::run;
pub(crate) use scheduled_tasks::start_scheduled_task_jobs;
pub(crate) use sdk::assemble_sdk_api;
pub(crate) use sessions::{
    assemble_sessions_api, start_session_maintenance_jobs, SessionRuntimeDependencies,
};
pub(crate) use skills::{assemble_skill_api, assemble_skill_tool_api};
pub(crate) use ssh_connections::assemble_ssh_connections_api;
pub(crate) use workspaces::assemble_workspace_api;
