//! Serialized shape of the capability-gate command contract.
//!
//! The frontend and the Web/mock adapter both encode these; a silent rename or a collapsed status
//! member would only surface as a runtime parse failure otherwise.

use super::dto::{
    ExtensionPlatformFeatureDto, FeatureGateDto, FeatureGateFreshnessDto, FeatureGateOverviewDto,
    FeatureGateStatusDto, SetFeatureGateRequest,
};
use serde_json::{json, Value};

#[test]
fn feature_identifiers_serialize_as_stable_snake_case() {
    let encoded = serde_json::to_value(ExtensionPlatformFeatureDto::WasmModuleRuntime)
        .expect("feature should encode");
    assert_eq!(encoded, Value::String("wasm_module_runtime".to_string()));

    let decoded: ExtensionPlatformFeatureDto =
        serde_json::from_value(json!("external_packages")).expect("feature should decode");
    assert_eq!(decoded, ExtensionPlatformFeatureDto::ExternalPackages);
}

#[test]
fn status_is_a_tagged_union_that_keeps_not_compiled_distinct_from_runtime_disabled() {
    let not_compiled =
        serde_json::to_value(FeatureGateStatusDto::NotCompiled).expect("status should encode");
    let runtime_disabled =
        serde_json::to_value(FeatureGateStatusDto::RuntimeDisabled).expect("status should encode");

    assert_eq!(not_compiled, json!({ "kind": "not_compiled" }));
    assert_eq!(runtime_disabled, json!({ "kind": "runtime_disabled" }));
    assert_ne!(not_compiled, runtime_disabled);
}

#[test]
fn blocked_and_forced_statuses_carry_their_reason() {
    let blocked = serde_json::to_value(FeatureGateStatusDto::BlockedByPrerequisite {
        reason: "sandbox_self_test_unavailable".to_string(),
    })
    .expect("status should encode");
    assert_eq!(
        blocked,
        json!({
            "kind": "blocked_by_prerequisite",
            "reason": "sandbox_self_test_unavailable"
        })
    );

    let forced = serde_json::to_value(FeatureGateStatusDto::ForcedDisabled {
        reason: "incident".to_string(),
    })
    .expect("status should encode");
    assert_eq!(
        forced,
        json!({ "kind": "forced_disabled", "reason": "incident" })
    );
}

#[test]
fn a_gate_serializes_with_camel_case_fields() {
    let overview = FeatureGateOverviewDto {
        gates: vec![FeatureGateDto {
            feature: ExtensionPlatformFeatureDto::Catalog,
            status: FeatureGateStatusDto::RuntimeDisabled,
            build_available: true,
            desired_enabled: false,
            revision: 0,
            updated_at: None,
            updated_by: None,
            reason: None,
        }],
        freshness: FeatureGateFreshnessDto::Current,
    };

    let encoded = serde_json::to_value(&overview).expect("overview should encode");
    assert_eq!(
        encoded,
        json!({
            "freshness": { "kind": "current" },
            "gates": [{
                "feature": "catalog",
                "status": { "kind": "runtime_disabled" },
                "buildAvailable": true,
                "desiredEnabled": false,
                "revision": 0,
                "updatedAt": null,
                "updatedBy": null,
                "reason": null
            }]
        })
    );
}

#[test]
fn freshness_is_a_tagged_union_that_names_its_degradation() {
    let degraded = serde_json::to_value(FeatureGateFreshnessDto::Degraded {
        degradation: "reload_failed".to_string(),
    })
    .expect("freshness should encode");

    assert_eq!(
        degraded,
        json!({ "kind": "degraded", "degradation": "reload_failed" })
    );
    assert_ne!(
        degraded,
        serde_json::to_value(FeatureGateFreshnessDto::Current).expect("freshness should encode")
    );
}

#[test]
fn a_set_request_requires_the_observed_revision() {
    let decoded: SetFeatureGateRequest = serde_json::from_value(json!({
        "feature": "connectors",
        "desiredEnabled": true,
        "expectedRevision": 3,
        "reason": "gate 4 parity"
    }))
    .expect("request should decode");

    assert_eq!(decoded.feature, ExtensionPlatformFeatureDto::Connectors);
    assert!(decoded.desired_enabled);
    assert_eq!(decoded.expected_revision, 3);
    assert_eq!(decoded.reason.as_deref(), Some("gate 4 parity"));

    // Omitting the revision is a decode failure rather than a defaulted overwrite of whatever
    // state happens to be current.
    let missing_revision = serde_json::from_value::<SetFeatureGateRequest>(json!({
        "feature": "connectors",
        "desiredEnabled": true
    }));
    assert!(missing_revision.is_err());
}
