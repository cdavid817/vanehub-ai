#![cfg_attr(not(test), allow(dead_code))]

use super::configuration_repository::{
    SkillConfigurationSave, SkillConfigurationWrite, SqliteSkillConfigurationRepository,
    StoredSkillConfiguration,
};
use super::configuration_resolution::{
    require_canonical_workspace, resolve_from_records, ResolvedSkillConfiguration,
};
use super::configuration_secrets::{SecretRecovery, SkillConfigurationSecrets, SkillSecretStore};
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
    schema: &SkillConfigSchema,
    request: &SkillConfigurationRequest,
) -> Result<SkillConfigurationSaveResult, SkillConfigurationError> {
    validate_request(schema, request)?;

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
    let others = repository
        .load_all_scopes(&request.skill_id, &request.workspace_identity)
        .map_err(|error| SkillConfigurationError::RepositoryFailure {
            reason: error.to_string(),
        })?;
    Ok(SkillConfigurationSaveResult {
        preview: resolve_from_records(schema, &others),
        record,
        recovery,
    })
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
