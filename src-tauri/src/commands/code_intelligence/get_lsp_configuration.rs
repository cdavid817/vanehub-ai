use super::dto::LspConfigurationDto;
use crate::contexts::code_intelligence::api::CodeIntelligenceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_lsp_configuration(
    api: State<'_, CodeIntelligenceApi>,
) -> Result<LspConfigurationDto, String> {
    execute(api.inner())
}

pub(crate) fn execute(api: &CodeIntelligenceApi) -> Result<LspConfigurationDto, String> {
    api.configuration()
        .map(LspConfigurationDto::from)
        .map_err(|error| error.to_string())
}
