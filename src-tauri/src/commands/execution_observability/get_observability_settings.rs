use super::{dto, mapper};
use crate::contexts::execution_observability::api::ExecutionObservabilityApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_observability_settings(
    api: State<'_, ExecutionObservabilityApi>,
) -> Result<dto::ObservabilitySettingsDto, dto::ObservabilityCommandErrorDto> {
    api.settings()
        .map(mapper::settings_to_dto)
        .map_err(mapper::adapter_error)
}
