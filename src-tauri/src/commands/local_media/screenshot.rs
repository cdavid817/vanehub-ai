use tauri::{AppHandle, State, WebviewWindow};

use crate::bootstrap::screenshot_capture::{
    self, CancelScreenshotRequest, CommitScreenshotRequest, ScreenshotCaptureState,
    StartScreenshotRequest,
};
use crate::contexts::local_media::api::LocalMediaApi;
use crate::contexts::local_media::domain::StagedOcrSource;

#[tauri::command]
pub(crate) async fn select_and_stage_screenshot_region(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, ScreenshotCaptureState>,
    api: State<'_, LocalMediaApi>,
    request: StartScreenshotRequest,
) -> Result<Option<StagedOcrSource>, String> {
    screenshot_capture::select_and_stage_screenshot_region(app, window, state, api, request).await
}

#[tauri::command]
pub(crate) fn commit_screenshot_selection(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, ScreenshotCaptureState>,
    request: CommitScreenshotRequest,
) -> Result<(), String> {
    screenshot_capture::commit_screenshot_selection(app, window, state, request)
}

#[tauri::command]
pub(crate) fn cancel_screenshot_selection(
    app: AppHandle,
    state: State<'_, ScreenshotCaptureState>,
    request: CancelScreenshotRequest,
) -> Result<(), String> {
    screenshot_capture::cancel_screenshot_selection(app, state, request)
}

#[tauri::command]
pub(crate) fn cancel_active_screenshot_selection(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, ScreenshotCaptureState>,
) -> Result<(), String> {
    screenshot_capture::cancel_active_screenshot_selection(app, window, state)
}
