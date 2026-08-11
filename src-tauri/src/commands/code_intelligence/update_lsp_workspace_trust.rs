use super::dto::{LspWorkspaceTrustDto, LspWorkspaceTrustUpdateDto};
use crate::contexts::code_intelligence::api::CodeIntelligenceApi;
use std::path::Path;
use tauri::State;

#[tauri::command]
pub(crate) fn update_lsp_workspace_trust(
    api: State<'_, CodeIntelligenceApi>,
    update: LspWorkspaceTrustUpdateDto,
) -> Result<LspWorkspaceTrustDto, String> {
    execute(api.inner(), update)
}

pub(crate) fn execute(
    api: &CodeIntelligenceApi,
    update: LspWorkspaceTrustUpdateDto,
) -> Result<LspWorkspaceTrustDto, String> {
    api.update_workspace_trust(Path::new(&update.canonical_root), update.trusted)
        .map(LspWorkspaceTrustDto::from)
        .map_err(|error| error.to_string())
}
