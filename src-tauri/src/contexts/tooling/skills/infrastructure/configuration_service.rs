#![cfg_attr(not(test), allow(dead_code))]

use super::configuration_logging::{
    record_cleanup, record_drift, record_lifecycle, record_save, record_secret_mutation,
    record_validation_failure,
};
use super::configuration_repository::{
    SkillConfigurationSave, SkillConfigurationWrite, SqliteSkillConfigurationRepository,
    StoredSkillConfiguration,
};
use super::configuration_resolution::{
    require_canonical_workspace, resolve_from_records, ResolvedSkillConfiguration,
};
use super::configuration_secrets::{SecretRecovery, SkillConfigurationSecrets, SkillSecretStore};
use crate::contexts::tooling::skills::application::{SkillLogAction, SkillLoggingPort};
use crate::contexts::tooling::skills::domain::{
    validate_value, SkillConfigDrift, SkillConfigRevision, SkillConfigSchema, SkillConfigScope,
    SkillConfigValue, SkillSecretIntent,
};

/// A save that exceeds this is rejected whole. Values are operational settings, not documents,
/// and an unbounded payload would reach both SQLite and the model-visible configuration block.
pub(crate) const MAX_CONFIGURATION_PAYLOAD_BYTES: usize = 32 * 1_024;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SkillConfigurationRequest {
    pub(crate) skill_id: String,
    pub(crate) scope: SkillConfigScope,
    pub(crate) workspace_identity: String,
    pub(crate) schema_hash: String,
    pub(crate) base_revision: String,
    pub(crate) expected_revision: Option<SkillConfigRevision>,
    pub(crate) values: Vec<(String, SkillConfigValue)>,
    pub(crate) secret_intents: Vec<(String, SkillSecretIntent)>,
}

/// Every variant names what to fix and, where the caller can recover, carries the current record
/// so a refresh does not need another read that could race again. None of them carries a secret
/// value or a credential alias.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SkillConfigurationError {
    UnknownProperty {
        key: String,
    },
    NotConfigurable {
        key: String,
    },
    InvalidValue {
        key: String,
        reason: String,
    },
    PayloadTooLarge {
        bytes: usize,
    },
    SchemaChanged {
        expected: String,
        actual: String,
    },
    BaseRevisionChanged {
        expected: String,
        actual: String,
    },
    Stale {
        current: Option<Box<StoredSkillConfiguration>>,
    },
    InvalidWorkspace,
    CredentialFailure {
        key: String,
        reason: &'static str,
    },
    RepositoryFailure {
        reason: String,
    },
    RecoveryRequired {
        properties: Vec<String>,
    },
    ReconciliationIncomplete {
        key: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SkillConfigurationSaveResult {
    pub(crate) record: StoredSkillConfiguration,
    pub(crate) preview: ResolvedSkillConfiguration,
    pub(crate) recovery: SecretRecovery,
}

/// Validates everything before anything is written. Ordering matters: the schema-identity and
/// scope checks run first, because a request aimed at the wrong revision or an unusable workspace
/// must not be judged on its values at all.
pub(crate) fn validate_request(
    schema: &SkillConfigSchema,
    request: &SkillConfigurationRequest,
) -> Result<(), SkillConfigurationError> {
    if schema.hash != request.schema_hash {
        return Err(SkillConfigurationError::SchemaChanged {
            expected: schema.hash.clone(),
            actual: request.schema_hash.clone(),
        });
    }
    if request.scope == SkillConfigScope::Project
        && require_canonical_workspace(&request.workspace_identity).is_err()
    {
        return Err(SkillConfigurationError::InvalidWorkspace);
    }
    if request.scope == SkillConfigScope::User && !request.workspace_identity.is_empty() {
        // A User record is workspace-independent; accepting one here would create a second User
        // row the compare-and-swap could not arbitrate.
        return Err(SkillConfigurationError::InvalidWorkspace);
    }

    let payload = request
        .values
        .iter()
        .map(|(key, value)| key.len() + value.canonical().len())
        .sum::<usize>();
    if payload > MAX_CONFIGURATION_PAYLOAD_BYTES {
        return Err(SkillConfigurationError::PayloadTooLarge { bytes: payload });
    }

    for (key, value) in &request.values {
        let Some(field) = schema.field(key) else {
            return Err(SkillConfigurationError::UnknownProperty { key: key.clone() });
        };
        if field.secret {
            // Secret values never travel as plain values, so this is a malformed request rather
            // than an invalid value.
            return Err(SkillConfigurationError::NotConfigurable { key: key.clone() });
        }
        validate_value(
            value,
            field.field_type,
            &field.constraints,
            &field.choices,
            key,
        )
        .map_err(|error| SkillConfigurationError::InvalidValue {
            key: key.clone(),
            reason: error.to_string(),
        })?;
    }

    for (key, _) in &request.secret_intents {
        match schema.field(key) {
            None => return Err(SkillConfigurationError::UnknownProperty { key: key.clone() }),
            Some(field) if !field.secret => {
                return Err(SkillConfigurationError::NotConfigurable { key: key.clone() })
            }
            Some(_) => {}
        }
    }
    Ok(())
}

/// The schema hash alone does not pin the revision: two Skill revisions can carry an identical
/// `config_schema` and still differ in the instructions the values feed. Binding the base revision
/// as well is what makes a stored record traceable to the exact Skill it was configured against.
pub(crate) fn require_base_revision(
    effective_base_revision: &str,
    request: &SkillConfigurationRequest,
) -> Result<(), SkillConfigurationError> {
    if effective_base_revision == request.base_revision {
        return Ok(());
    }
    Err(SkillConfigurationError::BaseRevisionChanged {
        expected: effective_base_revision.to_string(),
        actual: request.base_revision.clone(),
    })
}

/// Preview without writing. The caller sees exactly what a save would resolve to, including
/// provenance and readiness, so an editor can show the effect before committing.
pub(crate) fn preview(
    schema: &SkillConfigSchema,
    existing: &[StoredSkillConfiguration],
    request: &SkillConfigurationRequest,
) -> Result<ResolvedSkillConfiguration, SkillConfigurationError> {
    validate_request(schema, request)?;
    let mut records = existing
        .iter()
        .filter(|record| record.scope != request.scope)
        .cloned()
        .collect::<Vec<_>>();
    let mut projected = existing
        .iter()
        .find(|record| record.scope == request.scope)
        .cloned()
        .unwrap_or_else(|| blank_record(request, schema));
    projected.values = request.values.clone();
    projected.secret_keys = projected_secret_keys(&projected.secret_keys, &request.secret_intents);
    records.push(projected);
    Ok(resolve_from_records(schema, &records))
}

pub(crate) fn save<S: SkillSecretStore>(
    repository: &SqliteSkillConfigurationRepository,
    secrets: &SkillConfigurationSecrets<S>,
    logging: &dyn SkillLoggingPort,
    schema: &SkillConfigSchema,
    request: &SkillConfigurationRequest,
) -> Result<SkillConfigurationSaveResult, SkillConfigurationError> {
    if let Err(error) = validate_request(schema, request) {
        record_validation_failure(logging, &request.skill_id, &error);
        return Err(error);
    }
    for (property_key, intent) in &request.secret_intents {
        record_secret_mutation(
            logging,
            &request.skill_id,
            request.scope,
            &request.workspace_identity,
            match intent {
                SkillSecretIntent::Preserve => "preserve",
                SkillSecretIntent::Replace(_) => "replace",
                SkillSecretIntent::Clear => "clear",
            },
            property_key,
        );
    }

    let record_id = format!(
        "{}:{}:{}",
        request.skill_id,
        request.scope.as_str(),
        request.workspace_identity
    );
    // Credentials are staged first because they are the resource that cannot participate in the
    // SQLite transaction. Staging is reversible; the commit that follows is the point of no
    // return for the non-secret record.
    let staged = secrets
        .stage(&record_id, &request.secret_intents)
        .map_err(|failure| SkillConfigurationError::CredentialFailure {
            key: failure.property_key,
            reason: failure.reason,
        })?;

    let write = repository.save(&SkillConfigurationSave {
        skill_id: request.skill_id.clone(),
        scope: request.scope,
        workspace_identity: request.workspace_identity.clone(),
        schema_hash: request.schema_hash.clone(),
        base_revision: request.base_revision.clone(),
        expected_revision: request.expected_revision,
        values: request.values.clone(),
        secret_keys: staged.configured_keys(),
        validation_state: SkillConfigDrift::Compatible,
    });

    let record = match write {
        Err(error) => {
            let recovery = staged.compensate();
            return Err(match recovery {
                SecretRecovery::Clean => SkillConfigurationError::RepositoryFailure {
                    reason: error.to_string(),
                },
                SecretRecovery::Incomplete { properties } => {
                    SkillConfigurationError::RecoveryRequired { properties }
                }
            });
        }
        Ok(SkillConfigurationWrite::Stale(current)) => {
            // The record the caller expected is gone, so the credentials they staged must not
            // stand either.
            let recovery = staged.compensate();
            return Err(match recovery {
                SecretRecovery::Clean => SkillConfigurationError::Stale {
                    current: current.map(Box::new),
                },
                SecretRecovery::Incomplete { properties } => {
                    SkillConfigurationError::RecoveryRequired { properties }
                }
            });
        }
        Ok(SkillConfigurationWrite::Saved(record)) => record,
    };

    let recovery = staged.finalize();
    record_save(
        logging,
        &request.skill_id,
        request.scope,
        &request.workspace_identity,
        record.stored_revision.value(),
        &request
            .values
            .iter()
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>(),
        &record.secret_keys,
    );
    if recovery != SecretRecovery::Clean {
        record_cleanup(logging, &request.skill_id, &recovery);
    }
    let others = repository
        .load_all_scopes(&request.skill_id, &request.workspace_identity)
        .map_err(|error| SkillConfigurationError::RepositoryFailure {
            reason: error.to_string(),
        })?;
    let preview = resolve_from_records(schema, &others);
    record_drift(
        logging,
        &request.skill_id,
        preview.drift,
        &request.schema_hash,
    );
    Ok(SkillConfigurationSaveResult {
        preview,
        record,
        recovery,
    })
}

/// Removes one property from one scope. Routed through the same save path so the reset is a
/// compare-and-swap like any other write; a reset that races a concurrent edit is stale rather
/// than silently discarding the newer value.
pub(crate) fn reset_property<S: SkillSecretStore>(
    repository: &SqliteSkillConfigurationRepository,
    secrets: &SkillConfigurationSecrets<S>,
    logging: &dyn SkillLoggingPort,
    schema: &SkillConfigSchema,
    request: &SkillConfigurationRequest,
    key: &str,
) -> Result<SkillConfigurationSaveResult, SkillConfigurationError> {
    let Some(field) = schema.field(key) else {
        return Err(SkillConfigurationError::UnknownProperty {
            key: key.to_string(),
        });
    };
    let mut reset = request.clone();
    if field.secret {
        // Resetting a secret is clearing its credential, not deleting a stored value.
        reset.secret_intents = vec![(key.to_string(), SkillSecretIntent::Clear)];
    } else {
        reset.values.retain(|(stored, _)| stored != key);
    }
    let result = save(repository, secrets, logging, schema, &reset)?;
    record_lifecycle(
        logging,
        SkillLogAction::ConfigurationReset,
        &request.skill_id,
        "applied",
    );
    Ok(result)
}

/// Deletes the scope's record and its credentials. Absence is what makes the effective value fall
/// through to the next scope, so an empty record would not be the same thing.
pub(crate) fn reset_scope<S: SkillSecretStore>(
    repository: &SqliteSkillConfigurationRepository,
    secrets: &SkillConfigurationSecrets<S>,
    logging: &dyn SkillLoggingPort,
    schema: &SkillConfigSchema,
    skill_id: &str,
    scope: SkillConfigScope,
    workspace_identity: &str,
) -> Result<ResolvedSkillConfiguration, SkillConfigurationError> {
    if scope == SkillConfigScope::Project
        && require_canonical_workspace(workspace_identity).is_err()
    {
        return Err(SkillConfigurationError::InvalidWorkspace);
    }
    let record_id = format!("{skill_id}:{}:{workspace_identity}", scope.as_str());
    let existing = repository
        .load(skill_id, scope, workspace_identity)
        .map_err(repository_failure)?;
    if let Some(record) = &existing {
        let intents = record
            .secret_keys
            .iter()
            .map(|key| (key.clone(), SkillSecretIntent::Clear))
            .collect::<Vec<_>>();
        let staged = secrets.stage(&record_id, &intents).map_err(|failure| {
            SkillConfigurationError::CredentialFailure {
                key: failure.property_key,
                reason: failure.reason,
            }
        })?;
        repository
            .reset_scope(skill_id, scope, workspace_identity)
            .map_err(repository_failure)?;
        if let SecretRecovery::Incomplete { properties } = staged.finalize() {
            return Err(SkillConfigurationError::RecoveryRequired { properties });
        }
    }
    let remaining = repository
        .load_all_scopes(skill_id, workspace_identity)
        .map_err(repository_failure)?;
    record_lifecycle(
        logging,
        SkillLogAction::ConfigurationReset,
        skill_id,
        "scope-cleared",
    );
    Ok(resolve_from_records(schema, &remaining))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ObsoleteFieldChoice {
    /// Move the stored value to a property that still exists. The value is revalidated against
    /// the target, so a rename into an incompatible type is refused rather than coerced.
    MapTo(String),
    Discard,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReconciliationPlan {
    pub(crate) fields: Vec<(String, ObsoleteFieldChoice)>,
    pub(crate) clear_secrets: Vec<String>,
}

/// Explicit reconciliation after a schema change. Every obsolete field needs a decision; nothing
/// is converted, renamed, or dropped on the user's behalf, which is the whole point of requiring
/// a plan rather than migrating automatically.
pub(crate) fn reconcile<S: SkillSecretStore>(
    repository: &SqliteSkillConfigurationRepository,
    secrets: &SkillConfigurationSecrets<S>,
    logging: &dyn SkillLoggingPort,
    schema: &SkillConfigSchema,
    request: &SkillConfigurationRequest,
    plan: &ReconciliationPlan,
) -> Result<SkillConfigurationSaveResult, SkillConfigurationError> {
    let existing = repository
        .load(
            &request.skill_id,
            request.scope,
            &request.workspace_identity,
        )
        .map_err(repository_failure)?;
    let stored = existing.map(|record| record.values).unwrap_or_default();

    let mut values = Vec::new();
    for (key, value) in &stored {
        if schema.field(key).is_some_and(|field| !field.secret) {
            values.push((key.clone(), value.clone()));
            continue;
        }
        let Some((_, choice)) = plan.fields.iter().find(|(planned, _)| planned == key) else {
            // Leaving it out would be a silent discard, and keeping it would be a silent carry.
            return Err(SkillConfigurationError::ReconciliationIncomplete { key: key.clone() });
        };
        match choice {
            ObsoleteFieldChoice::Discard => {}
            ObsoleteFieldChoice::MapTo(target) => {
                values.retain(|(existing, _)| existing != target);
                values.push((target.clone(), value.clone()));
            }
        }
    }

    let mut reconciled = request.clone();
    reconciled.values = values;
    reconciled.secret_intents = plan
        .clear_secrets
        .iter()
        .map(|key| (key.clone(), SkillSecretIntent::Clear))
        .collect();
    let result = save(repository, secrets, logging, schema, &reconciled)?;
    record_lifecycle(
        logging,
        SkillLogAction::ConfigurationReconcile,
        &request.skill_id,
        "applied",
    );
    Ok(result)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeletionRetention {
    Retain,
    Delete,
}

/// Deleting a configured Skill needs an explicit decision. Retain keeps the rows recoverable if
/// the same identity returns; Delete removes them and attempts credential cleanup, reporting an
/// explicit recovery state rather than claiming success it cannot verify.
pub(crate) fn apply_deletion_retention<S: SkillSecretStore>(
    repository: &SqliteSkillConfigurationRepository,
    secrets: &SkillConfigurationSecrets<S>,
    logging: &dyn SkillLoggingPort,
    skill_id: &str,
    workspace_identity: &str,
    retention: DeletionRetention,
) -> Result<SecretRecovery, SkillConfigurationError> {
    let records = repository
        .load_all_scopes(skill_id, workspace_identity)
        .map_err(repository_failure)?;
    if retention == DeletionRetention::Retain {
        repository
            .mark_orphaned(skill_id)
            .map_err(repository_failure)?;
        record_lifecycle(
            logging,
            SkillLogAction::ConfigurationRetention,
            skill_id,
            "retained",
        );
        return Ok(SecretRecovery::Clean);
    }

    let mut failed = Vec::new();
    for record in &records {
        let record_id = format!(
            "{skill_id}:{}:{}",
            record.scope.as_str(),
            record.workspace_identity
        );
        let intents = record
            .secret_keys
            .iter()
            .map(|key| (key.clone(), SkillSecretIntent::Clear))
            .collect::<Vec<_>>();
        match secrets.stage(&record_id, &intents) {
            Ok(staged) => {
                if let SecretRecovery::Incomplete { properties } = staged.finalize() {
                    failed.extend(properties);
                }
            }
            Err(failure) => failed.push(failure.property_key),
        }
    }
    repository.purge(skill_id).map_err(repository_failure)?;
    record_lifecycle(
        logging,
        SkillLogAction::ConfigurationDelete,
        skill_id,
        "purged",
    );
    if failed.is_empty() {
        record_cleanup(logging, skill_id, &SecretRecovery::Clean);
        return Ok(SecretRecovery::Clean);
    }
    // The rows are gone but some credentials survived; the cleanup state records that a
    // maintenance pass still has work to do.
    repository
        .set_cleanup_state(
            skill_id,
            super::configuration_repository::SkillConfigCleanupState::Failed,
        )
        .map_err(repository_failure)?;
    let recovery = SecretRecovery::Incomplete { properties: failed };
    record_cleanup(logging, skill_id, &recovery);
    Ok(recovery)
}

fn repository_failure(
    error: crate::contexts::tooling::skills::application::SkillApplicationError,
) -> SkillConfigurationError {
    SkillConfigurationError::RepositoryFailure {
        reason: error.to_string(),
    }
}

fn projected_secret_keys(
    existing: &[String],
    intents: &[(String, SkillSecretIntent)],
) -> Vec<String> {
    let mut keys = existing.to_vec();
    for (key, intent) in intents {
        match intent {
            SkillSecretIntent::Replace(_) => {
                if !keys.contains(key) {
                    keys.push(key.clone());
                }
            }
            SkillSecretIntent::Clear => keys.retain(|stored| stored != key),
            SkillSecretIntent::Preserve => {}
        }
    }
    keys.sort();
    keys
}

fn blank_record(
    request: &SkillConfigurationRequest,
    schema: &SkillConfigSchema,
) -> StoredSkillConfiguration {
    StoredSkillConfiguration {
        skill_id: request.skill_id.clone(),
        scope: request.scope,
        workspace_identity: request.workspace_identity.clone(),
        schema_hash: schema.hash.clone(),
        base_revision: request.base_revision.clone(),
        stored_revision: SkillConfigRevision::INITIAL,
        validation_state: SkillConfigDrift::Compatible,
        values: Vec::new(),
        secret_keys: Vec::new(),
        orphaned_at: None,
        cleanup_state: super::configuration_repository::SkillConfigCleanupState::None,
    }
}

#[cfg(test)]
#[path = "configuration_service_tests.rs"]
mod tests;
