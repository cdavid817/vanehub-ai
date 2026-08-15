#![cfg_attr(not(test), allow(dead_code))]

use super::configuration_repository::{
    SqliteSkillConfigurationRepository, StoredSkillConfiguration,
};
use crate::contexts::tooling::skills::application::SkillApplicationError;
use crate::contexts::tooling::skills::domain::{
    classify_drift, readiness_for, resolve_effective, validate_value, SkillConfigDrift,
    SkillConfigProperty, SkillConfigReadiness, SkillConfigRevision, SkillConfigSchema,
    SkillConfigScope, SkillConfigValue, SkillSecretState,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScopeConfigurationState {
    pub(crate) scope: SkillConfigScope,
    pub(crate) stored_revision: SkillConfigRevision,
    pub(crate) schema_hash: String,
    pub(crate) drift: SkillConfigDrift,
    pub(crate) configured_property_count: usize,
    pub(crate) configured_secret_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedSkillConfiguration {
    pub(crate) properties: Vec<SkillConfigProperty>,
    pub(crate) drift: SkillConfigDrift,
    pub(crate) readiness: SkillConfigReadiness,
    pub(crate) scopes: Vec<ScopeConfigurationState>,
}

/// A Project value is meaningless without something to scope it to, and an empty or relative path
/// would let two different directories share one record. Rejecting is the whole behaviour: there
/// is deliberately no fallback to User, because silently widening a Project write is worse than
/// refusing it.
pub(crate) fn require_canonical_workspace(
    workspace_identity: &str,
) -> Result<&str, SkillApplicationError> {
    let trimmed = workspace_identity.trim();
    if trimmed.is_empty() || trimmed != workspace_identity {
        return Err(SkillApplicationError::Validation(
            "Project-scoped Skill configuration requires a canonical workspace".to_string(),
        ));
    }
    Ok(trimmed)
}

pub(crate) fn resolve_stored_configuration(
    repository: &SqliteSkillConfigurationRepository,
    schema: &SkillConfigSchema,
    skill_id: &str,
    workspace_identity: &str,
) -> Result<ResolvedSkillConfiguration, SkillApplicationError> {
    let records = repository.load_all_scopes(skill_id, workspace_identity)?;
    Ok(resolve_from_records(schema, &records))
}

pub(crate) fn resolve_from_records(
    schema: &SkillConfigSchema,
    records: &[StoredSkillConfiguration],
) -> ResolvedSkillConfiguration {
    let mut scopes = Vec::new();
    let mut worst = SkillConfigDrift::Compatible;
    let mut user_values = Vec::new();
    let mut project_values = Vec::new();
    let mut secret_states: Vec<(String, SkillSecretState)> = Vec::new();

    for record in records {
        let drift = classify_drift(schema, &record.values, &record.secret_keys);
        worst = worse_of(worst, drift);
        scopes.push(ScopeConfigurationState {
            scope: record.scope,
            stored_revision: record.stored_revision,
            schema_hash: record.schema_hash.clone(),
            drift,
            configured_property_count: record.values.len(),
            configured_secret_count: record.secret_keys.len(),
        });

        // Only values that still validate feed resolution. A stored value that no longer matches
        // its property must not reach a runtime snapshot, and dropping it here is what keeps the
        // rest of the Skill's configuration usable while the drift is reported separately.
        let usable = record
            .values
            .iter()
            .filter(|(key, value)| validates(schema, key, value))
            .cloned()
            .collect::<Vec<_>>();
        match record.scope {
            SkillConfigScope::User => user_values = usable,
            SkillConfigScope::Project => project_values = usable,
        }
        for key in &record.secret_keys {
            if schema.field(key).is_some_and(|field| field.secret) {
                secret_states.push((key.clone(), SkillSecretState::Configured));
            }
        }
    }

    let properties = resolve_effective(schema, &user_values, &project_values, &secret_states);
    let readiness = readiness_for(schema, &properties, worst);
    scopes.sort_by_key(|state| state.scope);
    ResolvedSkillConfiguration {
        properties,
        drift: worst,
        readiness,
        scopes,
    }
}

fn validates(schema: &SkillConfigSchema, key: &str, value: &SkillConfigValue) -> bool {
    schema.field(key).is_some_and(|field| {
        !field.secret
            && validate_value(
                value,
                field.field_type,
                &field.constraints,
                &field.choices,
                key,
            )
            .is_ok()
    })
}

fn worse_of(left: SkillConfigDrift, right: SkillConfigDrift) -> SkillConfigDrift {
    let rank = |drift: SkillConfigDrift| match drift {
        SkillConfigDrift::Compatible => 0,
        SkillConfigDrift::MigrationRequired => 1,
        SkillConfigDrift::Invalid => 2,
    };
    if rank(right) > rank(left) {
        right
    } else {
        left
    }
}

#[cfg(test)]
#[path = "configuration_resolution_tests.rs"]
mod tests;
