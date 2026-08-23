//! Published in-process facade for `tooling::extension_platform`.
//!
//! Hooks, Permissions, Connectors, Agent Runtime, Tauri commands, and the frontend read capability
//! gates through here and nowhere else. Reaching into the repository would let a caller observe
//! desired state without the build-availability, forced-disable, and prerequisite layers applied —
//! which is exactly the reading that turns "operator asked for it" into "it is on".

use super::application::{ActiveContributionReader, FeatureGateService};
use std::sync::Arc;

pub(crate) use super::application::{FeatureGateSnapshot, FeatureGateView};
pub(crate) use super::domain::{
    ActiveContribution, ActiveContributionError, ExtensionPlatformFeature, FeatureGateError,
    FeatureGateStatus,
};

#[derive(Clone)]
pub(crate) struct ExtensionPlatformApi {
    feature_gates: Arc<FeatureGateService>,
    active_contributions: Arc<dyn ActiveContributionReader>,
}

impl ExtensionPlatformApi {
    pub(crate) fn new(
        feature_gates: Arc<FeatureGateService>,
        active_contributions: Arc<dyn ActiveContributionReader>,
    ) -> Self {
        Self {
            feature_gates,
            active_contributions,
        }
    }

    /// What the platform currently runs for one contribution id.
    ///
    /// The authority chain is `Installation -> Active Generation Pointer -> Runtime Generation ->
    /// Snapshot`, and this is the only published way to walk it. A consumer that instead picked
    /// "the most recently recorded definition" would answer with a version that has been recorded
    /// but never activated, and would answer with the newer version after a rollback to an older
    /// one -- in both cases dispatching something the platform is not running.
    pub(crate) fn active_contribution(
        &self,
        global_id: &str,
    ) -> Result<ActiveContribution, ActiveContributionError> {
        self.active_contributions.active(global_id)
    }

    /// The immutable current gate snapshot. Callers that make several decisions in one unit of
    /// work should hold this rather than calling `is_enabled` repeatedly, so their decisions all
    /// come from the same generation.
    pub(crate) fn feature_gates(&self) -> Arc<FeatureGateSnapshot> {
        self.feature_gates.snapshot()
    }

    /// Whether one gate is effectively enabled right now.
    ///
    /// Published ahead of its callers: Hooks, Permissions, Connectors, and Agent Runtime consume
    /// it as their task groups land. It exists now so those groups cannot be tempted to reach for
    /// the repository instead, which would skip the build-availability and override layers.
    #[allow(dead_code)]
    pub(crate) fn is_feature_enabled(&self, feature: ExtensionPlatformFeature) -> bool {
        self.feature_gates.is_enabled(feature)
    }

    /// Changes a gate's desired state. Fails with `FeatureUnavailableInBuild` rather than
    /// recording an enablement this build cannot honour, and with `StaleRevision` rather than
    /// overwriting a concurrent edit.
    pub(crate) fn set_feature_desired_state(
        &self,
        feature: ExtensionPlatformFeature,
        desired_enabled: bool,
        expected_revision: i64,
        actor: &str,
        reason: Option<String>,
    ) -> Result<Arc<FeatureGateSnapshot>, FeatureGateError> {
        self.feature_gates.set_desired_state(
            feature,
            desired_enabled,
            expected_revision,
            actor,
            reason,
        )
    }

    /// Re-reads persisted state. Construction already loads once; this is for a later caller that
    /// needs to re-read after an out-of-band write.
    #[allow(dead_code)]
    pub(crate) fn reload_feature_gates(
        &self,
    ) -> Result<Arc<FeatureGateSnapshot>, FeatureGateError> {
        self.feature_gates.reload()
    }
}
