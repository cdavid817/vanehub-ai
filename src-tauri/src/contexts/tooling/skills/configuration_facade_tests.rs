use super::SkillConfigurationFacade;
use crate::contexts::tooling::skills::application::{
    SkillApplicationError, SkillLogEvent, SkillLoggingPort,
};
use crate::contexts::tooling::skills::domain::{
    parse_config_schema, SkillConfigDrift, SkillConfigScope, SkillConfigValue, SkillSecretIntent,
};
use crate::contexts::tooling::skills::infrastructure::{
    DeletionRetention, SecretRecovery, SkillConfigurationRequest, SkillSecretStore,
    SqliteSkillConfigurationRepository,
};
use crate::platform::database::NativeDatabase;
use crate::platform::error::InfrastructureError;
use crate::test_support::TempDirectory;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use zeroize::Zeroizing;

const SCHEMA: &str = "  properties:\n    endpoint:\n      type: string\n      default: https://default.example\n    api_key:\n      type: string\n      x-vanehub-secret: true\n";
const SECRET: &str = "sk-facade-secret";

#[derive(Default)]
struct FakeStore {
    entries: Mutex<BTreeMap<String, String>>,
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

struct SilentLogging;
impl SkillLoggingPort for SilentLogging {
    fn record(&self, _event: &SkillLogEvent) -> Result<(), SkillApplicationError> {
        Ok(())
    }
}

#[test]
fn the_facade_carries_one_configuration_through_save_read_activation_and_deletion() {
    let directory = TempDirectory::new("skill-configuration-facade");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let facade = SkillConfigurationFacade::new(
        SqliteSkillConfigurationRepository::new(database),
        FakeStore::default(),
        Arc::new(SilentLogging),
    );
    let schema = parse_config_schema(SCHEMA).expect("schema");

    let saved = facade
        .save(
            &schema,
            &SkillConfigurationRequest {
                skill_id: "configured-skill".to_string(),
                scope: SkillConfigScope::User,
                workspace_identity: String::new(),
                schema_hash: schema.hash.clone(),
                base_revision: "rev-1".to_string(),
                expected_revision: None,
                values: vec![(
                    "endpoint".to_string(),
                    SkillConfigValue::Text("https://live.example".to_string()),
                )],
                secret_intents: vec![(
                    "api_key".to_string(),
                    SkillSecretIntent::Replace(SECRET.to_string()),
                )],
            },
        )
        .expect("save");
    assert_eq!(saved.recovery, SecretRecovery::Clean);

    let read = facade.read(&schema, "configured-skill", "").expect("read");
    assert_eq!(read.drift, SkillConfigDrift::Compatible);

    let snapshot = facade
        .activation_snapshot(&schema, "configured-skill", "rev-1", "")
        .expect("resolve")
        .expect("ready");
    // The same facade that stored the secret hands the activation presence, never the value.
    assert_eq!(snapshot.property("api_key").expect("api_key").value, None);
    assert!(!format!("{snapshot:?}").contains(SECRET));

    let recovery = facade
        .apply_deletion_retention("configured-skill", "", DeletionRetention::Delete)
        .expect("delete");
    assert_eq!(recovery, SecretRecovery::Clean);
    assert!(facade
        .read(&schema, "configured-skill", "")
        .expect("read after delete")
        .scopes
        .is_empty());
}

#[test]
fn the_facade_exposes_reset_and_reconcile_against_the_same_stored_record() {
    let directory = TempDirectory::new("skill-configuration-facade-reset");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let facade = SkillConfigurationFacade::new(
        SqliteSkillConfigurationRepository::new(database),
        FakeStore::default(),
        Arc::new(SilentLogging),
    );
    let schema = parse_config_schema(SCHEMA).expect("schema");
    let base =
        |values: Vec<(String, SkillConfigValue)>,
         expected: Option<crate::contexts::tooling::skills::domain::SkillConfigRevision>| {
            SkillConfigurationRequest {
                skill_id: "configured-skill".to_string(),
                scope: SkillConfigScope::User,
                workspace_identity: String::new(),
                schema_hash: schema.hash.clone(),
                base_revision: "rev-1".to_string(),
                expected_revision: expected,
                values,
                secret_intents: Vec::new(),
            }
        };

    let saved = facade
        .save(
            &schema,
            &base(
                vec![(
                    "endpoint".to_string(),
                    SkillConfigValue::Text("stored".to_string()),
                )],
                None,
            ),
        )
        .expect("save");

    let reset = facade
        .reset_property(
            &schema,
            &base(
                vec![(
                    "endpoint".to_string(),
                    SkillConfigValue::Text("stored".to_string()),
                )],
                Some(saved.record.stored_revision),
            ),
            "endpoint",
        )
        .expect("reset property");
    assert!(reset.record.values.is_empty());

    // Reconciling with an empty plan is a no-op here: every stored key still exists in the schema.
    let reconciled = facade
        .reconcile(
            &schema,
            &base(Vec::new(), Some(reset.record.stored_revision)),
            &crate::contexts::tooling::skills::infrastructure::ReconciliationPlan::default(),
        )
        .expect("reconcile");
    assert!(reconciled.record.values.is_empty());

    let remaining = facade
        .reset_scope(&schema, "configured-skill", SkillConfigScope::User, "")
        .expect("reset scope");
    assert!(remaining.scopes.is_empty());
}
