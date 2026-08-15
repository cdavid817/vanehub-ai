use super::{
    apply_deletion_retention, preview, reconcile, require_base_revision, reset_property,
    reset_scope, save, validate_request, DeletionRetention, ObsoleteFieldChoice,
    ReconciliationPlan, SkillConfigurationError, SkillConfigurationRequest,
    MAX_CONFIGURATION_PAYLOAD_BYTES,
};
use crate::contexts::tooling::skills::application::{
    SkillApplicationError, SkillLogEvent, SkillLoggingPort,
};
use crate::contexts::tooling::skills::domain::{
    parse_config_schema, SkillConfigProvenance, SkillConfigSchema, SkillConfigScope,
    SkillConfigValue, SkillSecretIntent,
};
use crate::contexts::tooling::skills::infrastructure::SecretRecovery;

/// The service must not depend on a log sink to function, so the tests give it one that records
/// nothing and asserts nothing; the event shapes are covered in configuration_logging_tests.
struct SilentLogging;
impl SkillLoggingPort for SilentLogging {
    fn record(&self, _event: &SkillLogEvent) -> Result<(), SkillApplicationError> {
        Ok(())
    }
}
use crate::contexts::tooling::skills::infrastructure::{
    SkillConfigurationSecrets, SkillSecretStore, SqliteSkillConfigurationRepository,
};
use crate::platform::database::NativeDatabase;
use crate::platform::error::InfrastructureError;
use crate::test_support::TempDirectory;
use std::collections::BTreeMap;
use std::sync::Mutex;
use zeroize::Zeroizing;

const SCHEMA: &str = "  properties:\n    endpoint:\n      type: string\n      default: https://default.example\n    retries:\n      type: integer\n      minimum: 0\n      maximum: 10\n      default: 3\n    api_key:\n      type: string\n      x-vanehub-secret: true\n";
const SECRET: &str = "sk-service-secret";

#[derive(Default)]
struct FakeStore {
    entries: Mutex<BTreeMap<String, String>>,
    fail_writes: Mutex<bool>,
}

impl SkillSecretStore for FakeStore {
    fn read(&self, alias: &str) -> Result<Option<Zeroizing<String>>, InfrastructureError> {
        Ok(self
            .entries
            .lock()
            .expect("entries")
            .get(alias)
            .map(|value| Zeroizing::new(value.clone())))
    }

    fn write(&self, alias: &str, value: &str) -> Result<(), InfrastructureError> {
        if *self.fail_writes.lock().expect("flag") {
            return Err(InfrastructureError::Credential("nope".to_string()));
        }
        self.entries
            .lock()
            .expect("entries")
            .insert(alias.to_string(), value.to_string());
        Ok(())
    }

    fn remove(&self, alias: &str) -> Result<(), InfrastructureError> {
        self.entries.lock().expect("entries").remove(alias);
        Ok(())
    }
}

fn schema() -> SkillConfigSchema {
    parse_config_schema(SCHEMA).expect("schema")
}

fn text(value: &str) -> SkillConfigValue {
    SkillConfigValue::Text(value.to_string())
}

fn request(values: Vec<(String, SkillConfigValue)>) -> SkillConfigurationRequest {
    SkillConfigurationRequest {
        skill_id: "configured-skill".to_string(),
        scope: SkillConfigScope::User,
        workspace_identity: String::new(),
        schema_hash: schema().hash,
        base_revision: "rev-1".to_string(),
        expected_revision: None,
        values,
        secret_intents: Vec::new(),
    }
}

fn harness() -> (
    TempDirectory,
    SqliteSkillConfigurationRepository,
    SkillConfigurationSecrets<FakeStore>,
) {
    let directory = TempDirectory::new("skill-configuration-service");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("open test database");
    (
        directory,
        SqliteSkillConfigurationRepository::new(database),
        SkillConfigurationSecrets::new(FakeStore::default()),
    )
}

#[test]
fn a_request_aimed_at_another_schema_is_rejected_before_its_values_are_judged() {
    let mut invalid = request(vec![("unknown".to_string(), text("value"))]);
    invalid.schema_hash = "a-different-hash".to_string();

    // The unknown property is never reported: the request was aimed at a revision that is gone,
    // and judging its values against this schema would describe the wrong problem.
    assert!(matches!(
        validate_request(&schema(), &invalid),
        Err(SkillConfigurationError::SchemaChanged { .. })
    ));
}

#[test]
fn unknown_keys_wrong_types_and_out_of_range_values_are_rejected() {
    let schema = schema();

    assert!(matches!(
        validate_request(&schema, &request(vec![("nope".to_string(), text("v"))])),
        Err(SkillConfigurationError::UnknownProperty { key }) if key == "nope"
    ));
    assert!(matches!(
        validate_request(&schema, &request(vec![("retries".to_string(), text("many"))])),
        Err(SkillConfigurationError::InvalidValue { key, .. }) if key == "retries"
    ));
    assert!(matches!(
        validate_request(
            &schema,
            &request(vec![("retries".to_string(), SkillConfigValue::Integer(99))])
        ),
        Err(SkillConfigurationError::InvalidValue { key, .. }) if key == "retries"
    ));
}

#[test]
fn a_secret_property_cannot_be_written_as_a_plain_value() {
    assert!(matches!(
        validate_request(
            &schema(),
            &request(vec![("api_key".to_string(), text(SECRET))])
        ),
        Err(SkillConfigurationError::NotConfigurable { key }) if key == "api_key"
    ));
}

#[test]
fn scope_and_workspace_must_agree() {
    let schema = schema();

    let mut project_without_workspace = request(Vec::new());
    project_without_workspace.scope = SkillConfigScope::Project;
    assert_eq!(
        validate_request(&schema, &project_without_workspace),
        Err(SkillConfigurationError::InvalidWorkspace)
    );

    let mut user_with_workspace = request(Vec::new());
    user_with_workspace.workspace_identity = "/workspace/one".to_string();
    assert_eq!(
        validate_request(&schema, &user_with_workspace),
        Err(SkillConfigurationError::InvalidWorkspace)
    );
}

#[test]
fn an_oversized_payload_is_rejected_whole() {
    let oversized = request(vec![(
        "endpoint".to_string(),
        text(&"a".repeat(MAX_CONFIGURATION_PAYLOAD_BYTES + 1)),
    )]);

    assert!(matches!(
        validate_request(&schema(), &oversized),
        Err(SkillConfigurationError::PayloadTooLarge { .. })
    ));
}

#[test]
fn preview_shows_the_effect_without_writing_anything() {
    let (_directory, repository, _secrets) = harness();
    let schema = schema();

    let resolved = preview(
        &schema,
        &[],
        &request(vec![("endpoint".to_string(), text("previewed"))]),
    )
    .expect("preview");

    assert_eq!(
        resolved
            .properties
            .iter()
            .find(|property| property.key == "endpoint")
            .expect("endpoint")
            .value,
        Some(text("previewed"))
    );
    // Nothing was stored.
    assert_eq!(
        repository
            .load("configured-skill", SkillConfigScope::User, "")
            .expect("load"),
        None
    );
}

#[test]
fn a_save_stores_values_and_secret_presence_and_returns_the_effective_preview() {
    let (_directory, repository, secrets) = harness();
    let schema = schema();
    let mut request = request(vec![("endpoint".to_string(), text("saved"))]);
    request.secret_intents = vec![(
        "api_key".to_string(),
        SkillSecretIntent::Replace(SECRET.to_string()),
    )];

    let result = save(&repository, &secrets, &SilentLogging, &schema, &request).expect("save");

    assert_eq!(
        result.record.values,
        vec![("endpoint".to_string(), text("saved"))]
    );
    assert_eq!(result.record.secret_keys, vec!["api_key".to_string()]);
    // The secret value is nowhere in what the operation returns.
    assert!(!format!("{:?}", result.record).contains(SECRET));
    assert!(!format!("{:?}", result.preview).contains(SECRET));
    assert_eq!(
        result
            .preview
            .properties
            .iter()
            .find(|property| property.key == "endpoint")
            .expect("endpoint")
            .provenance,
        SkillConfigProvenance::User
    );
}

#[test]
fn a_stale_save_preserves_the_prior_record_and_returns_it_for_refresh() {
    let (_directory, repository, secrets) = harness();
    let schema = schema();
    save(
        &repository,
        &secrets,
        &SilentLogging,
        &schema,
        &request(vec![("endpoint".to_string(), text("first"))]),
    )
    .expect("first save");

    let error = save(
        &repository,
        &secrets,
        &SilentLogging,
        &schema,
        &request(vec![("endpoint".to_string(), text("second"))]),
    )
    .expect_err("stale");

    match error {
        SkillConfigurationError::Stale {
            current: Some(current),
        } => {
            assert_eq!(
                current.values,
                vec![("endpoint".to_string(), text("first"))]
            );
        }
        other => panic!("expected stale, got {other:?}"),
    }
}

#[test]
fn a_stale_save_does_not_leave_its_staged_credential_behind() {
    let (_directory, repository, secrets) = harness();
    let schema = schema();
    save(
        &repository,
        &secrets,
        &SilentLogging,
        &schema,
        &request(vec![("endpoint".to_string(), text("first"))]),
    )
    .expect("first save");

    let mut stale = request(vec![("endpoint".to_string(), text("second"))]);
    stale.secret_intents = vec![(
        "api_key".to_string(),
        SkillSecretIntent::Replace(SECRET.to_string()),
    )];
    let error = save(&repository, &secrets, &SilentLogging, &schema, &stale).expect_err("stale");
    assert!(matches!(error, SkillConfigurationError::Stale { .. }));

    // The record the caller expected is gone, so the credential they staged must not stand.
    let reloaded = repository
        .load("configured-skill", SkillConfigScope::User, "")
        .expect("load")
        .expect("record");
    assert!(reloaded.secret_keys.is_empty());
}

#[test]
fn a_credential_failure_reports_the_property_without_the_value() {
    let (_directory, repository, secrets) = harness();
    let schema = schema();
    *secrets.store_for_tests().fail_writes.lock().expect("flag") = true;
    let mut request = request(Vec::new());
    request.secret_intents = vec![(
        "api_key".to_string(),
        SkillSecretIntent::Replace(SECRET.to_string()),
    )];

    let error = save(&repository, &secrets, &SilentLogging, &schema, &request)
        .expect_err("credential failure");

    match &error {
        SkillConfigurationError::CredentialFailure { key, .. } => assert_eq!(key, "api_key"),
        other => panic!("expected a credential failure, got {other:?}"),
    }
    assert!(!format!("{error:?}").contains(SECRET));
    // Nothing was written, so no record exists at all.
    assert_eq!(
        repository
            .load("configured-skill", SkillConfigScope::User, "")
            .expect("load"),
        None
    );
}

#[test]
fn a_request_bound_to_a_superseded_base_revision_is_rejected() {
    let request = request(Vec::new());

    assert!(require_base_revision("rev-1", &request).is_ok());
    // Two Skill revisions can share a config_schema and still differ in the instructions the
    // values feed, so the schema hash alone is not enough to pin the revision.
    assert!(matches!(
        require_base_revision("rev-2", &request),
        Err(SkillConfigurationError::BaseRevisionChanged { .. })
    ));
}

#[test]
fn resetting_a_property_leaves_the_rest_and_re_resolves_the_inherited_value() {
    let (_directory, repository, secrets) = harness();
    let schema = schema();
    let first = save(
        &repository,
        &secrets,
        &SilentLogging,
        &schema,
        &request(vec![
            ("endpoint".to_string(), text("kept")),
            ("retries".to_string(), SkillConfigValue::Integer(7)),
        ]),
    )
    .expect("save");

    let mut follow_up = request(vec![
        ("endpoint".to_string(), text("kept")),
        ("retries".to_string(), SkillConfigValue::Integer(7)),
    ]);
    follow_up.expected_revision = Some(first.record.stored_revision);
    let result = reset_property(
        &repository,
        &secrets,
        &SilentLogging,
        &schema,
        &follow_up,
        "retries",
    )
    .expect("reset");

    assert_eq!(
        result.record.values,
        vec![("endpoint".to_string(), text("kept"))]
    );
    // Falls back to the schema default rather than becoming unset.
    assert_eq!(
        result
            .preview
            .properties
            .iter()
            .find(|property| property.key == "retries")
            .expect("retries")
            .provenance,
        SkillConfigProvenance::SchemaDefault
    );
}

#[test]
fn resetting_a_scope_removes_its_record_and_clears_its_credentials() {
    let (_directory, repository, secrets) = harness();
    let schema = schema();
    let mut seed = request(vec![("endpoint".to_string(), text("stored"))]);
    seed.secret_intents = vec![(
        "api_key".to_string(),
        SkillSecretIntent::Replace(SECRET.to_string()),
    )];
    save(&repository, &secrets, &SilentLogging, &schema, &seed).expect("save");

    let resolved = reset_scope(
        &repository,
        &secrets,
        &SilentLogging,
        &schema,
        "configured-skill",
        SkillConfigScope::User,
        "",
    )
    .expect("reset scope");

    assert_eq!(
        repository
            .load("configured-skill", SkillConfigScope::User, "")
            .expect("load"),
        None
    );
    assert!(secrets
        .store_for_tests()
        .entries
        .lock()
        .expect("entries")
        .is_empty());
    assert_eq!(
        resolved
            .properties
            .iter()
            .find(|property| property.key == "endpoint")
            .expect("endpoint")
            .provenance,
        SkillConfigProvenance::SchemaDefault
    );
}

#[test]
fn reconciliation_refuses_to_decide_for_the_user() {
    let (_directory, repository, secrets) = harness();
    let schema = schema();
    // A value for a property this schema no longer has.
    repository
        .save(
            &crate::contexts::tooling::skills::infrastructure::SkillConfigurationSave {
                skill_id: "configured-skill".to_string(),
                scope: SkillConfigScope::User,
                workspace_identity: String::new(),
                schema_hash: schema.hash.clone(),
                base_revision: "rev-1".to_string(),
                expected_revision: None,
                values: vec![("obsolete".to_string(), text("carried"))],
                secret_keys: Vec::new(),
                validation_state:
                    crate::contexts::tooling::skills::domain::SkillConfigDrift::MigrationRequired,
            },
        )
        .expect("seed obsolete record");

    let mut follow_up = request(Vec::new());
    follow_up.expected_revision =
        Some(crate::contexts::tooling::skills::domain::SkillConfigRevision::new(1));

    let error = reconcile(
        &repository,
        &secrets,
        &SilentLogging,
        &schema,
        &follow_up,
        &ReconciliationPlan::default(),
    )
    .expect_err("an unaddressed obsolete field must not be resolved silently");

    assert!(matches!(
        error,
        SkillConfigurationError::ReconciliationIncomplete { key } if key == "obsolete"
    ));
}

#[test]
fn reconciliation_maps_or_discards_only_what_the_plan_says() {
    let (_directory, repository, secrets) = harness();
    let schema = schema();
    repository
        .save(
            &crate::contexts::tooling::skills::infrastructure::SkillConfigurationSave {
                skill_id: "configured-skill".to_string(),
                scope: SkillConfigScope::User,
                workspace_identity: String::new(),
                schema_hash: schema.hash.clone(),
                base_revision: "rev-1".to_string(),
                expected_revision: None,
                values: vec![
                    ("old_endpoint".to_string(), text("carried")),
                    ("dropped".to_string(), text("gone")),
                ],
                secret_keys: Vec::new(),
                validation_state:
                    crate::contexts::tooling::skills::domain::SkillConfigDrift::MigrationRequired,
            },
        )
        .expect("seed");

    let mut follow_up = request(Vec::new());
    follow_up.expected_revision =
        Some(crate::contexts::tooling::skills::domain::SkillConfigRevision::new(1));
    let result = reconcile(
        &repository,
        &secrets,
        &SilentLogging,
        &schema,
        &follow_up,
        &ReconciliationPlan {
            fields: vec![
                (
                    "old_endpoint".to_string(),
                    ObsoleteFieldChoice::MapTo("endpoint".to_string()),
                ),
                ("dropped".to_string(), ObsoleteFieldChoice::Discard),
            ],
            clear_secrets: Vec::new(),
        },
    )
    .expect("reconcile");

    assert_eq!(
        result.record.values,
        vec![("endpoint".to_string(), text("carried"))]
    );
}

#[test]
fn deletion_can_retain_the_records_for_a_returning_skill() {
    let (_directory, repository, secrets) = harness();
    let schema = schema();
    save(
        &repository,
        &secrets,
        &SilentLogging,
        &schema,
        &request(vec![("endpoint".to_string(), text("kept"))]),
    )
    .expect("save");

    let recovery = apply_deletion_retention(
        &repository,
        &secrets,
        &SilentLogging,
        "configured-skill",
        "",
        DeletionRetention::Retain,
    )
    .expect("retain");

    assert_eq!(recovery, SecretRecovery::Clean);
    let retained = repository
        .load("configured-skill", SkillConfigScope::User, "")
        .expect("load")
        .expect("record survives");
    assert!(retained.orphaned_at.is_some());
    assert_eq!(
        retained.values,
        vec![("endpoint".to_string(), text("kept"))]
    );
}

#[test]
fn deletion_can_remove_the_records_and_their_credentials() {
    let (_directory, repository, secrets) = harness();
    let schema = schema();
    let mut seed = request(vec![("endpoint".to_string(), text("kept"))]);
    seed.secret_intents = vec![(
        "api_key".to_string(),
        SkillSecretIntent::Replace(SECRET.to_string()),
    )];
    save(&repository, &secrets, &SilentLogging, &schema, &seed).expect("save");

    let recovery = apply_deletion_retention(
        &repository,
        &secrets,
        &SilentLogging,
        "configured-skill",
        "",
        DeletionRetention::Delete,
    )
    .expect("delete");

    assert_eq!(recovery, SecretRecovery::Clean);
    assert_eq!(
        repository
            .load("configured-skill", SkillConfigScope::User, "")
            .expect("load"),
        None
    );
    assert!(secrets
        .store_for_tests()
        .entries
        .lock()
        .expect("entries")
        .is_empty());
}
