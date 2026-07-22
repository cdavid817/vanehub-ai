use super::runtime_logging::record_runtime_error;
use crate::contexts::desktop::application::{
    FloatingAssistantApplicationError, FloatingAssistantWindowPort,
};
use crate::contexts::desktop::domain::{
    position_for_monitor, FloatingAssistantConfig, FloatingAssistantPlatform, MonitorWorkArea,
    ScreenPosition, SurfaceSize, SurfaceTransition, WindowPlacement,
};
use crate::contexts::operations::application::DiagnosticLogPort;
use std::sync::Arc;
use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

pub(crate) const FLOATING_ASSISTANT_LABEL: &str = "floating-assistant";

#[derive(Clone)]
pub(crate) struct TauriFloatingAssistantWindowAdapter {
    app: AppHandle,
    logging: Arc<dyn DiagnosticLogPort>,
}

impl TauriFloatingAssistantWindowAdapter {
    pub(crate) fn new(app: AppHandle, logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self { app, logging }
    }

    fn window(&self) -> Result<WebviewWindow, FloatingAssistantApplicationError> {
        self.app
            .get_webview_window(FLOATING_ASSISTANT_LABEL)
            .ok_or_else(|| {
                FloatingAssistantApplicationError::Window(
                    "floating assistant window is unavailable".to_string(),
                )
            })
    }

    fn runtime_error(
        &self,
        operation: &str,
        error: impl std::fmt::Display,
    ) -> FloatingAssistantApplicationError {
        let message = error.to_string();
        record_runtime_error(
            self.logging.as_ref(),
            "floating-assistant.window",
            operation,
            &message,
        );
        FloatingAssistantApplicationError::Window(message)
    }

    fn monitor_for_config(
        &self,
        config: &FloatingAssistantConfig,
    ) -> Result<Option<tauri::Monitor>, FloatingAssistantApplicationError> {
        let monitors = self
            .app
            .available_monitors()
            .map_err(|error| self.runtime_error("available-monitors", error))?;
        if let Some(name) = config.anchor().and_then(|anchor| anchor.monitor_name()) {
            if let Some(monitor) = monitors
                .iter()
                .find(|monitor| monitor.name().is_some_and(|candidate| candidate == name))
            {
                return Ok(Some(monitor.clone()));
            }
        }
        self.app
            .primary_monitor()
            .map_err(|error| self.runtime_error("primary-monitor", error))
    }

    fn position_window(
        &self,
        window: &WebviewWindow,
        config: &FloatingAssistantConfig,
    ) -> Result<(), FloatingAssistantApplicationError> {
        let Some(monitor) = self.monitor_for_config(config)? else {
            return Ok(());
        };
        let size = window
            .outer_size()
            .map_err(|error| self.runtime_error("outer-size", error))?;
        let work_area = monitor_work_area(&monitor);
        let position = position_for_monitor(
            config.anchor(),
            SurfaceSize {
                width: size.width,
                height: size.height,
            },
            &work_area,
        );
        window
            .set_position(PhysicalPosition::new(position.x, position.y))
            .map_err(|error| self.runtime_error("position", error))
    }
}

impl FloatingAssistantWindowPort for TauriFloatingAssistantWindowAdapter {
    fn platform(&self) -> FloatingAssistantPlatform {
        if cfg!(target_os = "windows") {
            FloatingAssistantPlatform::Windows
        } else {
            FloatingAssistantPlatform::Unsupported
        }
    }

    fn ensure(
        &self,
        config: &FloatingAssistantConfig,
    ) -> Result<(), FloatingAssistantApplicationError> {
        if let Some(window) = self.app.get_webview_window(FLOATING_ASSISTANT_LABEL) {
            if config.enabled() {
                window
                    .show()
                    .map_err(|error| self.runtime_error("show", error))?;
            }
            return Ok(());
        }
        let (width, height) =
            crate::contexts::desktop::domain::FloatingAssistantSurfaceMode::Collapsed
                .logical_size();
        let builder = WebviewWindowBuilder::new(
            &self.app,
            FLOATING_ASSISTANT_LABEL,
            WebviewUrl::App("index.html?surface=floating-assistant".into()),
        )
        .title("VaneHub AI")
        .inner_size(width, height)
        .decorations(false);
        // The transparent native surface is Windows-only; macOS does not expose this builder API.
        #[cfg(target_os = "windows")]
        let builder = builder.transparent(true);
        let window = builder
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .visible(false)
            .build()
            .map_err(|error| self.runtime_error("build", error))?;
        window
            .set_decorations(false)
            .map_err(|error| self.runtime_error("decorations", error))?;
        window
            .set_always_on_top(true)
            .map_err(|error| self.runtime_error("always-on-top", error))?;
        window
            .set_skip_taskbar(true)
            .map_err(|error| self.runtime_error("skip-taskbar", error))?;
        window
            .set_resizable(false)
            .map_err(|error| self.runtime_error("resizable", error))?;
        window
            .set_shadow(false)
            .map_err(|error| self.runtime_error("shadow", error))?;
        window
            .set_size(LogicalSize::new(width, height))
            .map_err(|error| self.runtime_error("size", error))?;
        self.position_window(&window, config)?;
        if config.enabled() {
            window
                .show()
                .map_err(|error| self.runtime_error("show", error))?;
        }
        Ok(())
    }

    fn destroy(&self) -> Result<(), FloatingAssistantApplicationError> {
        if let Some(window) = self.app.get_webview_window(FLOATING_ASSISTANT_LABEL) {
            window
                .destroy()
                .map_err(|error| self.runtime_error("destroy", error))?;
        }
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.app
            .get_webview_window(FLOATING_ASSISTANT_LABEL)
            .is_some()
    }

    fn placement(&self) -> Result<WindowPlacement, FloatingAssistantApplicationError> {
        let window = self.window()?;
        let position = window
            .outer_position()
            .map_err(|error| self.runtime_error("outer-position", error))?;
        let size = window
            .outer_size()
            .map_err(|error| self.runtime_error("outer-size", error))?;
        let scale_factor = window
            .scale_factor()
            .map_err(|error| self.runtime_error("scale-factor", error))?;
        let monitor = window
            .current_monitor()
            .map_err(|error| self.runtime_error("current-monitor", error))?
            .as_ref()
            .map(monitor_work_area);
        Ok(WindowPlacement {
            position: ScreenPosition {
                x: position.x,
                y: position.y,
            },
            size: SurfaceSize {
                width: size.width,
                height: size.height,
            },
            scale_factor,
            monitor,
        })
    }

    fn apply_surface(
        &self,
        transition: &SurfaceTransition,
    ) -> Result<(), FloatingAssistantApplicationError> {
        let window = self.window()?;
        window
            .set_size(LogicalSize::new(
                f64::from(transition.logical_width),
                f64::from(transition.logical_height),
            ))
            .map_err(|error| self.runtime_error("surface-size", error))?;
        if transition.should_reposition {
            window
                .set_position(PhysicalPosition::new(
                    transition.position.x,
                    transition.position.y,
                ))
                .map_err(|error| self.runtime_error("surface-position", error))?;
        }
        Ok(())
    }

    fn start_dragging(&self) -> Result<(), FloatingAssistantApplicationError> {
        self.window()?
            .start_dragging()
            .map_err(|error| self.runtime_error("start-dragging", error))
    }

    fn show_main_window(&self) -> Result<(), FloatingAssistantApplicationError> {
        let window = self.app.get_webview_window("main").ok_or_else(|| {
            FloatingAssistantApplicationError::Window("main window is unavailable".to_string())
        })?;
        window
            .show()
            .map_err(|error| self.runtime_error("show-main", error))?;
        window
            .unminimize()
            .map_err(|error| self.runtime_error("unminimize-main", error))?;
        window
            .set_focus()
            .map_err(|error| self.runtime_error("focus-main", error))
    }
}

fn monitor_work_area(monitor: &tauri::Monitor) -> MonitorWorkArea {
    let work_area = monitor.work_area();
    MonitorWorkArea {
        position: ScreenPosition {
            x: work_area.position.x,
            y: work_area.position.y,
        },
        size: SurfaceSize {
            width: work_area.size.width,
            height: work_area.size.height,
        },
        name: monitor.name().cloned(),
    }
}
