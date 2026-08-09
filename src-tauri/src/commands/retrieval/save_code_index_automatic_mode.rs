use crate::contexts::retrieval::api::RetrievalApi;
use crate::contexts::retrieval::domain::CodeIndexAutomaticMode;
use tauri::State;

#[tauri::command]
pub(crate) fn save_code_index_automatic_mode(
    api: State<'_, RetrievalApi>,
    mode: String,
) -> Result<(), String> {
    let mode = CodeIndexAutomaticMode::parse(&mode).ok_or_else(|| "validation".to_string())?;
    api.save_automatic_code_index_mode(mode)
        .map_err(|error| error.category().to_string())
}
