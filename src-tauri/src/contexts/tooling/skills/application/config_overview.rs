#![cfg_attr(not(test), allow(dead_code))]

use crate::contexts::tooling::skills::domain::{
    SkillConfigDrift, SkillConfigReadiness, SkillConfigSchema, SkillConfigScope, SkillMetadata,
};

/// What a configuration operation needs before it can touch storage: the normalized schema of the
/// winning revision and the overview that says which revision that was. Both are derived from the
/// same metadata in one read, because resolving them separately would let a Skill replacement
/// slip between them and bind values to a revision that is no longer effective.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SkillConfigurationContext {
    /// `Some` only when the winning revision declares a schema that normalizes; an unsupported
    /// schema is reported through `overview.state` instead of degrading to a permissive one.
    pub(crate) schema: Option<SkillConfigSchema>,
    pub(crate) overview: SkillConfigurationOverview,
}

impl SkillConfigurationContext {
    pub(crate) fn from_winning_metadata(
        metadata: &SkillMetadata,
        overview: SkillConfigurationOverview,
    ) -> Self {
        Self {
            schema: metadata
                .config_schema()
                .and_then(Result::ok)
                .filter(|_| overview.is_configurable()),
            overview,
        }
    }
}

/// Distinguishes "this revision declares no schema" from "we did not resolve one here". The
/// registry rebuilds metadata without the frontmatter block, so a record read from storage cannot
/// answer the question, and reporting it as not-configurable would silently hide a configurable
/// Skill's settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillConfigurableState {
    NotEvaluated,
    NotConfigurable,
    Configurable { schema_hash: String },
    SchemaUnsupported { reason: String },
}

impl SkillConfigurableState {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::NotEvaluated => "not-evaluated",
            Self::NotConfigurable => "not-configurable",
            Self::Configurable { .. } => "configurable",
            Self::SchemaUnsupported { .. } => "schema-unsupported",
        }
    }

    pub(crate) fn schema_hash(&self) -> Option<&str> {
        match self {
            Self::Configurable { schema_hash } => Some(schema_hash.as_str()),
            _ => None,
        }
    }
}

/// Counts and witnesses only. Property keys are already public in the Skill package, but stored
/// values and credential aliases are not, and an overview is rendered in list contexts where no
/// value is needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillScopedConfigSummary {
    pub(crate) scope: SkillConfigScope,
    pub(crate) configured_property_count: usize,
    pub(crate) configured_secret_count: usize,
    pub(crate) stored_revision: Option<u64>,
    pub(crate) schema_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillConfigurationOverview {
    pub(crate) state: SkillConfigurableState,
    pub(crate) base_revision: Option<String>,
    pub(crate) available_scopes: Vec<SkillConfigScope>,
    pub(crate) readiness: SkillConfigReadiness,
    pub(crate) drift: Option<SkillConfigDrift>,
    pub(crate) scoped: Vec<SkillScopedConfigSummary>,
}

impl SkillConfigurationOverview {
    pub(crate) fn not_evaluated() -> Self {
        Self {
            state: SkillConfigurableState::NotEvaluated,
            base_revision: None,
            available_scopes: Vec::new(),
            readiness: SkillConfigReadiness::NotConfigurable,
            drift: None,
            scoped: Vec::new(),
        }
    }

    /// Derives the overview from the winning revision's metadata. Callers must pass the effective
    /// package's metadata: a shadowed revision's schema must not determine the active one.
    pub(crate) fn from_winning_metadata(
        metadata: &SkillMetadata,
        base_revision: Option<String>,
        workspace_available: bool,
    ) -> Self {
        let Some(parsed) = metadata.config_schema() else {
            return Self {
                state: SkillConfigurableState::NotConfigurable,
                base_revision,
                available_scopes: Vec::new(),
                readiness: SkillConfigReadiness::NotConfigurable,
                drift: None,
                scoped: Vec::new(),
            };
        };
        let mut available_scopes = vec![SkillConfigScope::User];
        if workspace_available {
            available_scopes.push(SkillConfigScope::Project);
        }
        match parsed {
            Err(error) => Self {
                state: SkillConfigurableState::SchemaUnsupported {
                    reason: error.to_string(),
                },
                base_revision,
                available_scopes,
                readiness: SkillConfigReadiness::Invalid,
                drift: Some(SkillConfigDrift::Invalid),
                scoped: Vec::new(),
            },
            Ok(schema) => {
                // Readiness stays pessimistic until stored values are resolved: a required
                // property with no default has no effective value yet, and claiming Ready here
                // would let an activation start that must not.
                let readiness = if schema
                    .fields
                    .iter()
                    .any(|field| field.required && field.default.is_none())
                {
                    SkillConfigReadiness::MissingRequired
                } else {
                    SkillConfigReadiness::Ready
                };
                Self {
                    state: SkillConfigurableState::Configurable {
                        schema_hash: schema.hash.clone(),
                    },
                    base_revision,
                    available_scopes,
                    readiness,
                    drift: Some(SkillConfigDrift::Compatible),
                    scoped: Vec::new(),
                }
            }
        }
    }

    pub(crate) fn with_scoped_summaries(mut self, scoped: Vec<SkillScopedConfigSummary>) -> Self {
        self.scoped = scoped;
        self
    }

    pub(crate) fn is_configurable(&self) -> bool {
        matches!(self.state, SkillConfigurableState::Configurable { .. })
    }

    pub(crate) fn permits_activation(&self) -> bool {
        match self.state {
            SkillConfigurableState::NotEvaluated | SkillConfigurableState::NotConfigurable => true,
            _ => self.readiness.permits_activation(),
        }
    }
}
