#![cfg_attr(not(test), allow(dead_code))]

use super::config_schema::{validate_value, SkillConfigField, SkillConfigSchema, SkillConfigValue};
use sha2::{Digest, Sha256};
use std::fmt::{self, Write};

/// No System or Remote scope exists: defaults come only from the schema, so a third writable
/// layer would have no owner and no reset semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SkillConfigScope {
    User,
    Project,
}

impl SkillConfigScope {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillConfigProvenance {
    Project,
    User,
    SchemaDefault,
    Missing,
}

impl SkillConfigProvenance {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::User => "user",
            Self::SchemaDefault => "default",
            Self::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillSecretState {
    Configured,
    Missing,
    Error,
}

impl SkillSecretState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Configured => "configured",
            Self::Missing => "missing",
            Self::Error => "error",
        }
    }
}

/// Three-way intent rather than a nullable value: a save that round-tripped the stored secret
/// would require reading it back out of the credential store and through a DTO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillSecretIntent {
    Preserve,
    Replace(String),
    Clear,
}

impl fmt::Debug for RedactedSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RedactedSecret(<redacted>)")
    }
}

/// Wrapper whose Debug never prints the value, so a secret cannot reach a log line through an
/// incidental `{:?}` on a surrounding struct.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct RedactedSecret(String);

impl RedactedSecret {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillConfigDrift {
    Compatible,
    MigrationRequired,
    Invalid,
}

impl SkillConfigDrift {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::MigrationRequired => "migration-required",
            Self::Invalid => "invalid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillConfigReadiness {
    Ready,
    MissingRequired,
    MigrationRequired,
    Invalid,
    NotConfigurable,
}

impl SkillConfigReadiness {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::MissingRequired => "missing-required",
            Self::MigrationRequired => "migration-required",
            Self::Invalid => "invalid",
            Self::NotConfigurable => "not-configurable",
        }
    }

    pub(crate) fn permits_activation(self) -> bool {
        matches!(self, Self::Ready | Self::NotConfigurable)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SkillConfigRevision(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SkillConfigRevisionOverflow;

impl fmt::Display for SkillConfigRevisionOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("Skill configuration revision cannot advance beyond the supported range")
    }
}

impl SkillConfigRevision {
    pub(crate) const INITIAL: Self = Self(0);

    pub(crate) fn new(value: u64) -> Self {
        Self(value)
    }

    pub(crate) fn value(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> Result<Self, SkillConfigRevisionOverflow> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(SkillConfigRevisionOverflow)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SkillConfigProperty {
    pub(crate) key: String,
    pub(crate) value: Option<SkillConfigValue>,
    pub(crate) provenance: SkillConfigProvenance,
    pub(crate) secret: bool,
    pub(crate) secret_state: Option<SkillSecretState>,
}

/// Immutable by construction: the activation that resolved it keeps it for its lifetime, so a
/// concurrent edit changes only later activations.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SkillConfigSnapshot {
    skill_id: String,
    base_revision: String,
    schema_hash: String,
    workspace: Option<String>,
    properties: Vec<SkillConfigProperty>,
    readiness: SkillConfigReadiness,
    digest: String,
}

impl SkillConfigSnapshot {
    pub(crate) fn new(
        skill_id: impl Into<String>,
        base_revision: impl Into<String>,
        schema_hash: impl Into<String>,
        workspace: Option<String>,
        mut properties: Vec<SkillConfigProperty>,
        readiness: SkillConfigReadiness,
    ) -> Self {
        let skill_id = skill_id.into();
        let base_revision = base_revision.into();
        let schema_hash = schema_hash.into();
        properties.sort_by(|left, right| left.key.cmp(&right.key));
        let mut canonical = format!(
            "skill:{skill_id}|base:{base_revision}|schema:{schema_hash}|workspace:{}|readiness:{}",
            workspace.clone().unwrap_or_default(),
            readiness.as_str()
        );
        for property in &properties {
            let _ = write!(
                canonical,
                "\n{}={}|from:{}|secret:{}|state:{}",
                property.key,
                property
                    .value
                    .as_ref()
                    .map(SkillConfigValue::canonical)
                    .unwrap_or_default(),
                property.provenance.as_str(),
                property.secret,
                property
                    .secret_state
                    .map(SkillSecretState::as_str)
                    .unwrap_or("n/a"),
            );
        }
        let digest = Sha256::digest(canonical.as_bytes()).iter().fold(
            String::with_capacity(64),
            |mut encoded, byte| {
                let _ = write!(encoded, "{byte:02x}");
                encoded
            },
        );
        Self {
            skill_id,
            base_revision,
            schema_hash,
            workspace,
            properties,
            readiness,
            digest,
        }
    }

    pub(crate) fn skill_id(&self) -> &str {
        &self.skill_id
    }

    pub(crate) fn base_revision(&self) -> &str {
        &self.base_revision
    }

    pub(crate) fn schema_hash(&self) -> &str {
        &self.schema_hash
    }

    pub(crate) fn workspace(&self) -> Option<&str> {
        self.workspace.as_deref()
    }

    pub(crate) fn properties(&self) -> &[SkillConfigProperty] {
        &self.properties
    }

    pub(crate) fn readiness(&self) -> SkillConfigReadiness {
        self.readiness
    }

    pub(crate) fn digest(&self) -> &str {
        &self.digest
    }

    pub(crate) fn property(&self, key: &str) -> Option<&SkillConfigProperty> {
        self.properties.iter().find(|property| property.key == key)
    }
}

/// Resolves one property at a time as Project over User over schema default. An inherited value
/// is reported with the scope it came from and is never materialized into the higher scope, so
/// clearing the lower scope later still changes the effective value.
pub(crate) fn resolve_effective(
    schema: &SkillConfigSchema,
    user: &[(String, SkillConfigValue)],
    project: &[(String, SkillConfigValue)],
    secret_states: &[(String, SkillSecretState)],
) -> Vec<SkillConfigProperty> {
    let lookup = |records: &[(String, SkillConfigValue)], key: &str| {
        records
            .iter()
            .find(|(stored, _)| stored == key)
            .map(|(_, value)| value.clone())
    };
    let mut resolved = schema
        .fields
        .iter()
        .map(|field| {
            if field.secret {
                let state = secret_states
                    .iter()
                    .find(|(key, _)| key == &field.key)
                    .map(|(_, state)| *state)
                    .unwrap_or(SkillSecretState::Missing);
                return SkillConfigProperty {
                    key: field.key.clone(),
                    value: None,
                    provenance: match state {
                        SkillSecretState::Configured => SkillConfigProvenance::User,
                        _ => SkillConfigProvenance::Missing,
                    },
                    secret: true,
                    secret_state: Some(state),
                };
            }
            let (value, provenance) = if let Some(value) = lookup(project, &field.key) {
                (Some(value), SkillConfigProvenance::Project)
            } else if let Some(value) = lookup(user, &field.key) {
                (Some(value), SkillConfigProvenance::User)
            } else if let Some(default) = field.default.clone() {
                (Some(default), SkillConfigProvenance::SchemaDefault)
            } else {
                (None, SkillConfigProvenance::Missing)
            };
            SkillConfigProperty {
                key: field.key.clone(),
                value,
                provenance,
                secret: false,
                secret_state: None,
            }
        })
        .collect::<Vec<_>>();
    resolved.sort_by(|left, right| left.key.cmp(&right.key));
    resolved
}

/// Classifies stored values against a schema without mutating them. Adding an optional property
/// stays compatible; a removed, retyped, or reclassified property requires explicit migration.
pub(crate) fn classify_drift(
    schema: &SkillConfigSchema,
    stored: &[(String, SkillConfigValue)],
    stored_secret_keys: &[String],
) -> SkillConfigDrift {
    for (key, value) in stored {
        let Some(field) = schema.field(key) else {
            return SkillConfigDrift::MigrationRequired;
        };
        if field.secret {
            // The value moved out from under the credential store; refuse to reuse it either way.
            return SkillConfigDrift::MigrationRequired;
        }
        if validate_value(
            value,
            field.field_type,
            &field.constraints,
            &field.choices,
            key,
        )
        .is_err()
        {
            return SkillConfigDrift::MigrationRequired;
        }
    }
    for key in stored_secret_keys {
        match schema.field(key) {
            None => return SkillConfigDrift::MigrationRequired,
            Some(field) if !field.secret => return SkillConfigDrift::MigrationRequired,
            Some(_) => {}
        }
    }
    SkillConfigDrift::Compatible
}

pub(crate) fn readiness_for(
    schema: &SkillConfigSchema,
    resolved: &[SkillConfigProperty],
    drift: SkillConfigDrift,
) -> SkillConfigReadiness {
    match drift {
        SkillConfigDrift::Invalid => return SkillConfigReadiness::Invalid,
        SkillConfigDrift::MigrationRequired => return SkillConfigReadiness::MigrationRequired,
        SkillConfigDrift::Compatible => {}
    }
    let missing_required =
        schema
            .fields
            .iter()
            .filter(|field| field.required)
            .any(|field: &SkillConfigField| {
                match resolved.iter().find(|property| property.key == field.key) {
                    None => true,
                    Some(property) if field.secret => {
                        property.secret_state != Some(SkillSecretState::Configured)
                    }
                    Some(property) => property.value.is_none(),
                }
            });
    if missing_required {
        SkillConfigReadiness::MissingRequired
    } else {
        SkillConfigReadiness::Ready
    }
}
