use super::api::DesktopSettingsApi;
use super::domain::UpdateChannel;
use super::infrastructure::{TauriUpdateRuntime, UpdateSnapshot};
use crate::contexts::operations::api::{OperationKind, OperationsApi};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdatePreferences {
    pub(crate) automatic_check: bool,
    pub(crate) channel: UpdateChannel,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateReceipt {
    pub(crate) operation_id: String,
    pub(crate) snapshot: UpdateSnapshot,
}

pub(crate) struct DesktopUpdateApi {
    app: AppHandle,
    runtime: TauriUpdateRuntime,
    settings: DesktopSettingsApi,
    operations: OperationsApi,
}

impl DesktopUpdateApi {
    pub(crate) fn new(
        app: AppHandle,
        settings: DesktopSettingsApi,
        operations: OperationsApi,
    ) -> Self {
        Self {
            app,
            runtime: TauriUpdateRuntime::new(env!("CARGO_PKG_VERSION")),
            settings,
            operations,
        }
    }
    pub(crate) fn snapshot(&self) -> Result<UpdateSnapshot, String> {
        self.runtime.snapshot()
    }
    pub(crate) fn preferences(&self) -> Result<UpdatePreferences, String> {
        let settings = self
            .settings
            .get_settings()
            .map_err(|error| error.to_string())?;
        Ok(UpdatePreferences {
            automatic_check: settings.settings.update_automatic_check(),
            channel: parse_channel(settings.settings.update_channel()),
        })
    }
    pub(crate) fn save_preferences(
        &self,
        input: UpdatePreferences,
    ) -> Result<UpdatePreferences, String> {
        self.settings
            .save_setting("updateAutomaticCheck", &input.automatic_check.to_string())
            .map_err(|error| error.to_string())?;
        self.settings
            .save_setting("updateChannel", channel_name(input.channel))
            .map_err(|error| error.to_string())?;
        Ok(input)
    }
    pub(crate) fn start_check(&self) -> Result<UpdateReceipt, String> {
        self.runtime.set_channel(self.preferences()?.channel)?;
        let operation = self
            .operations
            .start(
                OperationKind::Agent,
                None,
                Some("desktop update check".into()),
            )
            .map_err(|error| error.to_string())?;
        let snapshot = self.runtime.queue(operation.id.clone(), "queued")?;
        let operation_id = operation.id.clone();
        let app = self.app.clone();
        tauri::async_runtime::spawn(async move {
            let api = app.state::<DesktopUpdateApi>();
            let _ = api
                .operations
                .append_log(&operation_id, "checking trusted updater endpoint".into());
            match api.runtime.check(&app).await {
                Ok(snapshot) => {
                    let _ = api
                        .operations
                        .complete(&operation_id, serde_json::to_value(snapshot).ok());
                }
                Err(_) => {
                    let _ = api
                        .operations
                        .fail(&operation_id, "The signed update check failed".into());
                }
            }
        });
        Ok(UpdateReceipt {
            operation_id: operation.id,
            snapshot,
        })
    }
    pub(crate) fn start_install(&self) -> Result<UpdateReceipt, String> {
        let operation = self
            .operations
            .start(
                OperationKind::Agent,
                None,
                Some("signed desktop update install".into()),
            )
            .map_err(|error| error.to_string())?;
        let snapshot = self.runtime.queue(operation.id.clone(), "queued")?;
        let operation_id = operation.id.clone();
        let app = self.app.clone();
        tauri::async_runtime::spawn(async move {
            let api = app.state::<DesktopUpdateApi>();
            let _ = api.operations.append_log(
                &operation_id,
                "downloading and verifying signed updater artifact".into(),
            );
            match api.runtime.install().await {
                Ok(snapshot) => {
                    let _ = api
                        .operations
                        .complete(&operation_id, serde_json::to_value(snapshot).ok());
                }
                Err(_) => {
                    let _ = api.operations.fail(
                        &operation_id,
                        "The signed update installation failed".into(),
                    );
                }
            }
        });
        Ok(UpdateReceipt {
            operation_id: operation.id,
            snapshot,
        })
    }
    pub(crate) fn restart(&self) -> Result<(), String> {
        if self.runtime.snapshot()?.phase != "ready-to-restart" {
            return Err("verified update is not ready".into());
        }
        self.app.restart();
    }
}

fn parse_channel(value: &str) -> UpdateChannel {
    if value == "stable" {
        UpdateChannel::Stable
    } else {
        UpdateChannel::Preview
    }
}
fn channel_name(value: UpdateChannel) -> &'static str {
    match value {
        UpdateChannel::Stable => "stable",
        UpdateChannel::Preview => "preview",
    }
}
