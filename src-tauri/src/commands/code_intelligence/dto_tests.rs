use super::dto::*;
use serde_json::json;

#[test]
fn configuration_contract_uses_stable_language_ids_and_camel_case_fields() {
    let configuration = LspConfigurationDto {
        enabled: true,
        languages: vec![
            LspLanguageConfigurationDto {
                language: LspLanguageIdDto::Rust,
                enabled: true,
                executable_override: Some("C:/tools/rust-analyzer.exe".to_string()),
                initialization_options: json!({"check": {"command": "clippy"}}),
            },
            LspLanguageConfigurationDto {
                language: LspLanguageIdDto::TypeScriptJavaScript,
                enabled: false,
                executable_override: None,
                initialization_options: json!({}),
            },
        ],
    };

    let value = serde_json::to_value(&configuration).expect("serialize configuration");
    assert_eq!(value["languages"][0]["language"], "rust");
    assert_eq!(value["languages"][1]["language"], "typescript_javascript");
    assert_eq!(
        value["languages"][0]["executableOverride"],
        "C:/tools/rust-analyzer.exe"
    );
    assert_eq!(
        serde_json::from_value::<LspConfigurationDto>(value.clone())
            .expect("deserialize configuration"),
        configuration
    );
}

#[test]
fn trust_and_discovery_contracts_use_camel_case_and_safe_reason_codes() {
    let trust = LspWorkspaceTrustDto {
        canonical_root: "C:/workspace".to_string(),
        trusted: true,
        revision: 3,
    };
    assert_eq!(
        serde_json::to_value(trust).expect("serialize trust"),
        json!({
            "canonicalRoot": "C:/workspace",
            "trusted": true,
            "revision": 3
        })
    );
    assert_eq!(
        serde_json::from_value::<LspWorkspaceTrustUpdateDto>(json!({
            "canonicalRoot": "C:/workspace",
            "trusted": false
        }))
        .expect("deserialize trust update"),
        LspWorkspaceTrustUpdateDto {
            canonical_root: "C:/workspace".to_string(),
            trusted: false,
        }
    );

    let discovery = LspServerDiscoveryDto {
        language: LspLanguageIdDto::Rust,
        server: LspServerKindDto::RustAnalyzer,
        availability: LspDiscoveryAvailabilityDto::Unavailable,
        executable_path: None,
        arguments: Vec::new(),
        reason_code: Some(LspSafeReasonCodeDto::ExecutableNotFound),
    };
    let value = serde_json::to_value(discovery).expect("serialize discovery");
    assert_eq!(value["availability"], "unavailable");
    assert_eq!(value["reasonCode"], "executable_not_found");
    assert!(value["executablePath"].is_null());
}

#[test]
fn server_test_contract_serializes_phases_and_optional_negotiated_capabilities() {
    assert_eq!(
        serde_json::from_value::<LspServerTestInputDto>(json!({
            "language": "typescript_javascript"
        }))
        .expect("deserialize server test input"),
        LspServerTestInputDto {
            language: LspLanguageIdDto::TypeScriptJavaScript,
        }
    );
    let result = LspServerTestResultDto {
        server: LspServerKindDto::TypeScriptLanguageServer,
        phases: vec![LspServerTestPhaseResultDto {
            phase: LspServerTestPhaseDto::Initialize,
            status: LspServerTestPhaseStatusDto::Failed,
            reason_code: Some(LspSafeReasonCodeDto::InitializeTimedOut),
        }],
        negotiated_capabilities: None,
    };
    let value = serde_json::to_value(result).expect("serialize server test");
    assert_eq!(value["server"], "typescript_language_server");
    assert_eq!(value["phases"][0]["phase"], "initialize");
    assert_eq!(value["phases"][0]["status"], "failed");
    assert_eq!(value["phases"][0]["reasonCode"], "initialize_timed_out");
    assert!(value["negotiatedCapabilities"].is_null());
}

#[test]
fn status_contract_covers_every_process_state_and_optional_capabilities() {
    let states = [
        LspProcessStateDto::Absent,
        LspProcessStateDto::Starting,
        LspProcessStateDto::Initializing,
        LspProcessStateDto::Ready,
        LspProcessStateDto::Stopping,
        LspProcessStateDto::Backoff,
        LspProcessStateDto::Failed,
    ];
    assert_eq!(
        serde_json::to_value(states).expect("serialize states"),
        json!([
            "absent",
            "starting",
            "initializing",
            "ready",
            "stopping",
            "backoff",
            "failed"
        ])
    );

    let status = LspServerStatusDto {
        language: LspLanguageIdDto::Rust,
        server: LspServerKindDto::RustAnalyzer,
        relative_project_root: "crates/core".to_string(),
        state: LspProcessStateDto::Ready,
        restart_count: 1,
        last_response_at: Some("2026-08-10T08:01:02Z".to_string()),
        diagnostic_count: 4,
        reason_code: None,
        negotiated_capabilities: Some(LspNegotiatedCapabilitiesDto {
            position_encoding: LspPositionEncodingDto::Utf16,
            document_sync: LspDocumentSyncDto::Incremental,
            definition: true,
            references: true,
            hover: true,
            diagnostics: true,
        }),
    };
    let value = serde_json::to_value(status).expect("serialize status");
    assert_eq!(value["relativeProjectRoot"], "crates/core");
    assert_eq!(value["lastResponseAt"], "2026-08-10T08:01:02Z");
    assert_eq!(value["negotiatedCapabilities"]["positionEncoding"], "utf16");
    assert_eq!(
        value["negotiatedCapabilities"]["documentSync"],
        "incremental"
    );
}
