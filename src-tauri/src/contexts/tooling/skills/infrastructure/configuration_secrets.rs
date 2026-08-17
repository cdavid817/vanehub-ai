#![cfg_attr(not(test), allow(dead_code))]

use crate::contexts::tooling::skills::domain::{SkillSecretIntent, SkillSecretState};
use crate::platform::credentials::OsCredentialStore;
use crate::platform::error::InfrastructureError;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use zeroize::Zeroizing;

/// Injectable so tests never touch the real OS keychain, and so a platform whose credential API
/// behaves differently can be substituted without reshaping the compensation sequence.
pub(crate) trait SkillSecretStore: Send + Sync {
    fn read(&self, alias: &str) -> Result<Option<Zeroizing<String>>, InfrastructureError>;
    fn write(&self, alias: &str, value: &str) -> Result<(), InfrastructureError>;
    fn remove(&self, alias: &str) -> Result<(), InfrastructureError>;
}

impl SkillSecretStore for OsCredentialStore {
    fn read(&self, alias: &str) -> Result<Option<Zeroizing<String>>, InfrastructureError> {
        self.get(alias)
    }

    fn write(&self, alias: &str, value: &str) -> Result<(), InfrastructureError> {
        self.set(alias, value)
    }

    fn remove(&self, alias: &str) -> Result<(), InfrastructureError> {
        self.delete(alias)
    }
}

/// One-way. The keychain account name must not disclose which Skill or property it belongs to,
/// and nothing may reconstruct a lookup key from a value, so the alias is a digest rather than a
/// readable composite.
pub(crate) fn secret_alias(record_id: &str, property_key: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(record_id.as_bytes());
    digest.update([0]);
    digest.update(property_key.as_bytes());
    digest
        .finalize()
        .iter()
        .fold(String::with_capacity(64), |mut encoded, byte| {
            let _ = write!(encoded, "{byte:02x}");
            encoded
        })
}

/// Never carries a value or an alias. `Error` is a first-class state because a keychain that is
/// locked or unavailable must not read as "not configured", which would invite an overwrite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SecretSlotState {
    pub(crate) property_key: String,
    pub(crate) state: SkillSecretState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SecretRecovery {
    Clean,
    /// Redacted by construction: property names are schema-public, values and aliases are not.
    Incomplete {
        properties: Vec<String>,
    },
}

#[derive(Clone)]
pub(crate) struct SkillConfigurationSecrets<S: SkillSecretStore> {
    store: S,
}

/// Holds what a rollback would need. Restoring requires the prior values, so this deliberately
/// does not implement Debug or Clone — a staged transaction must not be printable or duplicable.
pub(crate) struct StagedSecrets<'a, S: SkillSecretStore> {
    store: &'a S,
    restore: Vec<(String, String, Option<Zeroizing<String>>)>,
    deferred_removals: Vec<(String, String)>,
    configured: Vec<String>,
}

impl<S: SkillSecretStore> SkillConfigurationSecrets<S> {
    pub(crate) fn new(store: S) -> Self {
        Self { store }
    }

    #[cfg(test)]
    pub(crate) fn store_for_tests(&self) -> &S {
        &self.store
    }

    pub(crate) fn slot_state(&self, record_id: &str, property_key: &str) -> SecretSlotState {
        let state = match self.store.read(&secret_alias(record_id, property_key)) {
            Ok(Some(_)) => SkillSecretState::Configured,
            Ok(None) => SkillSecretState::Missing,
            Err(_) => SkillSecretState::Error,
        };
        SecretSlotState {
            property_key: property_key.to_string(),
            state,
        }
    }

    /// Stages credential changes ahead of the SQLite commit. Replacements are written now and
    /// their prior values retained for rollback; clears are deferred until after the commit, so a
    /// SQLite failure can still restore the complete prior state. Nothing is applied unless every
    /// replacement succeeds.
    pub(crate) fn stage<'a>(
        &'a self,
        record_id: &str,
        intents: &[(String, SkillSecretIntent)],
    ) -> Result<StagedSecrets<'a, S>, SkillSecretFailure> {
        let mut staged = StagedSecrets {
            store: &self.store,
            restore: Vec::new(),
            deferred_removals: Vec::new(),
            configured: Vec::new(),
        };
        for (property_key, intent) in intents {
            let alias = secret_alias(record_id, property_key);
            match intent {
                SkillSecretIntent::Preserve => {
                    if matches!(self.store.read(&alias), Ok(Some(_)) | Err(_)) {
                        staged.configured.push(property_key.clone());
                    }
                }
                SkillSecretIntent::Replace(value) => {
                    let prior = self.store.read(&alias).map_err(|_| {
                        SkillSecretFailure::new(property_key, "credential read failed")
                    })?;
                    if self.store.write(&alias, value).is_err() {
                        // Undo whatever this call already applied before reporting.
                        let _ = staged.compensate();
                        return Err(SkillSecretFailure::new(
                            property_key,
                            "credential write failed",
                        ));
                    }
                    staged.restore.push((property_key.clone(), alias, prior));
                    staged.configured.push(property_key.clone());
                }
                SkillSecretIntent::Clear => {
                    staged.deferred_removals.push((property_key.clone(), alias));
                }
            }
        }
        Ok(staged)
    }
}

impl<S: SkillSecretStore> crate::contexts::tooling::skills::api::SkillSecretReadPort
    for SkillConfigurationSecrets<S>
{
    fn read_secret(
        &self,
        record_id: &str,
        property_key: &str,
    ) -> Result<Option<Zeroizing<String>>, crate::contexts::tooling::skills::api::SkillError> {
        self.store
            .read(&secret_alias(record_id, property_key))
            .map_err(|_| {
                crate::contexts::tooling::skills::api::SkillError::Repository(
                    "Skill credential is unavailable".to_string(),
                )
            })
    }
}

impl<S: SkillSecretStore> StagedSecrets<'_, S> {
    /// Property names whose credential is present after this save. Used to write the record's
    /// secret-presence metadata; it is names only, never values or aliases.
    pub(crate) fn configured_keys(&self) -> Vec<String> {
        let mut keys = self.configured.clone();
        keys.sort();
        keys.dedup();
        keys
    }

    /// Runs after the SQLite commit succeeds: the deferred clears are now safe to perform.
    pub(crate) fn finalize(self) -> SecretRecovery {
        let mut failed = Vec::new();
        for (property_key, alias) in &self.deferred_removals {
            if self.store.remove(alias).is_err() {
                failed.push(property_key.clone());
            }
        }
        recovery(failed)
    }

    /// Runs when the SQLite commit fails: restore every replaced credential to its prior value.
    /// Deferred clears never ran, so they need no undo.
    pub(crate) fn compensate(&self) -> SecretRecovery {
        let mut failed = Vec::new();
        for (property_key, alias, prior) in &self.restore {
            let restored = match prior {
                Some(value) => self.store.write(alias, value),
                None => self.store.remove(alias),
            };
            if restored.is_err() {
                failed.push(property_key.clone());
            }
        }
        recovery(failed)
    }
}

fn recovery(mut properties: Vec<String>) -> SecretRecovery {
    if properties.is_empty() {
        return SecretRecovery::Clean;
    }
    properties.sort();
    properties.dedup();
    SecretRecovery::Incomplete { properties }
}

/// Names the property and a fixed reason. It never carries the value that failed to store, the
/// alias, or the underlying platform error string, any of which could reach a log or a response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillSecretFailure {
    pub(crate) property_key: String,
    pub(crate) reason: &'static str,
}

impl SkillSecretFailure {
    fn new(property_key: &str, reason: &'static str) -> Self {
        Self {
            property_key: property_key.to_string(),
            reason,
        }
    }
}

impl std::fmt::Display for SkillSecretFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.property_key, self.reason)
    }
}

#[cfg(test)]
#[path = "configuration_secrets_tests.rs"]
mod tests;
