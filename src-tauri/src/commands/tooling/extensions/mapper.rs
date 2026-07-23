use super::dto;
use crate::contexts::operations::api::{OperationKind, OperationTask};
use crate::contexts::operations::domain::OperationStatus;
#[cfg(test)]
use crate::contexts::tooling::extensions::api::ExtensionOperationRequest;
use crate::contexts::tooling::extensions::api::{
    ExtensionAction, ExtensionCapabilityId, ExtensionEnvironment, ExtensionFrameworkDefinition,
    ExtensionFrameworkId, ExtensionFrameworkStatus, ExtensionInstallPreview,
    ExtensionLifecycleStatus, ExtensionModelRequirement, ExtensionOverview,
    StartedExtensionOperation,
};

pub(super) fn overview_to_dto(overview: ExtensionOverview) -> dto::ExtensionOverview {
    dto::ExtensionOverview {
        definitions: overview
            .definitions
            .into_iter()
            .map(definition_to_dto)
            .collect(),
        statuses: overview.statuses.into_iter().map(status_to_dto).collect(),
        environment: environment_to_dto(overview.environment),
    }
}

pub(super) fn preview_to_dto(preview: ExtensionInstallPreview) -> dto::ExtensionInstallPreview {
    dto::ExtensionInstallPreview {
        framework_id: id_to_dto(preview.framework_id),
        supported: preview.supported,
        install_path: preview.install_path,
        python_path: preview.python_path,
        packages: preview.packages,
        models: preview.models.into_iter().map(model_to_dto).collect(),
        estimated_download_mb: preview.estimated_download_mb,
        estimated_disk_mb: preview.estimated_disk_mb,
        inference_local_only: preview.inference_local_only,
        reason: preview.reason,
    }
}

#[cfg(test)]
pub(super) fn operation_request(
    framework_id: dto::ExtensionFrameworkId,
    action: ExtensionAction,
) -> ExtensionOperationRequest {
    ExtensionOperationRequest {
        framework_id: framework_id_from_dto(framework_id),
        action,
    }
}

pub(super) fn framework_id_from_dto(
    framework_id: dto::ExtensionFrameworkId,
) -> ExtensionFrameworkId {
    id_from_dto(framework_id)
}

pub(super) fn enable_action(enabled: bool) -> ExtensionAction {
    if enabled {
        ExtensionAction::Enable
    } else {
        ExtensionAction::Disable
    }
}

pub(super) fn started_operation_to_dto(operation: &StartedExtensionOperation) -> OperationTask {
    OperationTask {
        id: operation.id.clone(),
        execution_run_id: None,
        trace_id: None,
        kind: OperationKind::Extension,
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

fn definition_to_dto(
    definition: ExtensionFrameworkDefinition,
) -> dto::ExtensionFrameworkDefinition {
    dto::ExtensionFrameworkDefinition {
        id: id_to_dto(definition.id),
        capability_id: capability_to_dto(definition.capability_id),
        name_key: definition.name_key.to_string(),
        description_key: definition.description_key.to_string(),
        default_port: definition.default_port,
        requirement: dto::ExtensionRequirement {
            runtime: definition.requirement.runtime.to_string(),
            packages: definition
                .requirement
                .packages
                .iter()
                .map(|package| (*package).to_string())
                .collect(),
            estimated_download_mb: definition.requirement.estimated_download_mb,
            estimated_disk_mb: definition.requirement.estimated_disk_mb,
            models: definition
                .requirement
                .models
                .iter()
                .copied()
                .map(model_to_dto)
                .collect(),
        },
    }
}

fn model_to_dto(model: ExtensionModelRequirement) -> dto::ExtensionModelRequirement {
    dto::ExtensionModelRequirement {
        id: model.id.to_string(),
        size_mb: model.size_mb,
        description_key: model.description_key.to_string(),
    }
}

fn status_to_dto(status: ExtensionFrameworkStatus) -> dto::ExtensionFrameworkStatus {
    dto::ExtensionFrameworkStatus {
        framework_id: id_to_dto(status.framework_id),
        capability_id: capability_to_dto(status.capability_id),
        status: lifecycle_to_dto(status.status),
        installed: status.installed,
        enabled: status.enabled,
        running: status.running,
        port: status.port,
        install_path: status.install_path,
        installed_version: status.installed_version,
        last_health_check: status.last_health_check,
        last_error: status.last_error,
        last_operation_id: status.last_operation_id,
    }
}

fn environment_to_dto(environment: ExtensionEnvironment) -> dto::ExtensionEnvironment {
    let reason = environment.reason_key().map(str::to_string);
    dto::ExtensionEnvironment {
        runtime: environment.runtime.to_string(),
        os: environment.os,
        arch: environment.arch,
        supported: environment.supported,
        native_operations_available: environment.native_operations_available,
        python_path: environment
            .python
            .as_ref()
            .map(|python| python.path.clone()),
        python_version: environment
            .python
            .as_ref()
            .map(|python| python.version.clone()),
        reason,
    }
}

fn id_from_dto(id: dto::ExtensionFrameworkId) -> ExtensionFrameworkId {
    match id {
        dto::ExtensionFrameworkId::Paddleocr => ExtensionFrameworkId::Paddleocr,
        dto::ExtensionFrameworkId::FasterWhisper => ExtensionFrameworkId::FasterWhisper,
        dto::ExtensionFrameworkId::SherpaOnnx => ExtensionFrameworkId::SherpaOnnx,
    }
}

fn id_to_dto(id: ExtensionFrameworkId) -> dto::ExtensionFrameworkId {
    match id {
        ExtensionFrameworkId::Paddleocr => dto::ExtensionFrameworkId::Paddleocr,
        ExtensionFrameworkId::FasterWhisper => dto::ExtensionFrameworkId::FasterWhisper,
        ExtensionFrameworkId::SherpaOnnx => dto::ExtensionFrameworkId::SherpaOnnx,
    }
}

fn capability_to_dto(id: ExtensionCapabilityId) -> dto::ExtensionCapabilityId {
    match id {
        ExtensionCapabilityId::Ocr => dto::ExtensionCapabilityId::Ocr,
        ExtensionCapabilityId::Asr => dto::ExtensionCapabilityId::Asr,
        ExtensionCapabilityId::Tts => dto::ExtensionCapabilityId::Tts,
    }
}

fn lifecycle_to_dto(status: ExtensionLifecycleStatus) -> dto::ExtensionLifecycleStatus {
    match status {
        ExtensionLifecycleStatus::NotInstalled => dto::ExtensionLifecycleStatus::NotInstalled,
        ExtensionLifecycleStatus::Installing => dto::ExtensionLifecycleStatus::Installing,
        ExtensionLifecycleStatus::Installed => dto::ExtensionLifecycleStatus::Installed,
        ExtensionLifecycleStatus::Starting => dto::ExtensionLifecycleStatus::Starting,
        ExtensionLifecycleStatus::Running => dto::ExtensionLifecycleStatus::Running,
        ExtensionLifecycleStatus::Stopping => dto::ExtensionLifecycleStatus::Stopping,
        ExtensionLifecycleStatus::Uninstalling => dto::ExtensionLifecycleStatus::Uninstalling,
        ExtensionLifecycleStatus::Error => dto::ExtensionLifecycleStatus::Error,
        ExtensionLifecycleStatus::Unsupported => dto::ExtensionLifecycleStatus::Unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::extensions::api::ExtensionEnvironmentReason;
    use crate::contexts::tooling::extensions::domain::{
        definition, ExtensionFrameworkState, ExtensionHealth, PythonRuntime,
    };

    #[test]
    fn overview_serialization_preserves_legacy_frontend_contract() {
        let definition = definition(ExtensionFrameworkId::Paddleocr);
        let state = ExtensionFrameworkState::seeded(definition);
        let overview = overview_to_dto(ExtensionOverview {
            definitions: vec![definition],
            statuses: vec![ExtensionFrameworkStatus {
                framework_id: state.framework_id,
                capability_id: state.capability_id,
                status: ExtensionLifecycleStatus::NotInstalled,
                installed: false,
                enabled: false,
                running: false,
                port: state.port,
                install_path: None,
                installed_version: None,
                last_health_check: None,
                last_error: Some("extensions.environment.windowsX64Only".to_string()),
                last_operation_id: None,
                health: ExtensionHealth {
                    installation_ready: false,
                    installation_drift: Vec::new(),
                    runtime_healthy: false,
                    runtime_error: None,
                },
            }],
            environment: ExtensionEnvironment {
                runtime: "tauri",
                os: "windows".to_string(),
                arch: "x86_64".to_string(),
                supported: true,
                native_operations_available: true,
                python: Some(PythonRuntime {
                    path: "python".to_string(),
                    version: "3.12.4".to_string(),
                }),
                reason: None,
            },
        });
        let value = serde_json::to_value(overview).expect("overview");

        assert_eq!(value["definitions"][0]["id"], "paddleocr");
        assert_eq!(value["definitions"][0]["capabilityId"], "ocr");
        assert_eq!(
            value["definitions"][0]["requirement"]["models"][0]["id"],
            "PP-OCRv5-mobile"
        );
        assert_eq!(value["statuses"][0]["status"], "not-installed");
        assert!(value["statuses"][0].get("health").is_none());
        assert_eq!(value["environment"]["pythonVersion"], "3.12.4");
    }

    #[test]
    fn preview_requests_and_operation_task_shapes_remain_stable() {
        let request: dto::ExtensionFrameworkRequest =
            serde_json::from_value(serde_json::json!({"frameworkId": "faster-whisper"}))
                .expect("request");
        let request = operation_request(request.framework_id, ExtensionAction::Start);
        assert_eq!(request.framework_id, ExtensionFrameworkId::FasterWhisper);
        assert_eq!(request.action, ExtensionAction::Start);

        assert!(
            serde_json::from_value::<dto::ExtensionFrameworkRequest>(serde_json::json!({
                "frameworkId": "unknown"
            }))
            .is_err()
        );

        let operation = started_operation_to_dto(&StartedExtensionOperation {
            id: "extension-op-fixed".to_string(),
            related_entity_id: Some("faster-whisper".to_string()),
            message: Some("Start local extension".to_string()),
            created_at: "100".to_string(),
            updated_at: "100".to_string(),
        });
        let value = serde_json::to_value(operation).expect("operation");
        assert_eq!(value["kind"], "extension");
        assert_eq!(value["status"], "running");
        assert_eq!(value["relatedEntityId"], "faster-whisper");

        let unsupported = environment_to_dto(ExtensionEnvironment {
            runtime: "tauri",
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            supported: false,
            native_operations_available: false,
            python: None,
            reason: Some(ExtensionEnvironmentReason::WindowsX64Only),
        });
        assert_eq!(
            unsupported.reason.as_deref(),
            Some("extensions.environment.windowsX64Only")
        );
    }
}
