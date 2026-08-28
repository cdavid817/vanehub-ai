use super::dto::{LspSafeReasonCodeDto, LspServerInstallInputDto};
use crate::contexts::code_intelligence::api::{
    resolve_language, CodeIntelligenceApi, CodeIntelligenceApiError, ManagedInstallFailure,
};
use tauri::State;

#[tauri::command]
pub(crate) async fn install_lsp_server(
    api: State<'_, CodeIntelligenceApi>,
    input: LspServerInstallInputDto,
) -> Result<(), String> {
    execute(api.inner(), input).await
}

pub(crate) async fn execute(
    api: &CodeIntelligenceApi,
    input: LspServerInstallInputDto,
) -> Result<(), String> {
    // The wire carries an arbitrary string, so an unregistered id is refused here rather than
    // reaching a download.
    let language =
        resolve_language(&input.language).ok_or_else(|| "unsupported_language".to_owned())?;
    api.install_language_server(language.id)
        .await
        .map_err(install_reason)
}

/// A closed reason code rather than the error's message.
///
/// Everything this boundary returns is rendered by the settings page from a fixed set; a message
/// would be text the frontend has to display without knowing what it says.
fn install_reason(error: CodeIntelligenceApiError) -> String {
    let code = match error {
        CodeIntelligenceApiError::Managed(failure) => match failure {
            ManagedInstallFailure::Refused => LspSafeReasonCodeDto::InstallRefused,
            ManagedInstallFailure::Transfer => LspSafeReasonCodeDto::InstallFailed,
            ManagedInstallFailure::TimedOut => LspSafeReasonCodeDto::InstallTimedOut,
            ManagedInstallFailure::Cancelled => LspSafeReasonCodeDto::Cancelled,
            ManagedInstallFailure::ChecksumMismatch => LspSafeReasonCodeDto::ChecksumMismatch,
        },
        _ => LspSafeReasonCodeDto::InvalidConfiguration,
    };
    serde_json::to_value(code)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "install_failed".to_owned())
}
