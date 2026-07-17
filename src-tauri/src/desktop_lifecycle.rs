use crate::im::runtime::ImRuntimeManager;
use crate::{fallback_log_dir, logging};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager, Window, WindowEvent};
use tauri_plugin_dialog::DialogExt;

pub struct DesktopLifecycleState {
    tray_available: AtomicBool,
    quitting: AtomicBool,
    close_notice_shown: AtomicBool,
    copy: TrayCopy,
}

#[derive(Clone)]
struct TrayCopy {
    show: &'static str,
    hide: &'static str,
    quit: &'static str,
    notice_title: &'static str,
    notice: &'static str,
}

impl TrayCopy {
    fn for_language(language: &str) -> Self {
        if language == "en" {
            Self {
                show: "Show VaneHub AI",
                hide: "Hide VaneHub AI",
                quit: "Quit",
                notice_title: "VaneHub AI is still running",
                notice: "VaneHub AI will keep receiving IM messages in the background. Use the system tray to restore or quit.",
            }
        } else {
            Self {
                show: "显示 VaneHub AI",
                hide: "隐藏 VaneHub AI",
                quit: "退出",
                notice_title: "VaneHub AI 仍在运行",
                notice: "VaneHub AI 将在后台继续接收 IM 消息。可通过系统托盘恢复窗口或退出。",
            }
        }
    }
}

impl DesktopLifecycleState {
    fn new(language: &str) -> Self {
        Self {
            tray_available: AtomicBool::new(false),
            quitting: AtomicBool::new(false),
            close_notice_shown: AtomicBool::new(false),
            copy: TrayCopy::for_language(language),
        }
    }
}

pub fn initialize(app: &mut App, language: &str) -> Result<(), String> {
    app.manage(DesktopLifecycleState::new(language));
    let show = MenuItem::with_id(
        app,
        "show",
        TrayCopy::for_language(language).show,
        true,
        None::<&str>,
    )
    .map_err(|_| "tray-menu-create-failed".to_string())?;
    let hide = MenuItem::with_id(
        app,
        "hide",
        TrayCopy::for_language(language).hide,
        true,
        None::<&str>,
    )
    .map_err(|_| "tray-menu-create-failed".to_string())?;
    let quit = MenuItem::with_id(
        app,
        "quit",
        TrayCopy::for_language(language).quit,
        true,
        None::<&str>,
    )
    .map_err(|_| "tray-menu-create-failed".to_string())?;
    let menu = Menu::with_items(app, &[&show, &hide, &quit])
        .map_err(|_| "tray-menu-create-failed".to_string())?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| "tray-icon-unavailable".to_string())?;
    TrayIconBuilder::with_id("vanehub-main")
        .icon(icon)
        .tooltip("VaneHub AI")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "hide" => hide_main_window(app),
            "quit" => request_quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|_| "tray-build-failed".to_string())?;
    app.state::<DesktopLifecycleState>()
        .tray_available
        .store(true, Ordering::Release);
    Ok(())
}

pub fn handle_window_event(window: &Window, event: &WindowEvent) {
    if !matches!(event, WindowEvent::CloseRequested { .. }) {
        return;
    }
    let state = window.state::<DesktopLifecycleState>();
    if !should_hide_on_close(
        state.tray_available.load(Ordering::Acquire),
        state.quitting.load(Ordering::Acquire),
    ) {
        return;
    }
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
    }
    let _ = window.hide();
    if !state.close_notice_shown.swap(true, Ordering::AcqRel) {
        window
            .app_handle()
            .dialog()
            .message(state.copy.notice)
            .title(state.copy.notice_title)
            .show(|_| {});
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

pub(crate) fn request_quit(app: &AppHandle) {
    let state = app.state::<DesktopLifecycleState>();
    if state.quitting.swap(true, Ordering::AcqRel) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let runtime = app.state::<Arc<ImRuntimeManager>>().inner().clone();
        let result = tokio::time::timeout(Duration::from_secs(8), runtime.shutdown()).await;
        if !matches!(result, Ok(Ok(()))) {
            let mut context = BTreeMap::new();
            context.insert("operation".to_string(), "explicit-quit".to_string());
            let _ = logging::write_message(
                &fallback_log_dir(),
                logging::LogLevel::Warn,
                "desktop.lifecycle",
                "Connector shutdown exceeded its graceful boundary",
                context,
            );
        }
        app.exit(0);
    });
}

pub fn should_hide_on_close(tray_available: bool, quitting: bool) -> bool {
    tray_available && !quitting
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_interception_requires_a_working_tray_and_non_quit_state() {
        assert!(should_hide_on_close(true, false));
        assert!(!should_hide_on_close(false, false));
        assert!(!should_hide_on_close(true, true));
    }

    #[test]
    fn tray_copy_is_localized() {
        assert_eq!(TrayCopy::for_language("en").quit, "Quit");
        assert_eq!(TrayCopy::for_language("zh-CN").quit, "退出");
    }
}
