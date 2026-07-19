use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::extensions::api::ExtensionApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_extension_install_preview(
    api: State<'_, ExtensionApi>,
    request: dto::ExtensionFrameworkRequest,
) -> Result<dto::ExtensionInstallPreview, CommandError> {
    api.install_preview(mapper::framework_id_from_dto(request.framework_id))
        .map(mapper::preview_to_dto)
        .map_err(map_command_error)
}
