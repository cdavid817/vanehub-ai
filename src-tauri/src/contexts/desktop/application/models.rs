use crate::contexts::desktop::domain::{DesktopSettingKey, DesktopSettings};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredDesktopSetting {
    pub(crate) key: DesktopSettingKey,
    pub(crate) value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DesktopLogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopLoggingPolicy {
    pub(crate) retention_days: i64,
    pub(crate) archive_enabled: bool,
    pub(crate) redaction_enabled: bool,
    pub(crate) levels: Vec<DesktopLogLevel>,
    pub(crate) can_open_directory: bool,
}

impl DesktopLoggingPolicy {
    pub(crate) fn native() -> Self {
        Self {
            retention_days: 30,
            archive_enabled: true,
            redaction_enabled: true,
            levels: vec![
                DesktopLogLevel::Error,
                DesktopLogLevel::Warn,
                DesktopLogLevel::Info,
                DesktopLogLevel::Debug,
            ],
            can_open_directory: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopSettingsView {
    pub(crate) settings: DesktopSettings,
    pub(crate) logging_policy: DesktopLoggingPolicy,
    /// The revision the personalization fields in `settings` were read at.
    ///
    /// `None` while the dedicated policy is unreachable, in which case those fields are whatever
    /// the legacy rows still hold and no personalization save will be accepted. Carried on the view
    /// because the settings page has no version of its own, and echoing this back on save is what
    /// makes its concurrency check real rather than a re-read at write time.
    pub(crate) personalization_revision: Option<u64>,
}

impl DesktopSettingsView {
    pub(crate) fn native(settings: DesktopSettings) -> Self {
        Self {
            settings,
            logging_policy: DesktopLoggingPolicy::native(),
            personalization_revision: None,
        }
    }

    pub(crate) fn with_personalization_revision(mut self, revision: u64) -> Self {
        self.personalization_revision = Some(revision);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DataManagementInformation {
    pub(crate) database_path: String,
    pub(crate) database_directory: String,
    pub(crate) can_open_directory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NodeInformation {
    pub(crate) available: bool,
    pub(crate) path: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NetworkProxyTestResult {
    pub(crate) success: bool,
    pub(crate) latency_ms: u64,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DetectedNetworkProxy {
    pub(crate) url: String,
    pub(crate) proxy_type: String,
    pub(crate) port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClientLogEventKind {
    ErrorBoundary,
    CriticalOperationFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClientLogEvent {
    pub(crate) level: DesktopLogLevel,
    pub(crate) kind: ClientLogEventKind,
    pub(crate) message: String,
    pub(crate) source: String,
    pub(crate) details: Option<BTreeMap<String, String>>,
    pub(crate) stack: Option<String>,
}
