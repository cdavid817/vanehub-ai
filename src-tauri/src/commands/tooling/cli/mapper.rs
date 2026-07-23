use super::dto::{
    CliConflictState, CliEnvironmentType, CliInstallSource, CliInstallation,
    CliLifecycleEligibility, CliToolStatus as CliToolStatusDto, CliVersionCheckStatus,
};
use crate::contexts::operations::api::{OperationKind, OperationTask};
use crate::contexts::operations::domain::OperationStatus;
use crate::contexts::tooling::cli::api::{CliToolStatus, StartedCliOperation};
use crate::contexts::tooling::cli::domain::{
    ConflictState, EnvironmentType, InstallSource, Installation, LifecycleEligibility,
    VersionCheckStatus,
};

pub(super) fn status_to_dto(status: CliToolStatus) -> CliToolStatusDto {
    CliToolStatusDto {
        agent_id: status.agent_id,
        display_name: status.display_name,
        provider: status.provider,
        executable_name: status.executable_name,
        package_name: status.package_name,
        installed: status.installed,
        current_version: status.current_version,
        latest_version: status.latest_version,
        available_versions: status.available_versions,
        detected_path: status.detected_path,
        install_command: status.install_command,
        last_checked_at: status.last_checked_at,
        last_error: status.last_error,
        last_operation_id: status.last_operation_id,
        version_check_status: version_check_status(status.version_check_status),
        environment_type: environment_type(status.environment_type),
        installations: status
            .installations
            .into_iter()
            .map(installation_to_dto)
            .collect(),
        active_installation_path: status.active_installation_path,
        conflict_state: conflict_state(status.conflict_state),
        lifecycle_eligibility: lifecycle_eligibility(status.lifecycle_eligibility),
    }
}

pub(super) fn started_operation_to_dto(operation: &StartedCliOperation) -> OperationTask {
    OperationTask {
        id: operation.id.clone(),
        execution_run_id: None,
        trace_id: None,
        kind: OperationKind::Agent,
        status: OperationStatus::Running,
        related_entity_id: operation.related_entity_id.clone(),
        message: operation.message.clone(),
        logs: Vec::new(),
        result: None,
        error: None,
        created_at: operation.created_at.clone(),
        updated_at: operation.updated_at.clone(),
    }
}

fn installation_to_dto(installation: Installation) -> CliInstallation {
    CliInstallation {
        path: installation.path,
        version: installation.version,
        runnable: installation.runnable,
        error: installation.error,
        source: install_source(installation.source),
        environment_type: environment_type(installation.environment_type),
        is_active: installation.is_active,
    }
}

fn version_check_status(status: VersionCheckStatus) -> CliVersionCheckStatus {
    match status {
        VersionCheckStatus::Unsupported => CliVersionCheckStatus::Unsupported,
        VersionCheckStatus::NotDetected => CliVersionCheckStatus::NotDetected,
        VersionCheckStatus::Succeeded => CliVersionCheckStatus::Succeeded,
        VersionCheckStatus::Failed => CliVersionCheckStatus::Failed,
    }
}

fn environment_type(environment: EnvironmentType) -> CliEnvironmentType {
    match environment {
        EnvironmentType::Windows => CliEnvironmentType::Windows,
        EnvironmentType::Macos => CliEnvironmentType::Macos,
        EnvironmentType::Linux => CliEnvironmentType::Linux,
        EnvironmentType::Unknown => CliEnvironmentType::Unknown,
    }
}

fn install_source(source: InstallSource) -> CliInstallSource {
    match source {
        InstallSource::Npm => CliInstallSource::Npm,
        InstallSource::Winget => CliInstallSource::Winget,
        InstallSource::Desktop => CliInstallSource::Desktop,
        InstallSource::Homebrew => CliInstallSource::Homebrew,
        InstallSource::Volta => CliInstallSource::Volta,
        InstallSource::Bun => CliInstallSource::Bun,
        InstallSource::Vendor => CliInstallSource::Vendor,
        InstallSource::System => CliInstallSource::System,
        InstallSource::Unknown => CliInstallSource::Unknown,
    }
}

fn conflict_state(state: ConflictState) -> CliConflictState {
    match state {
        ConflictState::None => CliConflictState::None,
        ConflictState::Multiple => CliConflictState::Multiple,
        ConflictState::VersionMismatch => CliConflictState::VersionMismatch,
        ConflictState::RunnableMismatch => CliConflictState::RunnableMismatch,
    }
}

fn lifecycle_eligibility(eligibility: LifecycleEligibility) -> CliLifecycleEligibility {
    match eligibility {
        LifecycleEligibility::Npm => CliLifecycleEligibility::Npm,
        LifecycleEligibility::Wget => CliLifecycleEligibility::Wget,
        LifecycleEligibility::Winget => CliLifecycleEligibility::Winget,
        LifecycleEligibility::Manual => CliLifecycleEligibility::Manual,
        LifecycleEligibility::Unavailable => CliLifecycleEligibility::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::cli::domain::definition;

    #[test]
    fn status_mapping_preserves_frontend_field_and_enum_contract() {
        let definition = definition("codex-cli").expect("definition");
        let mut status = CliToolStatus::unavailable(
            definition,
            EnvironmentType::Windows,
            "npm install -g @openai/codex@latest".to_string(),
        );
        status.conflict_state = ConflictState::VersionMismatch;
        status.lifecycle_eligibility = LifecycleEligibility::Npm;
        status.installations = vec![Installation {
            path: "C:\\fixture\\codex.cmd".to_string(),
            version: Some("1.2.3".to_string()),
            runnable: true,
            error: None,
            source: InstallSource::Npm,
            environment_type: EnvironmentType::Windows,
            is_active: true,
        }];

        let value = serde_json::to_value(status_to_dto(status)).expect("serialize");

        assert_eq!(value["agentId"], "codex-cli");
        assert_eq!(value["versionCheckStatus"], "not-detected");
        assert_eq!(value["environmentType"], "windows");
        assert_eq!(value["conflictState"], "version-mismatch");
        assert_eq!(value["lifecycleEligibility"], "npm");
        assert_eq!(value["installations"][0]["isActive"], true);
        assert!(value.get("agent_id").is_none());
    }

    #[test]
    fn started_operation_mapping_keeps_agent_operation_shape() {
        let value = serde_json::to_value(started_operation_to_dto(&StartedCliOperation {
            id: "op-fixed".to_string(),
            related_entity_id: Some("codex-cli".to_string()),
            message: Some("Installing".to_string()),
            created_at: "100".to_string(),
            updated_at: "100".to_string(),
        }))
        .expect("serialize");

        assert_eq!(value["kind"], "agent");
        assert_eq!(value["status"], "running");
        assert_eq!(value["relatedEntityId"], "codex-cli");
        assert_eq!(value["logs"], serde_json::json!([]));
    }
}
