use super::dto::LspServerInstallInputDto;
use crate::contexts::code_intelligence::api::{resolve_language, CodeIntelligenceApi};
use tauri::State;

#[tauri::command]
pub(crate) async fn uninstall_lsp_server(
    api: State<'_, CodeIntelligenceApi>,
    input: LspServerInstallInputDto,
) -> Result<(), String> {
    execute(api.inner(), input).await
}

pub(crate) async fn execute(
    api: &CodeIntelligenceApi,
    input: LspServerInstallInputDto,
) -> Result<(), String> {
    let language =
        resolve_language(&input.language).ok_or_else(|| "unsupported_language".to_owned())?;
    // Removes only the directory VaneHub created. A manual override names one the user made, and
    // this must not touch it.
    api.uninstall_language_server(language.id)
        .await
        .map_err(|error| error.to_string())
}
