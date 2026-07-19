use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliToolStatus {
    pub(crate) agent_id: String,
    pub(crate) display_name: String,
    pub(crate) provider: String,
    pub(crate) executable_name: String,
    pub(crate) package_name: String,
    pub(crate) installed: Option<bool>,
    pub(crate) current_version: Option<String>,
    pub(crate) latest_version: Option<String>,
    pub(crate) available_versions: Vec<String>,
    pub(crate) detected_path: Option<String>,
    pub(crate) install_command: String,
    pub(crate) last_checked_at: Option<String>,
    pub(crate) last_error: Option<String>,
    pub(crate) last_operation_id: Option<String>,
    pub(crate) version_check_status: CliVersionCheckStatus,
    pub(crate) environment_type: CliEnvironmentType,
    pub(crate) installations: Vec<CliInstallation>,
    pub(crate) active_installation_path: Option<String>,
    pub(crate) conflict_state: CliConflictState,
    pub(crate) lifecycle_eligibility: CliLifecycleEligibility,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliVersionCheckStatus {
    Unsupported,
    NotDetected,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CliEnvironmentType {
    Windows,
    Macos,
    Linux,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliInstallSource {
    Npm,
    Winget,
    Desktop,
    Homebrew,
    Volta,
    Bun,
    Vendor,
    System,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliConflictState {
    None,
    Multiple,
    VersionMismatch,
    RunnableMismatch,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliLifecycleEligibility {
    Npm,
    Wget,
    Winget,
    Manual,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliInstallation {
    pub(crate) path: String,
    pub(crate) version: Option<String>,
    pub(crate) runnable: bool,
    pub(crate) error: Option<String>,
    pub(crate) source: CliInstallSource,
    pub(crate) environment_type: CliEnvironmentType,
    pub(crate) is_active: bool,
}
