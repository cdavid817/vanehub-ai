use super::dto::{LspServerTestInputDto, LspServerTestResultDto};
use crate::contexts::code_intelligence::api::{resolve_language, CodeIntelligenceApi};
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
    // The wire carries an arbitrary string now that storage no longer constrains the set, so an
    // unregistered id is refused here rather than starting anything.
    let language =
        resolve_language(&input.language).ok_or_else(|| "unsupported_language".to_owned())?;
    api.test_server(language)
        .await
        .map(|result| LspServerTestResultDto::from_result(language, result))
        .map_err(|error| error.to_string())
}
