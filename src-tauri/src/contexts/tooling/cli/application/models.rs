use crate::contexts::tooling::cli::domain::{
    ConflictState, EnvironmentType, Installation, LifecycleEligibility, ToolDefinition,
    VersionCheckStatus,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub(crate) version_check_status: VersionCheckStatus,
    pub(crate) environment_type: EnvironmentType,
    pub(crate) installations: Vec<Installation>,
    pub(crate) active_installation_path: Option<String>,
    pub(crate) conflict_state: ConflictState,
    pub(crate) lifecycle_eligibility: LifecycleEligibility,
}

impl CliToolStatus {
    pub(crate) fn unavailable(
        definition: ToolDefinition,
        environment_type: EnvironmentType,
        install_command: String,
    ) -> Self {
        Self {
            agent_id: definition.agent_id.to_string(),
            display_name: definition.display_name.to_string(),
            provider: definition.provider.to_string(),
            executable_name: definition.executable_name.to_string(),
            package_name: definition.package_name.to_string(),
            installed: None,
            current_version: None,
            latest_version: None,
            available_versions: Vec::new(),
            detected_path: None,
            install_command,
            last_checked_at: None,
            last_error: None,
            last_operation_id: None,
            version_check_status: VersionCheckStatus::NotDetected,
            environment_type,
            installations: Vec::new(),
            active_installation_path: None,
            conflict_state: ConflictState::None,
            lifecycle_eligibility: LifecycleEligibility::Unavailable,
        }
    }

    pub(super) fn associate_detection(&mut self, operation_id: &str, checked_at: String) {
        self.last_operation_id = Some(operation_id.to_string());
        self.last_checked_at = Some(checked_at);
    }

    pub(super) fn record_failure(&mut self, operation_id: &str, error: String) {
        self.last_operation_id = Some(operation_id.to_string());
        self.last_error = Some(error);
        self.version_check_status = VersionCheckStatus::Failed;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliDetectionResult {
    pub(crate) status: CliToolStatus,
    pub(crate) warnings: Vec<String>,
    pub(crate) events: Vec<CliLogEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliOperationType {
    Refresh,
    Install,
    UpgradeAll,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliOperationRequest {
    pub(crate) operation_type: CliOperationType,
    pub(crate) related_agent_id: Option<String>,
    pub(crate) message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartedCliOperation {
    pub(crate) id: String,
    pub(crate) related_entity_id: Option<String>,
    pub(crate) message: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CliOperationResult {
    Refresh {
        agent_ids: Vec<String>,
        failed: Vec<String>,
    },
    Install {
        agent_id: String,
        target_version: String,
    },
    UpgradeAll {
        upgraded: Vec<String>,
        skipped: Vec<String>,
        failed: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliLogLevel {
    Error,
    Warn,
    Info,
    #[expect(
        dead_code,
        reason = "CLI logging preserves the four-level log contract"
    )]
    Debug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliLogCategory {
    Operation,
    Diagnostic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliLogEvent {
    pub(crate) operation_id: String,
    pub(crate) agent_id: Option<String>,
    pub(crate) level: CliLogLevel,
    pub(crate) category: CliLogCategory,
    pub(crate) message: String,
    pub(crate) context: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedCliRefresh {
    pub(crate) operation: StartedCliOperation,
    pub(super) agent_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedCliInstall {
    pub(crate) operation: StartedCliOperation,
    pub(super) definition: ToolDefinition,
    pub(super) status: CliToolStatus,
    pub(super) target_version: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedCliUpgradeAll {
    pub(crate) operation: StartedCliOperation,
    pub(super) statuses: Vec<CliToolStatus>,
    pub(super) acquired_agent_ids: Vec<String>,
}
