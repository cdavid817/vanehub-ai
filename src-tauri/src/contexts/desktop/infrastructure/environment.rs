use crate::contexts::desktop::application::{
    ClientLogEvent, ClientLogEventKind, DataManagementInformation, DesktopClientLoggingPort,
    DesktopDirectoryPort, DesktopLogLevel, DesktopNetworkProxyActionsPort, DesktopNodeInfoPort,
    DesktopSettingsApplicationError, DetectedNetworkProxy, NetworkProxyTestResult, NodeInformation,
};
use crate::contexts::desktop::domain::NetworkProxyPreferences;
use crate::platform::database::NativeDatabase;
use crate::platform::{network, process};
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct DesktopDirectoryAdapter {
    database: NativeDatabase,
}

impl DesktopDirectoryAdapter {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn database_directory(&self) -> Result<PathBuf, DesktopSettingsApplicationError> {
        self.database
            .db_path
            .parent()
            .map(PathBuf::from)
            .ok_or_else(|| {
                DesktopSettingsApplicationError::Directory(
                    "storage error: Database path has no containing directory.".to_string(),
                )
            })
    }
}

impl DesktopDirectoryPort for DesktopDirectoryAdapter {
    fn data_management_info(
        &self,
    ) -> Result<DataManagementInformation, DesktopSettingsApplicationError> {
        let database_directory = self.database_directory()?;
        Ok(DataManagementInformation {
            database_path: self.database.db_path.to_string_lossy().to_string(),
            database_directory: database_directory.to_string_lossy().to_string(),
            can_open_directory: database_directory.is_dir(),
        })
    }

    fn open_database_directory(&self) -> Result<(), DesktopSettingsApplicationError> {
        crate::platform::logging::open_directory(&self.database_directory()?)
            .map_err(|error| DesktopSettingsApplicationError::Directory(error.to_string()))
    }

    fn open_log_directory(&self, path: &str) -> Result<(), DesktopSettingsApplicationError> {
        crate::platform::logging::open_directory(&PathBuf::from(path))
            .map_err(|error| DesktopSettingsApplicationError::Directory(error.to_string()))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PlatformNodeInfoAdapter;

impl DesktopNodeInfoPort for PlatformNodeInfoAdapter {
    fn inspect(&self) -> NodeInformation {
        let version = command_output("node", &["--version"]);
        let resolver = if cfg!(windows) { "where" } else { "which" };
        let path = command_output(resolver, &["node"]).and_then(first_output_line);
        node_information(path, version)
    }
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let request = process::ProcessRequest::new(program)
        .args(args.iter().copied())
        .timeout(Duration::from_secs(5));
    let output = process::ProcessAdapter.execute(&request).ok()?;
    output
        .success()
        .then_some(output.stdout)
        .filter(|value| !value.trim().is_empty())
}

fn first_output_line(output: String) -> Option<String> {
    output
        .lines()
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn node_information(path: Option<String>, version: Option<String>) -> NodeInformation {
    let available = path.is_some() && version.is_some();
    NodeInformation {
        available,
        path,
        version,
        reason: (!available)
            .then(|| "Node.js executable or version could not be resolved.".to_string()),
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RuntimeNetworkProxyActionsAdapter;

#[async_trait]
impl DesktopNetworkProxyActionsPort for RuntimeNetworkProxyActionsAdapter {
    async fn test(
        &self,
        preferences: &NetworkProxyPreferences,
    ) -> Result<NetworkProxyTestResult, DesktopSettingsApplicationError> {
        network::test_proxy(network::TestNetworkProxyInput {
            url: preferences.url().to_string(),
            bypass: preferences.bypass().to_string(),
        })
        .await
        .map(proxy_test_result)
        .map_err(|error| DesktopSettingsApplicationError::NetworkProxy(error.to_string()))
    }

    async fn scan(&self) -> Vec<DetectedNetworkProxy> {
        network::scan_local()
            .await
            .into_iter()
            .map(detected_proxy)
            .collect()
    }
}

fn proxy_test_result(value: network::NetworkProxyTestResult) -> NetworkProxyTestResult {
    NetworkProxyTestResult {
        success: value.success,
        latency_ms: value.latency_ms,
        error: value.error,
    }
}

fn detected_proxy(value: network::DetectedNetworkProxy) -> DetectedNetworkProxy {
    DetectedNetworkProxy {
        url: value.url,
        proxy_type: value.proxy_type,
        port: value.port,
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct UnifiedClientLoggingAdapter;

impl DesktopClientLoggingPort for UnifiedClientLoggingAdapter {
    fn record(
        &self,
        log_directory: &str,
        event: ClientLogEvent,
    ) -> Result<(), DesktopSettingsApplicationError> {
        crate::platform::logging::write_client_event(
            &PathBuf::from(log_directory),
            crate::platform::logging::ClientLogEvent {
                level: log_level(event.level),
                kind: log_kind(event.kind),
                message: event.message,
                source: event.source,
                details: event.details,
                stack: event.stack,
            },
        )
        .map_err(|error| DesktopSettingsApplicationError::ClientLogging(error.to_string()))
    }
}

fn log_level(level: DesktopLogLevel) -> crate::platform::logging::LogLevel {
    match level {
        DesktopLogLevel::Error => crate::platform::logging::LogLevel::Error,
        DesktopLogLevel::Warn => crate::platform::logging::LogLevel::Warn,
        DesktopLogLevel::Info => crate::platform::logging::LogLevel::Info,
        DesktopLogLevel::Debug => crate::platform::logging::LogLevel::Debug,
    }
}

fn log_kind(kind: ClientLogEventKind) -> crate::platform::logging::ClientLogEventKind {
    match kind {
        ClientLogEventKind::ErrorBoundary => {
            crate::platform::logging::ClientLogEventKind::ErrorBoundary
        }
        ClientLogEventKind::CriticalOperationFailure => {
            crate::platform::logging::ClientLogEventKind::CriticalOperationFailure
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::logging::LOG_FILE_NAME;
    use crate::test_support::TempDirectory;
    use std::collections::BTreeMap;

    #[test]
    fn data_management_information_uses_the_existing_database_contract() {
        let directory = TempDirectory::new("desktop-data-information");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let adapter = DesktopDirectoryAdapter::new(database.clone());

        let information = adapter.data_management_info().expect("information");

        assert_eq!(
            information.database_path,
            database.db_path.to_string_lossy()
        );
        assert_eq!(
            information.database_directory,
            directory.path().to_string_lossy()
        );
        assert!(information.can_open_directory);
    }

    #[test]
    fn node_mapping_requires_both_path_and_version() {
        let available = node_information(
            Some("C:/Program Files/nodejs/node.exe".to_string()),
            Some("v22.0.0".to_string()),
        );
        assert!(available.available);
        assert_eq!(available.reason, None);

        let unavailable = node_information(None, Some("v22.0.0".to_string()));
        assert!(!unavailable.available);
        assert_eq!(
            unavailable.reason.as_deref(),
            Some("Node.js executable or version could not be resolved.")
        );
    }

    #[test]
    fn client_logging_adapter_keeps_unified_redaction() {
        let directory = TempDirectory::new("desktop-client-logging");
        let mut details = BTreeMap::new();
        details.insert("token".to_string(), "fixture-secret".to_string());

        UnifiedClientLoggingAdapter
            .record(
                &directory.path().to_string_lossy(),
                ClientLogEvent {
                    level: DesktopLogLevel::Error,
                    kind: ClientLogEventKind::ErrorBoundary,
                    message: "password=message-secret".to_string(),
                    source: "desktop-test".to_string(),
                    details: Some(details),
                    stack: None,
                },
            )
            .expect("client log");

        let raw = std::fs::read_to_string(directory.path().join(LOG_FILE_NAME)).expect("log file");
        assert!(raw.contains("frontend.client"));
        assert!(raw.contains("[REDACTED]"));
        assert!(!raw.contains("fixture-secret"));
        assert!(!raw.contains("message-secret"));
    }

    #[test]
    fn proxy_result_mappers_preserve_serialized_fields() {
        let result = proxy_test_result(network::NetworkProxyTestResult {
            success: true,
            latency_ms: 42,
            error: Some("fixture".to_string()),
        });
        let proxy = detected_proxy(network::DetectedNetworkProxy {
            url: "socks5://127.0.0.1:7891".to_string(),
            proxy_type: "socks5".to_string(),
            port: 7891,
        });

        assert_eq!(result.latency_ms, 42);
        assert_eq!(result.error.as_deref(), Some("fixture"));
        assert_eq!(proxy.url, "socks5://127.0.0.1:7891");
        assert_eq!(proxy.proxy_type, "socks5");
        assert_eq!(proxy.port, 7891);
    }
}
