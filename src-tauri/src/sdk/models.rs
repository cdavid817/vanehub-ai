use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SdkId {
    ClaudeSdk,
    CodexSdk,
}

impl SdkId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ClaudeSdk => "claude-sdk",
            Self::CodexSdk => "codex-sdk",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "claude-sdk" => Some(Self::ClaudeSdk),
            "codex-sdk" => Some(Self::CodexSdk),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkDefinition {
    pub id: SdkId,
    pub display_name: String,
    pub npm_package: String,
    pub companion_packages: Vec<String>,
    pub fallback_versions: Vec<String>,
    pub description: String,
    pub related_providers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SdkInstallStatus {
    Installed,
    NotInstalled,
    Installing,
    Uninstalling,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkStatus {
    pub id: SdkId,
    pub display_name: String,
    pub npm_package: String,
    pub description: String,
    pub related_providers: Vec<String>,
    pub status: SdkInstallStatus,
    pub installed_version: Option<String>,
    pub latest_version: Option<String>,
    pub has_update: bool,
    pub install_path: Option<String>,
    pub last_checked: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SdkVersionSource {
    Remote,
    Fallback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkVersionInfo {
    pub sdk_id: SdkId,
    pub versions: Vec<String>,
    pub fallback_versions: Vec<String>,
    pub source: SdkVersionSource,
    pub latest_version: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkEnvironmentStatus {
    pub available: bool,
    pub node_path: Option<String>,
    pub node_version: Option<String>,
    pub npm_path: Option<String>,
    pub npm_version: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SdkOperationType {
    Install,
    Update,
    Rollback,
    Uninstall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkOperationLog {
    pub sdk_id: SdkId,
    pub operation: SdkOperationType,
    pub line: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkOperationRequest {
    pub sdk_id: SdkId,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkOperationResult {
    pub success: bool,
    pub operation_id: Option<String>,
    pub sdk_id: SdkId,
    pub operation: SdkOperationType,
    pub installed_version: Option<String>,
    pub requested_version: Option<String>,
    pub logs: Vec<SdkOperationLog>,
    pub error: Option<String>,
}

pub type SdkStatusMap = BTreeMap<SdkId, SdkStatus>;
pub type SdkVersionMap = BTreeMap<SdkId, SdkVersionInfo>;
pub type SdkUpdateMap = BTreeMap<SdkId, SdkUpdateInfo>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkUpdateInfo {
    pub id: SdkId,
    pub latest_version: Option<String>,
    pub has_update: bool,
    pub error_message: Option<String>,
}
