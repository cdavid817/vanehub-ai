use super::dto::WeChatAuthorizationView;
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::communications::api::WeChatAuthorizationApi;
use tauri::State;

#[tauri::command]
pub(crate) async fn begin_wechat_authorization(
    api: State<'_, WeChatAuthorizationApi>,
) -> Result<WeChatAuthorizationView, CommandError> {
    api.begin()
        .await
        .map(mapper::authorization)
        .map_err(map_command_error)
}
