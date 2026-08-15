#![cfg_attr(not(test), allow(dead_code))]

use super::configuration_activation::render_snapshot_block;
use crate::contexts::tooling::skills::domain::{SkillConfigSchema, SkillConfigSnapshot};

/// The configuration block is budgeted separately from the Skill body so that a large
/// configuration cannot silently crowd out the instructions it is supposed to parameterise. Both
/// budgets apply; neither borrows from the other.
pub(crate) const MAX_CONFIGURATION_SUBSECTION_CHARACTERS: usize = 2_048;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConfigurationSubsection {
    Omitted,
    Rendered(String),
    OverBudget { characters: usize },
}

/// Renders the model-visible subsection for an eligible native API context. Over-budget is
/// reported rather than trimmed: the caller fails the activation, because a truncated block would
/// present a partial value as if it were the configured one.
pub(crate) fn render_configuration_subsection(
    snapshot: Option<&SkillConfigSnapshot>,
) -> ConfigurationSubsection {
    let Some(snapshot) = snapshot else {
        return ConfigurationSubsection::Omitted;
    };
    let block = render_snapshot_block(snapshot);
    if block.is_empty() {
        return ConfigurationSubsection::Omitted;
    }
    let characters = block.chars().count();
    if characters > MAX_CONFIGURATION_SUBSECTION_CHARACTERS {
        return ConfigurationSubsection::OverBudget { characters };
    }
    ConfigurationSubsection::Rendered(block)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfigurationConsumption {
    /// A native API context reads the snapshot directly.
    Native,
    /// An external CLI receives the Skill package and nothing else. Stated rather than left
    /// implicit so the UI can say so instead of letting a user assume their values apply.
    UnsupportedExternalCli,
}

impl ConfigurationConsumption {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::UnsupportedExternalCli => "unsupported-external-cli",
        }
    }

    pub(crate) fn is_supported(self) -> bool {
        matches!(self, Self::Native)
    }
}

/// There is no bridge, so every external CLI binding reports unsupported. A future bridge has to
/// define its own security and lifecycle behaviour before this can vary by agent.
pub(crate) fn consumption_for_binding(is_native_api_agent: bool) -> ConfigurationConsumption {
    if is_native_api_agent {
        ConfigurationConsumption::Native
    } else {
        ConfigurationConsumption::UnsupportedExternalCli
    }
}

/// An Overlay can change `config_schema` because it rewrites non-executable SKILL.md text. The
/// schema that results is a different effective schema, so its hash differs and stored records
/// bound to the old hash drift. This is derivation only — an Overlay never writes a User or
/// Project record, and it has no path to a credential slot.
pub(crate) fn overlay_schema_changed(
    previous: Option<&SkillConfigSchema>,
    next: Option<&SkillConfigSchema>,
) -> bool {
    match (previous, next) {
        (None, None) => false,
        (Some(previous), Some(next)) => previous.hash != next.hash,
        _ => true,
    }
}

/// Structural facts only. Evolution signals, dossiers, Curator records, and notifications receive
/// these instead of the configuration itself: whether a Skill is configurable, how many properties
/// it declares, how many are secret, and whether it is ready. No value, key, or credential alias
/// is carried, so an evolution candidate cannot be seeded from a user's operational settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConfigurationEvolutionSignal {
    pub(crate) configurable: bool,
    pub(crate) property_count: usize,
    pub(crate) secret_property_count: usize,
    pub(crate) ready: bool,
}

pub(crate) fn evolution_signal(
    schema: Option<&SkillConfigSchema>,
    snapshot: Option<&SkillConfigSnapshot>,
) -> ConfigurationEvolutionSignal {
    let Some(schema) = schema else {
        return ConfigurationEvolutionSignal {
            configurable: false,
            property_count: 0,
            secret_property_count: 0,
            ready: true,
        };
    };
    ConfigurationEvolutionSignal {
        configurable: true,
        property_count: schema.fields.len(),
        secret_property_count: schema.fields.iter().filter(|field| field.secret).count(),
        ready: snapshot
            .map(|snapshot| snapshot.readiness().permits_activation())
            .unwrap_or(false),
    }
}

#[cfg(test)]
#[path = "configuration_boundaries_tests.rs"]
mod tests;
