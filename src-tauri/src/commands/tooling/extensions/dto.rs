use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ExtensionCapabilityId {
    Ocr,
    Asr,
    Tts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ExtensionFrameworkId {
    Paddleocr,
    FasterWhisper,
    SherpaOnnx,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionModelRequirement {
    pub(crate) id: String,
    pub(crate) size_mb: u64,
    pub(crate) description_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionRequirement {
    pub(crate) runtime: String,
    pub(crate) packages: Vec<String>,
    pub(crate) estimated_download_mb: u64,
    pub(crate) estimated_disk_mb: u64,
    pub(crate) models: Vec<ExtensionModelRequirement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionFrameworkDefinition {
    pub(crate) id: ExtensionFrameworkId,
    pub(crate) capability_id: ExtensionCapabilityId,
    pub(crate) name_key: String,
    pub(crate) description_key: String,
    pub(crate) default_port: u16,
    pub(crate) requirement: ExtensionRequirement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ExtensionLifecycleStatus {
    NotInstalled,
    Installing,
    Installed,
    Starting,
    Running,
    Stopping,
    Uninstalling,
    Error,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionEnvironment {
    pub(crate) runtime: String,
    pub(crate) os: String,
    pub(crate) arch: String,
    pub(crate) supported: bool,
    pub(crate) native_operations_available: bool,
    pub(crate) python_path: Option<String>,
    pub(crate) python_version: Option<String>,
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionFrameworkStatus {
    pub(crate) framework_id: ExtensionFrameworkId,
    pub(crate) capability_id: ExtensionCapabilityId,
    pub(crate) status: ExtensionLifecycleStatus,
    pub(crate) installed: bool,
    pub(crate) enabled: bool,
    pub(crate) running: bool,
    pub(crate) port: u16,
    pub(crate) install_path: Option<String>,
    pub(crate) installed_version: Option<String>,
    pub(crate) last_health_check: Option<String>,
    pub(crate) last_error: Option<String>,
    pub(crate) last_operation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionOverview {
    pub(crate) definitions: Vec<ExtensionFrameworkDefinition>,
    pub(crate) statuses: Vec<ExtensionFrameworkStatus>,
    pub(crate) environment: ExtensionEnvironment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionInstallPreview {
    pub(crate) framework_id: ExtensionFrameworkId,
    pub(crate) supported: bool,
    pub(crate) install_path: String,
    pub(crate) python_path: Option<String>,
    pub(crate) packages: Vec<String>,
    pub(crate) models: Vec<ExtensionModelRequirement>,
    pub(crate) estimated_download_mb: u64,
    pub(crate) estimated_disk_mb: u64,
    pub(crate) inference_local_only: bool,
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionFrameworkRequest {
    pub(crate) framework_id: ExtensionFrameworkId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionEnableRequest {
    pub(crate) framework_id: ExtensionFrameworkId,
    pub(crate) enabled: bool,
}
