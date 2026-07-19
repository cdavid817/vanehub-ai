use super::dto;
use crate::contexts::operations::api::{OperationKind, OperationTask};
use crate::contexts::operations::domain::OperationStatus;
use crate::contexts::tooling::sdk::api::{
    sdk_definition, SdkDefinition, SdkEnvironmentStatus, SdkId, SdkInstallStatus, SdkOperationLog,
    SdkOperationRequest, SdkOperationType, SdkStatus, SdkUpdateInfo, SdkVersionInfo,
    SdkVersionSource, StartedSdkOperation,
};
use std::collections::BTreeMap;

pub(super) fn definition_to_dto(definition: SdkDefinition) -> dto::SdkDefinition {
    dto::SdkDefinition {
        id: id_to_dto(definition.id),
        display_name: definition.display_name.to_string(),
        npm_package: definition.npm_package.to_string(),
        companion_packages: strings(definition.companion_packages),
        fallback_versions: strings(definition.fallback_versions),
        description: definition.description.to_string(),
        related_providers: strings(definition.related_providers),
    }
}

pub(super) fn status_map_to_dto(statuses: Vec<SdkStatus>) -> dto::SdkStatusMap {
    statuses
        .into_iter()
        .map(|status| (id_to_dto(status.id), status_to_dto(status)))
        .collect()
}

pub(super) fn environment_to_dto(status: SdkEnvironmentStatus) -> dto::SdkEnvironmentStatus {
    dto::SdkEnvironmentStatus {
        available: status.available,
        node_path: status.node_path,
        node_version: status.node_version,
        npm_path: status.npm_path,
        npm_version: status.npm_version,
        error: status.error,
    }
}

pub(super) fn version_map_to_dto(versions: BTreeMap<SdkId, SdkVersionInfo>) -> dto::SdkVersionMap {
    versions
        .into_iter()
        .map(|(id, version)| (id_to_dto(id), version_to_dto(version)))
        .collect()
}

pub(super) fn update_map_to_dto(updates: BTreeMap<SdkId, SdkUpdateInfo>) -> dto::SdkUpdateMap {
    updates
        .into_iter()
        .map(|(id, update)| (id_to_dto(id), update_to_dto(update)))
        .collect()
}

pub(super) fn operation_logs_to_dto(logs: Vec<SdkOperationLog>) -> Vec<dto::SdkOperationLog> {
    logs.into_iter().map(operation_log_to_dto).collect()
}

pub(super) fn operation_request(
    request: dto::SdkOperationRequest,
    operation: SdkOperationType,
) -> SdkOperationRequest {
    SdkOperationRequest {
        sdk_id: id_from_dto(request.sdk_id),
        operation,
        version: request.version,
    }
}

pub(super) fn uninstall_request(sdk_id: dto::SdkId) -> SdkOperationRequest {
    SdkOperationRequest {
        sdk_id: id_from_dto(sdk_id),
        operation: SdkOperationType::Uninstall,
        version: None,
    }
}

pub(super) fn optional_id_from_dto(sdk_id: Option<dto::SdkId>) -> Option<SdkId> {
    sdk_id.map(id_from_dto)
}

pub(super) fn started_operation_to_dto(operation: &StartedSdkOperation) -> OperationTask {
    OperationTask {
        id: operation.id.clone(),
        kind: OperationKind::Sdk,
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

fn status_to_dto(status: SdkStatus) -> dto::SdkStatus {
    let definition = sdk_definition(status.id);
    dto::SdkStatus {
        id: id_to_dto(status.id),
        display_name: definition.display_name.to_string(),
        npm_package: definition.npm_package.to_string(),
        description: definition.description.to_string(),
        related_providers: strings(definition.related_providers),
        status: install_status_to_dto(status.status),
        installed_version: status.installed_version,
        latest_version: status.latest_version,
        has_update: status.has_update,
        install_path: status.install_path,
        last_checked: status.last_checked,
        error_message: status.error_message,
    }
}

fn version_to_dto(version: SdkVersionInfo) -> dto::SdkVersionInfo {
    dto::SdkVersionInfo {
        sdk_id: id_to_dto(version.sdk_id),
        versions: version.versions,
        fallback_versions: version.fallback_versions,
        source: match version.source {
            SdkVersionSource::Remote => dto::SdkVersionSource::Remote,
            SdkVersionSource::Fallback => dto::SdkVersionSource::Fallback,
        },
        latest_version: version.latest_version,
        error: version.error,
    }
}

fn update_to_dto(update: SdkUpdateInfo) -> dto::SdkUpdateInfo {
    dto::SdkUpdateInfo {
        id: id_to_dto(update.id),
        latest_version: update.latest_version,
        has_update: update.has_update,
        error_message: update.error_message,
    }
}

fn operation_log_to_dto(log: SdkOperationLog) -> dto::SdkOperationLog {
    dto::SdkOperationLog {
        sdk_id: id_to_dto(log.sdk_id),
        operation: match log.operation {
            SdkOperationType::Install => dto::SdkOperationType::Install,
            SdkOperationType::Update => dto::SdkOperationType::Update,
            SdkOperationType::Rollback => dto::SdkOperationType::Rollback,
            SdkOperationType::Uninstall => dto::SdkOperationType::Uninstall,
        },
        line: log.line,
        timestamp: log.timestamp,
    }
}

fn id_from_dto(id: dto::SdkId) -> SdkId {
    match id {
        dto::SdkId::ClaudeSdk => SdkId::ClaudeSdk,
        dto::SdkId::CodexSdk => SdkId::CodexSdk,
    }
}

fn id_to_dto(id: SdkId) -> dto::SdkId {
    match id {
        SdkId::ClaudeSdk => dto::SdkId::ClaudeSdk,
        SdkId::CodexSdk => dto::SdkId::CodexSdk,
    }
}

fn install_status_to_dto(status: SdkInstallStatus) -> dto::SdkInstallStatus {
    match status {
        SdkInstallStatus::Installed => dto::SdkInstallStatus::Installed,
        SdkInstallStatus::NotInstalled => dto::SdkInstallStatus::NotInstalled,
        SdkInstallStatus::Error => dto::SdkInstallStatus::Error,
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definition_status_and_enum_mapping_preserve_frontend_contract() {
        let definition = definition_to_dto(sdk_definition(SdkId::ClaudeSdk));
        let status = SdkStatus::observed(
            SdkId::ClaudeSdk,
            Some("0.2.81".to_string()),
            Some("0.2.88".to_string()),
            Some("C:\\fixture\\claude-sdk".to_string()),
            Some("now".to_string()),
            None,
        );
        let statuses = status_map_to_dto(vec![status]);
        let value = serde_json::json!({
            "definitions": [definition],
            "statuses": statuses,
        });

        assert_eq!(value["definitions"][0]["id"], "claude-sdk");
        assert_eq!(value["definitions"][0]["displayName"], "Claude Code SDK");
        assert!(value["definitions"][0].get("defaultVersion").is_none());
        assert_eq!(value["statuses"]["claude-sdk"]["status"], "installed");
        assert_eq!(
            value["statuses"]["claude-sdk"]["relatedProviders"],
            serde_json::json!(["anthropic", "bedrock"])
        );
    }

    #[test]
    fn version_update_log_request_and_operation_shapes_remain_stable() {
        let request: dto::SdkOperationRequest = serde_json::from_value(serde_json::json!({
            "sdkId": "codex-sdk",
            "version": "0.117.0"
        }))
        .expect("request");
        let request = operation_request(request, SdkOperationType::Rollback);
        assert_eq!(request.sdk_id, SdkId::CodexSdk);
        assert_eq!(request.operation, SdkOperationType::Rollback);

        let operation = started_operation_to_dto(&StartedSdkOperation {
            id: "sdk-op-fixed".to_string(),
            related_entity_id: Some("codex-sdk".to_string()),
            message: Some("Rollback SDK operation".to_string()),
            created_at: "100".to_string(),
            updated_at: "100".to_string(),
        });
        let value = serde_json::to_value(operation).expect("operation");
        assert_eq!(value["kind"], "sdk");
        assert_eq!(value["status"], "running");
        assert_eq!(value["relatedEntityId"], "codex-sdk");

        let logs = operation_logs_to_dto(vec![SdkOperationLog {
            sdk_id: SdkId::CodexSdk,
            operation: SdkOperationType::Rollback,
            line: "rolled back".to_string(),
            timestamp: "now".to_string(),
        }]);
        let value = serde_json::to_value(logs).expect("logs");
        assert_eq!(value[0]["sdkId"], "codex-sdk");
        assert_eq!(value[0]["operation"], "rollback");

        let versions = version_map_to_dto(BTreeMap::from([(
            SdkId::CodexSdk,
            SdkVersionInfo {
                sdk_id: SdkId::CodexSdk,
                versions: vec!["0.117.0".to_string()],
                fallback_versions: vec!["0.116.0".to_string()],
                source: SdkVersionSource::Fallback,
                latest_version: Some("0.117.0".to_string()),
                error: Some("offline".to_string()),
            },
        )]));
        let updates = update_map_to_dto(BTreeMap::from([(
            SdkId::CodexSdk,
            SdkUpdateInfo {
                id: SdkId::CodexSdk,
                latest_version: Some("0.117.0".to_string()),
                has_update: true,
                error_message: None,
            },
        )]));
        let environment = environment_to_dto(SdkEnvironmentStatus {
            available: true,
            node_path: Some("node".to_string()),
            node_version: Some("v22.0.0".to_string()),
            npm_path: Some("npm".to_string()),
            npm_version: Some("10.0.0".to_string()),
            error: None,
        });
        let value = serde_json::json!({
            "versions": versions,
            "updates": updates,
            "environment": environment,
        });
        assert_eq!(value["versions"]["codex-sdk"]["source"], "fallback");
        assert_eq!(value["versions"]["codex-sdk"]["latestVersion"], "0.117.0");
        assert_eq!(value["updates"]["codex-sdk"]["hasUpdate"], true);
        assert_eq!(value["environment"]["nodeVersion"], "v22.0.0");
    }
}
