use super::dto::LspWorkspaceTrustDto;
use crate::contexts::code_intelligence::api::CodeIntelligenceApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_lsp_workspace_trust(
    api: State<'_, CodeIntelligenceApi>,
) -> Result<Vec<LspWorkspaceTrustDto>, String> {
    execute(api.inner())
}

pub(crate) fn execute(api: &CodeIntelligenceApi) -> Result<Vec<LspWorkspaceTrustDto>, String> {
    api.list_workspace_trust()
        .map(|records| {
            records
                .into_iter()
                .map(LspWorkspaceTrustDto::from)
                .collect()
        })
        .map_err(|error| error.to_string())
}
