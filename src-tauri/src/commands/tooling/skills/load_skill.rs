use super::dto::{SkillLoadInput, SkillLoadOutcome};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn load_skill(
    api: State<'_, SkillApi>,
    input: SkillLoadInput,
) -> Result<SkillLoadOutcome, CommandError> {
    api.load_for_agent(mapper::load_request(input))
        .map(mapper::load_outcome_to_dto)
        .map_err(map_command_error)
}
