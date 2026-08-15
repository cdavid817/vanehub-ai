#![cfg_attr(not(test), allow(dead_code))]

use super::configuration_repository::SqliteSkillConfigurationRepository;
use super::configuration_resolution::resolve_stored_configuration;
use crate::contexts::tooling::skills::application::SkillApplicationError;
use crate::contexts::tooling::skills::domain::{
    SkillConfigReadiness, SkillConfigSchema, SkillConfigSnapshot, SkillConfigValue,
};

/// The configuration block competes with the Skill's own instructions for the model's context, so
/// it is budgeted separately from the Skill body. Over-budget fails the activation rather than
/// truncating, because a half-written value is worse than no value: the model would act on it.
pub(crate) const MAX_SNAPSHOT_VALUE_BYTES: usize = 4 * 1_024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActivationRefusal {
    MissingRequired { properties: Vec<String> },
    MigrationRequired,
    Invalid,
    Oversized { bytes: usize },
}

impl ActivationRefusal {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::MissingRequired { .. } => "missing-required",
            Self::MigrationRequired => "migration-required",
            Self::Invalid => "invalid",
            Self::Oversized { .. } => "oversized",
        }
    }
}

/// Owns a snapshot for the lifetime of one activation. The snapshot is moved in and only handed
/// out by reference, so a Role context or a delegated child cannot be re-pointed at a newer
/// configuration partway through — a later edit changes only a later activation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SkillActivationContext {
    snapshot: SkillConfigSnapshot,
}

impl SkillActivationContext {
    pub(crate) fn new(snapshot: SkillConfigSnapshot) -> Self {
        Self { snapshot }
    }

    pub(crate) fn snapshot(&self) -> &SkillConfigSnapshot {
        &self.snapshot
    }

    /// Handed to a delegated child. It is a clone of an immutable value, so the child observes
    /// exactly what the parent resolved even if the parent later re-activates.
    pub(crate) fn delegate(&self) -> SkillConfigSnapshot {
        self.snapshot.clone()
    }
}

/// Resolves one snapshot immediately before an activation. A Skill that declares no schema is not
/// refused — it activates with no snapshot at all, which is what keeps configuration optional.
pub(crate) fn resolve_activation_snapshot(
    repository: &SqliteSkillConfigurationRepository,
    schema: &SkillConfigSchema,
    skill_id: &str,
    base_revision: &str,
    workspace_identity: &str,
) -> Result<Result<SkillConfigSnapshot, ActivationRefusal>, SkillApplicationError> {
    let resolved = resolve_stored_configuration(repository, schema, skill_id, workspace_identity)?;

    match resolved.readiness {
        SkillConfigReadiness::MigrationRequired => {
            return Ok(Err(ActivationRefusal::MigrationRequired))
        }
        SkillConfigReadiness::Invalid => return Ok(Err(ActivationRefusal::Invalid)),
        SkillConfigReadiness::MissingRequired => {
            let properties = schema
                .fields
                .iter()
                .filter(|field| field.required)
                .filter(|field| !satisfied(&resolved.properties, &field.key))
                .map(|field| field.key.clone())
                .collect();
            return Ok(Err(ActivationRefusal::MissingRequired { properties }));
        }
        SkillConfigReadiness::Ready | SkillConfigReadiness::NotConfigurable => {}
    }

    let bytes = resolved
        .properties
        .iter()
        .filter_map(|property| property.value.as_ref())
        .map(|value| value.canonical().len())
        .sum::<usize>();
    if bytes > MAX_SNAPSHOT_VALUE_BYTES {
        return Ok(Err(ActivationRefusal::Oversized { bytes }));
    }

    Ok(Ok(SkillConfigSnapshot::new(
        skill_id,
        base_revision,
        &schema.hash,
        (!workspace_identity.is_empty()).then(|| workspace_identity.to_string()),
        resolved.properties,
        resolved.readiness,
    )))
}

fn satisfied(
    properties: &[crate::contexts::tooling::skills::domain::SkillConfigProperty],
    key: &str,
) -> bool {
    properties.iter().any(|property| {
        property.key == key
            && (property.value.is_some()
                || property.secret_state
                    == Some(crate::contexts::tooling::skills::domain::SkillSecretState::Configured))
    })
}

/// Serializes the non-secret part of a snapshot for a model-visible instruction context, in a
/// stable key order so an unchanged configuration produces an unchanged prompt prefix. Secret
/// properties contribute their presence and nothing else.
pub(crate) fn render_snapshot_block(snapshot: &SkillConfigSnapshot) -> String {
    let mut lines = Vec::new();
    for property in snapshot.properties() {
        if property.secret {
            lines.push(format!(
                "{}: <{}>",
                property.key,
                property
                    .secret_state
                    .map(|state| state.as_str())
                    .unwrap_or("missing")
            ));
            continue;
        }
        let Some(value) = &property.value else {
            continue;
        };
        lines.push(format!("{}: {}", property.key, render_value(value)));
    }
    lines.join("\n")
}

fn render_value(value: &SkillConfigValue) -> String {
    match value {
        SkillConfigValue::Text(text) => text.clone(),
        SkillConfigValue::Integer(number) => number.to_string(),
        SkillConfigValue::Number(number) => format!("{number}"),
        SkillConfigValue::Boolean(flag) => flag.to_string(),
        SkillConfigValue::List(items) => items
            .iter()
            .map(render_value)
            .collect::<Vec<_>>()
            .join(", "),
    }
}

#[cfg(test)]
#[path = "configuration_activation_tests.rs"]
mod tests;
