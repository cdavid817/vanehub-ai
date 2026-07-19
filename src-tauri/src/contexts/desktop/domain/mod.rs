mod error;
mod floating_assistant;
mod lifecycle;
mod settings;

pub(crate) use error::DesktopSettingsDomainError;
pub(crate) use floating_assistant::{
    position_for_monitor, should_intercept_main_close, FloatingAssistantAnchor,
    FloatingAssistantConfig, FloatingAssistantDomainError, FloatingAssistantMainAction,
    FloatingAssistantPlatform, FloatingAssistantSurfaceMode, MonitorWorkArea, ScreenPosition,
    SurfaceSize, SurfaceTransition, WindowPlacement,
};
pub(crate) use lifecycle::should_hide_main_for_tray;
pub(crate) use settings::{
    AutomaticArchivalSettings, DesktopSettingKey, DesktopSettingMutation, DesktopSettings,
    NetworkProxyPreferences, StartupPreference,
};
