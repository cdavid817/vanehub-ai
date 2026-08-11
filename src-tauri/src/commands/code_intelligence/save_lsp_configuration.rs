use super::dto::LspConfigurationDto;
use crate::contexts::code_intelligence::api::{CodeIntelligenceApi, LspConfiguration};
use tauri::State;

#[tauri::command]
pub(crate) fn save_lsp_configuration(
    api: State<'_, CodeIntelligenceApi>,
    configuration: LspConfigurationDto,
) -> Result<(), String> {
    execute(api.inner(), configuration)
}

pub(crate) fn execute(
    api: &CodeIntelligenceApi,
    configuration: LspConfigurationDto,
) -> Result<(), String> {
    let configuration =
        LspConfiguration::try_from(configuration).map_err(|error| error.to_string())?;
    api.save_configuration(&configuration)
        .map_err(|error| error.to_string())
}
