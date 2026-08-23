//! Deterministic definition fixtures shared by the domain unit tests. Production code never
//! constructs definitions by hand; it loads them from the canonical registry.

use super::definition::{
    CliLaunchScope, CliParameterAudit, CliParameterCategory, CliParameterCompatibility,
    CliParameterConstraints, CliParameterControl, CliParameterDefinition, CliParameterDependencies,
    CliParameterMaturity, CliParameterOption, CliParameterOwnership, CliParameterRisk,
    CliParameterVerification,
};
use super::rendering::{CliArgumentSlot, CliParameterRenderer};
use super::selection::CliParameterSelection;

fn audit() -> CliParameterAudit {
    CliParameterAudit {
        source_id: "test-fixture".to_string(),
        source_url: "https://example.invalid/reference".to_string(),
        reviewed_at: "2026-08-22".to_string(),
        reviewed_state: "Domain unit-test fixture; no external artefact.".to_string(),
        verification: CliParameterVerification::RepositoryVerified,
        note: "Domain unit-test fixture.".to_string(),
    }
}

fn option(value: &str) -> CliParameterOption {
    CliParameterOption {
        value: value.to_string(),
        label_key: format!("cliParameters.test.values.{value}.label"),
        description_key: format!("cliParameters.test.values.{value}.description"),
        compatibility: None,
    }
}

fn base(
    id: &str,
    control: CliParameterControl,
    renderer: CliParameterRenderer,
) -> CliParameterDefinition {
    CliParameterDefinition {
        id: id.to_string(),
        agent_id: "claude-code".to_string(),
        category: CliParameterCategory::Model,
        ownership: CliParameterOwnership::UserEditable,
        maturity: CliParameterMaturity::Stable,
        control,
        label_key: format!("cliParameters.test.{id}.label"),
        description_key: format!("cliParameters.test.{id}.description"),
        default_selection: CliParameterSelection::Inherit,
        launch_scopes: vec![CliLaunchScope::Interactive, CliLaunchScope::Chat],
        risk: CliParameterRisk::Normal,
        advanced: false,
        options: Vec::new(),
        renderer,
        constraints: CliParameterConstraints::default(),
        compatibility: CliParameterCompatibility::default(),
        dependencies: CliParameterDependencies::default(),
        diagnostics: Vec::new(),
        audit: audit(),
    }
}

pub(crate) fn custom_text_definition() -> CliParameterDefinition {
    let mut definition = base(
        "model",
        CliParameterControl::CustomText,
        CliParameterRenderer::FlagValue {
            flag: "--model".to_string(),
            slot: CliArgumentSlot::Global,
        },
    );
    definition.options = vec![option("sonnet"), option("opus")];
    definition.constraints = CliParameterConstraints {
        max_length: Some(64),
        pattern: Some("^[A-Za-z0-9][A-Za-z0-9._:@/+-]*$".to_string()),
        ..CliParameterConstraints::default()
    };
    definition
}

pub(crate) fn enum_definition() -> CliParameterDefinition {
    let mut definition = base(
        "effort",
        CliParameterControl::Enum,
        CliParameterRenderer::FlagValue {
            flag: "--effort".to_string(),
            slot: CliArgumentSlot::Global,
        },
    );
    definition.options = vec![option("low"), option("medium"), option("high")];
    definition
}

pub(crate) fn boolean_definition() -> CliParameterDefinition {
    base(
        "search",
        CliParameterControl::BooleanFlag,
        CliParameterRenderer::PresenceFlag {
            flag: "--search".to_string(),
            slot: CliArgumentSlot::Global,
        },
    )
}

pub(crate) fn tri_state_definition() -> CliParameterDefinition {
    base(
        "chrome",
        CliParameterControl::TriState,
        CliParameterRenderer::PositiveNegativeFlag {
            positive_flag: "--chrome".to_string(),
            negative_flag: "--no-chrome".to_string(),
            slot: CliArgumentSlot::Global,
        },
    )
}

pub(crate) fn list_definition() -> CliParameterDefinition {
    let mut definition = base(
        "extensions",
        CliParameterControl::OrderedStringList,
        CliParameterRenderer::RepeatFlagValue {
            flag: "--extensions".to_string(),
            slot: CliArgumentSlot::Global,
        },
    );
    definition.constraints = CliParameterConstraints {
        max_items: Some(8),
        item_max_length: Some(64),
        item_pattern: Some("^[A-Za-z0-9][A-Za-z0-9._-]*$".to_string()),
        dedupe: true,
        exclusive_values: vec!["none".to_string()],
        ..CliParameterConstraints::default()
    };
    definition
}
