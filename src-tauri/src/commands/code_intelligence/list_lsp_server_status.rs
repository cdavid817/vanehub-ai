use super::dto::LspServerStatusDto;
use crate::contexts::code_intelligence::api::CodeIntelligenceApi;
use tauri::State;

#[tauri::command]
pub(crate) async fn list_lsp_server_status(
    api: State<'_, CodeIntelligenceApi>,
) -> Result<Vec<LspServerStatusDto>, String> {
    execute(api.inner()).await
}

pub(crate) async fn execute(api: &CodeIntelligenceApi) -> Result<Vec<LspServerStatusDto>, String> {
    api.server_statuses()
        .await
        .map(|statuses| statuses.into_iter().map(LspServerStatusDto::from).collect())
        .map_err(|error| error.to_string())
}
