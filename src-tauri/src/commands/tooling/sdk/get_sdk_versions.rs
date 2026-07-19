use super::{dto, mapper};
use crate::contexts::tooling::sdk::api::SdkApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_sdk_versions(
    api: State<'_, SdkApi>,
    sdk_id: Option<dto::SdkId>,
) -> dto::SdkVersionMap {
    mapper::version_map_to_dto(api.get_versions(mapper::optional_id_from_dto(sdk_id)))
}
