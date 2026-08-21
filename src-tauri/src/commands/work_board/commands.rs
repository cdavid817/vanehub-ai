use crate::contexts::work_board::{api, models::*};
use crate::platform::database::NativeDatabase;
use tauri::State;

#[tauri::command]
pub(crate) fn list_work_items(
    database: State<'_, NativeDatabase>,
    filters: WorkItemFilters,
) -> Result<Vec<WorkItem>, String> {
    api::list_items(&database, filters)
}

#[tauri::command]
pub(crate) fn create_work_item(
    database: State<'_, NativeDatabase>,
    input: CreateWorkItemInput,
) -> Result<WorkItem, String> {
    api::create(&database, input)
}

#[tauri::command]
pub(crate) fn update_work_item(
    database: State<'_, NativeDatabase>,
    input: UpdateWorkItemInput,
) -> Result<WorkItem, String> {
    api::update(&database, input)
}

#[tauri::command]
pub(crate) fn move_work_item(
    database: State<'_, NativeDatabase>,
    input: MoveWorkItemInput,
) -> Result<WorkItem, String> {
    api::move_item(&database, input)
}

#[tauri::command]
pub(crate) fn link_work_item_source(
    database: State<'_, NativeDatabase>,
    input: LinkWorkItemSourceInput,
) -> Result<WorkItem, String> {
    api::link(&database, input)
}

#[tauri::command]
pub(crate) fn archive_work_item(
    database: State<'_, NativeDatabase>,
    work_item_id: String,
) -> Result<WorkItem, String> {
    api::set_archived(&database, &work_item_id, true)
}

#[tauri::command]
pub(crate) fn restore_work_item(
    database: State<'_, NativeDatabase>,
    work_item_id: String,
) -> Result<WorkItem, String> {
    api::set_archived(&database, &work_item_id, false)
}

#[tauri::command]
pub(crate) fn delete_work_item(
    database: State<'_, NativeDatabase>,
    work_item_id: String,
) -> Result<(), String> {
    api::delete(&database, &work_item_id)
}
