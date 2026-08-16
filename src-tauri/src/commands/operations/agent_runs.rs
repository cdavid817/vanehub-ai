use crate::bootstrap::AgentRunControlsApi;
use crate::commands::error::CommandError;
use crate::contexts::operations::api::{
    AgentRun, AgentRunsApi, CreateAgentRun, RunEvent, RunListFilter, RunPage,
};
use tauri::State;

#[tauri::command]
pub(crate) fn create_agent_run(
    api: State<'_, AgentRunsApi>,
    input: CreateAgentRun,
) -> Result<AgentRun, CommandError> {
    api.create(input).map_err(Into::into)
}

#[tauri::command]
pub(crate) fn get_agent_run(
    api: State<'_, AgentRunsApi>,
    run_id: String,
) -> Result<AgentRun, CommandError> {
    api.get(&run_id).map_err(Into::into)
}

#[tauri::command]
pub(crate) fn list_agent_runs(
    api: State<'_, AgentRunsApi>,
    filter: Option<RunListFilter>,
    offset: usize,
    limit: usize,
) -> Result<RunPage, CommandError> {
    api.list(filter.unwrap_or_default(), offset, limit)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn list_agent_run_events(
    api: State<'_, AgentRunsApi>,
    run_id: String,
    offset: usize,
    limit: usize,
) -> Result<Vec<RunEvent>, CommandError> {
    api.events(&run_id, offset, limit).map_err(Into::into)
}

#[tauri::command]
pub(crate) fn cancel_agent_run(
    api: State<'_, AgentRunControlsApi>,
    run_id: String,
    version: u64,
) -> Result<AgentRun, CommandError> {
    api.cancel(&run_id, version).map_err(Into::into)
}

#[tauri::command]
pub(crate) fn resume_agent_run(
    api: State<'_, AgentRunControlsApi>,
    run_id: String,
    version: u64,
) -> Result<AgentRun, CommandError> {
    api.resume(&run_id, version).map_err(Into::into)
}
