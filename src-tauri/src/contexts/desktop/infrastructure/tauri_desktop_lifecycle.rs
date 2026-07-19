use super::runtime_logging::{
    record_exit_requested, record_runtime_error, record_shutdown_warning,
};
use super::tauri_floating_assistant_window::FLOATING_ASSISTANT_LABEL;
use crate::contexts::desktop::api::FloatingAssistantApi;
use crate::contexts::desktop::application::{
    DesktopLifecycleApplicationError, DesktopLifecyclePort, DesktopShutdownPort,
};
use crate::contexts::desktop::domain::should_hide_main_for_tray;
use crate::contexts::operations::application::DiagnosticLogPort;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Window, WindowEvent};
use tauri_plugin_dialog::DialogExt;

#[derive(Clone)]
pub(crate) struct TauriDesktopLifecycleAdapter {
    runtime: DesktopLifecycleRuntime,
}

impl TauriDesktopLifecycleAdapter {
    pub(crate) fn new(
        app: AppHandle,
        language: &str,
        shutdown: Arc<dyn DesktopShutdownPort>,
        logging: Arc<dyn DiagnosticLogPort>,
    ) -> Self {
        Self {
            runtime: DesktopLifecycleRuntime {
                inner: Arc::new(DesktopLifecycleRuntimeInner {
                    app,
                    shutdown,
                    logging,
                    tray_available: AtomicBool::new(false),
                    quitting: AtomicBool::new(false),
                    close_notice_shown: AtomicBool::new(false),
                    copy: TrayCopy::for_language(language),
                }),
            },
        }
    }
}

impl DesktopLifecyclePort for TauriDesktopLifecycleAdapter {
    fn initialize(&self) -> Result<(), DesktopLifecycleApplicationError> {
        self.runtime.initialize()
    }

    fn request_exit(&self) {
        self.runtime.request_exit();
    }
}

#[derive(Clone)]
struct DesktopLifecycleRuntime {
    inner: Arc<DesktopLifecycleRuntimeInner>,
}

struct DesktopLifecycleRuntimeInner {
    app: AppHandle,
    shutdown: Arc<dyn DesktopShutdownPort>,
    logging: Arc<dyn DiagnosticLogPort>,
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

impl DesktopLifecycleRuntime {
    fn initialize(&self) -> Result<(), DesktopLifecycleApplicationError> {
        self.inner.app.manage(self.clone());
        let show = MenuItem::with_id(
            &self.inner.app,
            "show",
            self.inner.copy.show,
            true,
            None::<&str>,
        )
        .map_err(|_| lifecycle_error("tray-menu-create-failed"))?;
        let hide = MenuItem::with_id(
            &self.inner.app,
            "hide",
            self.inner.copy.hide,
            true,
            None::<&str>,
        )
        .map_err(|_| lifecycle_error("tray-menu-create-failed"))?;
        let quit = MenuItem::with_id(
            &self.inner.app,
            "quit",
            self.inner.copy.quit,
            true,
            None::<&str>,
        )
        .map_err(|_| lifecycle_error("tray-menu-create-failed"))?;
        let menu = Menu::with_items(&self.inner.app, &[&show, &hide, &quit])
            .map_err(|_| lifecycle_error("tray-menu-create-failed"))?;
        let icon = self
            .inner
            .app
            .default_window_icon()
            .cloned()
            .ok_or_else(|| lifecycle_error("tray-icon-unavailable"))?;
        TrayIconBuilder::with_id("vanehub-main")
            .icon(icon)
            .tooltip("VaneHub AI")
            .menu(&menu)
            .on_menu_event(|app, event| {
                let runtime = app.state::<DesktopLifecycleRuntime>();
                match event.id().as_ref() {
                    "show" => runtime.show_main_window(),
                    "hide" => runtime.hide_main_window(),
                    "quit" => runtime.request_exit(),
                    _ => {}
                }
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
                    tray.app_handle()
                        .state::<DesktopLifecycleRuntime>()
                        .show_main_window();
                }
            })
            .build(&self.inner.app)
            .map_err(|_| lifecycle_error("tray-build-failed"))?;
        self.inner.tray_available.store(true, Ordering::Release);
        Ok(())
    }

    fn show_main_window(&self) {
        if let Some(window) = self.inner.app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }

    fn hide_main_window(&self) {
        if let Some(window) = self.inner.app.get_webview_window("main") {
            let _ = window.hide();
        }
    }

    fn request_exit(&self) {
        if self.inner.quitting.swap(true, Ordering::AcqRel) {
            return;
        }
        record_exit_requested(self.inner.logging.as_ref());
        let runtime = self.clone();
        tauri::async_runtime::spawn(async move {
            let completed =
                shutdown_with_timeout(runtime.inner.shutdown.as_ref(), Duration::from_secs(8))
                    .await;
            if !completed {
                record_shutdown_warning(runtime.inner.logging.as_ref());
            }
            runtime.inner.app.exit(0);
        });
    }

    fn intercept_for_tray(&self, window: &Window, event: &WindowEvent) {
        if !should_hide_main_for_tray(
            self.inner.tray_available.load(Ordering::Acquire),
            self.inner.quitting.load(Ordering::Acquire),
        ) {
            return;
        }
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
        }
        if let Err(error) = window.hide() {
            record_runtime_error(
                self.inner.logging.as_ref(),
                "desktop.lifecycle",
                "hide-main-for-tray",
                &error.to_string(),
            );
        }
        if !self.inner.close_notice_shown.swap(true, Ordering::AcqRel) {
            window
                .app_handle()
                .dialog()
                .message(self.inner.copy.notice)
                .title(self.inner.copy.notice_title)
                .show(|_| {});
        }
    }
}

fn lifecycle_error(code: &str) -> DesktopLifecycleApplicationError {
    DesktopLifecycleApplicationError::Runtime(code.to_string())
}

async fn shutdown_with_timeout(shutdown: &dyn DesktopShutdownPort, duration: Duration) -> bool {
    matches!(
        tokio::time::timeout(duration, shutdown.shutdown()).await,
        Ok(Ok(()))
    )
}

pub(crate) fn handle_main_window_event(window: &Window, event: &WindowEvent) {
    if window.label() != "main" || !matches!(event, WindowEvent::CloseRequested { .. }) {
        return;
    }
    let app = window.app_handle();
    let Some(runtime) = app.try_state::<DesktopLifecycleRuntime>() else {
        return;
    };
    if intercept_for_floating_assistant(window, event, &runtime) {
        return;
    }
    runtime.intercept_for_tray(window, event);
}

fn intercept_for_floating_assistant(
    main_window: &Window,
    event: &WindowEvent,
    runtime: &DesktopLifecycleRuntime,
) -> bool {
    let app = main_window.app_handle();
    let Some(api) = app.try_state::<FloatingAssistantApi>() else {
        return false;
    };
    let should_intercept = match api.should_hide_main_on_close() {
        Ok(value) => value,
        Err(error) => {
            record_runtime_error(
                runtime.inner.logging.as_ref(),
                "floating-assistant.hide-main",
                "load-visibility",
                &error.to_string(),
            );
            return false;
        }
    };
    if !should_intercept {
        return false;
    }
    let Some(floating_window) = app.get_webview_window(FLOATING_ASSISTANT_LABEL) else {
        record_runtime_error(
            runtime.inner.logging.as_ref(),
            "floating-assistant.hide-main",
            "resolve-window",
            "floating window missing; using desktop lifecycle fallback",
        );
        return false;
    };
    if let Err(error) = floating_window.show() {
        record_runtime_error(
            runtime.inner.logging.as_ref(),
            "floating-assistant.hide-main",
            "show-floating",
            &error.to_string(),
        );
        return false;
    }
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
    }
    if let Err(error) = main_window.hide() {
        record_runtime_error(
            runtime.inner.logging.as_ref(),
            "floating-assistant.hide-main",
            "hide-main",
            &error.to_string(),
        );
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct ImmediateShutdown(Result<(), String>);

    #[async_trait]
    impl DesktopShutdownPort for ImmediateShutdown {
        async fn shutdown(&self) -> Result<(), String> {
            self.0.clone()
        }
    }

    struct DelayedShutdown;

    #[async_trait]
    impl DesktopShutdownPort for DelayedShutdown {
        async fn shutdown(&self) -> Result<(), String> {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok(())
        }
    }

    #[test]
    fn tray_copy_is_localized() {
        assert_eq!(TrayCopy::for_language("en").quit, "Quit");
        assert_eq!(TrayCopy::for_language("zh-CN").quit, "退出");
    }

    #[tokio::test]
    async fn graceful_shutdown_distinguishes_success_failure_and_timeout() {
        assert!(shutdown_with_timeout(&ImmediateShutdown(Ok(())), Duration::from_millis(20)).await);
        assert!(
            !shutdown_with_timeout(
                &ImmediateShutdown(Err("connector failed".to_string())),
                Duration::from_millis(20)
            )
            .await
        );
        assert!(!shutdown_with_timeout(&DelayedShutdown, Duration::from_millis(1)).await);
    }
}
