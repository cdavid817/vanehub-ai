use super::{
    consumption_for_binding, evolution_signal, overlay_schema_changed,
    render_configuration_subsection, ConfigurationConsumption, ConfigurationSubsection,
    MAX_CONFIGURATION_SUBSECTION_CHARACTERS,
};
use crate::contexts::tooling::skills::domain::{
    parse_config_schema, SkillConfigProperty, SkillConfigProvenance, SkillConfigReadiness,
    SkillConfigSchema, SkillConfigSnapshot, SkillConfigValue, SkillSecretState,
};

const SCHEMA: &str = "  properties:\n    endpoint:\n      type: string\n    retries:\n      type: integer\n    api_key:\n      type: string\n      x-vanehub-secret: true\n";
const SECRET: &str = "sk-boundary-secret";

fn schema() -> SkillConfigSchema {
    parse_config_schema(SCHEMA).expect("schema")
}

fn property(key: &str, value: Option<SkillConfigValue>) -> SkillConfigProperty {
    SkillConfigProperty {
        key: key.to_string(),
        value,
        provenance: SkillConfigProvenance::User,
        secret: false,
        secret_state: None,
    }
}

fn secret_property(key: &str, state: SkillSecretState) -> SkillConfigProperty {
    SkillConfigProperty {
        key: key.to_string(),
        value: None,
        provenance: SkillConfigProvenance::User,
        secret: true,
        secret_state: Some(state),
    }
}

fn snapshot(properties: Vec<SkillConfigProperty>) -> SkillConfigSnapshot {
    SkillConfigSnapshot::new(
        "configured-skill",
        "rev-1",
        &schema().hash,
        None,
        properties,
        SkillConfigReadiness::Ready,
    )
}

#[test]
fn the_subsection_is_ordered_by_key_and_shows_secrets_as_state_only() {
    let rendered = render_configuration_subsection(Some(&snapshot(vec![
        property(
            "endpoint",
            Some(SkillConfigValue::Text("https://live.example".to_string())),
        ),
        property("retries", Some(SkillConfigValue::Integer(5))),
        secret_property("api_key", SkillSecretState::Configured),
    ])));

    match rendered {
        ConfigurationSubsection::Rendered(block) => {
            assert_eq!(
                block,
                "api_key: <configured>\nendpoint: https://live.example\nretries: 5"
            );
            assert!(!block.contains(SECRET));
        }
        other => panic!("expected a rendered subsection, got {other:?}"),
    }
}

#[test]
fn a_missing_secret_renders_its_state_rather_than_being_omitted() {
    let rendered = render_configuration_subsection(Some(&snapshot(vec![secret_property(
        "api_key",
        SkillSecretState::Missing,
    )])));

    // The model has to be able to tell "not configured" from "not declared".
    assert_eq!(
        rendered,
        ConfigurationSubsection::Rendered("api_key: <missing>".to_string())
    );
}

#[test]
fn a_skill_with_no_snapshot_or_no_values_contributes_no_subsection() {
    assert_eq!(
        render_configuration_subsection(None),
        ConfigurationSubsection::Omitted
    );
    assert_eq!(
        render_configuration_subsection(Some(&snapshot(vec![property("endpoint", None)]))),
        ConfigurationSubsection::Omitted
    );
}

#[test]
fn an_over_budget_subsection_is_reported_rather_than_trimmed() {
    let oversized = render_configuration_subsection(Some(&snapshot(vec![property(
        "endpoint",
        Some(SkillConfigValue::Text(
            "a".repeat(MAX_CONFIGURATION_SUBSECTION_CHARACTERS + 1),
        )),
    )])));

    // Trimming would present a partial value as if it were the configured one.
    assert!(matches!(
        oversized,
        ConfigurationSubsection::OverBudget { .. }
    ));
}

#[test]
fn an_external_cli_binding_reports_configuration_consumption_as_unsupported() {
    assert_eq!(
        consumption_for_binding(true),
        ConfigurationConsumption::Native
    );
    assert!(ConfigurationConsumption::Native.is_supported());

    let external = consumption_for_binding(false);
    assert_eq!(external, ConfigurationConsumption::UnsupportedExternalCli);
    // Stated, not implicit: the UI has to be able to say so rather than let a user assume their
    // values reach the CLI.
    assert!(!external.is_supported());
    assert_eq!(external.as_str(), "unsupported-external-cli");
}

#[test]
fn a_mounted_skill_document_carries_the_schema_but_never_a_value() {
    use crate::contexts::tooling::skills::domain::SkillMetadata;

    let metadata = SkillMetadata::new(
        "configured-skill",
        "Configured",
        "Has settings",
        "testing",
        "1.0.0",
        Vec::new(),
    )
    .expect("metadata")
    .with_config_schema_block(Some(SCHEMA.to_string()));

    // The document is what an external CLI mount receives. Values live in SQLite and the
    // credential store, and there is no code path that writes either into this text.
    let composed = format!("{metadata:?}");
    assert!(composed.contains("config_schema_block"));
    assert!(!composed.contains(SECRET));
    assert!(!composed.contains("https://live.example"));
}

#[test]
fn an_overlay_that_rewrites_the_schema_produces_a_different_effective_schema() {
    let original = schema();
    let widened = parse_config_schema(
        "  properties:\n    endpoint:\n      type: string\n    retries:\n      type: integer\n    api_key:\n      type: string\n      x-vanehub-secret: true\n    added:\n      type: boolean\n",
    )
    .expect("widened");

    assert!(overlay_schema_changed(Some(&original), Some(&widened)));
    assert!(!overlay_schema_changed(Some(&original), Some(&schema())));
    // Declaring or removing a schema entirely is also a change.
    assert!(overlay_schema_changed(None, Some(&original)));
    assert!(overlay_schema_changed(Some(&original), None));
    assert!(!overlay_schema_changed(None, None));
}

#[test]
fn an_evolution_signal_carries_shape_and_readiness_but_no_configuration() {
    let schema = schema();
    let signal = evolution_signal(
        Some(&schema),
        Some(&snapshot(vec![
            property(
                "endpoint",
                Some(SkillConfigValue::Text("https://live.example".to_string())),
            ),
            secret_property("api_key", SkillSecretState::Configured),
        ])),
    );

    assert!(signal.configurable);
    assert_eq!(signal.property_count, 3);
    assert_eq!(signal.secret_property_count, 1);
    assert!(signal.ready);

    let rendered = format!("{signal:?}");
    // No value, no property key, no alias — a candidate cannot be seeded from operational settings.
    assert!(!rendered.contains("https://live.example"));
    assert!(!rendered.contains("endpoint"));
    assert!(!rendered.contains("api_key"));
}

#[test]
fn an_unconfigurable_skill_reports_a_ready_empty_signal() {
    let signal = evolution_signal(None, None);

    assert!(!signal.configurable);
    assert_eq!(signal.property_count, 0);
    assert_eq!(signal.secret_property_count, 0);
    // Nothing to configure cannot be the reason a Skill is held back.
    assert!(signal.ready);
}
