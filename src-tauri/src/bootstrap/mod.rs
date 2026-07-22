//! Native composition root and Tauri runtime bootstrap.

mod agent_runtime;
mod cli;
mod cli_parameters;
mod communications;
mod desktop;
mod extensions;
mod mcp;
mod operations;
mod plugin_integrations;
mod prompt_hooks;
mod runtime;
mod scheduled_tasks;
mod sdk;
mod sessions;
mod skills;
mod ssh_connections;
mod workspaces;

pub(crate) use crate::contexts::sessions::infrastructure::scheduled_tasks::ScheduledTaskLogDirectory;
pub(crate) use agent_runtime::{assemble_agent_runtime_api, AgentRuntimeDependencies};
pub(crate) use cli::{assemble_cli_api, start_initial_cli_refresh};
pub(crate) use cli_parameters::assemble_cli_parameters_api;
pub(crate) use communications::{assemble_communications, CommunicationsDependencies};
pub(crate) use desktop::{
    assemble_desktop_lifecycle_api, assemble_desktop_settings_api, assemble_floating_assistant_api,
    initialize_desktop_runtime,
};
pub(crate) use extensions::assemble_extension_api;
pub(crate) use mcp::assemble_mcp_api;
pub(crate) use operations::assemble_operations_api;
pub(crate) use plugin_integrations::assemble_plugin_integration_api;
pub(crate) use prompt_hooks::assemble_prompt_hook_api;
pub(crate) use runtime::run;
pub(crate) use scheduled_tasks::start_scheduled_task_jobs;
pub(crate) use sdk::assemble_sdk_api;
pub(crate) use sessions::{assemble_sessions_api, start_session_maintenance_jobs};
pub(crate) use skills::assemble_skill_api;
pub(crate) use ssh_connections::assemble_ssh_connections_api;
pub(crate) use workspaces::assemble_workspace_api;
