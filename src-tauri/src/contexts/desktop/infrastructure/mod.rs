mod environment;
mod folder_openers;
mod locale_bridge;
mod runtime_logging;
mod runtime_settings;
mod sqlite_floating_assistant_repository;
mod sqlite_settings_repository;
mod tauri_desktop_lifecycle;
mod tauri_floating_assistant_window;
mod webview_recovery;

pub(crate) use environment::{
    DesktopDirectoryAdapter, PlatformNodeInfoAdapter, RuntimeNetworkProxyActionsAdapter,
    UnifiedClientLoggingAdapter,
};
pub(crate) use folder_openers::{
    FolderOpenerAvailability, FolderOpenerId, FolderOpenerPreferencesView, FolderOpenerService,
    OpenSessionFolderResult, SaveFolderOpenerPreferences,
};
pub(crate) use locale_bridge::DesktopLocaleBridge;
pub(crate) use runtime_settings::{
    RuntimeLogDirectoryAdapter, RuntimeNetworkProxyAdapter, SystemDesktopClock,
    TauriDesktopStartupAdapter,
};
pub(crate) use sqlite_floating_assistant_repository::{
    apply_schema as apply_floating_assistant_schema, SqliteFloatingAssistantRepository,
};
pub(crate) use sqlite_settings_repository::SqliteDesktopSettingsRepository;
pub(crate) use tauri_desktop_lifecycle::{handle_main_window_event, TauriDesktopLifecycleAdapter};
pub(crate) use tauri_floating_assistant_window::TauriFloatingAssistantWindowAdapter;
pub(crate) use webview_recovery::install_main_webview_recovery;
