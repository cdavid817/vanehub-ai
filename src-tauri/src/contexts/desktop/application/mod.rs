mod environment_service;
mod error;
mod floating_assistant;
mod lifecycle;
mod models;
mod ports;
mod service;

pub(crate) use environment_service::DesktopEnvironmentApplicationService;
pub(crate) use error::DesktopSettingsApplicationError;
pub(crate) use floating_assistant::{
    FloatingAssistantApplicationError, FloatingAssistantApplicationService,
    FloatingAssistantRepository, FloatingAssistantWindowPort,
};
pub(crate) use lifecycle::{
    DesktopLifecycleApplicationError, DesktopLifecycleApplicationService, DesktopLifecyclePort,
    DesktopShutdownPort,
};
pub(crate) use models::{
    ClientLogEvent, ClientLogEventKind, DataManagementInformation, DesktopLogLevel,
    DesktopLoggingPolicy, DesktopSettingsView, DetectedNetworkProxy, NetworkProxyTestResult,
    NodeInformation, StoredDesktopSetting,
};
pub(crate) use ports::{
    DesktopClientLoggingPort, DesktopClockPort, DesktopDirectoryPort, DesktopLogDirectoryPort,
    DesktopNetworkProxyActionsPort, DesktopNetworkProxyPort, DesktopNodeInfoPort,
    DesktopSettingsRepository, DesktopStartupPort,
};
pub(crate) use service::DesktopSettingsApplicationService;

#[cfg(test)]
mod tests;
