use crate::contexts::tooling::extensions::domain::{
    ExtensionAction, ExtensionEnvironment, ExtensionFrameworkDefinition, ExtensionFrameworkId,
    ExtensionFrameworkStatus, ExtensionModelRequirement,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionOverview {
    pub(crate) definitions: Vec<ExtensionFrameworkDefinition>,
    pub(crate) statuses: Vec<ExtensionFrameworkStatus>,
    pub(crate) environment: ExtensionEnvironment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExtensionOperationRequest {
    pub(crate) framework_id: ExtensionFrameworkId,
    pub(crate) action: ExtensionAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExtensionLogLevel {
    Error,
    Warn,
    Info,
    #[expect(
        dead_code,
        reason = "extension logging preserves the four-level log contract"
    )]
    Debug,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionExecutionLog {
    pub(crate) level: ExtensionLogLevel,
    pub(crate) line: String,
    pub(crate) context: BTreeMap<String, String>,
}

impl ExtensionExecutionLog {
    pub(crate) fn info(line: impl Into<String>) -> Self {
        Self {
            level: ExtensionLogLevel::Info,
            line: line.into(),
            context: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionLogEvent {
    pub(crate) operation_id: String,
    pub(crate) framework_id: ExtensionFrameworkId,
    pub(crate) action: ExtensionAction,
    pub(crate) level: ExtensionLogLevel,
    pub(crate) line: String,
    pub(crate) timestamp: String,
    pub(crate) context: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartedExtensionOperation {
    pub(crate) id: String,
    pub(crate) related_entity_id: Option<String>,
    pub(crate) message: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedExtensionOperation {
    pub(crate) operation: StartedExtensionOperation,
    pub(crate) request: ExtensionOperationRequest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstalledExtension {
    pub(crate) install_path: String,
    pub(crate) installed_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionOperationResult {
    pub(crate) success: bool,
    pub(crate) operation_id: String,
    pub(crate) framework_id: ExtensionFrameworkId,
    pub(crate) action: ExtensionAction,
    pub(crate) message: String,
    pub(crate) logs: Vec<String>,
    pub(crate) error: Option<String>,
}
