use super::dto::*;
use crate::contexts::code_intelligence::api::{
    LanguageConfiguration, LspConfiguration, LspLanguageId, LANGUAGE_DEFINITIONS,
};
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn configuration_contract_uses_stable_language_ids_and_camel_case_fields() {
    let configuration = LspConfigurationDto {
        enabled: true,
        languages: vec![
            LspLanguageConfigurationDto {
                language: "rust".to_owned(),
                enabled: true,
                executable_override: Some("C:/tools/rust-analyzer.exe".to_string()),
                startup_arguments: None,
                initialization_options: json!({"check": {"command": "clippy"}}),
            },
            LspLanguageConfigurationDto {
                language: "typescript_javascript".to_owned(),
                enabled: false,
                executable_override: None,
                startup_arguments: Some(vec!["--stdio".to_owned()]),
                initialization_options: json!({}),
            },
        ],
        descriptors: Vec::new(),
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
fn a_caller_may_omit_descriptors_and_startup_arguments_when_saving() {
    // The desktop end-to-end layer calls `save_lsp_configuration` through `core.invoke` directly,
    // without going through the frontend adapter, so its payload carries neither field. Requiring
    // `descriptors` would reject that caller for restating a fact the backend authored, and
    // requiring `startupArguments` would reject one written before the field existed.
    let configuration = serde_json::from_value::<LspConfigurationDto>(json!({
        "enabled": true,
        "languages": [{
            "language": "rust",
            "enabled": true,
            "executableOverride": null,
            "initializationOptions": {}
        }]
    }))
    .expect("a payload without descriptors or startup arguments deserializes");

    assert!(configuration.enabled);
    assert!(configuration.descriptors.is_empty());
    assert_eq!(configuration.languages[0].startup_arguments, None);

    // Descriptors are output only: a caller that sends them is not trusted to define them.
    let ignored = serde_json::from_value::<LspConfigurationDto>(json!({
        "enabled": false,
        "languages": [],
        "descriptors": [{
            "language": "go",
            "server": "gopls",
            "supportedOnHost": true,
            "defaultStartupArguments": []
        }]
    }))
    .expect("descriptors sent by a caller are accepted and discarded");
    assert!(ignored.descriptors.is_empty());
}

#[test]
fn unset_and_empty_startup_arguments_are_distinguishable_on_the_wire() {
    // The whole point of the nullable column is that these two survive a round trip as different
    // values. If serde collapsed them, clearing the field in the UI would silently mean "use the
    // registry default" and `--stdio` would come back.
    let unset = LspLanguageConfigurationDto {
        language: "rust".to_owned(),
        enabled: true,
        executable_override: None,
        startup_arguments: None,
        initialization_options: json!({}),
    };
    let empty = LspLanguageConfigurationDto {
        startup_arguments: Some(Vec::new()),
        ..unset.clone()
    };

    let unset_value = serde_json::to_value(&unset).expect("serialize unset");
    let empty_value = serde_json::to_value(&empty).expect("serialize empty");
    assert!(unset_value["startupArguments"].is_null());
    assert_eq!(empty_value["startupArguments"], json!([]));
    assert_ne!(unset_value, empty_value);
    assert_eq!(
        serde_json::from_value::<LspLanguageConfigurationDto>(unset_value).expect("round trip"),
        unset
    );
    assert_eq!(
        serde_json::from_value::<LspLanguageConfigurationDto>(empty_value).expect("round trip"),
        empty
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
        language: "rust".to_owned(),
        server: "rust_analyzer".to_owned(),
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
            language: "typescript_javascript".to_owned(),
        }
    );
    let result = LspServerTestResultDto {
        server: "typescript_language_server".to_owned(),
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

// The assertions above check individual fields, so an added field passes them silently. These
// three pin the whole serialized object for each command result instead, which is what makes a
// contract change show up as a reviewable diff rather than as nothing at all.
#[test]
fn get_lsp_configuration_result_serializes_to_an_exact_object() {
    // One configured language, pinned exactly. The list used to hold every registered language,
    // which meant this expectation had to be rewritten each time one was added and grew a table
    // that nobody would read. What it needs to catch is a field appearing, disappearing, or
    // changing shape, and one entry catches that as well as five.
    let mut configuration = LspConfiguration {
        enabled: true,
        languages: BTreeMap::new(),
    };
    configuration.languages.insert(
        LspLanguageId::new("rust").expect("rust language id"),
        LanguageConfiguration {
            enabled: true,
            executable_override: Some("C:/tools/rust-analyzer.exe".to_owned()),
            startup_arguments: None,
            initialization_options: json!({"check": {"command": "clippy"}}),
        },
    );

    let value = serde_json::to_value(LspConfigurationDto::from(configuration)).expect("serialize");
    assert_eq!(
        value["languages"],
        json!([{
            "language": "rust",
            "enabled": true,
            "executableOverride": "C:/tools/rust-analyzer.exe",
            "startupArguments": null,
            "initializationOptions": {"check": {"command": "clippy"}}
        }])
    );
    assert_eq!(value["enabled"], json!(true));
    assert_eq!(
        // serde_json orders object keys, so this is the key set rather than the field order.
        value
            .as_object()
            .expect("object")
            .keys()
            .collect::<Vec<_>>(),
        vec!["descriptors", "enabled", "languages"]
    );
}

#[test]
fn get_lsp_configuration_describes_every_registered_language() {
    // The descriptor list comes from the registry, so asserting a literal here would only restate
    // the registry and would have to be edited whenever it changed. What is worth pinning is the
    // relationship: one descriptor per registered language, in the same order, each with the four
    // fields the settings page renders from.
    let value = serde_json::to_value(LspConfigurationDto::from(LspConfiguration::default()))
        .expect("serialize");
    let descriptors = value["descriptors"].as_array().expect("descriptor array");

    assert_eq!(descriptors.len(), LANGUAGE_DEFINITIONS.len());
    for (descriptor, definition) in descriptors.iter().zip(LANGUAGE_DEFINITIONS) {
        assert_eq!(descriptor["language"], json!(definition.id));
        assert_eq!(descriptor["server"], json!(definition.server_id));
        assert_eq!(
            descriptor["supportedOnHost"],
            json!(definition.supports_host())
        );
        assert_eq!(
            descriptor["defaultStartupArguments"],
            json!(definition.default_startup_arguments)
        );
        assert_eq!(
            descriptor
                .as_object()
                .expect("descriptor object")
                .keys()
                .collect::<Vec<_>>(),
            vec![
                "defaultStartupArguments",
                "language",
                "server",
                "supportedOnHost"
            ]
        );
    }
}

#[test]
fn discover_lsp_servers_result_serializes_to_an_exact_object() {
    let discovered = vec![
        LspServerDiscoveryDto {
            language: "rust".to_owned(),
            server: "rust_analyzer".to_owned(),
            availability: LspDiscoveryAvailabilityDto::Available,
            executable_path: Some("C:/tools/rust-analyzer.exe".to_string()),
            arguments: Vec::new(),
            reason_code: None,
        },
        LspServerDiscoveryDto {
            language: "typescript_javascript".to_owned(),
            server: "typescript_language_server".to_owned(),
            availability: LspDiscoveryAvailabilityDto::Unavailable,
            executable_path: None,
            arguments: vec!["--stdio".to_string()],
            reason_code: Some(LspSafeReasonCodeDto::ExecutableNotFound),
        },
    ];

    assert_eq!(
        serde_json::to_value(&discovered).expect("serialize discovery"),
        json!([
            {
                "language": "rust",
                "server": "rust_analyzer",
                "availability": "available",
                "executablePath": "C:/tools/rust-analyzer.exe",
                "arguments": [],
                "reasonCode": null
            },
            {
                "language": "typescript_javascript",
                "server": "typescript_language_server",
                "availability": "unavailable",
                "executablePath": null,
                "arguments": ["--stdio"],
                "reasonCode": "executable_not_found"
            }
        ])
    );
}

#[test]
fn list_lsp_server_status_result_serializes_to_an_exact_object() {
    let statuses = vec![LspServerStatusDto {
        language: "rust".to_owned(),
        server: "rust_analyzer".to_owned(),
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
    }];

    assert_eq!(
        serde_json::to_value(&statuses).expect("serialize status"),
        json!([{
            "language": "rust",
            "server": "rust_analyzer",
            "relativeProjectRoot": "crates/core",
            "state": "ready",
            "restartCount": 1,
            "lastResponseAt": "2026-08-10T08:01:02Z",
            "diagnosticCount": 4,
            "reasonCode": null,
            "negotiatedCapabilities": {
                "positionEncoding": "utf16",
                "documentSync": "incremental",
                "definition": true,
                "references": true,
                "hover": true,
                "diagnostics": true
            }
        }])
    );
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
        language: "rust".to_owned(),
        server: "rust_analyzer".to_owned(),
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
