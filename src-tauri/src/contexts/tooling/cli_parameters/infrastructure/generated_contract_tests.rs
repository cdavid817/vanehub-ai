//! Registry parity: the committed frontend contract must be a faithful projection of the
//! canonical native registry. A hand edit on either side fails here and in `contracts:check`.

use super::EmbeddedCliParameterCatalog;
use crate::contexts::tooling::cli_parameters::application::ports::CliParameterCatalogPort;
use serde_json::Value;

const GENERATED_CONTRACT: &str =
    include_str!("../../../../../../src/generated/cli-parameter-catalog.json");

/// `CliParameterDefinition::audit` is `skip_serializing`, so the native projection already omits
/// the review prose the generated artifact must not contain.
fn native_projection() -> Value {
    let catalog = EmbeddedCliParameterCatalog
        .catalog()
        .expect("canonical registry");
    serde_json::to_value(catalog.as_ref()).expect("serialize catalog")
}

/// JSON cannot carry a comment header, so the generator marks the artifact with a reserved
/// `$generated` key. It is documentation, not registry data, and is dropped before comparison.
fn without_generated_marker(source: &str) -> Value {
    let mut value: Value = serde_json::from_str(source).expect("generated contract");
    let marker = value
        .as_object_mut()
        .and_then(|object| object.remove("$generated"));
    let marker = marker.expect("the generated artifact must declare itself generated");
    let marker = marker.as_str().expect("marker is text");
    assert!(marker.contains("Do not edit by hand"));
    assert!(marker.contains("contracts:generate"));
    value
}

#[test]
fn the_generated_frontend_contract_matches_the_canonical_registry() {
    assert_eq!(
        native_projection(),
        without_generated_marker(GENERATED_CONTRACT),
        "run `npm run contracts:generate` after changing the canonical registry"
    );
}

#[test]
fn a_stale_generated_contract_is_detected() {
    let mut stale = without_generated_marker(GENERATED_CONTRACT);
    stale["catalogVersion"] = Value::String("0.0.0-stale".to_string());
    assert_ne!(native_projection(), stale);
}

#[test]
fn the_generated_contract_carries_no_audit_prose() {
    let generated: Value = serde_json::from_str(GENERATED_CONTRACT).expect("generated contract");
    let encoded = generated.to_string();
    assert!(!encoded.contains("\"audit\""));
    assert!(!encoded.contains("reviewedAt"));
}

#[test]
fn the_generated_contract_exposes_the_same_ownership_and_scopes() {
    let generated: Value = serde_json::from_str(GENERATED_CONTRACT).expect("generated contract");
    let catalog = EmbeddedCliParameterCatalog
        .catalog()
        .expect("canonical registry");
    for agent in generated["agents"].as_array().expect("agents") {
        let agent_id = agent["agentId"].as_str().expect("agent id");
        for parameter in agent["parameters"].as_array().expect("parameters") {
            let parameter_id = parameter["id"].as_str().expect("parameter id");
            let definition = catalog
                .definition(agent_id, parameter_id)
                .expect("native definition");
            assert_eq!(
                parameter["ownership"].as_str().expect("ownership"),
                serde_json::to_value(definition.ownership)
                    .expect("ownership")
                    .as_str()
                    .expect("ownership string")
            );
            assert_eq!(
                parameter["launchScopes"].as_array().expect("scopes").len(),
                definition.launch_scopes.len()
            );
        }
    }
}
