use super::dto::LspServerDiscoveryDto;
use crate::contexts::code_intelligence::api::CodeIntelligenceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn discover_lsp_servers(
    api: State<'_, CodeIntelligenceApi>,
) -> Result<Vec<LspServerDiscoveryDto>, String> {
    execute(api.inner())
}

pub(crate) fn execute(api: &CodeIntelligenceApi) -> Result<Vec<LspServerDiscoveryDto>, String> {
    api.discover_servers()
        .map(|servers| {
            servers
                .into_iter()
                .map(LspServerDiscoveryDto::from)
                .collect()
        })
        .map_err(|error| error.to_string())
}
