use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExtensionCapabilityId {
    Ocr,
    Asr,
    Tts,
}

impl ExtensionCapabilityId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ocr => "ocr",
            Self::Asr => "asr",
            Self::Tts => "tts",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExtensionFrameworkId {
    Paddleocr,
    FasterWhisper,
    SherpaOnnx,
}

impl ExtensionFrameworkId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Paddleocr => "paddleocr",
            Self::FasterWhisper => "faster-whisper",
            Self::SherpaOnnx => "sherpa-onnx",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionModelRequirement {
    pub id: String,
    pub size_mb: u64,
    pub description_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionRequirement {
    pub runtime: String,
    pub packages: Vec<String>,
    pub estimated_download_mb: u64,
    pub estimated_disk_mb: u64,
    pub models: Vec<ExtensionModelRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionFrameworkDefinition {
    pub id: ExtensionFrameworkId,
    pub capability_id: ExtensionCapabilityId,
    pub name_key: String,
    pub description_key: String,
    pub default_port: u16,
    pub requirement: ExtensionRequirement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExtensionLifecycleStatus {
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

impl ExtensionLifecycleStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotInstalled => "not-installed",
            Self::Installing => "installing",
            Self::Installed => "installed",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Uninstalling => "uninstalling",
            Self::Error => "error",
            Self::Unsupported => "unsupported",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "installing" => Self::Installing,
            "installed" => Self::Installed,
            "starting" => Self::Starting,
            "running" => Self::Running,
            "stopping" => Self::Stopping,
            "uninstalling" => Self::Uninstalling,
            "error" => Self::Error,
            "unsupported" => Self::Unsupported,
            _ => Self::NotInstalled,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionEnvironment {
    pub runtime: String,
    pub os: String,
    pub arch: String,
    pub supported: bool,
    pub native_operations_available: bool,
    pub python_path: Option<String>,
    pub python_version: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionFrameworkStatus {
    pub framework_id: ExtensionFrameworkId,
    pub capability_id: ExtensionCapabilityId,
    pub status: ExtensionLifecycleStatus,
    pub installed: bool,
    pub enabled: bool,
    pub running: bool,
    pub port: u16,
    pub install_path: Option<String>,
    pub installed_version: Option<String>,
    pub last_health_check: Option<String>,
    pub last_error: Option<String>,
    pub last_operation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionOverview {
    pub definitions: Vec<ExtensionFrameworkDefinition>,
    pub statuses: Vec<ExtensionFrameworkStatus>,
    pub environment: ExtensionEnvironment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionInstallPreview {
    pub framework_id: ExtensionFrameworkId,
    pub supported: bool,
    pub install_path: String,
    pub python_path: Option<String>,
    pub packages: Vec<String>,
    pub models: Vec<ExtensionModelRequirement>,
    pub estimated_download_mb: u64,
    pub estimated_disk_mb: u64,
    pub inference_local_only: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionFrameworkRequest {
    pub framework_id: ExtensionFrameworkId,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionEnableRequest {
    pub framework_id: ExtensionFrameworkId,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExtensionAction {
    Install,
    Uninstall,
    Enable,
    Disable,
    Start,
    Stop,
    SelfTest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionOperationResult {
    pub success: bool,
    pub framework_id: ExtensionFrameworkId,
    pub action: ExtensionAction,
    pub message: String,
    pub logs: Vec<String>,
    pub error: Option<String>,
}
