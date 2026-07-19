use super::{dto, mapper};
use crate::contexts::desktop::api::DesktopSettingsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn get_node_info(api: State<'_, DesktopSettingsApi>) -> dto::NodeInfo {
    mapper::node_information_to_dto(api.node_information())
}
