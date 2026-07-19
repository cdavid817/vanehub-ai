use super::{dto, mapper};
use crate::contexts::tooling::sdk::api::SdkApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_sdk_definitions(api: State<'_, SdkApi>) -> Vec<dto::SdkDefinition> {
    api.list_definitions()
        .into_iter()
        .map(mapper::definition_to_dto)
        .collect()
}
