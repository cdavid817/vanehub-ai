use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::communications::api::CommunicationsApi;
use crate::contexts::communications::domain::ConnectorKind;
use crate::contexts::communications::infrastructure::{
    FeishuDesktopFixture, FeishuFixtureEvent, FeishuFixtureLedgerEntry, FeishuFixtureSetupResult,
};
use crate::platform::database::NativeDatabase;
use tauri::State;

#[tauri::command]
pub(crate) fn fixture_feishu_im_setup(
    database: State<'_, NativeDatabase>,
    fixture: State<'_, FeishuDesktopFixture>,
    session_id: String,
    connector: Option<ConnectorKind>,
) -> Result<FeishuFixtureSetupResult, CommandError> {
    fixture
        .setup(
            database.inner(),
            &session_id,
            connector.unwrap_or(ConnectorKind::Feishu),
        )
        .map_err(map_command_error)
}

#[tauri::command]
pub(crate) async fn fixture_feishu_im_inject(
    api: State<'_, CommunicationsApi>,
    fixture: State<'_, FeishuDesktopFixture>,
    input: FeishuFixtureEvent,
) -> Result<FeishuFixtureLedgerEntry, CommandError> {
    fixture
        .inject(api.inner(), input)
        .await
        .map_err(map_command_error)
}

#[tauri::command]
pub(crate) fn fixture_feishu_im_set_fault(
    fixture: State<'_, FeishuDesktopFixture>,
    fault: String,
) -> Result<(), CommandError> {
    fixture.set_fixture_fault(&fault).map_err(map_command_error)
}

#[tauri::command]
pub(crate) fn fixture_feishu_im_ledger(
    fixture: State<'_, FeishuDesktopFixture>,
) -> Result<Vec<FeishuFixtureLedgerEntry>, CommandError> {
    fixture.fixture_ledger().map_err(map_command_error)
}

#[tauri::command]
pub(crate) fn fixture_feishu_im_reset(
    fixture: State<'_, FeishuDesktopFixture>,
) -> Result<(), CommandError> {
    fixture.reset_fixture().map_err(map_command_error)
}
