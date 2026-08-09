use super::dto::{safe_error, CodeIndexConfigurationInput, CodeIndexWorkspaceDto};
use crate::contexts::retrieval::api::CodeIndexApi;
use tauri::State;

#[tauri::command]
pub(crate) fn save_code_index_configuration(
    workspace_id: String,
    configuration: CodeIndexConfigurationInput,
    api: State<'_, CodeIndexApi>,
) -> Result<CodeIndexWorkspaceDto, String> {
    let update = configuration.try_into().map_err(safe_error)?;
    let workspace = api
        .save_configuration(&workspace_id, update)
        .map_err(safe_error)?;
    let status = api.workspace_status(&workspace_id).map_err(safe_error)?;
    Ok(CodeIndexWorkspaceDto::new(workspace, status))
}
