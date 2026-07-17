use crate::{current_timestamp, record_native_log, AppError, NativeLogLevel, RegistryStore};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

pub(crate) const FLOATING_ASSISTANT_LABEL: &str = "floating-assistant";
const COLLAPSED_SIZE: (f64, f64) = (76.0, 76.0);
const MENU_SIZE: (f64, f64) = (304.0, 316.0);
const CHAT_SIZE: (f64, f64) = (408.0, 620.0);
const SCREEN_MARGIN: i32 = 20;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FloatingAssistantSurfaceMode {
    Collapsed,
    Menu,
    Chat,
}

impl FloatingAssistantSurfaceMode {
    fn size(self) -> (f64, f64) {
        match self {
            Self::Collapsed => COLLAPSED_SIZE,
            Self::Menu => MENU_SIZE,
            Self::Chat => CHAT_SIZE,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FloatingAssistantRuntimeInfo {
    native_available: bool,
    platform: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
enum FloatingAssistantEvent {
    ConfigurationChanged { config: FloatingAssistantConfig },
    SurfaceChanged { mode: FloatingAssistantSurfaceMode },
    MainAction { action: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FloatingAssistantAnchor {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) monitor_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FloatingAssistantConfig {
    pub(crate) enabled: bool,
    pub(crate) anchor: Option<FloatingAssistantAnchor>,
}

pub(crate) fn apply_schema(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS floating_assistant_config (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
            anchor_x REAL,
            anchor_y REAL,
            monitor_name TEXT,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO floating_assistant_config (id, enabled, updated_at) VALUES (1, 0, ?1)",
        params![current_timestamp()],
    )?;
    Ok(())
}

fn valid_coordinate(value: f64) -> bool {
    value.is_finite() && value.abs() <= 10_000_000.0
}

fn normalize_anchor(anchor: Option<FloatingAssistantAnchor>) -> Option<FloatingAssistantAnchor> {
    anchor.filter(|value| valid_coordinate(value.x) && valid_coordinate(value.y))
}

pub(crate) fn load_from_conn(conn: &Connection) -> Result<FloatingAssistantConfig, AppError> {
    let row = conn.query_row(
        "SELECT enabled, anchor_x, anchor_y, monitor_name FROM floating_assistant_config WHERE id = 1",
        [],
        |row| {
            Ok((
                row.get::<_, i64>(0)? != 0,
                row.get::<_, Option<f64>>(1)?,
                row.get::<_, Option<f64>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        },
    )?;
    let anchor = match (row.1, row.2) {
        (Some(x), Some(y)) => normalize_anchor(Some(FloatingAssistantAnchor {
            x,
            y,
            monitor_name: row.3,
        })),
        _ => None,
    };
    Ok(FloatingAssistantConfig {
        enabled: row.0,
        anchor,
    })
}

pub(crate) fn save_to_conn(
    conn: &Connection,
    config: FloatingAssistantConfig,
) -> Result<FloatingAssistantConfig, AppError> {
    let anchor = normalize_anchor(config.anchor);
    conn.execute(
        "UPDATE floating_assistant_config
         SET enabled = ?1, anchor_x = ?2, anchor_y = ?3, monitor_name = ?4, updated_at = ?5
         WHERE id = 1",
        params![
            i64::from(config.enabled),
            anchor.as_ref().map(|value| value.x),
            anchor.as_ref().map(|value| value.y),
            anchor
                .as_ref()
                .and_then(|value| value.monitor_name.as_deref()),
            current_timestamp(),
        ],
    )?;
    Ok(FloatingAssistantConfig {
        enabled: config.enabled,
        anchor,
    })
}

fn storage_error(error: impl std::fmt::Display) -> AppError {
    AppError::Storage(error.to_string())
}

fn clamp_position(
    position: PhysicalPosition<i32>,
    window_size: (u32, u32),
    monitor_position: PhysicalPosition<i32>,
    monitor_size: (u32, u32),
) -> PhysicalPosition<i32> {
    let min_x = monitor_position.x + SCREEN_MARGIN;
    let min_y = monitor_position.y + SCREEN_MARGIN;
    let max_x = monitor_position.x + monitor_size.0 as i32 - window_size.0 as i32 - SCREEN_MARGIN;
    let max_y = monitor_position.y + monitor_size.1 as i32 - window_size.1 as i32 - SCREEN_MARGIN;
    PhysicalPosition::new(
        position.x.clamp(min_x, max_x.max(min_x)),
        position.y.clamp(min_y, max_y.max(min_y)),
    )
}

fn anchor_for_position(
    position: PhysicalPosition<i32>,
    window_size: (u32, u32),
    monitor_name: Option<String>,
) -> FloatingAssistantAnchor {
    FloatingAssistantAnchor {
        x: f64::from(position.x) + f64::from(window_size.0),
        y: f64::from(position.y) + f64::from(window_size.1),
        monitor_name,
    }
}

fn position_for_anchor(
    anchor: &FloatingAssistantAnchor,
    window_size: (u32, u32),
) -> PhysicalPosition<i32> {
    PhysicalPosition::new(
        anchor.x.round() as i32 - window_size.0 as i32,
        anchor.y.round() as i32 - window_size.1 as i32,
    )
}

fn monitor_for_anchor(
    app: &AppHandle,
    anchor: Option<&FloatingAssistantAnchor>,
) -> Result<Option<tauri::Monitor>, AppError> {
    let monitors = app.available_monitors().map_err(storage_error)?;
    if let Some(name) = anchor.and_then(|value| value.monitor_name.as_deref()) {
        if let Some(monitor) = monitors
            .iter()
            .find(|monitor| monitor.name().is_some_and(|candidate| candidate == name))
        {
            return Ok(Some(monitor.clone()));
        }
    }
    app.primary_monitor().map_err(storage_error)
}

fn position_window(
    app: &AppHandle,
    window: &WebviewWindow,
    anchor: Option<&FloatingAssistantAnchor>,
) -> Result<(), AppError> {
    let Some(monitor) = monitor_for_anchor(app, anchor)? else {
        return Ok(());
    };
    let work_area = monitor.work_area();
    let monitor_position = work_area.position;
    let monitor_size = work_area.size;
    let window_size = window.outer_size().map_err(storage_error)?;
    let requested = anchor.map_or_else(
        || {
            let default_anchor = FloatingAssistantAnchor {
                x: f64::from(monitor_position.x + monitor_size.width as i32 - SCREEN_MARGIN),
                y: f64::from(monitor_position.y + monitor_size.height as i32 - SCREEN_MARGIN),
                monitor_name: monitor.name().cloned(),
            };
            position_for_anchor(&default_anchor, (window_size.width, window_size.height))
        },
        |value| position_for_anchor(value, (window_size.width, window_size.height)),
    );
    window
        .set_position(clamp_position(
            requested,
            (window_size.width, window_size.height),
            monitor_position,
            (monitor_size.width, monitor_size.height),
        ))
        .map_err(storage_error)
}

pub(crate) fn ensure_window(
    app: &AppHandle,
    config: &FloatingAssistantConfig,
) -> Result<WebviewWindow, AppError> {
    if let Some(window) = app.get_webview_window(FLOATING_ASSISTANT_LABEL) {
        if config.enabled {
            window.show().map_err(storage_error)?;
        }
        return Ok(window);
    }
    let window = WebviewWindowBuilder::new(
        app,
        FLOATING_ASSISTANT_LABEL,
        WebviewUrl::App("index.html?surface=floating-assistant".into()),
    )
    .title("VaneHub AI")
    .inner_size(COLLAPSED_SIZE.0, COLLAPSED_SIZE.1)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .shadow(false)
    .visible(false)
    .build()
    .map_err(storage_error)?;
    window.set_decorations(false).map_err(storage_error)?;
    window.set_always_on_top(true).map_err(storage_error)?;
    window.set_skip_taskbar(true).map_err(storage_error)?;
    window.set_resizable(false).map_err(storage_error)?;
    window.set_shadow(false).map_err(storage_error)?;
    window
        .set_size(LogicalSize::new(COLLAPSED_SIZE.0, COLLAPSED_SIZE.1))
        .map_err(storage_error)?;
    position_window(app, &window, config.anchor.as_ref())?;
    if config.enabled {
        window.show().map_err(storage_error)?;
    }
    Ok(window)
}

pub(crate) fn initialize(app: &AppHandle) -> Result<(), AppError> {
    let config = {
        let state = app.state::<Mutex<RegistryStore>>();
        let store = state.lock().map_err(storage_error)?;
        let conn = store.connection()?;
        load_from_conn(&conn)?
    };
    if config.enabled {
        ensure_window(app, &config)?;
    }
    Ok(())
}

pub(crate) fn should_hide_main_on_close(app: &AppHandle) -> bool {
    let state = app.state::<Mutex<RegistryStore>>();
    state
        .lock()
        .ok()
        .and_then(|store| store.connection().ok())
        .and_then(|conn| load_from_conn(&conn).ok())
        .is_some_and(|config| config.enabled)
}

pub(crate) fn should_intercept_main_close(enabled: bool, floating_window_available: bool) -> bool {
    enabled && floating_window_available
}

fn valid_main_action(action: &str) -> bool {
    matches!(action, "new-session" | "current-session" | "settings")
}

#[tauri::command]
pub(crate) fn get_floating_assistant_runtime_info() -> FloatingAssistantRuntimeInfo {
    FloatingAssistantRuntimeInfo {
        native_available: cfg!(target_os = "windows"),
        platform: if cfg!(target_os = "windows") {
            "windows"
        } else {
            "unsupported"
        },
    }
}

#[tauri::command]
pub(crate) fn get_floating_assistant_config(
    state: State<'_, Mutex<RegistryStore>>,
) -> Result<FloatingAssistantConfig, AppError> {
    let store = state.lock().map_err(storage_error)?;
    let conn = store.connection()?;
    load_from_conn(&conn)
}

#[tauri::command]
pub(crate) async fn set_floating_assistant_enabled(
    app: AppHandle,
    state: State<'_, Mutex<RegistryStore>>,
    enabled: bool,
) -> Result<FloatingAssistantConfig, AppError> {
    if enabled && !cfg!(target_os = "windows") {
        return Err(AppError::Validation(
            "floating assistant is currently available on Windows only".to_string(),
        ));
    }
    let previous = {
        let store = state.lock().map_err(storage_error)?;
        let conn = store.connection()?;
        load_from_conn(&conn)?
    };
    let next = FloatingAssistantConfig {
        enabled,
        anchor: previous.anchor.clone(),
    };
    if enabled {
        if let Err(error) = ensure_window(&app, &next) {
            record_native_log(
                NativeLogLevel::Error,
                "floating-assistant.enable",
                &error.to_string(),
            );
            return Err(error);
        }
    } else if let Some(window) = app.get_webview_window(FLOATING_ASSISTANT_LABEL) {
        window.destroy().map_err(storage_error)?;
    }
    let saved = {
        let store = state.lock().map_err(storage_error)?;
        let conn = store.connection()?;
        save_to_conn(&conn, next)?
    };
    app.emit(
        "floating-assistant:event",
        FloatingAssistantEvent::ConfigurationChanged {
            config: saved.clone(),
        },
    )
    .map_err(storage_error)?;
    record_native_log(
        NativeLogLevel::Info,
        "floating-assistant.configuration",
        if enabled { "enabled" } else { "disabled" },
    );
    Ok(saved)
}

#[tauri::command]
pub(crate) fn save_floating_assistant_anchor(
    state: State<'_, Mutex<RegistryStore>>,
    anchor: FloatingAssistantAnchor,
) -> Result<FloatingAssistantConfig, AppError> {
    let store = state.lock().map_err(storage_error)?;
    let conn = store.connection()?;
    let current = load_from_conn(&conn)?;
    save_to_conn(
        &conn,
        FloatingAssistantConfig {
            enabled: current.enabled,
            anchor: Some(anchor),
        },
    )
}

#[tauri::command]
pub(crate) fn persist_floating_assistant_position(
    app: AppHandle,
    state: State<'_, Mutex<RegistryStore>>,
) -> Result<FloatingAssistantConfig, AppError> {
    let window = app
        .get_webview_window(FLOATING_ASSISTANT_LABEL)
        .ok_or_else(|| AppError::Storage("floating assistant window is unavailable".to_string()))?;
    let position = window.outer_position().map_err(storage_error)?;
    let size = window.outer_size().map_err(storage_error)?;
    let monitor_name = window
        .current_monitor()
        .map_err(storage_error)?
        .and_then(|monitor| monitor.name().cloned());
    save_floating_assistant_anchor(
        state,
        anchor_for_position(position, (size.width, size.height), monitor_name),
    )
}

#[tauri::command]
pub(crate) fn set_floating_assistant_surface(
    app: AppHandle,
    mode: FloatingAssistantSurfaceMode,
) -> Result<(), AppError> {
    let window = app
        .get_webview_window(FLOATING_ASSISTANT_LABEL)
        .ok_or_else(|| AppError::Storage("floating assistant window is unavailable".to_string()))?;
    let previous_position = window.outer_position().map_err(storage_error)?;
    let previous_size = window.outer_size().map_err(storage_error)?;
    let scale = window.scale_factor().map_err(storage_error)?;
    let (width, height) = mode.size();
    window
        .set_size(LogicalSize::new(width, height))
        .map_err(storage_error)?;
    let next_width = (width * scale).round() as i32;
    let next_height = (height * scale).round() as i32;
    let stable_anchor = anchor_for_position(
        previous_position,
        (previous_size.width, previous_size.height),
        None,
    );
    let requested = position_for_anchor(
        &stable_anchor,
        (next_width.max(0) as u32, next_height.max(0) as u32),
    );
    if let Some(monitor) = window.current_monitor().map_err(storage_error)? {
        let work_area = monitor.work_area();
        window
            .set_position(clamp_position(
                requested,
                (next_width.max(0) as u32, next_height.max(0) as u32),
                work_area.position,
                (work_area.size.width, work_area.size.height),
            ))
            .map_err(storage_error)?;
    }
    app.emit(
        "floating-assistant:event",
        FloatingAssistantEvent::SurfaceChanged { mode },
    )
    .map_err(storage_error)
}

#[tauri::command]
pub(crate) fn start_floating_assistant_drag(app: AppHandle) -> Result<(), AppError> {
    app.get_webview_window(FLOATING_ASSISTANT_LABEL)
        .ok_or_else(|| AppError::Storage("floating assistant window is unavailable".to_string()))?
        .start_dragging()
        .map_err(storage_error)
}

#[tauri::command]
pub(crate) fn show_main_window(app: AppHandle, action: String) -> Result<(), AppError> {
    if !valid_main_action(&action) {
        return Err(AppError::Validation(
            "invalid main-window action".to_string(),
        ));
    }
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::Storage("main window is unavailable".to_string()))?;
    window.show().map_err(storage_error)?;
    window.unminimize().map_err(storage_error)?;
    window.set_focus().map_err(storage_error)?;
    app.emit_to(
        "main",
        "floating-assistant:event",
        FloatingAssistantEvent::MainAction { action },
    )
    .map_err(storage_error)
}

#[tauri::command]
pub(crate) fn exit_application(app: AppHandle) {
    record_native_log(
        NativeLogLevel::Info,
        "floating-assistant.exit",
        "application exit requested",
    );
    app.exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::test_conn;

    #[test]
    fn defaults_disabled_and_round_trips_anchor() {
        let conn = test_conn();
        assert_eq!(
            load_from_conn(&conn).expect("defaults"),
            FloatingAssistantConfig {
                enabled: false,
                anchor: None,
            }
        );

        let saved = save_to_conn(
            &conn,
            FloatingAssistantConfig {
                enabled: true,
                anchor: Some(FloatingAssistantAnchor {
                    x: 1280.5,
                    y: 720.25,
                    monitor_name: Some("DISPLAY1".to_string()),
                }),
            },
        )
        .expect("save config");
        assert!(saved.enabled);
        assert_eq!(load_from_conn(&conn).expect("reload"), saved);
    }

    #[test]
    fn invalid_anchor_falls_back_to_none() {
        let conn = test_conn();
        let saved = save_to_conn(
            &conn,
            FloatingAssistantConfig {
                enabled: true,
                anchor: Some(FloatingAssistantAnchor {
                    x: f64::NAN,
                    y: 1.0,
                    monitor_name: None,
                }),
            },
        )
        .expect("save invalid anchor");
        assert_eq!(saved.anchor, None);
    }

    #[test]
    fn clamp_keeps_surface_inside_monitor_work_area() {
        assert_eq!(
            clamp_position(
                PhysicalPosition::new(5000, -50),
                (400, 600),
                PhysicalPosition::new(100, 200),
                (1920, 1080),
            ),
            PhysicalPosition::new(1600, 220),
        );
    }

    #[test]
    fn persisted_anchor_is_the_window_bottom_right_point() {
        let anchor = anchor_for_position(
            PhysicalPosition::new(1755, 868),
            (76, 76),
            Some("DISPLAY1".to_string()),
        );

        assert_eq!(anchor.x, 1831.0);
        assert_eq!(anchor.y, 944.0);
        assert_eq!(anchor.monitor_name.as_deref(), Some("DISPLAY1"));
        assert_eq!(
            position_for_anchor(&anchor, (76, 76)),
            PhysicalPosition::new(1755, 868)
        );
    }

    #[test]
    fn bottom_right_anchor_stays_stable_across_surface_sizes() {
        let anchor = anchor_for_position(PhysicalPosition::new(1000, 700), (76, 76), None);

        assert_eq!(
            position_for_anchor(&anchor, (304, 316)),
            PhysicalPosition::new(772, 460)
        );
        assert_eq!(
            position_for_anchor(&anchor, (408, 620)),
            PhysicalPosition::new(668, 156)
        );
        assert_eq!(
            position_for_anchor(&anchor, (76, 76)),
            PhysicalPosition::new(1000, 700)
        );
    }

    #[test]
    fn close_interception_requires_enabled_and_available_floating_window() {
        assert!(should_intercept_main_close(true, true));
        assert!(!should_intercept_main_close(false, true));
        assert!(!should_intercept_main_close(true, false));
    }

    #[test]
    fn main_action_allowlist_rejects_unknown_routes() {
        assert!(valid_main_action("new-session"));
        assert!(valid_main_action("current-session"));
        assert!(valid_main_action("settings"));
        assert!(!valid_main_action("close"));
    }
}
