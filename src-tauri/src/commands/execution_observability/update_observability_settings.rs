use super::{dto, mapper};
use crate::contexts::execution_observability::api::ExecutionObservabilityApi;
use tauri::State;

#[tauri::command]
pub(crate) fn update_observability_settings(
    api: State<'_, ExecutionObservabilityApi>,
    settings: dto::ObservabilitySettingsDto,
) -> Result<dto::ObservabilitySettingsDto, dto::ObservabilityCommandErrorDto> {
    let (settings, auth_token) = mapper::settings_from_dto(settings);
    api.update_settings(
        &settings,
        auth_token.as_deref(),
        &chrono::Utc::now().to_rfc3339(),
    )
    .map(mapper::settings_to_dto)
    .map_err(mapper::adapter_error)
}
