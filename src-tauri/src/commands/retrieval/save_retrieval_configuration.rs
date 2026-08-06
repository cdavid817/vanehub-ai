//! `add-retrieval-vector-search` Task 15: saves the global retrieval configuration singleton
//! (design doc §7.4).

use crate::contexts::retrieval::api::RetrievalApi;
use tauri::State;

#[tauri::command]
pub(crate) fn save_retrieval_configuration(
    api: State<'_, RetrievalApi>,
    profile_id: String,
    model_id: String,
) -> Result<(), String> {
    api.save_configuration(&profile_id, &model_id)
        .map_err(|error| error.category().to_string())
}
