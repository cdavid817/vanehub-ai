use super::{background, dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::operations::api::OperationTask;
use crate::contexts::tooling::extensions::api::ExtensionApi;
use tauri::State;

#[tauri::command]
pub(crate) fn set_extension_enabled(
    api: State<'_, ExtensionApi>,
    request: dto::ExtensionEnableRequest,
) -> Result<OperationTask, CommandError> {
    background::start_operation(
        api.inner(),
        mapper::framework_id_from_dto(request.framework_id),
        mapper::enable_action(request.enabled),
    )
    .map_err(map_command_error)
}
