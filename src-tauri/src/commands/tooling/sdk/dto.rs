use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SdkId {
    ClaudeSdk,
    CodexSdk,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SdkDefinition {
    pub(crate) id: SdkId,
    pub(crate) display_name: String,
    pub(crate) npm_package: String,
    pub(crate) companion_packages: Vec<String>,
    pub(crate) fallback_versions: Vec<String>,
    pub(crate) description: String,
    pub(crate) related_providers: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SdkInstallStatus {
    Installed,
    NotInstalled,
    Installing,
    Uninstalling,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SdkStatus {
    pub(crate) id: SdkId,
    pub(crate) display_name: String,
    pub(crate) npm_package: String,
    pub(crate) description: String,
    pub(crate) related_providers: Vec<String>,
    pub(crate) status: SdkInstallStatus,
    pub(crate) installed_version: Option<String>,
    pub(crate) latest_version: Option<String>,
    pub(crate) has_update: bool,
    pub(crate) install_path: Option<String>,
    pub(crate) last_checked: Option<String>,
    pub(crate) error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SdkVersionSource {
    Remote,
    Fallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SdkVersionInfo {
    pub(crate) sdk_id: SdkId,
    pub(crate) versions: Vec<String>,
    pub(crate) fallback_versions: Vec<String>,
    pub(crate) source: SdkVersionSource,
    pub(crate) latest_version: Option<String>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SdkEnvironmentStatus {
    pub(crate) available: bool,
    pub(crate) node_path: Option<String>,
    pub(crate) node_version: Option<String>,
    pub(crate) npm_path: Option<String>,
    pub(crate) npm_version: Option<String>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SdkOperationType {
    Install,
    Update,
    Rollback,
    Uninstall,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SdkOperationLog {
    pub(crate) sdk_id: SdkId,
    pub(crate) operation: SdkOperationType,
    pub(crate) line: String,
    pub(crate) timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SdkOperationRequest {
    pub(crate) sdk_id: SdkId,
    pub(crate) version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SdkUpdateInfo {
    pub(crate) id: SdkId,
    pub(crate) latest_version: Option<String>,
    pub(crate) has_update: bool,
    pub(crate) error_message: Option<String>,
}

pub(crate) type SdkStatusMap = BTreeMap<SdkId, SdkStatus>;
pub(crate) type SdkVersionMap = BTreeMap<SdkId, SdkVersionInfo>;
pub(crate) type SdkUpdateMap = BTreeMap<SdkId, SdkUpdateInfo>;
