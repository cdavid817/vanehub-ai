use crate::commands::error::CommandError;
use crate::contexts::operations::api::{AgentRunsApi, MissionControlRunDetail};
use tauri::State;

#[tauri::command]
pub(crate) fn get_mission_control_run(
    api: State<'_, AgentRunsApi>,
    run_id: String,
) -> Result<MissionControlRunDetail, CommandError> {
    api.mission_control_run(&run_id).map_err(Into::into)
}
