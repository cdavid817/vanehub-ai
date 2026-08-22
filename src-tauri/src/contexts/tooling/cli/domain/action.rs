//! What VaneHub may do to a CLI, decided here and nowhere else.
//!
//! React renders what this module returns. It does not compare versions, infer upgrade versus
//! downgrade, or decide whether a source is manageable -- the page that did produced the two
//! defects this replaces: a target equal to the active version was derived as `upgrade`, and the
//! version the user selected never reached the request because the action was resolved from
//! `latestVersion` instead.
//!
//! `resolve_target` is the structural fix for the first. Equality has its own variant, and no
//! mutation kind can be produced from it.

use super::definition::{CliDistributionAction, CliDistributionDefinition};
use super::ids::CliSourceId;
use super::source::{CliPlatform, CliSourceConfidence, CliTargetVersionMode};
use super::version::NormalizedCliVersion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliActionKind {
    Install,
    Upgrade,
    Downgrade,
    Reinstall,
    Uninstall,
    Repair,
}

impl CliActionKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Upgrade => "upgrade",
            Self::Downgrade => "downgrade",
            Self::Reinstall => "reinstall",
            Self::Uninstall => "uninstall",
            Self::Repair => "repair",
        }
    }

    /// The four actions that carry a target version. `uninstall` and `repair` do not.
    fn as_distribution_action(self) -> Option<CliDistributionAction> {
        match self {
            Self::Install => Some(CliDistributionAction::Install),
            Self::Upgrade => Some(CliDistributionAction::Upgrade),
            Self::Downgrade => Some(CliDistributionAction::Downgrade),
            Self::Reinstall => Some(CliDistributionAction::Reinstall),
            Self::Uninstall | Self::Repair => None,
        }
    }
}

/// Why an action is offered, or why one the user might expect is absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliActionReasonCode {
    /// The selected target is already the active version. There is nothing to run.
    AlreadyCurrent,
    /// VaneHub can see this source but does not manage it in this change.
    DetectOnlySource,
    CatalogUnavailable,
    /// The source exists but cannot act on this platform -- including a vendor with no template
    /// for it.
    SourceUnavailableOnPlatform,
    /// One of the versions does not parse, so upgrade and downgrade cannot be told apart.
    UnorderedVersions,
    /// Nothing establishes that this source owns the active installation, so mutating through it
    /// could act on a different install than the one the user is looking at.
    SourceOwnershipUnproven,
    ActionUnsupportedBySource,
    /// The active executable does not run, so an action that assumes a working install is unsafe.
    ActiveInstallationBroken,
}

impl CliActionReasonCode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::AlreadyCurrent => "already-current",
            Self::DetectOnlySource => "detect-only-source",
            Self::CatalogUnavailable => "catalog-unavailable",
            Self::SourceUnavailableOnPlatform => "source-unavailable-on-platform",
            Self::UnorderedVersions => "unordered-versions",
            Self::SourceOwnershipUnproven => "source-ownership-unproven",
            Self::ActionUnsupportedBySource => "action-unsupported-by-source",
            Self::ActiveInstallationBroken => "active-installation-broken",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliAllowedAction {
    pub(crate) action: CliActionKind,
    pub(crate) source_id: CliSourceId,
    pub(crate) target_mode: CliTargetVersionMode,
    /// What the action aims at when the user picks nothing. Never substituted for a target the
    /// user *did* pick.
    pub(crate) default_target: Option<String>,
    pub(crate) requires_target_selection: bool,
    pub(crate) reason_code: Option<CliActionReasonCode>,
}

/// What running toward `target` would mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliTargetResolution {
    /// Nothing is installed, so this is a first install.
    Install,
    Upgrade,
    Downgrade,
    /// The target is already active. No mutation exists for this, by construction -- there is no
    /// `CliActionKind` this variant maps to.
    Current,
    /// The two versions cannot be ordered. Upgrade and downgrade are both unprovable, so neither
    /// is claimed.
    Indeterminate,
}

impl CliTargetResolution {
    /// The mutation this resolution calls for, if any. `Current` and `Indeterminate` yield `None`,
    /// which is what makes a redundant install unrepresentable rather than merely discouraged.
    pub(crate) fn mutation(self) -> Option<CliActionKind> {
        match self {
            Self::Install => Some(CliActionKind::Install),
            Self::Upgrade => Some(CliActionKind::Upgrade),
            Self::Downgrade => Some(CliActionKind::Downgrade),
            Self::Current | Self::Indeterminate => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Upgrade => "upgrade",
            Self::Downgrade => "downgrade",
            Self::Current => "current",
            Self::Indeterminate => "indeterminate",
        }
    }
}

/// Compares an exact target with the active version.
///
/// This is the single answer to "what would selecting version X do", and every caller -- planning,
/// the UI, bulk preparation -- goes through it with the target the user actually chose.
pub(crate) fn resolve_target(
    active: Option<&NormalizedCliVersion>,
    target: &NormalizedCliVersion,
) -> CliTargetResolution {
    let Some(active) = active else {
        return CliTargetResolution::Install;
    };
    match active.compare(target) {
        Some(std::cmp::Ordering::Equal) => CliTargetResolution::Current,
        Some(std::cmp::Ordering::Less) => CliTargetResolution::Upgrade,
        Some(std::cmp::Ordering::Greater) => CliTargetResolution::Downgrade,
        None => CliTargetResolution::Indeterminate,
    }
}

/// Everything the derivation needs about one tool and one candidate source.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CliActionContext<'a> {
    pub(crate) distribution: &'a CliDistributionDefinition,
    pub(crate) platform: CliPlatform,
    pub(crate) is_installed: bool,
    pub(crate) active_version: Option<&'a NormalizedCliVersion>,
    /// Whether the active installation is believed to come from this distribution's source.
    pub(crate) active_source_matches: bool,
    pub(crate) active_source_confidence: CliSourceConfidence,
    pub(crate) active_executable_healthy: bool,
    pub(crate) catalog_latest: Option<&'a NormalizedCliVersion>,
    pub(crate) catalog_available: bool,
    /// Result of the source's dynamic preflight, where it has one. `false` until preflight says
    /// otherwise -- a dynamic capability is never assumed.
    pub(crate) repair_preflight_passed: bool,
}

impl CliActionContext<'_> {
    /// Whether mutating through this source would act on the installation the user is looking at.
    ///
    /// A path heuristic yields `Inferred`, which is enough. `Unknown` is not: acting on it could
    /// mutate a different install than the one on screen.
    fn owns_active_installation(&self) -> bool {
        self.active_source_matches && self.active_source_confidence >= CliSourceConfidence::Inferred
    }

    fn target_mode(&self, action: CliActionKind) -> CliTargetVersionMode {
        action
            .as_distribution_action()
            .map(|distribution_action| {
                self.distribution
                    .target_mode_on(distribution_action, self.platform)
            })
            .unwrap_or(CliTargetVersionMode::Unsupported)
    }
}

/// Every action this source may perform on this tool right now, with the reason attached when one
/// is withheld for a reason worth showing.
pub(crate) fn derive_allowed_actions(context: CliActionContext<'_>) -> Vec<CliAllowedAction> {
    let Ok(source_id) = context.distribution.source_id() else {
        return Vec::new();
    };
    if !context.distribution.is_actionable_on(context.platform) {
        return vec![blocked(
            source_id,
            first_relevant_action(context.is_installed),
            CliActionReasonCode::SourceUnavailableOnPlatform,
        )];
    }
    if !context.distribution.capabilities.manages_anything() {
        return vec![blocked(
            source_id,
            first_relevant_action(context.is_installed),
            CliActionReasonCode::DetectOnlySource,
        )];
    }

    if !context.is_installed {
        return vec![install_action(&context, source_id)];
    }
    if !context.owns_active_installation() {
        return vec![blocked(
            source_id,
            CliActionKind::Upgrade,
            CliActionReasonCode::SourceOwnershipUnproven,
        )];
    }

    let mut actions = Vec::new();
    if let Some(action) = version_change_action(&context, &source_id) {
        actions.push(action);
    }
    if context.target_mode(CliActionKind::Reinstall).is_supported() {
        actions.push(CliAllowedAction {
            action: CliActionKind::Reinstall,
            source_id: source_id.clone(),
            target_mode: context.target_mode(CliActionKind::Reinstall),
            default_target: context.active_version.map(|v| v.as_str().to_string()),
            requires_target_selection: false,
            reason_code: None,
        });
    }
    if context.distribution.capabilities.uninstall {
        actions.push(CliAllowedAction {
            action: CliActionKind::Uninstall,
            source_id: source_id.clone(),
            target_mode: CliTargetVersionMode::Unsupported,
            default_target: None,
            requires_target_selection: false,
            reason_code: None,
        });
    }
    // Repair is a dynamic capability: offered only after a preflight actually confirmed it, never
    // because the source *might* support it.
    if context.distribution.capabilities.repair.needs_preflight() && context.repair_preflight_passed
    {
        actions.push(CliAllowedAction {
            action: CliActionKind::Repair,
            source_id,
            target_mode: CliTargetVersionMode::Unsupported,
            default_target: None,
            requires_target_selection: false,
            reason_code: None,
        });
    }
    actions
}

fn first_relevant_action(is_installed: bool) -> CliActionKind {
    if is_installed {
        CliActionKind::Upgrade
    } else {
        CliActionKind::Install
    }
}

fn blocked(
    source_id: CliSourceId,
    action: CliActionKind,
    reason: CliActionReasonCode,
) -> CliAllowedAction {
    CliAllowedAction {
        action,
        source_id,
        target_mode: CliTargetVersionMode::Unsupported,
        default_target: None,
        requires_target_selection: false,
        reason_code: Some(reason),
    }
}

fn install_action(context: &CliActionContext<'_>, source_id: CliSourceId) -> CliAllowedAction {
    let mode = context.target_mode(CliActionKind::Install);
    if !mode.is_supported() {
        return blocked(
            source_id,
            CliActionKind::Install,
            CliActionReasonCode::ActionUnsupportedBySource,
        );
    }
    CliAllowedAction {
        action: CliActionKind::Install,
        source_id,
        target_mode: mode,
        default_target: context.catalog_latest.map(|v| v.as_str().to_string()),
        // An exact-capable source with no readable catalog cannot offer a version list, so the
        // user cannot be asked to choose one.
        requires_target_selection: mode.accepts_exact_target() && context.catalog_available,
        reason_code: (!context.catalog_available)
            .then_some(CliActionReasonCode::CatalogUnavailable),
    }
}

/// The upgrade/downgrade/current decision for an installed tool, against the source's own catalog.
fn version_change_action(
    context: &CliActionContext<'_>,
    source_id: &CliSourceId,
) -> Option<CliAllowedAction> {
    if !context.active_executable_healthy {
        return Some(blocked(
            source_id.clone(),
            CliActionKind::Upgrade,
            CliActionReasonCode::ActiveInstallationBroken,
        ));
    }
    if !context.catalog_available {
        return Some(blocked(
            source_id.clone(),
            CliActionKind::Upgrade,
            CliActionReasonCode::CatalogUnavailable,
        ));
    }
    let latest = context.catalog_latest?;

    match resolve_target(context.active_version, latest) {
        // The defect, structurally removed: equality produces no mutation and says why.
        CliTargetResolution::Current => Some(blocked(
            source_id.clone(),
            CliActionKind::Upgrade,
            CliActionReasonCode::AlreadyCurrent,
        )),
        CliTargetResolution::Indeterminate => Some(blocked(
            source_id.clone(),
            CliActionKind::Upgrade,
            CliActionReasonCode::UnorderedVersions,
        )),
        resolution => {
            let action = resolution.mutation()?;
            let mode = context.target_mode(action);
            if !mode.is_supported() {
                return Some(blocked(
                    source_id.clone(),
                    action,
                    CliActionReasonCode::ActionUnsupportedBySource,
                ));
            }
            Some(CliAllowedAction {
                action,
                source_id: source_id.clone(),
                target_mode: mode,
                default_target: Some(latest.as_str().to_string()),
                requires_target_selection: false,
                reason_code: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::cli::domain::registry::{
        definition, SOURCE_NPM, SOURCE_VENDOR, SOURCE_WINGET,
    };

    fn version(raw: &str) -> NormalizedCliVersion {
        NormalizedCliVersion::parse(raw)
    }

    fn npm_context<'a>(
        active: Option<&'a NormalizedCliVersion>,
        latest: Option<&'a NormalizedCliVersion>,
    ) -> CliActionContext<'a> {
        let tool = definition("claude-code").expect("claude-code");
        CliActionContext {
            distribution: tool.distribution(SOURCE_NPM).expect("npm"),
            platform: CliPlatform::Linux,
            is_installed: active.is_some(),
            active_version: active,
            active_source_matches: true,
            active_source_confidence: CliSourceConfidence::Inferred,
            active_executable_healthy: true,
            catalog_latest: latest,
            catalog_available: true,
            repair_preflight_passed: false,
        }
    }

    fn kinds(actions: &[CliAllowedAction]) -> Vec<CliActionKind> {
        actions.iter().map(|action| action.action).collect()
    }

    fn reason(actions: &[CliAllowedAction], kind: CliActionKind) -> Option<CliActionReasonCode> {
        actions
            .iter()
            .find(|action| action.action == kind)
            .and_then(|action| action.reason_code)
    }

    #[test]
    fn an_equal_target_resolves_to_current_and_has_no_mutation() {
        // The defect: the page derived "upgrade" here and dispatched a redundant install.
        let active = version("1.2.0");
        let resolution = resolve_target(Some(&active), &version("1.2.0"));
        assert_eq!(resolution, CliTargetResolution::Current);
        assert_eq!(resolution.mutation(), None);

        // Textual difference, same version: still current.
        assert_eq!(
            resolve_target(Some(&active), &version("v1.2.0")).mutation(),
            None
        );
        assert_eq!(
            resolve_target(Some(&active), &version("1.2")).mutation(),
            None
        );
    }

    #[test]
    fn newer_and_older_targets_resolve_to_their_own_mutations() {
        let active = version("1.2.0");
        assert_eq!(
            resolve_target(Some(&active), &version("1.3.0")),
            CliTargetResolution::Upgrade
        );
        assert_eq!(
            resolve_target(Some(&active), &version("1.1.0")),
            CliTargetResolution::Downgrade
        );
        assert_eq!(
            resolve_target(Some(&active), &version("1.3.0")).mutation(),
            Some(CliActionKind::Upgrade)
        );
        assert_eq!(
            resolve_target(Some(&active), &version("1.1.0")).mutation(),
            Some(CliActionKind::Downgrade)
        );
    }

    #[test]
    fn nothing_installed_resolves_to_install_at_any_target() {
        assert_eq!(
            resolve_target(None, &version("1.0.0")),
            CliTargetResolution::Install
        );
        assert_eq!(
            resolve_target(None, &version("nightly")).mutation(),
            Some(CliActionKind::Install)
        );
    }

    #[test]
    fn an_unorderable_pair_claims_neither_upgrade_nor_downgrade() {
        let active = version("nightly");
        let resolution = resolve_target(Some(&active), &version("1.2.0"));
        assert_eq!(resolution, CliTargetResolution::Indeterminate);
        assert_eq!(resolution.mutation(), None);
        assert_eq!(resolution.as_str(), "indeterminate");

        // Identical opaque strings are still equal, so no redundant action is offered.
        assert_eq!(
            resolve_target(Some(&active), &version("nightly")),
            CliTargetResolution::Current
        );
    }

    #[test]
    fn a_tool_already_at_the_latest_version_is_offered_no_upgrade() {
        let active = version("1.3.0");
        let latest = version("1.3.0");
        let actions = derive_allowed_actions(npm_context(Some(&active), Some(&latest)));

        assert_eq!(
            reason(&actions, CliActionKind::Upgrade),
            Some(CliActionReasonCode::AlreadyCurrent)
        );
        // Uninstall and reinstall remain available -- being current does not disable everything.
        assert!(kinds(&actions).contains(&CliActionKind::Uninstall));
        assert!(kinds(&actions).contains(&CliActionKind::Reinstall));
    }

    #[test]
    fn an_outdated_tool_is_offered_an_upgrade_to_the_catalog_latest() {
        let active = version("1.2.0");
        let latest = version("1.3.0");
        let actions = derive_allowed_actions(npm_context(Some(&active), Some(&latest)));

        let upgrade = actions
            .iter()
            .find(|action| action.action == CliActionKind::Upgrade)
            .expect("upgrade");
        assert_eq!(upgrade.reason_code, None);
        assert_eq!(upgrade.default_target.as_deref(), Some("1.3.0"));
        assert_eq!(upgrade.target_mode, CliTargetVersionMode::Exact);
    }

    #[test]
    fn a_missing_tool_is_offered_an_install_that_asks_for_a_version() {
        let latest = version("1.3.0");
        let actions = derive_allowed_actions(npm_context(None, Some(&latest)));

        assert_eq!(kinds(&actions), vec![CliActionKind::Install]);
        let install = &actions[0];
        assert!(install.requires_target_selection);
        assert_eq!(install.default_target.as_deref(), Some("1.3.0"));
        assert_eq!(install.reason_code, None);
    }

    #[test]
    fn an_unreadable_catalog_withholds_the_action_instead_of_guessing() {
        let active = version("1.2.0");
        let context = CliActionContext {
            catalog_available: false,
            catalog_latest: None,
            ..npm_context(Some(&active), None)
        };
        let actions = derive_allowed_actions(context);
        assert_eq!(
            reason(&actions, CliActionKind::Upgrade),
            Some(CliActionReasonCode::CatalogUnavailable)
        );

        // Not installed and no catalog: install is still offered, but no version can be chosen.
        let context = CliActionContext {
            is_installed: false,
            active_version: None,
            catalog_available: false,
            catalog_latest: None,
            ..npm_context(None, None)
        };
        let install = &derive_allowed_actions(context)[0];
        assert!(!install.requires_target_selection);
        assert_eq!(
            install.reason_code,
            Some(CliActionReasonCode::CatalogUnavailable)
        );
    }

    #[test]
    fn a_detect_only_source_offers_nothing_but_says_why() {
        let tool = definition("claude-code").expect("claude-code");
        let vendor = tool.distribution(SOURCE_VENDOR).expect("vendor");
        let active = version("1.2.0");
        let latest = version("1.3.0");

        // Windows has no template for this vendor, so it is unactionable there.
        let context = CliActionContext {
            distribution: vendor,
            platform: CliPlatform::Windows,
            ..npm_context(Some(&active), Some(&latest))
        };
        let actions = derive_allowed_actions(context);
        assert_eq!(
            reason(&actions, CliActionKind::Upgrade),
            Some(CliActionReasonCode::SourceUnavailableOnPlatform)
        );
        assert!(actions
            .iter()
            .all(|action| action.target_mode == CliTargetVersionMode::Unsupported));
    }

    #[test]
    fn an_unproven_source_is_not_allowed_to_mutate_the_active_installation() {
        let active = version("1.2.0");
        let latest = version("1.3.0");
        let context = CliActionContext {
            active_source_confidence: CliSourceConfidence::Unknown,
            ..npm_context(Some(&active), Some(&latest))
        };
        let actions = derive_allowed_actions(context);
        assert_eq!(
            reason(&actions, CliActionKind::Upgrade),
            Some(CliActionReasonCode::SourceOwnershipUnproven)
        );

        // A different source owning the install is equally disqualifying.
        let context = CliActionContext {
            active_source_matches: false,
            active_source_confidence: CliSourceConfidence::Verified,
            ..npm_context(Some(&active), Some(&latest))
        };
        assert_eq!(
            reason(&derive_allowed_actions(context), CliActionKind::Upgrade),
            Some(CliActionReasonCode::SourceOwnershipUnproven)
        );
    }

    #[test]
    fn an_inferred_source_is_enough_to_manage_but_unknown_is_not() {
        let active = version("1.2.0");
        let latest = version("1.3.0");
        let inferred = derive_allowed_actions(CliActionContext {
            active_source_confidence: CliSourceConfidence::Inferred,
            ..npm_context(Some(&active), Some(&latest))
        });
        assert_eq!(reason(&inferred, CliActionKind::Upgrade), None);

        let unknown = derive_allowed_actions(CliActionContext {
            active_source_confidence: CliSourceConfidence::Unknown,
            ..npm_context(Some(&active), Some(&latest))
        });
        assert_eq!(
            reason(&unknown, CliActionKind::Upgrade),
            Some(CliActionReasonCode::SourceOwnershipUnproven)
        );
    }

    #[test]
    fn a_broken_active_installation_blocks_the_version_change() {
        let active = version("1.2.0");
        let latest = version("1.3.0");
        let context = CliActionContext {
            active_executable_healthy: false,
            ..npm_context(Some(&active), Some(&latest))
        };
        assert_eq!(
            reason(&derive_allowed_actions(context), CliActionKind::Upgrade),
            Some(CliActionReasonCode::ActiveInstallationBroken)
        );
    }

    #[test]
    fn winget_withholds_downgrade_because_its_adapter_does_not_verify_one() {
        let tool = definition("claude-code").expect("claude-code");
        let winget = tool.distribution(SOURCE_WINGET).expect("winget");
        let active = version("2.0.0");
        let latest = version("1.0.0");

        let context = CliActionContext {
            distribution: winget,
            platform: CliPlatform::Windows,
            ..npm_context(Some(&active), Some(&latest))
        };
        let actions = derive_allowed_actions(context);
        // The catalog is behind the install, so this would be a downgrade -- which WinGet does not
        // offer in this change.
        assert_eq!(
            reason(&actions, CliActionKind::Downgrade),
            Some(CliActionReasonCode::ActionUnsupportedBySource)
        );
        assert!(!kinds(&actions).contains(&CliActionKind::Reinstall));
        assert!(kinds(&actions).contains(&CliActionKind::Uninstall));
    }

    #[test]
    fn repair_appears_only_after_a_preflight_confirms_it() {
        let tool = definition("claude-code").expect("claude-code");
        let winget = tool.distribution(SOURCE_WINGET).expect("winget");
        let active = version("1.0.0");
        let latest = version("1.0.0");

        let without = derive_allowed_actions(CliActionContext {
            distribution: winget,
            platform: CliPlatform::Windows,
            repair_preflight_passed: false,
            ..npm_context(Some(&active), Some(&latest))
        });
        assert!(!kinds(&without).contains(&CliActionKind::Repair));

        let with = derive_allowed_actions(CliActionContext {
            distribution: winget,
            platform: CliPlatform::Windows,
            repair_preflight_passed: true,
            ..npm_context(Some(&active), Some(&latest))
        });
        assert!(kinds(&with).contains(&CliActionKind::Repair));
    }

    #[test]
    fn an_unorderable_active_version_withholds_the_version_change() {
        let active = version("nightly");
        let latest = version("1.3.0");
        let actions = derive_allowed_actions(npm_context(Some(&active), Some(&latest)));
        assert_eq!(
            reason(&actions, CliActionKind::Upgrade),
            Some(CliActionReasonCode::UnorderedVersions)
        );
    }

    #[test]
    fn every_action_and_reason_has_a_stable_wire_string() {
        assert_eq!(CliActionKind::Install.as_str(), "install");
        assert_eq!(CliActionKind::Upgrade.as_str(), "upgrade");
        assert_eq!(CliActionKind::Downgrade.as_str(), "downgrade");
        assert_eq!(CliActionKind::Reinstall.as_str(), "reinstall");
        assert_eq!(CliActionKind::Uninstall.as_str(), "uninstall");
        assert_eq!(CliActionKind::Repair.as_str(), "repair");

        assert_eq!(CliTargetResolution::Current.as_str(), "current");
        assert_eq!(CliTargetResolution::Install.as_str(), "install");
        assert_eq!(CliTargetResolution::Upgrade.as_str(), "upgrade");
        assert_eq!(CliTargetResolution::Downgrade.as_str(), "downgrade");

        for (reason, wire) in [
            (CliActionReasonCode::AlreadyCurrent, "already-current"),
            (CliActionReasonCode::DetectOnlySource, "detect-only-source"),
            (
                CliActionReasonCode::CatalogUnavailable,
                "catalog-unavailable",
            ),
            (
                CliActionReasonCode::SourceUnavailableOnPlatform,
                "source-unavailable-on-platform",
            ),
            (CliActionReasonCode::UnorderedVersions, "unordered-versions"),
            (
                CliActionReasonCode::SourceOwnershipUnproven,
                "source-ownership-unproven",
            ),
            (
                CliActionReasonCode::ActionUnsupportedBySource,
                "action-unsupported-by-source",
            ),
            (
                CliActionReasonCode::ActiveInstallationBroken,
                "active-installation-broken",
            ),
        ] {
            assert_eq!(reason.as_str(), wire);
        }
    }
}
