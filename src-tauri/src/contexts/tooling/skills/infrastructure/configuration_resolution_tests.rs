use super::{require_canonical_workspace, resolve_from_records, resolve_stored_configuration};
use crate::contexts::tooling::skills::domain::{
    parse_config_schema, SkillConfigDrift, SkillConfigProvenance, SkillConfigReadiness,
    SkillConfigRevision, SkillConfigSchema, SkillConfigScope, SkillConfigValue, SkillSecretState,
};
use crate::contexts::tooling::skills::infrastructure::{
    SkillConfigCleanupState, SkillConfigurationSave, SkillConfigurationWrite,
    SqliteSkillConfigurationRepository, StoredSkillConfiguration,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

const SCHEMA: &str = "  properties:\n    endpoint:\n      type: string\n      default: https://default.example\n    retries:\n      type: integer\n      default: 3\n    label:\n      type: string\n    api_key:\n      type: string\n      x-vanehub-secret: true\n  required:\n    - endpoint\n    - api_key\n";

fn schema() -> SkillConfigSchema {
    parse_config_schema(SCHEMA).expect("schema")
}

fn text(value: &str) -> SkillConfigValue {
    SkillConfigValue::Text(value.to_string())
}

fn record(
    scope: SkillConfigScope,
    workspace: &str,
    values: Vec<(String, SkillConfigValue)>,
    secret_keys: Vec<String>,
) -> StoredSkillConfiguration {
    StoredSkillConfiguration {
        skill_id: "configured-skill".to_string(),
        scope,
        workspace_identity: workspace.to_string(),
        schema_hash: schema().hash,
        base_revision: "rev-1".to_string(),
        stored_revision: SkillConfigRevision::new(1),
        validation_state: SkillConfigDrift::Compatible,
        values,
        secret_keys,
        orphaned_at: None,
        cleanup_state: SkillConfigCleanupState::None,
    }
}

fn provenance_of(resolved: &super::ResolvedSkillConfiguration, key: &str) -> SkillConfigProvenance {
    resolved
        .properties
        .iter()
        .find(|property| property.key == key)
        .expect("property")
        .provenance
}

fn value_of(resolved: &super::ResolvedSkillConfiguration, key: &str) -> Option<SkillConfigValue> {
    resolved
        .properties
        .iter()
        .find(|property| property.key == key)
        .expect("property")
        .value
        .clone()
}

#[test]
fn project_wins_over_user_which_wins_over_the_schema_default() {
    let schema = schema();
    let records = vec![
        record(
            SkillConfigScope::User,
            "",
            vec![
                ("endpoint".to_string(), text("user")),
                ("label".to_string(), text("user label")),
            ],
            vec!["api_key".to_string()],
        ),
        record(
            SkillConfigScope::Project,
            "/workspace/one",
            vec![("label".to_string(), text("project label"))],
            Vec::new(),
        ),
    ];

    let resolved = resolve_from_records(&schema, &records);

    assert_eq!(
        provenance_of(&resolved, "label"),
        SkillConfigProvenance::Project
    );
    assert_eq!(value_of(&resolved, "label"), Some(text("project label")));
    assert_eq!(
        provenance_of(&resolved, "endpoint"),
        SkillConfigProvenance::User
    );
    assert_eq!(
        provenance_of(&resolved, "retries"),
        SkillConfigProvenance::SchemaDefault
    );
    assert_eq!(resolved.readiness, SkillConfigReadiness::Ready);
    assert_eq!(resolved.drift, SkillConfigDrift::Compatible);
}

#[test]
fn an_inherited_value_is_not_written_into_the_higher_scope() {
    let schema = schema();
    let records = vec![
        record(
            SkillConfigScope::User,
            "",
            vec![("endpoint".to_string(), text("user"))],
            vec!["api_key".to_string()],
        ),
        record(
            SkillConfigScope::Project,
            "/workspace/one",
            Vec::new(),
            Vec::new(),
        ),
    ];

    let resolved = resolve_from_records(&schema, &records);

    // The Project scope reports zero configured properties even though the effective endpoint
    // resolves: clearing User later has to change the effective value.
    let project = resolved
        .scopes
        .iter()
        .find(|state| state.scope == SkillConfigScope::Project)
        .expect("project scope state");
    assert_eq!(project.configured_property_count, 0);
    assert_eq!(
        provenance_of(&resolved, "endpoint"),
        SkillConfigProvenance::User
    );
}

#[test]
fn properties_resolve_in_a_stable_order_regardless_of_stored_order() {
    let schema = schema();
    let forward = resolve_from_records(
        &schema,
        &[record(
            SkillConfigScope::User,
            "",
            vec![
                ("endpoint".to_string(), text("a")),
                ("label".to_string(), text("b")),
            ],
            Vec::new(),
        )],
    );
    let reversed = resolve_from_records(
        &schema,
        &[record(
            SkillConfigScope::User,
            "",
            vec![
                ("label".to_string(), text("b")),
                ("endpoint".to_string(), text("a")),
            ],
            Vec::new(),
        )],
    );

    let keys = |resolved: &super::ResolvedSkillConfiguration| {
        resolved
            .properties
            .iter()
            .map(|property| property.key.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(keys(&forward), keys(&reversed));
    assert_eq!(
        keys(&forward),
        vec!["api_key", "endpoint", "label", "retries"]
    );
}

#[test]
fn a_value_that_no_longer_validates_is_excluded_while_the_rest_stays_usable() {
    let schema = schema();
    let records = vec![record(
        SkillConfigScope::User,
        "",
        vec![
            ("endpoint".to_string(), text("kept")),
            // `retries` is an integer property; this value survived a schema change.
            ("retries".to_string(), text("not an integer")),
        ],
        vec!["api_key".to_string()],
    )];

    let resolved = resolve_from_records(&schema, &records);

    assert_eq!(resolved.drift, SkillConfigDrift::MigrationRequired);
    assert_eq!(resolved.readiness, SkillConfigReadiness::MigrationRequired);
    // The invalid value is gone from the resolution, and the valid one is untouched.
    assert_eq!(
        value_of(&resolved, "retries"),
        Some(SkillConfigValue::Integer(3))
    );
    assert_eq!(value_of(&resolved, "endpoint"), Some(text("kept")));
}

#[test]
fn the_worst_scope_determines_the_reported_drift() {
    let schema = schema();
    let records = vec![
        record(
            SkillConfigScope::User,
            "",
            vec![("endpoint".to_string(), text("fine"))],
            Vec::new(),
        ),
        record(
            SkillConfigScope::Project,
            "/workspace/one",
            vec![("gone".to_string(), text("removed property"))],
            Vec::new(),
        ),
    ];

    let resolved = resolve_from_records(&schema, &records);

    assert_eq!(resolved.drift, SkillConfigDrift::MigrationRequired);
    let user = resolved
        .scopes
        .iter()
        .find(|state| state.scope == SkillConfigScope::User)
        .expect("user scope");
    assert_eq!(user.drift, SkillConfigDrift::Compatible);
}

#[test]
fn a_missing_required_secret_keeps_the_configuration_unready() {
    let schema = schema();

    let without = resolve_from_records(
        &schema,
        &[record(
            SkillConfigScope::User,
            "",
            vec![("endpoint".to_string(), text("set"))],
            Vec::new(),
        )],
    );
    assert_eq!(without.readiness, SkillConfigReadiness::MissingRequired);

    let with = resolve_from_records(
        &schema,
        &[record(
            SkillConfigScope::User,
            "",
            vec![("endpoint".to_string(), text("set"))],
            vec!["api_key".to_string()],
        )],
    );
    assert_eq!(with.readiness, SkillConfigReadiness::Ready);
    let secret = with
        .properties
        .iter()
        .find(|property| property.key == "api_key")
        .expect("secret property");
    assert_eq!(secret.secret_state, Some(SkillSecretState::Configured));
    assert_eq!(secret.value, None);
}

#[test]
fn project_operations_require_a_canonical_workspace() {
    assert!(require_canonical_workspace("/workspace/one").is_ok());
    assert!(require_canonical_workspace("").is_err());
    assert!(require_canonical_workspace("   ").is_err());
    // Untrimmed input is rejected rather than normalized, so two spellings cannot address one
    // record from different call sites.
    assert!(require_canonical_workspace(" /workspace/one").is_err());
}

#[test]
fn resolution_reads_only_the_requested_workspace() {
    let directory = TempDirectory::new("skill-configuration-resolution");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("open test database");
    let repository = SqliteSkillConfigurationRepository::new(database);
    let schema = schema();

    for (workspace, value) in [("/workspace/one", "one"), ("/workspace/two", "two")] {
        let write = repository
            .save(&SkillConfigurationSave {
                skill_id: "configured-skill".to_string(),
                scope: SkillConfigScope::Project,
                workspace_identity: workspace.to_string(),
                schema_hash: schema.hash.clone(),
                base_revision: "rev-1".to_string(),
                expected_revision: None,
                values: vec![("label".to_string(), text(value))],
                secret_keys: Vec::new(),
                validation_state: SkillConfigDrift::Compatible,
            })
            .expect("save");
        assert!(matches!(write, SkillConfigurationWrite::Saved(_)));
    }

    let one =
        resolve_stored_configuration(&repository, &schema, "configured-skill", "/workspace/one")
            .expect("resolve one");
    let two =
        resolve_stored_configuration(&repository, &schema, "configured-skill", "/workspace/two")
            .expect("resolve two");

    assert_eq!(value_of(&one, "label"), Some(text("one")));
    assert_eq!(value_of(&two, "label"), Some(text("two")));
}

#[test]
fn a_shadowed_revisions_record_does_not_leak_into_another_skills_resolution() {
    let directory = TempDirectory::new("skill-configuration-isolation");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("open test database");
    let repository = SqliteSkillConfigurationRepository::new(database);
    let schema = schema();

    for skill_id in ["configured-skill", "configured-skill-extra"] {
        repository
            .save(&SkillConfigurationSave {
                skill_id: skill_id.to_string(),
                scope: SkillConfigScope::User,
                workspace_identity: String::new(),
                schema_hash: schema.hash.clone(),
                base_revision: "rev-1".to_string(),
                expected_revision: None,
                values: vec![("label".to_string(), text(skill_id))],
                secret_keys: Vec::new(),
                validation_state: SkillConfigDrift::Compatible,
            })
            .expect("save");
    }

    let resolved = resolve_stored_configuration(&repository, &schema, "configured-skill", "")
        .expect("resolve");

    assert_eq!(value_of(&resolved, "label"), Some(text("configured-skill")));
}
