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
    let installed = api.installed_languages();
    api.configuration()
        .map(LspConfigurationDto::from)
        .map(|mut configuration| {
            // Filled in here rather than in the conversion: whether a server is installed is a
            // fact about the filesystem, and the conversion from a stored configuration has none.
            for descriptor in &mut configuration.descriptors {
                descriptor.installed = installed.contains(&descriptor.language.as_str());
            }
            configuration
        })
        .map_err(|error| error.to_string())
}
