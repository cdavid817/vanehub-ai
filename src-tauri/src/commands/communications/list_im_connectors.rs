use super::dto::ConnectorView;
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::communications::api::CommunicationsApi;
use tauri::State;

#[tauri::command]
pub(crate) async fn list_im_connectors(
    api: State<'_, CommunicationsApi>,
) -> Result<Vec<ConnectorView>, CommandError> {
    api.list_connectors()
        .await
        .map(|summaries| summaries.into_iter().map(mapper::connector).collect())
        .map_err(map_command_error)
}
