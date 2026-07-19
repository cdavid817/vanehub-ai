use super::dto;
use crate::commands::error::CommandError;
use crate::contexts::desktop::api::{
    ClientLogEvent, ClientLogEventKind, DataManagementInformation, DesktopLogLevel,
    DesktopLoggingPolicy, DesktopSettingsView, DetectedNetworkProxy, FloatingAssistantConfig,
    FloatingAssistantMainAction, FloatingAssistantPlatform, FloatingAssistantSurfaceMode,
    NetworkProxyTestResult, NodeInformation,
};
use crate::contexts::desktop::domain::AutomaticArchivalSettings;

pub(super) fn setting_input(
    input: dto::SaveSettingInput,
) -> Result<(String, String), CommandError> {
    let value = match (&*input.key, input.value) {
        ("launchOnStartup", serde_json::Value::Bool(enabled)) => enabled.to_string(),
        (_, serde_json::Value::String(value)) => value,
        _ => {
            return Err(CommandError::validation(format!(
                "Invalid setting value type for key '{}'.",
                input.key
            )))
        }
    };
    Ok((input.key, value))
}

pub(super) fn settings_to_dto(view: DesktopSettingsView) -> dto::AppSettings {
    let settings = view.settings;
    let archival = settings.automatic_archival();
    dto::AppSettings {
        application_language: settings.application_language().as_str().to_string(),
        font_size: settings.font_size().as_str().to_string(),
        theme: settings.theme().as_str().to_string(),
        default_folder_path: settings.default_folder_path().to_string(),
        log_directory: settings.log_directory().to_string(),
        network_proxy_url: settings.network_proxy().url().to_string(),
        network_proxy_bypass: settings.network_proxy().bypass().to_string(),
        automatic_archival_enabled: archival.enabled(),
        automatic_archival_inactive_days: archival.inactive_days(),
        launch_on_startup: settings.startup().enabled(),
        logging_policy: logging_policy_to_dto(view.logging_policy),
    }
}

fn logging_policy_to_dto(policy: DesktopLoggingPolicy) -> dto::LoggingPolicy {
    dto::LoggingPolicy {
        retention_days: policy.retention_days,
        archive_enabled: policy.archive_enabled,
        redaction_enabled: policy.redaction_enabled,
        levels: policy.levels.into_iter().map(log_level_to_dto).collect(),
        can_open_directory: policy.can_open_directory,
    }
}

fn log_level_to_dto(level: DesktopLogLevel) -> dto::LogLevel {
    match level {
        DesktopLogLevel::Error => dto::LogLevel::Error,
        DesktopLogLevel::Warn => dto::LogLevel::Warn,
        DesktopLogLevel::Info => dto::LogLevel::Info,
        DesktopLogLevel::Debug => dto::LogLevel::Debug,
    }
}

fn log_level_to_domain(level: dto::LogLevel) -> DesktopLogLevel {
    match level {
        dto::LogLevel::Error => DesktopLogLevel::Error,
        dto::LogLevel::Warn => DesktopLogLevel::Warn,
        dto::LogLevel::Info => DesktopLogLevel::Info,
        dto::LogLevel::Debug => DesktopLogLevel::Debug,
    }
}

pub(super) fn archival_to_dto(
    settings: AutomaticArchivalSettings,
) -> dto::AutomaticArchivalSettings {
    dto::AutomaticArchivalSettings {
        enabled: settings.enabled(),
        inactive_days: settings.inactive_days(),
    }
}

pub(super) fn data_information_to_dto(
    information: DataManagementInformation,
) -> dto::DataManagementInfo {
    dto::DataManagementInfo {
        database_path: information.database_path,
        database_directory: information.database_directory,
        can_open_directory: information.can_open_directory,
    }
}

pub(super) fn node_information_to_dto(information: NodeInformation) -> dto::NodeInfo {
    dto::NodeInfo {
        available: information.available,
        path: information.path,
        version: information.version,
        reason: information.reason,
    }
}

pub(super) fn proxy_test_to_dto(result: NetworkProxyTestResult) -> dto::NetworkProxyTestResult {
    dto::NetworkProxyTestResult {
        success: result.success,
        latency_ms: result.latency_ms,
        error: result.error,
    }
}

pub(super) fn detected_proxy_to_dto(proxy: DetectedNetworkProxy) -> dto::DetectedNetworkProxy {
    dto::DetectedNetworkProxy {
        url: proxy.url,
        proxy_type: proxy.proxy_type,
        port: proxy.port,
    }
}

pub(super) fn client_log_to_domain(event: dto::ClientLogEvent) -> ClientLogEvent {
    ClientLogEvent {
        level: log_level_to_domain(event.level),
        kind: match event.kind {
            dto::ClientLogEventKind::ErrorBoundary => ClientLogEventKind::ErrorBoundary,
            dto::ClientLogEventKind::CriticalOperationFailure => {
                ClientLogEventKind::CriticalOperationFailure
            }
        },
        message: event.message,
        source: event.source,
        details: event.details,
        stack: event.stack,
    }
}

pub(super) fn floating_runtime_to_dto(
    platform: FloatingAssistantPlatform,
) -> dto::FloatingAssistantRuntimeInfo {
    dto::FloatingAssistantRuntimeInfo {
        native_available: platform.native_available(),
        platform: platform.as_str().to_string(),
    }
}

pub(super) fn floating_config_to_dto(
    config: FloatingAssistantConfig,
) -> dto::FloatingAssistantConfig {
    dto::FloatingAssistantConfig {
        enabled: config.enabled(),
        anchor: config.anchor().map(|anchor| dto::FloatingAssistantAnchor {
            x: anchor.x(),
            y: anchor.y(),
            monitor_name: anchor.monitor_name().map(ToOwned::to_owned),
        }),
    }
}

pub(super) fn floating_surface_to_domain(
    mode: dto::FloatingAssistantSurfaceMode,
) -> FloatingAssistantSurfaceMode {
    match mode {
        dto::FloatingAssistantSurfaceMode::Collapsed => FloatingAssistantSurfaceMode::Collapsed,
        dto::FloatingAssistantSurfaceMode::Menu => FloatingAssistantSurfaceMode::Menu,
        dto::FloatingAssistantSurfaceMode::Chat => FloatingAssistantSurfaceMode::Chat,
    }
}

pub(super) fn floating_surface_to_dto(
    mode: FloatingAssistantSurfaceMode,
) -> dto::FloatingAssistantSurfaceMode {
    match mode {
        FloatingAssistantSurfaceMode::Collapsed => dto::FloatingAssistantSurfaceMode::Collapsed,
        FloatingAssistantSurfaceMode::Menu => dto::FloatingAssistantSurfaceMode::Menu,
        FloatingAssistantSurfaceMode::Chat => dto::FloatingAssistantSurfaceMode::Chat,
    }
}

pub(super) fn floating_main_action(
    action: &str,
) -> Result<FloatingAssistantMainAction, CommandError> {
    FloatingAssistantMainAction::parse(action).map_err(|error| {
        CommandError::from(crate::contexts::desktop::api::FloatingAssistantError::Domain(error))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::desktop::application::DesktopSettingsView;
    use crate::contexts::desktop::domain::{
        DesktopSettingMutation, DesktopSettings, FloatingAssistantAnchor,
    };
    use serde_json::json;

    #[test]
    fn settings_response_preserves_the_complete_legacy_contract() {
        let mut settings = DesktopSettings::defaults("D:/data/logs");
        settings
            .apply(DesktopSettingMutation::parse("applicationLanguage", "en").expect("language"));
        settings.apply(DesktopSettingMutation::parse("fontSize", "18px").expect("font"));
        settings.apply(DesktopSettingMutation::parse("theme", "minimal").expect("theme"));
        let value = serde_json::to_value(settings_to_dto(DesktopSettingsView::native(settings)))
            .expect("settings DTO");

        assert_eq!(
            value,
            json!({
                "applicationLanguage": "en",
                "fontSize": "18px",
                "theme": "minimal",
                "defaultFolderPath": "",
                "logDirectory": "D:/data/logs",
                "networkProxyUrl": "",
                "networkProxyBypass": "localhost,127.0.0.1,::1",
                "automaticArchivalEnabled": true,
                "automaticArchivalInactiveDays": 10,
                "launchOnStartup": false,
                "loggingPolicy": {
                    "retentionDays": 30,
                    "archiveEnabled": true,
                    "redactionEnabled": true,
                    "levels": ["error", "warn", "info", "debug"],
                    "canOpenDirectory": true
                }
            })
        );
    }

    #[test]
    fn setting_input_preserves_string_and_startup_boolean_rules() {
        let string = setting_input(
            serde_json::from_value(json!({ "key": "fontSize", "value": "16px" }))
                .expect("string input"),
        )
        .expect("string mapping");
        let boolean = setting_input(
            serde_json::from_value(json!({ "key": "launchOnStartup", "value": true }))
                .expect("boolean input"),
        )
        .expect("boolean mapping");
        let error = setting_input(
            serde_json::from_value(json!({ "key": "fontSize", "value": 16 }))
                .expect("invalid input DTO"),
        )
        .expect_err("type error");

        assert_eq!(string, ("fontSize".to_string(), "16px".to_string()));
        assert_eq!(boolean, ("launchOnStartup".to_string(), "true".to_string()));
        assert_eq!(
            error.message(),
            "validation error: Invalid setting value type for key 'fontSize'."
        );
    }

    #[test]
    fn environment_responses_keep_camel_case_and_nullable_fields() {
        let data = serde_json::to_value(data_information_to_dto(DataManagementInformation {
            database_path: "D:/data/vanehub.sqlite".to_string(),
            database_directory: "D:/data".to_string(),
            can_open_directory: true,
        }))
        .expect("data DTO");
        let node = serde_json::to_value(node_information_to_dto(NodeInformation {
            available: false,
            path: None,
            version: Some("v22.0.0".to_string()),
            reason: Some("unresolved".to_string()),
        }))
        .expect("node DTO");
        let proxy = serde_json::to_value(detected_proxy_to_dto(DetectedNetworkProxy {
            url: "http://127.0.0.1:7890".to_string(),
            proxy_type: "http".to_string(),
            port: 7890,
        }))
        .expect("proxy DTO");

        assert_eq!(data["databasePath"], "D:/data/vanehub.sqlite");
        assert_eq!(data["canOpenDirectory"], true);
        assert!(node["path"].is_null());
        assert_eq!(node["version"], "v22.0.0");
        assert_eq!(proxy["proxyType"], "http");
        assert!(proxy.get("proxy_type").is_none());
    }

    #[test]
    fn client_log_input_preserves_kebab_case_kind_and_optional_fields() {
        let input: dto::ClientLogEvent = serde_json::from_value(json!({
            "level": "warn",
            "kind": "critical-operation-failure",
            "message": "fixture",
            "source": "settings-test",
            "details": { "operationId": "op-1" }
        }))
        .expect("client log DTO");

        let event = client_log_to_domain(input);

        assert_eq!(event.level, DesktopLogLevel::Warn);
        assert_eq!(event.kind, ClientLogEventKind::CriticalOperationFailure);
        assert_eq!(event.stack, None);
        assert_eq!(
            event
                .details
                .as_ref()
                .and_then(|details| details.get("operationId"))
                .map(String::as_str),
            Some("op-1")
        );
    }

    #[test]
    fn floating_assistant_dtos_preserve_the_existing_json_contract() {
        let runtime = floating_runtime_to_dto(FloatingAssistantPlatform::Windows);
        let config = floating_config_to_dto(FloatingAssistantConfig::new(
            true,
            FloatingAssistantAnchor::new(1280.5, 720.25, Some("DISPLAY1".to_string())),
        ));
        let event = dto::FloatingAssistantEvent::ConfigurationChanged {
            config: config.clone(),
        };

        assert_eq!(
            serde_json::to_value(runtime).expect("runtime"),
            serde_json::json!({"nativeAvailable": true, "platform": "windows"})
        );
        assert_eq!(
            serde_json::to_value(config).expect("config"),
            serde_json::json!({
                "enabled": true,
                "anchor": {"x": 1280.5, "y": 720.25, "monitorName": "DISPLAY1"}
            })
        );
        assert_eq!(
            serde_json::to_value(event).expect("event"),
            serde_json::json!({
                "kind": "configuration-changed",
                "config": {
                    "enabled": true,
                    "anchor": {"x": 1280.5, "y": 720.25, "monitorName": "DISPLAY1"}
                }
            })
        );
        assert_eq!(
            serde_json::to_value(dto::FloatingAssistantEvent::SurfaceChanged {
                mode: dto::FloatingAssistantSurfaceMode::Chat,
            })
            .expect("surface event"),
            serde_json::json!({"kind": "surface-changed", "mode": "chat"})
        );
    }

    #[test]
    fn floating_main_action_mapping_preserves_the_legacy_validation_error() {
        assert_eq!(
            floating_main_action("settings").expect("settings").as_str(),
            "settings"
        );
        assert_eq!(
            floating_main_action("close")
                .expect_err("invalid")
                .to_string(),
            "validation error: invalid main-window action"
        );
    }
}
