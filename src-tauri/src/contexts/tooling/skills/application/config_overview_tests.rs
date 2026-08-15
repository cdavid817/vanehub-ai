use super::config_overview::{
    SkillConfigurableState, SkillConfigurationOverview, SkillScopedConfigSummary,
};
use crate::contexts::tooling::skills::domain::{
    SkillConfigDrift, SkillConfigReadiness, SkillConfigScope, SkillMetadata,
};

const OPTIONAL_ONLY: &str =
    "  properties:\n    endpoint:\n      type: string\n      default: https://default.example\n";
const REQUIRES_INPUT: &str = "  properties:\n    api_key:\n      type: string\n      x-vanehub-secret: true\n  required:\n    - api_key\n";

fn metadata(block: Option<&str>) -> SkillMetadata {
    SkillMetadata::new(
        "configured-skill",
        "Configured",
        "Description",
        "testing",
        "1.0.0",
        Vec::new(),
    )
    .expect("metadata")
    .with_config_schema_block(block.map(str::to_string))
}

#[test]
fn a_configurable_winning_revision_reports_its_exact_schema_and_revision() {
    let overview = SkillConfigurationOverview::from_winning_metadata(
        &metadata(Some(OPTIONAL_ONLY)),
        Some("rev-7".to_string()),
        true,
    );

    assert!(overview.is_configurable());
    assert_eq!(overview.state.as_str(), "configurable");
    assert_eq!(overview.base_revision.as_deref(), Some("rev-7"));
    assert_eq!(overview.readiness, SkillConfigReadiness::Ready);
    assert_eq!(overview.drift, Some(SkillConfigDrift::Compatible));

    let hash = overview.state.schema_hash().expect("hash");
    assert_eq!(hash.len(), 64);
    assert!(overview.permits_activation());
}

#[test]
fn a_shadowed_revisions_schema_does_not_become_the_active_one() {
    let winning = SkillConfigurationOverview::from_winning_metadata(
        &metadata(Some(OPTIONAL_ONLY)),
        Some("rev-2".to_string()),
        false,
    );
    let shadowed = SkillConfigurationOverview::from_winning_metadata(
        &metadata(Some(REQUIRES_INPUT)),
        Some("rev-1".to_string()),
        false,
    );

    assert_ne!(
        winning.state.schema_hash().expect("winning hash"),
        shadowed.state.schema_hash().expect("shadowed hash")
    );
    // The winning revision needs no input; the shadowed one would have demanded a secret.
    assert_eq!(winning.readiness, SkillConfigReadiness::Ready);
    assert_eq!(shadowed.readiness, SkillConfigReadiness::MissingRequired);
}

#[test]
fn a_skill_without_a_schema_is_not_configurable_and_still_activates() {
    let overview = SkillConfigurationOverview::from_winning_metadata(&metadata(None), None, true);

    assert!(!overview.is_configurable());
    assert_eq!(overview.state, SkillConfigurableState::NotConfigurable);
    assert_eq!(overview.readiness, SkillConfigReadiness::NotConfigurable);
    assert!(overview.available_scopes.is_empty());
    assert_eq!(overview.drift, None);
    assert!(overview.permits_activation());
}

#[test]
fn an_unsupported_schema_blocks_activation_without_inferring_a_permissive_one() {
    let overview = SkillConfigurationOverview::from_winning_metadata(
        &metadata(Some("  properties:\n    field:\n      type: date\n")),
        Some("rev-3".to_string()),
        true,
    );

    assert!(!overview.is_configurable());
    assert_eq!(overview.state.as_str(), "schema-unsupported");
    assert_eq!(overview.state.schema_hash(), None);
    assert_eq!(overview.readiness, SkillConfigReadiness::Invalid);
    assert_eq!(overview.drift, Some(SkillConfigDrift::Invalid));
    assert!(!overview.permits_activation());
}

#[test]
fn required_configuration_without_a_default_is_not_ready_before_values_resolve() {
    let overview = SkillConfigurationOverview::from_winning_metadata(
        &metadata(Some(REQUIRES_INPUT)),
        Some("rev-4".to_string()),
        true,
    );

    assert!(overview.is_configurable());
    assert_eq!(overview.readiness, SkillConfigReadiness::MissingRequired);
    assert!(!overview.permits_activation());
}

#[test]
fn project_scope_is_offered_only_when_a_workspace_is_available() {
    let with_workspace = SkillConfigurationOverview::from_winning_metadata(
        &metadata(Some(OPTIONAL_ONLY)),
        None,
        true,
    );
    let without_workspace = SkillConfigurationOverview::from_winning_metadata(
        &metadata(Some(OPTIONAL_ONLY)),
        None,
        false,
    );

    assert_eq!(
        with_workspace.available_scopes,
        vec![SkillConfigScope::User, SkillConfigScope::Project]
    );
    assert_eq!(
        without_workspace.available_scopes,
        vec![SkillConfigScope::User]
    );
}

#[test]
fn an_unevaluated_overview_is_distinguishable_from_a_skill_that_has_no_schema() {
    let unevaluated = SkillConfigurationOverview::not_evaluated();

    assert_eq!(unevaluated.state, SkillConfigurableState::NotEvaluated);
    assert_ne!(unevaluated.state, SkillConfigurableState::NotConfigurable);
    assert!(!unevaluated.is_configurable());
    // Not knowing must not block a Skill that may well have no configuration at all.
    assert!(unevaluated.permits_activation());
}

#[test]
fn scoped_summaries_carry_counts_and_witnesses_but_no_values() {
    let overview = SkillConfigurationOverview::from_winning_metadata(
        &metadata(Some(REQUIRES_INPUT)),
        Some("rev-5".to_string()),
        true,
    )
    .with_scoped_summaries(vec![
        SkillScopedConfigSummary {
            scope: SkillConfigScope::User,
            configured_property_count: 2,
            configured_secret_count: 1,
            stored_revision: Some(4),
            schema_hash: Some("abc".to_string()),
        },
        SkillScopedConfigSummary {
            scope: SkillConfigScope::Project,
            configured_property_count: 0,
            configured_secret_count: 0,
            stored_revision: None,
            schema_hash: None,
        },
    ]);

    assert_eq!(overview.scoped.len(), 2);
    assert_eq!(overview.scoped[0].configured_secret_count, 1);
    assert_eq!(overview.scoped[0].stored_revision, Some(4));
    // The whole rendered overview must not be able to carry a stored value.
    let rendered = format!("{overview:?}");
    assert!(!rendered.contains("secret-value"));
    assert!(!rendered.contains("credential"));
}
