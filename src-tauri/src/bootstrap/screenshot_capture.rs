use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Mutex;

use image::{DynamicImage, ImageFormat, RgbaImage};
use serde::Deserialize;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::contexts::local_media::api::LocalMediaApi;
use crate::contexts::local_media::domain::{
    map_screenshot_selection, DisplayGeometry, LogicalSelection, StagedOcrSource,
};

const MAX_MONITORS: usize = 8;
const MAX_TOTAL_PIXELS: u64 = 48_000_000;
const CAPTURE_TIMEOUT_SECONDS: u64 = 120;
const MIN_SELECTION: f64 = 8.0;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartScreenshotRequest {
    composer_scope_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommitScreenshotRequest {
    run_id: String,
    display_token: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CancelScreenshotRequest {
    run_id: String,
}

struct DisplaySnapshot {
    token: String,
    window_label: String,
    logical_x: i32,
    logical_y: i32,
    logical_width: u32,
    logical_height: u32,
    image: RgbaImage,
    png: Vec<u8>,
}

struct CaptureRun {
    id: String,
    main_window_label: String,
    hidden_window_labels: Vec<String>,
    displays: HashMap<String, DisplaySnapshot>,
    completion: Option<oneshot::Sender<Result<Option<Vec<u8>>, String>>>,
}

struct OverlaySpec {
    token: String,
    window_label: String,
    logical_x: i32,
    logical_y: i32,
    logical_width: u32,
    logical_height: u32,
}

#[derive(Default)]
struct Registry {
    busy: bool,
    run: Option<CaptureRun>,
}

#[derive(Default)]
pub(crate) struct ScreenshotCaptureState {
    registry: Mutex<Registry>,
}

impl ScreenshotCaptureState {
    pub(crate) fn shutdown(&self, app: &AppHandle) {
        let run = self.registry.lock().ok().and_then(|mut registry| {
            registry.busy = false;
            registry.run.take()
        });
        if let Some(run) = run {
            restore_and_destroy(app, &run);
            if let Some(completion) = run.completion {
                let _ = completion.send(Ok(None));
            }
        }
    }
}

fn encode_png(image: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image.clone())
        .write_to(&mut bytes, ImageFormat::Png)
        .map_err(|_| "SCREENSHOT_CAPTURE_FAILED".to_string())?;
    Ok(bytes.into_inner())
}

fn xcap_error_code(error: xcap::XCapError) -> String {
    if matches!(error, xcap::XCapError::NotSupported) {
        return "SCREENSHOT_UNAVAILABLE".to_string();
    }
    let classification = error.to_string().to_ascii_lowercase();
    if classification.contains("permission")
        || classification.contains("denied")
        || classification.contains("not authorized")
    {
        "SCREENSHOT_PERMISSION_DENIED".to_string()
    } else {
        "SCREENSHOT_CAPTURE_FAILED".to_string()
    }
}

fn capture_displays() -> Result<Vec<DisplaySnapshot>, String> {
    let monitors = xcap::Monitor::all().map_err(xcap_error_code)?;
    if monitors.is_empty() {
        return Err("SCREENSHOT_NO_DISPLAYS".to_string());
    }
    if monitors.len() > MAX_MONITORS {
        return Err("SCREENSHOT_BUDGET_EXCEEDED".to_string());
    }
    let mut total_pixels = 0_u64;
    let mut snapshots = Vec::with_capacity(monitors.len());
    for monitor in monitors {
        let image = monitor.capture_image().map_err(xcap_error_code)?;
        total_pixels =
            total_pixels.saturating_add(u64::from(image.width()) * u64::from(image.height()));
        if total_pixels > MAX_TOTAL_PIXELS {
            return Err("SCREENSHOT_BUDGET_EXCEEDED".to_string());
        }
        let token = Uuid::new_v4().simple().to_string();
        snapshots.push(DisplaySnapshot {
            window_label: format!("region-capture-{token}"),
            token,
            logical_x: monitor
                .x()
                .map_err(|_| "SCREENSHOT_CAPTURE_FAILED".to_string())?,
            logical_y: monitor
                .y()
                .map_err(|_| "SCREENSHOT_CAPTURE_FAILED".to_string())?,
            logical_width: monitor
                .width()
                .map_err(|_| "SCREENSHOT_CAPTURE_FAILED".to_string())?,
            logical_height: monitor
                .height()
                .map_err(|_| "SCREENSHOT_CAPTURE_FAILED".to_string())?,
            png: encode_png(&image)?,
            image,
        });
    }
    Ok(snapshots)
}

fn restore_and_destroy(app: &AppHandle, run: &CaptureRun) {
    for display in run.displays.values() {
        if let Some(window) = app.get_webview_window(&display.window_label) {
            let _ = window.destroy();
        }
    }
    for label in &run.hidden_window_labels {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.show();
        }
    }
    if let Some(main) = app.get_webview_window(&run.main_window_label) {
        let _ = main.set_focus();
    }
}

fn complete_run(
    app: &AppHandle,
    state: &ScreenshotCaptureState,
    run_id: &str,
    result: Result<Option<Vec<u8>>, String>,
) -> Result<(), String> {
    let mut registry = state
        .registry
        .lock()
        .map_err(|_| "SCREENSHOT_CAPTURE_FAILED")?;
    let run = registry.run.take().ok_or("SCREENSHOT_INVALID_SELECTION")?;
    if run.id != run_id {
        registry.run = Some(run);
        return Err("SCREENSHOT_INVALID_SELECTION".to_string());
    }
    registry.busy = false;
    drop(registry);
    restore_and_destroy(app, &run);
    if let Some(completion) = run.completion {
        let _ = completion.send(result);
    }
    Ok(())
}

fn open_overlays(app: &AppHandle, run_id: &str, displays: &[OverlaySpec]) -> Result<(), String> {
    let mut first_label = None;
    for display in displays {
        let url = format!(
            "index.html?surface=region-capture&run={}&display={}",
            run_id, display.token
        );
        let window =
            WebviewWindowBuilder::new(app, &display.window_label, WebviewUrl::App(url.into()))
                .title("VaneHub Capture")
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(false)
                .shadow(false)
                .inner_size(display.logical_width as f64, display.logical_height as f64)
                .position(display.logical_x as f64, display.logical_y as f64)
                .build()
                .map_err(|_| "SCREENSHOT_CAPTURE_FAILED".to_string())?;
        if first_label.is_none() {
            first_label = Some(display.window_label.clone());
        }
        let event_app = app.clone();
        let event_run_id = run_id.to_string();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Some(state) = event_app.try_state::<ScreenshotCaptureState>() {
                    let _ = complete_run(&event_app, &state, &event_run_id, Ok(None));
                }
            }
        });
        let _ = window.set_position(LogicalPosition::new(display.logical_x, display.logical_y));
        let _ = window.set_size(LogicalSize::new(
            display.logical_width,
            display.logical_height,
        ));
    }
    if let Some(label) = first_label {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.set_focus();
        }
    }
    Ok(())
}

fn reset_before_run(
    app: &AppHandle,
    state: &ScreenshotCaptureState,
    main_label: &str,
    hidden_labels: &[String],
) {
    if let Ok(mut registry) = state.registry.lock() {
        registry.busy = false;
    }
    for label in hidden_labels {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.show();
        }
    }
    if let Some(main) = app.get_webview_window(main_label) {
        let _ = main.set_focus();
    }
}

fn hide_visible_app_windows(app: &AppHandle) -> Result<Vec<String>, String> {
    let mut hidden: Vec<String> = Vec::new();
    for (label, window) in app.webview_windows() {
        if window.is_visible().unwrap_or(false) {
            if window.hide().is_err() {
                for hidden_label in &hidden {
                    if let Some(hidden_window) = app.get_webview_window(hidden_label) {
                        let _ = hidden_window.show();
                    }
                }
                return Err("SCREENSHOT_CAPTURE_FAILED".to_string());
            }
            hidden.push(label);
        }
    }
    Ok(hidden)
}

pub(crate) async fn select_and_stage_screenshot_region(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, ScreenshotCaptureState>,
    api: State<'_, LocalMediaApi>,
    request: StartScreenshotRequest,
) -> Result<Option<StagedOcrSource>, String> {
    if request.composer_scope_id.trim().is_empty() || window.label() != "main" {
        return Err("SCREENSHOT_INVALID_SELECTION".to_string());
    }
    {
        let mut registry = state
            .registry
            .lock()
            .map_err(|_| "SCREENSHOT_CAPTURE_FAILED")?;
        if registry.busy {
            return Err("SCREENSHOT_BUSY".to_string());
        }
        registry.busy = true;
    }
    let hidden_window_labels = match hide_visible_app_windows(&app) {
        Ok(labels) => labels,
        Err(code) => {
            reset_before_run(&app, &state, window.label(), &[]);
            return Err(code);
        }
    };
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    let snapshots = match tauri::async_runtime::spawn_blocking(capture_displays).await {
        Ok(Ok(snapshots)) => snapshots,
        Ok(Err(code)) => {
            reset_before_run(&app, &state, window.label(), &hidden_window_labels);
            return Err(code);
        }
        Err(_) => {
            reset_before_run(&app, &state, window.label(), &hidden_window_labels);
            return Err("SCREENSHOT_CAPTURE_FAILED".to_string());
        }
    };
    let run_id = Uuid::new_v4().simple().to_string();
    let (completion, receiver) = oneshot::channel();
    let run = CaptureRun {
        id: run_id.clone(),
        main_window_label: window.label().to_string(),
        hidden_window_labels,
        displays: snapshots
            .into_iter()
            .map(|item| (item.token.clone(), item))
            .collect(),
        completion: Some(completion),
    };
    let overlays = run
        .displays
        .values()
        .map(|display| OverlaySpec {
            token: display.token.clone(),
            window_label: display.window_label.clone(),
            logical_x: display.logical_x,
            logical_y: display.logical_y,
            logical_width: display.logical_width,
            logical_height: display.logical_height,
        })
        .collect::<Vec<_>>();
    state
        .registry
        .lock()
        .map_err(|_| "SCREENSHOT_CAPTURE_FAILED")?
        .run = Some(run);
    if let Err(code) = open_overlays(&app, &run_id, &overlays) {
        let _ = complete_run(&app, &state, &run_id, Err(code.clone()));
        return Err(code);
    }
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(CAPTURE_TIMEOUT_SECONDS),
        receiver,
    )
    .await;
    let bytes = match outcome {
        Ok(Ok(result)) => result?,
        _ => {
            let _ = complete_run(&app, &state, &run_id, Err("SCREENSHOT_TIMEOUT".to_string()));
            return Err("SCREENSHOT_TIMEOUT".to_string());
        }
    };
    bytes
        .map(|png| {
            api.stage_screenshot_ocr(&png)
                .map_err(|error| error.code().as_str().to_string())
        })
        .transpose()
}

pub(crate) fn commit_screenshot_selection(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, ScreenshotCaptureState>,
    request: CommitScreenshotRequest,
) -> Result<(), String> {
    let png = {
        let registry = state
            .registry
            .lock()
            .map_err(|_| "SCREENSHOT_CAPTURE_FAILED")?;
        let run = registry
            .run
            .as_ref()
            .ok_or("SCREENSHOT_INVALID_SELECTION")?;
        if run.id != request.run_id {
            return Err("SCREENSHOT_INVALID_SELECTION".to_string());
        }
        let display = run
            .displays
            .get(&request.display_token)
            .ok_or("SCREENSHOT_INVALID_SELECTION")?;
        if window.label() != display.window_label {
            return Err("SCREENSHOT_INVALID_SELECTION".to_string());
        }
        let physical = map_screenshot_selection(
            LogicalSelection {
                x: request.x,
                y: request.y,
                width: request.width,
                height: request.height,
            },
            DisplayGeometry {
                logical_origin_x: display.logical_x,
                logical_origin_y: display.logical_y,
                logical_width: display.logical_width,
                logical_height: display.logical_height,
                physical_width: display.image.width(),
                physical_height: display.image.height(),
            },
            MIN_SELECTION,
        )
        .ok_or("SCREENSHOT_INVALID_SELECTION")?;
        let crop = image::imageops::crop_imm(
            &display.image,
            physical.x,
            physical.y,
            physical.width,
            physical.height,
        )
        .to_image();
        encode_png(&crop)?
    };
    complete_run(&app, &state, &request.run_id, Ok(Some(png)))
}

pub(crate) fn cancel_screenshot_selection(
    app: AppHandle,
    state: State<'_, ScreenshotCaptureState>,
    request: CancelScreenshotRequest,
) -> Result<(), String> {
    complete_run(&app, &state, &request.run_id, Ok(None))
}

pub(crate) fn cancel_active_screenshot_selection(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, ScreenshotCaptureState>,
) -> Result<(), String> {
    if window.label() != "main" {
        return Err("SCREENSHOT_INVALID_SELECTION".to_string());
    }
    let run_id = state
        .registry
        .lock()
        .map_err(|_| "SCREENSHOT_CAPTURE_FAILED")?
        .run
        .as_ref()
        .map(|run| run.id.clone());
    match run_id {
        Some(run_id) => complete_run(&app, &state, &run_id, Ok(None)),
        None => Ok(()),
    }
}

pub(crate) fn protocol_response(
    app: &AppHandle,
    webview_label: &str,
    path: &str,
) -> http::Response<Vec<u8>> {
    let not_found = || {
        http::Response::builder()
            .status(404)
            .body(Vec::new())
            .unwrap_or_default()
    };
    let Some(state) = app.try_state::<ScreenshotCaptureState>() else {
        return not_found();
    };
    let Ok(registry) = state.registry.lock() else {
        return not_found();
    };
    let Some(run) = registry.run.as_ref() else {
        return not_found();
    };
    let mut parts = path.trim_start_matches('/').split('/');
    let (Some(run_id), Some(token), None) = (parts.next(), parts.next(), parts.next()) else {
        return not_found();
    };
    let Some(display) = run.displays.get(token) else {
        return not_found();
    };
    if run.id != run_id || display.window_label != webview_label {
        return not_found();
    }
    http::Response::builder()
        .header(http::header::CONTENT_TYPE, "image/png")
        .header(http::header::CACHE_CONTROL, "no-store")
        .header(http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(display.png.clone())
        .unwrap_or_default()
}
