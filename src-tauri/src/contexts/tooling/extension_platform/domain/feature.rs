//! Extension Platform capability gates.
//!
//! Two layers that are deliberately never merged. **Build capability** answers "is this code in
//! the binary at all?" and comes from a Cargo feature. **Runtime desired state** answers "does the
//! operator want it on right now?" and is persisted. Collapsing them produces this design's worst
//! failure: an operator enabling a gate in a build that cannot honour it and reading silence as
//! success. `FeatureGateStatus` therefore keeps `NotCompiled` and `RuntimeDisabled` distinct, and
//! `set_desired_state` refuses rather than persisting an enabled row a build cannot serve.

use std::fmt;

/// The closed gate set. A string key/value map is not a domain interface: an unknown key must be
/// unrepresentable here rather than silently evaluating to some default at a call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ExtensionPlatformFeature {
    Catalog,
    ExternalPackages,
    LifecycleHooks,
    AuthorizationRules,
    Connectors,
    WasmModuleRuntime,
    SidecarRuntime,
}

/// Every gate, in stable order. Ordering is part of the contract: diagnostics, snapshots, and DTO
/// lists must not vary by map iteration order.
pub(crate) const ALL_FEATURES: [ExtensionPlatformFeature; 7] = [
    ExtensionPlatformFeature::Catalog,
    ExtensionPlatformFeature::ExternalPackages,
    ExtensionPlatformFeature::LifecycleHooks,
    ExtensionPlatformFeature::AuthorizationRules,
    ExtensionPlatformFeature::Connectors,
    ExtensionPlatformFeature::WasmModuleRuntime,
    ExtensionPlatformFeature::SidecarRuntime,
];

impl ExtensionPlatformFeature {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Catalog => "extension_platform.catalog",
            Self::ExternalPackages => "extension_platform.external_packages",
            Self::LifecycleHooks => "extension_platform.lifecycle_hooks",
            Self::AuthorizationRules => "extension_platform.authorization_rules",
            Self::Connectors => "extension_platform.connectors",
            Self::WasmModuleRuntime => "extension_platform.wasm_module_runtime",
            Self::SidecarRuntime => "extension_platform.sidecar_runtime",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        ALL_FEATURES
            .into_iter()
            .find(|feature| feature.as_str() == value)
    }

    /// Position in `ALL_FEATURES`. Lets a snapshot be a fixed-size array rather than a map, so
    /// lookup is total — no `expect`, no missing-key branch that could only fire if the two ever
    /// drifted apart. `all_features_and_indexes_agree` holds them together.
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Catalog => 0,
            Self::ExternalPackages => 1,
            Self::LifecycleHooks => 2,
            Self::AuthorizationRules => 3,
            Self::Connectors => 4,
            Self::WasmModuleRuntime => 5,
            Self::SidecarRuntime => 6,
        }
    }

    /// Whether this build contains the gate's code at all.
    ///
    /// Derived through `cfg!` on every call and never persisted: a database carried between a
    /// build that had the Cargo feature and one that did not would otherwise claim a capability
    /// the running binary does not have. Gates with no runtime-bearing code are always present.
    pub(crate) fn build_available(self) -> bool {
        match self {
            Self::WasmModuleRuntime => cfg!(feature = "extension-wasm-module-runtime"),
            Self::SidecarRuntime => cfg!(feature = "extension-sidecar-runtime"),
            Self::Catalog
            | Self::ExternalPackages
            | Self::LifecycleHooks
            | Self::AuthorizationRules
            | Self::Connectors => true,
        }
    }
}

impl fmt::Display for ExtensionPlatformFeature {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Why an otherwise-enabled gate still cannot serve work. Distinct from `RuntimeDisabled` because
/// the operator did ask for it — the platform is what is not ready.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PrerequisiteReason {
    /// No platform sandbox provider has passed its startup self-test. Sidecars fail closed rather
    /// than running as "isolated" on the strength of being a separate process.
    SandboxSelfTestUnavailable,
}

impl PrerequisiteReason {
    pub(crate) const fn as_str(&self) -> &'static str {
        match self {
            Self::SandboxSelfTestUnavailable => "sandbox_self_test_unavailable",
        }
    }
}

/// The five-way effective status. `NotCompiled` and `RuntimeDisabled` are separate members by
/// contract; a caller that only needs the boolean uses `is_enabled`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FeatureGateStatus {
    NotCompiled,
    RuntimeDisabled,
    Enabled,
    BlockedByPrerequisite(PrerequisiteReason),
    ForcedDisabled { reason: String },
}

impl FeatureGateStatus {
    pub(crate) fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled)
    }
}

/// Inputs to one gate's status, gathered by the application layer.
pub(crate) struct FeatureGateEvaluation {
    pub(crate) desired_enabled: bool,
    pub(crate) forced_disable_reason: Option<String>,
    pub(crate) unsatisfied_prerequisite: Option<PrerequisiteReason>,
}

/// `build_available AND persisted_enabled AND prerequisites_satisfied AND NOT forced_disabled`,
/// reported as the most specific reason rather than a bare boolean.
///
/// Order is what makes the report useful, not what makes the conjunction true. Build capability
/// comes first because nothing else is actionable without it; a forced disable outranks desired
/// state because it is an override of exactly that; and a prerequisite is only worth surfacing to
/// an operator who has already asked for the gate.
pub(crate) fn evaluate_gate(
    feature: ExtensionPlatformFeature,
    evaluation: FeatureGateEvaluation,
) -> FeatureGateStatus {
    if !feature.build_available() {
        return FeatureGateStatus::NotCompiled;
    }
    if let Some(reason) = evaluation.forced_disable_reason {
        return FeatureGateStatus::ForcedDisabled { reason };
    }
    if !evaluation.desired_enabled {
        return FeatureGateStatus::RuntimeDisabled;
    }
    if let Some(reason) = evaluation.unsatisfied_prerequisite {
        return FeatureGateStatus::BlockedByPrerequisite(reason);
    }
    FeatureGateStatus::Enabled
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evaluation(desired: bool) -> FeatureGateEvaluation {
        FeatureGateEvaluation {
            desired_enabled: desired,
            forced_disable_reason: None,
            unsatisfied_prerequisite: None,
        }
    }

    #[test]
    fn every_feature_has_a_unique_stable_key_that_round_trips() {
        let mut keys: Vec<&str> = ALL_FEATURES.iter().map(|f| f.as_str()).collect();
        let total = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), total);

        for feature in ALL_FEATURES {
            assert_eq!(
                ExtensionPlatformFeature::parse(feature.as_str()),
                Some(feature)
            );
        }
    }

    #[test]
    fn all_features_and_indexes_agree() {
        for (position, feature) in ALL_FEATURES.into_iter().enumerate() {
            assert_eq!(feature.index(), position, "{feature} index drifted");
        }
    }

    #[test]
    fn unknown_keys_do_not_parse() {
        assert_eq!(
            ExtensionPlatformFeature::parse("extension_platform.wasm"),
            None
        );
        assert_eq!(ExtensionPlatformFeature::parse(""), None);
    }

    #[test]
    fn runtime_bearing_gates_track_their_cargo_feature() {
        assert_eq!(
            ExtensionPlatformFeature::WasmModuleRuntime.build_available(),
            cfg!(feature = "extension-wasm-module-runtime")
        );
        assert_eq!(
            ExtensionPlatformFeature::SidecarRuntime.build_available(),
            cfg!(feature = "extension-sidecar-runtime")
        );
        assert!(ExtensionPlatformFeature::Catalog.build_available());
    }

    #[test]
    fn an_uncompiled_gate_reports_not_compiled_even_when_desired() {
        // Both runtime-bearing gates are off in the default build, which is exactly the state
        // this must not report as `RuntimeDisabled`.
        if ExtensionPlatformFeature::SidecarRuntime.build_available() {
            return;
        }
        let status = evaluate_gate(ExtensionPlatformFeature::SidecarRuntime, evaluation(true));
        assert_eq!(status, FeatureGateStatus::NotCompiled);
        assert_ne!(status, FeatureGateStatus::RuntimeDisabled);
    }

    #[test]
    fn a_compiled_gate_that_nobody_enabled_reports_runtime_disabled() {
        assert_eq!(
            evaluate_gate(ExtensionPlatformFeature::Catalog, evaluation(false)),
            FeatureGateStatus::RuntimeDisabled
        );
    }

    #[test]
    fn a_forced_disable_outranks_desired_state() {
        let status = evaluate_gate(
            ExtensionPlatformFeature::Catalog,
            FeatureGateEvaluation {
                desired_enabled: true,
                forced_disable_reason: Some("safety override".to_string()),
                unsatisfied_prerequisite: None,
            },
        );
        assert_eq!(
            status,
            FeatureGateStatus::ForcedDisabled {
                reason: "safety override".to_string()
            }
        );
        assert!(!status.is_enabled());
    }

    #[test]
    fn an_unsatisfied_prerequisite_blocks_an_enabled_gate() {
        let status = evaluate_gate(
            ExtensionPlatformFeature::Catalog,
            FeatureGateEvaluation {
                desired_enabled: true,
                forced_disable_reason: None,
                unsatisfied_prerequisite: Some(PrerequisiteReason::SandboxSelfTestUnavailable),
            },
        );
        assert_eq!(
            status,
            FeatureGateStatus::BlockedByPrerequisite(
                PrerequisiteReason::SandboxSelfTestUnavailable
            )
        );
        assert!(!status.is_enabled());
    }

    #[test]
    fn only_a_compiled_desired_unblocked_gate_is_enabled() {
        let status = evaluate_gate(ExtensionPlatformFeature::Catalog, evaluation(true));
        assert_eq!(status, FeatureGateStatus::Enabled);
        assert!(status.is_enabled());
    }

    #[test]
    fn not_compiled_and_runtime_disabled_are_distinct_states() {
        assert_ne!(
            FeatureGateStatus::NotCompiled,
            FeatureGateStatus::RuntimeDisabled
        );
    }
}
