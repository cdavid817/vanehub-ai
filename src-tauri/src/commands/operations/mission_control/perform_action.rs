use super::{receipt, MissionControlActionInput, MissionControlActionReceipt};
use crate::bootstrap::AgentRunControlsApi;
use crate::commands::error::CommandError;
use tauri::State;

#[tauri::command]
pub(crate) fn perform_mission_control_action(
    api: State<'_, AgentRunControlsApi>,
    input: MissionControlActionInput,
) -> Result<MissionControlActionReceipt, CommandError> {
    let run = api.perform_action(&input.run_id, input.version, &input.action)?;
    Ok(receipt(run))
}
