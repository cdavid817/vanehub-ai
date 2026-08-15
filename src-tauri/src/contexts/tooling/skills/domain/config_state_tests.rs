use super::config_schema::{parse_config_schema, SkillConfigSchema, SkillConfigValue};
use super::config_state::{
    classify_drift, readiness_for, resolve_effective, RedactedSecret, SkillConfigDrift,
    SkillConfigProvenance, SkillConfigReadiness, SkillConfigRevision, SkillConfigScope,
    SkillConfigSnapshot, SkillSecretIntent, SkillSecretState,
};

const SCHEMA: &str = "  properties:\n    endpoint:\n      type: string\n      default: https://default.example\n    retries:\n      type: integer\n      default: 3\n    label:\n      type: string\n    api_key:\n      type: string\n      x-vanehub-secret: true\n  required:\n    - endpoint\n    - api_key\n";

fn schema() -> SkillConfigSchema {
    parse_config_schema(SCHEMA).expect("schema")
}

fn text(value: &str) -> SkillConfigValue {
    SkillConfigValue::Text(value.to_string())
}

#[test]
fn project_overrides_one_property_and_the_rest_inherit() {
    let schema = schema();
    let user = vec![
        ("endpoint".to_string(), text("https://user.example")),
        ("label".to_string(), text("user label")),
    ];
    let project = vec![("label".to_string(), text("project label"))];

    let resolved = resolve_effective(&schema, &user, &project, &[]);

    let by_key = |key: &str| {
        resolved
            .iter()
            .find(|property| property.key == key)
            .expect("property")
            .clone()
    };
    assert_eq!(by_key("label").provenance, SkillConfigProvenance::Project);
    assert_eq!(by_key("label").value, Some(text("project label")));
    assert_eq!(by_key("endpoint").provenance, SkillConfigProvenance::User);
    assert_eq!(by_key("endpoint").value, Some(text("https://user.example")));
    assert_eq!(
        by_key("retries").provenance,
        SkillConfigProvenance::SchemaDefault
    );
    assert_eq!(by_key("retries").value, Some(SkillConfigValue::Integer(3)));
}

#[test]
fn a_property_with_no_value_anywhere_stays_missing_rather_than_inventing_one() {
    let resolved = resolve_effective(&schema(), &[], &[], &[]);
    let label = resolved
        .iter()
        .find(|property| property.key == "label")
        .expect("label");

    assert_eq!(label.value, None);
    assert_eq!(label.provenance, SkillConfigProvenance::Missing);
}

#[test]
fn secret_properties_resolve_to_state_only_and_never_carry_a_value() {
    let resolved = resolve_effective(
        &schema(),
        &[],
        &[],
        &[("api_key".to_string(), SkillSecretState::Configured)],
    );
    let secret = resolved
        .iter()
        .find(|property| property.key == "api_key")
        .expect("api_key");

    assert!(secret.secret);
    assert_eq!(secret.value, None);
    assert_eq!(secret.secret_state, Some(SkillSecretState::Configured));
}

#[test]
fn readiness_reports_missing_required_until_every_required_property_resolves() {
    let schema = schema();

    let nothing_stored = resolve_effective(&schema, &[], &[], &[]);
    assert_eq!(
        readiness_for(&schema, &nothing_stored, SkillConfigDrift::Compatible),
        SkillConfigReadiness::MissingRequired
    );

    let complete = resolve_effective(
        &schema,
        &[],
        &[],
        &[("api_key".to_string(), SkillSecretState::Configured)],
    );
    assert_eq!(
        readiness_for(&schema, &complete, SkillConfigDrift::Compatible),
        SkillConfigReadiness::Ready
    );
}

#[test]
fn drift_blocks_activation_without_deleting_or_coercing_stored_values() {
    let schema = schema();
    let resolved = resolve_effective(
        &schema,
        &[],
        &[],
        &[("api_key".to_string(), SkillSecretState::Configured)],
    );

    assert_eq!(
        readiness_for(&schema, &resolved, SkillConfigDrift::MigrationRequired),
        SkillConfigReadiness::MigrationRequired
    );
    assert_eq!(
        readiness_for(&schema, &resolved, SkillConfigDrift::Invalid),
        SkillConfigReadiness::Invalid
    );
    assert!(!SkillConfigReadiness::MigrationRequired.permits_activation());
    assert!(SkillConfigReadiness::Ready.permits_activation());
    assert!(SkillConfigReadiness::NotConfigurable.permits_activation());
}

#[test]
fn adding_an_optional_property_stays_compatible() {
    let widened = parse_config_schema(
        "  properties:\n    endpoint:\n      type: string\n      default: https://default.example\n    retries:\n      type: integer\n      default: 3\n    label:\n      type: string\n    added:\n      type: string\n      default: fresh\n    api_key:\n      type: string\n      x-vanehub-secret: true\n  required:\n    - endpoint\n    - api_key\n",
    )
    .expect("widened schema");

    assert_eq!(
        classify_drift(
            &widened,
            &[("endpoint".to_string(), text("https://stored.example"))],
            &["api_key".to_string()],
        ),
        SkillConfigDrift::Compatible
    );
}

#[test]
fn removed_retyped_and_reclassified_properties_require_migration() {
    let schema = schema();

    assert_eq!(
        classify_drift(&schema, &[("gone".to_string(), text("value"))], &[]),
        SkillConfigDrift::MigrationRequired
    );
    assert_eq!(
        classify_drift(
            &schema,
            &[("retries".to_string(), text("not an integer"))],
            &[],
        ),
        SkillConfigDrift::MigrationRequired
    );
    // A value stored in SQLite for what is now a secret property must not be reused.
    assert_eq!(
        classify_drift(&schema, &[("api_key".to_string(), text("leaked"))], &[]),
        SkillConfigDrift::MigrationRequired
    );
    // ...and a credential kept for what is now a plain property must not be reused either.
    assert_eq!(
        classify_drift(&schema, &[], &["endpoint".to_string()]),
        SkillConfigDrift::MigrationRequired
    );
}

#[test]
fn snapshots_are_order_independent_and_digest_their_content() {
    let schema = schema();
    let resolved = resolve_effective(
        &schema,
        &[("endpoint".to_string(), text("https://user.example"))],
        &[],
        &[("api_key".to_string(), SkillSecretState::Configured)],
    );
    let snapshot = SkillConfigSnapshot::new(
        "configured-skill",
        "rev-1",
        &schema.hash,
        Some("workspace-1".to_string()),
        resolved.clone(),
        SkillConfigReadiness::Ready,
    );

    let mut reordered = resolved.clone();
    reordered.reverse();
    let same = SkillConfigSnapshot::new(
        "configured-skill",
        "rev-1",
        &schema.hash,
        Some("workspace-1".to_string()),
        reordered,
        SkillConfigReadiness::Ready,
    );
    assert_eq!(snapshot.digest(), same.digest());
    assert_eq!(snapshot.digest().len(), 64);

    let other_workspace = SkillConfigSnapshot::new(
        "configured-skill",
        "rev-1",
        &schema.hash,
        Some("workspace-2".to_string()),
        resolved,
        SkillConfigReadiness::Ready,
    );
    assert_ne!(snapshot.digest(), other_workspace.digest());

    assert_eq!(snapshot.skill_id(), "configured-skill");
    assert_eq!(snapshot.base_revision(), "rev-1");
    assert_eq!(snapshot.schema_hash(), schema.hash);
    assert_eq!(snapshot.workspace(), Some("workspace-1"));
    assert_eq!(snapshot.readiness(), SkillConfigReadiness::Ready);
    assert_eq!(snapshot.properties().len(), 4);
    assert_eq!(snapshot.property("api_key").expect("api_key").value, None);
}

#[test]
fn revisions_advance_monotonically_and_refuse_to_wrap() {
    let revision = SkillConfigRevision::INITIAL;
    assert_eq!(revision.value(), 0);

    let next = revision.next().expect("advance");
    assert_eq!(next.value(), 1);
    assert!(next > revision);

    assert!(SkillConfigRevision::new(u64::MAX).next().is_err());
}

#[test]
fn secret_values_do_not_appear_in_debug_output() {
    let secret = RedactedSecret::new("super-secret-token");

    assert!(!format!("{secret:?}").contains("super-secret-token"));
    assert_eq!(secret.expose(), "super-secret-token");

    // The intent enum is compared, never printed, so replacement carries the value plainly.
    assert_eq!(
        SkillSecretIntent::Replace("value".to_string()),
        SkillSecretIntent::Replace("value".to_string())
    );
    assert_ne!(SkillSecretIntent::Preserve, SkillSecretIntent::Clear);
}

#[test]
fn scope_and_state_names_are_stable_wire_values() {
    assert_eq!(SkillConfigScope::User.as_str(), "user");
    assert_eq!(SkillConfigScope::Project.as_str(), "project");
    assert_eq!(SkillConfigProvenance::SchemaDefault.as_str(), "default");
    assert_eq!(SkillSecretState::Configured.as_str(), "configured");
    assert_eq!(SkillSecretState::Missing.as_str(), "missing");
    assert_eq!(SkillSecretState::Error.as_str(), "error");
    assert_eq!(
        SkillConfigDrift::MigrationRequired.as_str(),
        "migration-required"
    );
    assert_eq!(SkillConfigReadiness::Invalid.as_str(), "invalid");
}
