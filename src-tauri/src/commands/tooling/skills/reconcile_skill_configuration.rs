use super::dto::{SkillConfigurationReconciliationInput, SkillConfigurationSaveOutcome};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::skills::api::SkillApi;
use tauri::State;

/// Applies the user's decision for every property the new schema no longer accepts. Nothing is
/// converted or dropped on their behalf, so an obsolete property without a decision is rejected.
#[tauri::command]
pub(crate) fn reconcile_skill_configuration(
    api: State<'_, SkillApi>,
    skill_id: String,
    input: SkillConfigurationReconciliationInput,
) -> Result<SkillConfigurationSaveOutcome, CommandError> {
    let plan = mapper::reconciliation_plan(input.plan);
    let (key, request) =
        mapper::configuration_request(skill_id, input.request).map_err(map_command_error)?;
    api.reconcile_configuration(&key, &request, &plan)
        .map(mapper::save_outcome_to_dto)
        .map_err(map_command_error)
}
