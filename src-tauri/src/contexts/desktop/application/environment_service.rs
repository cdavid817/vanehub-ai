use super::{
    ClientLogEvent, DataManagementInformation, DesktopClientLoggingPort, DesktopDirectoryPort,
    DesktopNetworkProxyActionsPort, DesktopNodeInfoPort, DesktopSettingsApplicationError,
    DetectedNetworkProxy, NetworkProxyTestResult, NodeInformation,
};
use crate::contexts::desktop::domain::NetworkProxyPreferences;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct DesktopEnvironmentApplicationService {
    directories: Arc<dyn DesktopDirectoryPort>,
    node: Arc<dyn DesktopNodeInfoPort>,
    proxy_actions: Arc<dyn DesktopNetworkProxyActionsPort>,
    client_logging: Arc<dyn DesktopClientLoggingPort>,
}

impl DesktopEnvironmentApplicationService {
    pub(crate) fn new(
        directories: Arc<dyn DesktopDirectoryPort>,
        node: Arc<dyn DesktopNodeInfoPort>,
        proxy_actions: Arc<dyn DesktopNetworkProxyActionsPort>,
        client_logging: Arc<dyn DesktopClientLoggingPort>,
    ) -> Self {
        Self {
            directories,
            node,
            proxy_actions,
            client_logging,
        }
    }

    pub(crate) fn data_management_info(
        &self,
    ) -> Result<DataManagementInformation, DesktopSettingsApplicationError> {
        self.directories.data_management_info()
    }

    pub(crate) fn open_database_directory(&self) -> Result<(), DesktopSettingsApplicationError> {
        self.directories.open_database_directory()
    }

    pub(crate) fn open_log_directory(
        &self,
        path: &str,
    ) -> Result<(), DesktopSettingsApplicationError> {
        self.directories.open_log_directory(path)
    }

    pub(crate) fn node_information(&self) -> NodeInformation {
        self.node.inspect()
    }

    pub(crate) async fn test_network_proxy(
        &self,
        url: String,
        bypass: String,
    ) -> Result<NetworkProxyTestResult, DesktopSettingsApplicationError> {
        let preferences = NetworkProxyPreferences::new(url, bypass)?;
        self.proxy_actions.test(&preferences).await
    }

    pub(crate) async fn scan_network_proxies(&self) -> Vec<DetectedNetworkProxy> {
        self.proxy_actions.scan().await
    }

    pub(crate) fn report_client_log(
        &self,
        log_directory: &str,
        event: ClientLogEvent,
    ) -> Result<(), DesktopSettingsApplicationError> {
        self.client_logging.record(log_directory, event)
    }
}
