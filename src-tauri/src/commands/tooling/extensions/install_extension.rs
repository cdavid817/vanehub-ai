use super::{background, dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::operations::api::OperationTask;
use crate::contexts::tooling::extensions::api::{ExtensionAction, ExtensionApi};
use tauri::State;

#[tauri::command]
pub(crate) fn install_extension(
    api: State<'_, ExtensionApi>,
    request: dto::ExtensionFrameworkRequest,
) -> Result<OperationTask, CommandError> {
    background::start_operation(
        api.inner(),
        mapper::framework_id_from_dto(request.framework_id),
        ExtensionAction::Install,
    )
    .map_err(map_command_error)
}
