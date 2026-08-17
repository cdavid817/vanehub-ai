use crate::commands::error::CommandError;
use crate::contexts::operations::api::{AgentRunsApi, MissionControlOverview, MissionControlQuery};
use tauri::State;

#[tauri::command]
pub(crate) fn get_mission_control_overview(
    api: State<'_, AgentRunsApi>,
    query: MissionControlQuery,
) -> Result<MissionControlOverview, CommandError> {
    api.mission_control_overview(query).map_err(Into::into)
}
