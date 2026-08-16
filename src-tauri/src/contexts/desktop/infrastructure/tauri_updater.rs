use crate::contexts::desktop::domain::{admits_update, UpdateChannel};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Runtime};
use tauri_plugin_updater::{Update, UpdaterExt};
use url::Url;

const UPDATE_CHANNEL_BASE: &str = "https://github.com/cdavid817/vanehub-ai/releases/download";

fn channel_endpoint(channel: UpdateChannel) -> Result<Url, String> {
    let channel_tag = match channel {
        UpdateChannel::Stable => "update-stable",
        UpdateChannel::Preview => "update-preview",
    };
    Url::parse(&format!("{UPDATE_CHANNEL_BASE}/{channel_tag}/latest.json"))
        .map_err(|_| "configured update endpoint is invalid".to_string())
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateSnapshot {
    pub(crate) phase: String,
    pub(crate) current_version: String,
    pub(crate) channel: UpdateChannel,
    pub(crate) latest_version: Option<String>,
    pub(crate) release_notes: Option<String>,
    pub(crate) checked_at: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) downloaded_bytes: Option<u64>,
    pub(crate) total_bytes: Option<u64>,
    pub(crate) error: Option<String>,
}

pub(crate) struct TauriUpdateRuntime {
    pending: Mutex<Option<Update>>,
    snapshot: Mutex<UpdateSnapshot>,
}

impl TauriUpdateRuntime {
    pub(crate) fn new(version: &str) -> Self {
        Self {
            pending: Mutex::new(None),
            snapshot: Mutex::new(UpdateSnapshot {
                phase: "idle".into(),
                current_version: version.into(),
                channel: UpdateChannel::default_for(version),
                latest_version: None,
                release_notes: None,
                checked_at: None,
                operation_id: None,
                downloaded_bytes: None,
                total_bytes: None,
                error: None,
            }),
        }
    }

    pub(crate) fn snapshot(&self) -> Result<UpdateSnapshot, String> {
        self.snapshot
            .lock()
            .map(|value| value.clone())
            .map_err(|_| "update state unavailable".into())
    }

    pub(crate) fn set_channel(&self, channel: UpdateChannel) -> Result<(), String> {
        self.snapshot
            .lock()
            .map_err(|_| "update state unavailable".to_string())?
            .channel = channel;
        Ok(())
    }

    pub(crate) fn queue(
        &self,
        operation_id: String,
        phase: &str,
    ) -> Result<UpdateSnapshot, String> {
        let mut state = self
            .snapshot
            .lock()
            .map_err(|_| "update state unavailable")?;
        state.phase = phase.into();
        state.operation_id = Some(operation_id);
        state.error = None;
        Ok(state.clone())
    }

    pub(crate) async fn check<R: Runtime>(
        &self,
        app: &AppHandle<R>,
    ) -> Result<UpdateSnapshot, String> {
        {
            let mut state = self
                .snapshot
                .lock()
                .map_err(|_| "update state unavailable")?;
            state.phase = "checking".into();
        }
        let channel = self
            .snapshot
            .lock()
            .map_err(|_| "update state unavailable")?
            .channel;
        let update = app
            .updater_builder()
            .endpoints(vec![channel_endpoint(channel)?])
            .map_err(|error| error.to_string())?
            .build()
            .map_err(|error| error.to_string())?
            .check()
            .await
            .map_err(|error| self.fail(error.to_string()))?;
        let mut state = self
            .snapshot
            .lock()
            .map_err(|_| "update state unavailable")?;
        state.checked_at = Some(chrono::Utc::now().to_rfc3339());
        if let Some(update) = update {
            if !admits_update(&state.current_version, &update.version, state.channel) {
                state.phase = "up-to-date".into();
                return Ok(state.clone());
            }
            state.phase = "available".into();
            state.latest_version = Some(update.version.clone());
            state.release_notes = update.body.clone();
            *self
                .pending
                .lock()
                .map_err(|_| "update state unavailable")? = Some(update);
        } else {
            state.phase = "up-to-date".into();
        }
        Ok(state.clone())
    }

    pub(crate) async fn install(&self) -> Result<UpdateSnapshot, String> {
        let update = self
            .pending
            .lock()
            .map_err(|_| "update state unavailable")?
            .take()
            .ok_or("no verified update is available")?;
        {
            let mut state = self
                .snapshot
                .lock()
                .map_err(|_| "update state unavailable")?;
            state.phase = "downloading".into();
        }
        let mut downloaded = 0_u64;
        update
            .download_and_install(
                |chunk_length, content_length| {
                    downloaded = downloaded.saturating_add(chunk_length as u64);
                    if let Ok(mut state) = self.snapshot.lock() {
                        state.phase = "downloading".into();
                        state.downloaded_bytes = Some(downloaded);
                        state.total_bytes = content_length;
                    }
                },
                || {},
            )
            .await
            .map_err(|error| self.fail(error.to_string()))?;
        let mut state = self
            .snapshot
            .lock()
            .map_err(|_| "update state unavailable")?;
        state.phase = "ready-to-restart".into();
        Ok(state.clone())
    }

    fn fail(&self, message: String) -> String {
        if let Ok(mut state) = self.snapshot.lock() {
            state.phase = "failed".into();
            state.error = Some("The signed update operation failed".into());
        }
        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_channels_to_distinct_fixed_https_metadata() {
        let stable = channel_endpoint(UpdateChannel::Stable).expect("stable endpoint");
        let preview = channel_endpoint(UpdateChannel::Preview).expect("preview endpoint");

        assert_eq!(stable.scheme(), "https");
        assert_eq!(preview.scheme(), "https");
        assert!(stable.path().contains("/update-stable/"));
        assert!(preview.path().contains("/update-preview/"));
        assert_ne!(stable, preview);
    }
}
