use super::dto::{SkillResourceReadInput, SkillResourceReadOutcome};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

#[tauri::command]
pub(crate) fn read_skill_resource(
    api: State<'_, SkillApi>,
    input: SkillResourceReadInput,
) -> Result<SkillResourceReadOutcome, CommandError> {
    api.read_resource_for_agent(mapper::resource_read_request(input))
        .map(mapper::resource_read_outcome_to_dto)
        .map_err(map_command_error)
}
