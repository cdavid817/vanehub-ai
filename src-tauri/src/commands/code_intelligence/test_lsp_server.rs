use super::dto::{LspServerTestInputDto, LspServerTestResultDto};
use crate::contexts::code_intelligence::api::{CodeIntelligenceApi, LanguageFamily};
use tauri::State;

#[tauri::command]
pub(crate) async fn test_lsp_server(
    api: State<'_, CodeIntelligenceApi>,
    input: LspServerTestInputDto,
) -> Result<LspServerTestResultDto, String> {
    execute(api.inner(), input).await
}

pub(crate) async fn execute(
    api: &CodeIntelligenceApi,
    input: LspServerTestInputDto,
) -> Result<LspServerTestResultDto, String> {
    let language = LanguageFamily::from(input.language);
    let server = language.server_kind();
    api.test_server(language)
        .await
        .map(|result| LspServerTestResultDto::from_result(server, result))
        .map_err(|error| error.to_string())
}
