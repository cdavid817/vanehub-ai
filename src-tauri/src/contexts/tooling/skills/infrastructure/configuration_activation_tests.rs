use super::{
    render_snapshot_block, resolve_activation_snapshot, ActivationRefusal, SkillActivationContext,
    MAX_SNAPSHOT_VALUE_BYTES,
};
use crate::contexts::tooling::skills::domain::{
    parse_config_schema, SkillConfigDrift, SkillConfigReadiness, SkillConfigSchema,
    SkillConfigScope, SkillConfigValue, SkillSecretState,
};
use crate::contexts::tooling::skills::infrastructure::{
    SkillConfigurationSave, SqliteSkillConfigurationRepository,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

const SCHEMA: &str = "  properties:\n    endpoint:\n      type: string\n      default: https://default.example\n    retries:\n      type: integer\n      default: 3\n    api_key:\n      type: string\n      x-vanehub-secret: true\n  required:\n    - endpoint\n    - api_key\n";
const OPTIONAL_SCHEMA: &str =
    "  properties:\n    endpoint:\n      type: string\n      default: https://default.example\n";

fn schema() -> SkillConfigSchema {
    parse_config_schema(SCHEMA).expect("schema")
}

fn text(value: &str) -> SkillConfigValue {
    SkillConfigValue::Text(value.to_string())
}

fn harness() -> (TempDirectory, SqliteSkillConfigurationRepository) {
    let directory = TempDirectory::new("skill-configuration-activation");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("open test database");
    (directory, SqliteSkillConfigurationRepository::new(database))
}

fn seed(
    repository: &SqliteSkillConfigurationRepository,
    schema: &SkillConfigSchema,
    skill_id: &str,
    values: Vec<(String, SkillConfigValue)>,
    secret_keys: Vec<String>,
    expected_revision: Option<crate::contexts::tooling::skills::domain::SkillConfigRevision>,
) {
    repository
        .save(&SkillConfigurationSave {
            skill_id: skill_id.to_string(),
            scope: SkillConfigScope::User,
            workspace_identity: String::new(),
            schema_hash: schema.hash.clone(),
            base_revision: "rev-1".to_string(),
            expected_revision,
            values,
            secret_keys,
            validation_state: SkillConfigDrift::Compatible,
        })
        .expect("seed");
}

#[test]
fn a_ready_configuration_produces_a_revision_bound_snapshot() {
    let (_directory, repository) = harness();
    let schema = schema();
    seed(
        &repository,
        &schema,
        "configured-skill",
        vec![("endpoint".to_string(), text("https://live.example"))],
        vec!["api_key".to_string()],
        None,
    );

    let snapshot = resolve_activation_snapshot(
        &repository,
        &schema,
        "configured-skill",
        "rev-7",
        "/workspace/one",
    )
    .expect("resolve")
    .expect("ready");

    assert_eq!(snapshot.skill_id(), "configured-skill");
    assert_eq!(snapshot.base_revision(), "rev-7");
    assert_eq!(snapshot.schema_hash(), schema.hash);
    assert_eq!(snapshot.workspace(), Some("/workspace/one"));
    assert_eq!(snapshot.readiness(), SkillConfigReadiness::Ready);
    assert_eq!(snapshot.digest().len(), 64);
    assert_eq!(
        snapshot.property("api_key").expect("secret").secret_state,
        Some(SkillSecretState::Configured)
    );
    assert_eq!(snapshot.property("api_key").expect("secret").value, None);
}

#[test]
fn a_snapshot_held_by_an_activation_does_not_follow_later_edits() {
    let (_directory, repository) = harness();
    let schema = schema();
    seed(
        &repository,
        &schema,
        "configured-skill",
        vec![("endpoint".to_string(), text("original"))],
        vec!["api_key".to_string()],
        None,
    );
    let context = SkillActivationContext::new(
        resolve_activation_snapshot(&repository, &schema, "configured-skill", "rev-1", "")
            .expect("resolve")
            .expect("ready"),
    );

    // The user edits configuration while the Role context is loaded.
    seed(
        &repository,
        &schema,
        "configured-skill",
        vec![("endpoint".to_string(), text("edited"))],
        vec!["api_key".to_string()],
        Some(crate::contexts::tooling::skills::domain::SkillConfigRevision::new(1)),
    );

    assert_eq!(
        context
            .snapshot()
            .property("endpoint")
            .expect("endpoint")
            .value,
        Some(text("original"))
    );
    // A later activation is where the edit takes effect.
    let later = resolve_activation_snapshot(&repository, &schema, "configured-skill", "rev-1", "")
        .expect("resolve")
        .expect("ready");
    assert_eq!(
        later.property("endpoint").expect("endpoint").value,
        Some(text("edited"))
    );
    assert_ne!(context.snapshot().digest(), later.digest());
}

#[test]
fn a_delegated_child_keeps_the_parents_snapshot_across_a_concurrent_edit() {
    let (_directory, repository) = harness();
    let schema = schema();
    seed(
        &repository,
        &schema,
        "configured-skill",
        vec![("endpoint".to_string(), text("at-delegation"))],
        vec!["api_key".to_string()],
        None,
    );
    let parent = SkillActivationContext::new(
        resolve_activation_snapshot(&repository, &schema, "configured-skill", "rev-1", "")
            .expect("resolve")
            .expect("ready"),
    );

    let child = parent.delegate();
    seed(
        &repository,
        &schema,
        "configured-skill",
        vec![("endpoint".to_string(), text("changed-mid-flight"))],
        vec!["api_key".to_string()],
        Some(crate::contexts::tooling::skills::domain::SkillConfigRevision::new(1)),
    );

    assert_eq!(
        child.property("endpoint").expect("endpoint").value,
        Some(text("at-delegation"))
    );
    assert_eq!(child.digest(), parent.snapshot().digest());
}

#[test]
fn a_missing_required_property_refuses_the_activation_and_names_it() {
    let (_directory, repository) = harness();
    let schema = schema();
    // `endpoint` has a default, but the required secret has no credential.
    seed(
        &repository,
        &schema,
        "configured-skill",
        Vec::new(),
        Vec::new(),
        None,
    );

    let refusal =
        resolve_activation_snapshot(&repository, &schema, "configured-skill", "rev-1", "")
            .expect("resolve")
            .expect_err("refused");

    assert_eq!(refusal.as_str(), "missing-required");
    match refusal {
        ActivationRefusal::MissingRequired { properties } => {
            assert_eq!(properties, vec!["api_key".to_string()]);
        }
        other => panic!("expected missing-required, got {other:?}"),
    }
}

#[test]
fn refusal_reasons_have_stable_wire_names() {
    // The DTO layer and the audit trail both key off these, so they are part of the contract.
    assert_eq!(
        ActivationRefusal::MigrationRequired.as_str(),
        "migration-required"
    );
    assert_eq!(ActivationRefusal::Invalid.as_str(), "invalid");
    assert_eq!(
        ActivationRefusal::Oversized { bytes: 1 }.as_str(),
        "oversized"
    );
    assert_eq!(
        ActivationRefusal::MissingRequired {
            properties: Vec::new()
        }
        .as_str(),
        "missing-required"
    );
}

#[test]
fn drift_refuses_the_activation_before_any_work_begins() {
    let (_directory, repository) = harness();
    let schema = schema();
    seed(
        &repository,
        &schema,
        "configured-skill",
        // A value for a property that this schema declares as an integer.
        vec![("retries".to_string(), text("not an integer"))],
        vec!["api_key".to_string()],
        None,
    );

    let refusal =
        resolve_activation_snapshot(&repository, &schema, "configured-skill", "rev-1", "")
            .expect("resolve")
            .expect_err("refused");

    assert_eq!(refusal, ActivationRefusal::MigrationRequired);
}

#[test]
fn an_oversized_configuration_fails_activation_instead_of_being_truncated() {
    let (_directory, repository) = harness();
    let schema = schema();
    seed(
        &repository,
        &schema,
        "configured-skill",
        vec![(
            "endpoint".to_string(),
            text(&"a".repeat(MAX_SNAPSHOT_VALUE_BYTES + 1)),
        )],
        vec!["api_key".to_string()],
        None,
    );

    let refusal =
        resolve_activation_snapshot(&repository, &schema, "configured-skill", "rev-1", "")
            .expect("resolve")
            .expect_err("refused");

    // Truncating would hand the model a half-written value it would then act on.
    assert!(matches!(refusal, ActivationRefusal::Oversized { .. }));
}

#[test]
fn one_skills_refusal_leaves_another_skill_activatable() {
    let (_directory, repository) = harness();
    let schema = schema();
    let optional = parse_config_schema(OPTIONAL_SCHEMA).expect("optional schema");
    seed(
        &repository,
        &schema,
        "broken-skill",
        Vec::new(),
        Vec::new(),
        None,
    );
    seed(
        &repository,
        &optional,
        "healthy-skill",
        vec![("endpoint".to_string(), text("fine"))],
        Vec::new(),
        None,
    );

    let broken = resolve_activation_snapshot(&repository, &schema, "broken-skill", "rev-1", "")
        .expect("resolve");
    let healthy = resolve_activation_snapshot(&repository, &optional, "healthy-skill", "rev-1", "")
        .expect("resolve");

    assert!(broken.is_err());
    assert_eq!(
        healthy
            .expect("healthy activates")
            .property("endpoint")
            .expect("endpoint")
            .value,
        Some(text("fine"))
    );
}

#[test]
fn the_rendered_block_is_stable_and_shows_secrets_as_presence_only() {
    let (_directory, repository) = harness();
    let schema = schema();
    seed(
        &repository,
        &schema,
        "configured-skill",
        vec![("endpoint".to_string(), text("https://live.example"))],
        vec!["api_key".to_string()],
        None,
    );
    let snapshot =
        resolve_activation_snapshot(&repository, &schema, "configured-skill", "rev-1", "")
            .expect("resolve")
            .expect("ready");

    let block = render_snapshot_block(&snapshot);

    assert_eq!(
        block,
        "api_key: <configured>\nendpoint: https://live.example\nretries: 3"
    );
    // Rendering the same snapshot twice must not move the prompt prefix.
    assert_eq!(block, render_snapshot_block(&snapshot));
}
