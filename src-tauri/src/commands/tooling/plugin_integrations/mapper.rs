use super::dto;
use crate::contexts::tooling::plugin_integrations::api::{
    PluginIntegrationDefinition, PluginIntegrationEnvironment, PluginIntegrationId,
    PluginIntegrationOverview, PluginIntegrationState, PluginIntegrationStatus,
    PluginIntegrationTestResult,
};

pub(super) fn overview_to_dto(
    overview: PluginIntegrationOverview,
) -> dto::PluginIntegrationOverview {
    dto::PluginIntegrationOverview {
        definitions: overview
            .definitions
            .into_iter()
            .map(definition_to_dto)
            .collect(),
        states: overview.states.into_iter().map(state_to_dto).collect(),
        environment: environment_to_dto(overview.environment),
    }
}

pub(super) fn request_id(request: dto::PluginIntegrationRequest) -> &'static str {
    match request.integration_id {
        dto::PluginIntegrationId::Github => PluginIntegrationId::Github.as_str(),
    }
}

pub(super) fn test_result_to_dto(
    result: PluginIntegrationTestResult,
) -> dto::PluginIntegrationTestResult {
    dto::PluginIntegrationTestResult {
        integration_id: id_to_dto(result.integration_id),
        status: status_to_dto(result.status),
        configured: result.configured,
        message: result.message,
        checked_at: result.checked_at,
    }
}

fn definition_to_dto(definition: PluginIntegrationDefinition) -> dto::PluginIntegrationDefinition {
    dto::PluginIntegrationDefinition {
        id: id_to_dto(definition.id),
        name_key: definition.name_key.to_string(),
        description_key: definition.description_key.to_string(),
        version: definition.version.to_string(),
        provider: definition.provider.to_string(),
        icon: definition.icon.to_string(),
        docs_url: definition.docs_url.to_string(),
        setup_steps: definition
            .setup_steps
            .iter()
            .map(|step| dto::PluginIntegrationSetupStep {
                id: step.id.to_string(),
                label_key: step.label_key.to_string(),
            })
            .collect(),
    }
}

fn state_to_dto(state: PluginIntegrationState) -> dto::PluginIntegrationState {
    dto::PluginIntegrationState {
        integration_id: id_to_dto(state.integration_id),
        status: status_to_dto(state.status),
        configured: state.configured,
        can_test: state.can_test,
        last_checked_at: state.last_checked_at,
        status_reason_key: state.status_reason_key,
        message: state.message,
    }
}

fn environment_to_dto(
    environment: PluginIntegrationEnvironment,
) -> dto::PluginIntegrationEnvironment {
    dto::PluginIntegrationEnvironment {
        runtime: environment.runtime.to_string(),
        native_checks_available: environment.native_checks_available,
        reason_key: environment.reason_key.map(str::to_string),
    }
}

fn id_to_dto(id: PluginIntegrationId) -> dto::PluginIntegrationId {
    match id {
        PluginIntegrationId::Github => dto::PluginIntegrationId::Github,
    }
}

fn status_to_dto(status: PluginIntegrationStatus) -> dto::PluginIntegrationStatus {
    match status {
        PluginIntegrationStatus::Configured => dto::PluginIntegrationStatus::Configured,
        PluginIntegrationStatus::NotConfigured => dto::PluginIntegrationStatus::NotConfigured,
        PluginIntegrationStatus::MissingCli => dto::PluginIntegrationStatus::MissingCli,
        PluginIntegrationStatus::Error => dto::PluginIntegrationStatus::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::plugin_integrations::domain::definitions;

    #[test]
    fn overview_serialization_preserves_the_existing_frontend_contract() {
        let value = serde_json::to_value(overview_to_dto(PluginIntegrationOverview {
            definitions: definitions().to_vec(),
            states: vec![PluginIntegrationState::initial(PluginIntegrationId::Github)],
            environment: PluginIntegrationEnvironment {
                runtime: "tauri",
                native_checks_available: true,
                reason_key: None,
            },
        }))
        .expect("overview");

        assert_eq!(
            value,
            serde_json::json!({
                "definitions": [{
                    "id": "github",
                    "nameKey": "plugins.github.name",
                    "descriptionKey": "plugins.github.description",
                    "version": "1.0.0",
                    "provider": "GitHub",
                    "icon": "github",
                    "docsUrl": "https://cli.github.com/manual/gh_auth_login",
                    "setupSteps": [
                        { "id": "install", "labelKey": "plugins.github.setup.install" },
                        { "id": "auth", "labelKey": "plugins.github.setup.auth" }
                    ]
                }],
                "states": [{
                    "integrationId": "github",
                    "status": "not-configured",
                    "configured": false,
                    "canTest": true,
                    "lastCheckedAt": null,
                    "statusReasonKey": "plugins.statusReason.notChecked",
                    "message": null
                }],
                "environment": {
                    "runtime": "tauri",
                    "nativeChecksAvailable": true,
                    "reasonKey": null
                }
            })
        );
    }

    #[test]
    fn request_and_readiness_result_keep_camel_case_and_kebab_case_values() {
        let request: dto::PluginIntegrationRequest = serde_json::from_value(serde_json::json!({
            "integrationId": "github"
        }))
        .expect("request");
        assert_eq!(request_id(request), "github");
        assert!(
            serde_json::from_value::<dto::PluginIntegrationRequest>(serde_json::json!({
                "integrationId": "gitlab"
            }))
            .is_err()
        );

        let value = serde_json::to_value(test_result_to_dto(PluginIntegrationTestResult {
            integration_id: PluginIntegrationId::Github,
            status: PluginIntegrationStatus::MissingCli,
            configured: false,
            message: "plugins.statusReason.missingCli".to_string(),
            checked_at: "2026-07-18T00:00:00Z".to_string(),
        }))
        .expect("result");
        assert_eq!(value["integrationId"], "github");
        assert_eq!(value["status"], "missing-cli");
        assert_eq!(value["checkedAt"], "2026-07-18T00:00:00Z");
        assert!(value.get("checked_at").is_none());
    }
}
