use super::dto::{ConnectorView, SaveConnectorInput, WeChatAuthorizationView};
use crate::contexts::communications::domain::{
    builtin_descriptors, ConnectorConfig, ConnectorHealth, ConnectorKind, ConnectorLifecycle,
};

#[test]
fn connector_command_dtos_preserve_the_frontend_json_contract() {
    let input: SaveConnectorInput = serde_json::from_value(serde_json::json!({
        "kind": "weixin",
        "enabled": true,
        "displayName": "Work WeChat",
        "publicConfig": { "region": "cn" },
        "credentials": { "botToken": "fixture-secret" }
    }))
    .expect("save input");
    assert_eq!(input.kind, ConnectorKind::WeChat);
    assert_eq!(
        input
            .credentials
            .as_ref()
            .and_then(|values| values.get("botToken"))
            .map(String::as_str),
        Some("fixture-secret")
    );

    let view = ConnectorView {
        descriptor: builtin_descriptors()
            .into_iter()
            .find(|descriptor| descriptor.kind == ConnectorKind::WeChat)
            .expect("descriptor"),
        config: ConnectorConfig {
            kind: ConnectorKind::WeChat,
            enabled: true,
            display_name: Some("Work WeChat".to_string()),
            public_config: serde_json::json!({ "region": "cn" }),
            credential_ref: Some("weixin/default".to_string()),
        },
        health: ConnectorHealth {
            kind: ConnectorKind::WeChat,
            lifecycle: ConnectorLifecycle::Connected,
            generation: 2,
            safe_error_code: None,
            updated_at: "2026-07-18T00:00:00Z".to_string(),
        },
        has_credentials: true,
    };
    let serialized = serde_json::to_value(view).expect("connector view");
    assert_eq!(serialized["descriptor"]["kind"], "weixin");
    assert_eq!(serialized["config"]["displayName"], "Work WeChat");
    assert_eq!(
        serialized["health"]["safeErrorCode"],
        serde_json::Value::Null
    );
    assert_eq!(serialized["hasCredentials"], true);
    assert!(serialized.get("configuration").is_none());
}

#[test]
fn authorization_view_uses_existing_camel_case_fields_without_credentials() {
    let value = serde_json::to_value(WeChatAuthorizationView {
        status: "waiting".to_string(),
        image_data_url: Some("data:image/svg+xml;base64,fixture".to_string()),
        expires_at: Some("2026-07-18T00:05:00Z".to_string()),
        safe_error_code: None,
    })
    .expect("authorization view");

    assert_eq!(value["status"], "waiting");
    assert!(value.get("imageDataUrl").is_some());
    assert!(value.get("expiresAt").is_some());
    assert!(value.get("safeErrorCode").is_some());
    assert!(value.get("credentials").is_none());
}
