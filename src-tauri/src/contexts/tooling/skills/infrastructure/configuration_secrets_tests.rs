use super::{secret_alias, SecretRecovery, SkillConfigurationSecrets, SkillSecretStore};
use crate::contexts::tooling::skills::domain::{SkillSecretIntent, SkillSecretState};
use crate::platform::error::InfrastructureError;
use std::collections::BTreeMap;
use std::sync::Mutex;
use zeroize::Zeroizing;

const SECRET: &str = "sk-super-secret-token";
const RECORD: &str = "record-1";

#[derive(Default)]
struct FakeStore {
    entries: Mutex<BTreeMap<String, String>>,
    fail_writes_for: Mutex<Option<String>>,
    fail_removes: Mutex<bool>,
    fail_reads: Mutex<bool>,
}

impl FakeStore {
    fn seeded(alias: &str, value: &str) -> Self {
        let store = Self::default();
        store
            .entries
            .lock()
            .expect("entries")
            .insert(alias.to_string(), value.to_string());
        store
    }

    fn contains(&self, alias: &str) -> Option<String> {
        self.entries.lock().expect("entries").get(alias).cloned()
    }
}

impl SkillSecretStore for FakeStore {
    fn read(&self, alias: &str) -> Result<Option<Zeroizing<String>>, InfrastructureError> {
        if *self.fail_reads.lock().expect("fail_reads") {
            return Err(InfrastructureError::Credential("unavailable".to_string()));
        }
        Ok(self
            .entries
            .lock()
            .expect("entries")
            .get(alias)
            .map(|value| Zeroizing::new(value.clone())))
    }

    fn write(&self, alias: &str, value: &str) -> Result<(), InfrastructureError> {
        if self
            .fail_writes_for
            .lock()
            .expect("fail_writes_for")
            .as_deref()
            == Some(alias)
        {
            return Err(InfrastructureError::Credential(
                "keychain-detail-must-not-leak".to_string(),
            ));
        }
        self.entries
            .lock()
            .expect("entries")
            .insert(alias.to_string(), value.to_string());
        Ok(())
    }

    fn remove(&self, alias: &str) -> Result<(), InfrastructureError> {
        if *self.fail_removes.lock().expect("fail_removes") {
            return Err(InfrastructureError::Credential("remove failed".to_string()));
        }
        self.entries.lock().expect("entries").remove(alias);
        Ok(())
    }
}

#[test]
fn an_alias_discloses_neither_the_skill_the_property_nor_the_value() {
    let alias = secret_alias("configured-skill:user", "api_key");

    assert_eq!(alias.len(), 64);
    assert!(!alias.contains("configured-skill"));
    assert!(!alias.contains("api_key"));
    // Stable across calls, and distinct per property and per record.
    assert_eq!(alias, secret_alias("configured-skill:user", "api_key"));
    assert_ne!(alias, secret_alias("configured-skill:user", "other_key"));
    assert_ne!(alias, secret_alias("configured-skill:project", "api_key"));
}

#[test]
fn slot_state_reports_presence_only_and_never_the_value() {
    let alias = secret_alias(RECORD, "api_key");
    let secrets = SkillConfigurationSecrets::new(FakeStore::seeded(&alias, SECRET));

    let configured = secrets.slot_state(RECORD, "api_key");
    let missing = secrets.slot_state(RECORD, "absent_key");

    assert_eq!(configured.state, SkillSecretState::Configured);
    assert_eq!(missing.state, SkillSecretState::Missing);
    let rendered = format!("{configured:?} {missing:?}");
    assert!(!rendered.contains(SECRET));
    assert!(!rendered.contains(&alias));
}

#[test]
fn an_unavailable_keychain_reports_error_rather_than_missing() {
    let secrets = SkillConfigurationSecrets::new(FakeStore::default());
    *secrets_store(&secrets).fail_reads.lock().expect("flag") = true;

    // Reading as "missing" would invite an overwrite of a credential that is merely unreachable.
    assert_eq!(
        secrets.slot_state(RECORD, "api_key").state,
        SkillSecretState::Error
    );
}

fn secrets_store(secrets: &SkillConfigurationSecrets<FakeStore>) -> &FakeStore {
    // The field is private; tests reach it through the public surface they share a module with.
    secrets.store_for_tests()
}

#[test]
fn preserve_leaves_the_existing_credential_untouched() {
    let alias = secret_alias(RECORD, "api_key");
    let secrets = SkillConfigurationSecrets::new(FakeStore::seeded(&alias, SECRET));

    let staged = secrets
        .stage(
            RECORD,
            &[("api_key".to_string(), SkillSecretIntent::Preserve)],
        )
        .expect("stage");

    assert_eq!(staged.configured_keys(), vec!["api_key".to_string()]);
    assert_eq!(staged.finalize(), SecretRecovery::Clean);
    assert_eq!(
        secrets_store(&secrets).contains(&alias),
        Some(SECRET.to_string())
    );
}

#[test]
fn replace_writes_the_new_value_and_a_failed_commit_restores_the_prior_one() {
    let alias = secret_alias(RECORD, "api_key");
    let secrets = SkillConfigurationSecrets::new(FakeStore::seeded(&alias, SECRET));

    let staged = secrets
        .stage(
            RECORD,
            &[(
                "api_key".to_string(),
                SkillSecretIntent::Replace("replacement".to_string()),
            )],
        )
        .expect("stage");
    assert_eq!(
        secrets_store(&secrets).contains(&alias),
        Some("replacement".to_string())
    );

    // The SQLite commit fails, so the credential has to go back to what it was.
    assert_eq!(staged.compensate(), SecretRecovery::Clean);
    assert_eq!(
        secrets_store(&secrets).contains(&alias),
        Some(SECRET.to_string())
    );
}

#[test]
fn compensating_a_first_time_write_removes_it_rather_than_leaving_an_orphan() {
    let alias = secret_alias(RECORD, "api_key");
    let secrets = SkillConfigurationSecrets::new(FakeStore::default());

    let staged = secrets
        .stage(
            RECORD,
            &[(
                "api_key".to_string(),
                SkillSecretIntent::Replace("first".to_string()),
            )],
        )
        .expect("stage");
    assert_eq!(staged.compensate(), SecretRecovery::Clean);

    assert_eq!(secrets_store(&secrets).contains(&alias), None);
}

#[test]
fn a_clear_is_deferred_until_the_commit_succeeds() {
    let alias = secret_alias(RECORD, "api_key");
    let secrets = SkillConfigurationSecrets::new(FakeStore::seeded(&alias, SECRET));

    let staged = secrets
        .stage(RECORD, &[("api_key".to_string(), SkillSecretIntent::Clear)])
        .expect("stage");
    // Still present: a SQLite failure now must leave the complete prior state.
    assert_eq!(
        secrets_store(&secrets).contains(&alias),
        Some(SECRET.to_string())
    );
    assert!(staged.configured_keys().is_empty());

    assert_eq!(staged.finalize(), SecretRecovery::Clean);
    assert_eq!(secrets_store(&secrets).contains(&alias), None);
}

#[test]
fn a_clear_that_never_ran_needs_no_undo_when_the_commit_fails() {
    let alias = secret_alias(RECORD, "api_key");
    let secrets = SkillConfigurationSecrets::new(FakeStore::seeded(&alias, SECRET));

    let staged = secrets
        .stage(RECORD, &[("api_key".to_string(), SkillSecretIntent::Clear)])
        .expect("stage");
    assert_eq!(staged.compensate(), SecretRecovery::Clean);

    assert_eq!(
        secrets_store(&secrets).contains(&alias),
        Some(SECRET.to_string())
    );
}

#[test]
fn a_failed_write_undoes_the_replacements_that_already_landed() {
    let secrets = SkillConfigurationSecrets::new(FakeStore::default());
    let first = secret_alias(RECORD, "first_key");
    let second = secret_alias(RECORD, "second_key");
    *secrets_store(&secrets)
        .fail_writes_for
        .lock()
        .expect("flag") = Some(second.clone());

    let failure = secrets.stage(
        RECORD,
        &[
            (
                "first_key".to_string(),
                SkillSecretIntent::Replace("one".to_string()),
            ),
            (
                "second_key".to_string(),
                SkillSecretIntent::Replace("two".to_string()),
            ),
        ],
    );
    let failure = match failure {
        Ok(_) => panic!("the second write was expected to fail"),
        Err(failure) => failure,
    };

    assert_eq!(failure.property_key, "second_key");
    // The first replacement was rolled back, so the request applied nothing at all.
    assert_eq!(secrets_store(&secrets).contains(&first), None);
    assert_eq!(secrets_store(&secrets).contains(&second), None);
}

#[test]
fn an_unrecoverable_cleanup_reports_the_property_without_values_or_aliases() {
    let alias = secret_alias(RECORD, "api_key");
    let secrets = SkillConfigurationSecrets::new(FakeStore::seeded(&alias, SECRET));
    let staged = secrets
        .stage(RECORD, &[("api_key".to_string(), SkillSecretIntent::Clear)])
        .expect("stage");
    *secrets_store(&secrets).fail_removes.lock().expect("flag") = true;

    let recovery = staged.finalize();

    match &recovery {
        SecretRecovery::Incomplete { properties } => {
            assert_eq!(properties, &vec!["api_key".to_string()]);
        }
        other => panic!("expected an explicit recovery state, got {other:?}"),
    }
    let rendered = format!("{recovery:?}");
    assert!(!rendered.contains(SECRET));
    assert!(!rendered.contains(&alias));
}

#[test]
fn a_secret_failure_carries_no_value_alias_or_platform_error_text() {
    let secrets = SkillConfigurationSecrets::new(FakeStore::default());
    let alias = secret_alias(RECORD, "api_key");
    *secrets_store(&secrets)
        .fail_writes_for
        .lock()
        .expect("flag") = Some(alias.clone());

    let failure = secrets.stage(
        RECORD,
        &[(
            "api_key".to_string(),
            SkillSecretIntent::Replace(SECRET.to_string()),
        )],
    );
    let failure = match failure {
        Ok(_) => panic!("the write was expected to fail"),
        Err(failure) => failure,
    };

    for rendered in [format!("{failure}"), format!("{failure:?}")] {
        assert!(!rendered.contains(SECRET), "value leaked: {rendered}");
        assert!(!rendered.contains(&alias), "alias leaked: {rendered}");
        assert!(
            !rendered.contains("keychain-detail-must-not-leak"),
            "platform error text leaked: {rendered}"
        );
    }
}
