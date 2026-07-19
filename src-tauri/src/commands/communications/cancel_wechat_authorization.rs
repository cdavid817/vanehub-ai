use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::communications::api::WeChatAuthorizationApi;
use tauri::State;

#[tauri::command]
pub(crate) fn cancel_wechat_authorization(
    api: State<'_, WeChatAuthorizationApi>,
) -> Result<(), CommandError> {
    api.cancel().map_err(map_command_error)
}
