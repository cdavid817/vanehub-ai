use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoggingPolicy {
    pub(crate) retention_days: i64,
    pub(crate) archive_enabled: bool,
    pub(crate) redaction_enabled: bool,
    pub(crate) levels: Vec<LogLevel>,
    pub(crate) can_open_directory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppSettings {
    pub(crate) application_language: String,
    pub(crate) font_size: String,
    pub(crate) theme: String,
    pub(crate) default_folder_path: String,
    pub(crate) log_directory: String,
    pub(crate) network_proxy_url: String,
    pub(crate) network_proxy_bypass: String,
    pub(crate) automatic_archival_enabled: bool,
    pub(crate) automatic_archival_inactive_days: i64,
    pub(crate) launch_on_startup: bool,
    pub(crate) logging_policy: LoggingPolicy,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveSettingInput {
    pub(crate) key: String,
    pub(crate) value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutomaticArchivalSettings {
    pub(crate) enabled: bool,
    pub(crate) inactive_days: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsStateEvent {
    pub(crate) kind: &'static str,
    pub(crate) key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DataManagementInfo {
    pub(crate) database_path: String,
    pub(crate) database_directory: String,
    pub(crate) can_open_directory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NodeInfo {
    pub(crate) available: bool,
    pub(crate) path: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestNetworkProxyInput {
    pub(crate) url: String,
    pub(crate) bypass: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NetworkProxyTestResult {
    pub(crate) success: bool,
    pub(crate) latency_ms: u64,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DetectedNetworkProxy {
    pub(crate) url: String,
    pub(crate) proxy_type: String,
    pub(crate) port: u16,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ClientLogEventKind {
    ErrorBoundary,
    CriticalOperationFailure,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClientLogEvent {
    pub(crate) level: LogLevel,
    pub(crate) kind: ClientLogEventKind,
    pub(crate) message: String,
    pub(crate) source: String,
    pub(crate) details: Option<BTreeMap<String, String>>,
    pub(crate) stack: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FloatingAssistantRuntimeInfo {
    pub(crate) native_available: bool,
    pub(crate) platform: String,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FloatingAssistantSurfaceMode {
    Collapsed,
    Menu,
    Chat,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum FloatingAssistantEvent {
    ConfigurationChanged { config: FloatingAssistantConfig },
    SurfaceChanged { mode: FloatingAssistantSurfaceMode },
    MainAction { action: String },
}
