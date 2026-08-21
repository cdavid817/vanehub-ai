//! Consuming-side contracts for capability-gate state.

use crate::contexts::tooling::extension_platform::domain::{
    ExtensionPlatformFeature, FeatureGateDegradation, FeatureGateError, PrerequisiteReason,
};

/// One gate's persisted desired state. Storage holds nothing derived: build availability comes
/// from `cfg!` at evaluation time, so a database moved between builds can never claim a
/// capability the running binary lacks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PersistedFeatureGate {
    pub(crate) feature: ExtensionPlatformFeature,
    pub(crate) desired_enabled: bool,
    pub(crate) revision: i64,
    pub(crate) updated_at: String,
    pub(crate) updated_by: String,
    pub(crate) reason: Option<String>,
}

/// A requested desired-state change, guarded by the revision the caller last observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureGateWrite {
    pub(crate) feature: ExtensionPlatformFeature,
    pub(crate) desired_enabled: bool,
    pub(crate) expected_revision: i64,
    pub(crate) updated_at: String,
    pub(crate) updated_by: String,
    pub(crate) reason: Option<String>,
}

/// An append-only record of one gate set becoming or remaining stale.
///
/// Not a mutation, so it does not fit `FeatureGateAuditEntry`'s prior/new shape. Carries a stable
/// error code rather than the underlying message: a storage failure's text can contain a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureGateDegradationEntry {
    pub(crate) degradation: FeatureGateDegradation,
    pub(crate) code: &'static str,
    pub(crate) recorded_at: String,
}

/// An append-only record of one accepted gate mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureGateAuditEntry {
    pub(crate) feature: ExtensionPlatformFeature,
    pub(crate) previous_enabled: bool,
    pub(crate) new_enabled: bool,
    pub(crate) revision: i64,
    pub(crate) recorded_at: String,
    pub(crate) actor: String,
    pub(crate) reason: Option<String>,
}

pub(crate) trait FeatureGateRepository: Send + Sync {
    /// Every persisted gate. A gate with no row is not an error: it means "never configured",
    /// which the service resolves to disabled.
    fn load_all(&self) -> Result<Vec<PersistedFeatureGate>, FeatureGateError>;

    /// Applies a desired-state change if `expected_revision` still matches, returning the stored
    /// row. Rejects with `StaleRevision` otherwise.
    fn upsert(&self, write: &FeatureGateWrite) -> Result<PersistedFeatureGate, FeatureGateError>;
}

pub(crate) trait FeatureGateAuditSink: Send + Sync {
    fn record(&self, entry: &FeatureGateAuditEntry) -> Result<(), FeatureGateError>;

    /// Records that the published gate set is stale. Separate from `record` because a degradation
    /// has no prior/new state and no revision to attribute it to.
    fn record_degradation(
        &self,
        entry: &FeatureGateDegradationEntry,
    ) -> Result<(), FeatureGateError>;
}

/// Process-level overrides that outrank operator intent — a safety kill applied without editing
/// persisted state, so that turning the override off restores exactly what the operator had.
pub(crate) trait FeatureForcedDisablePort: Send + Sync {
    fn forced_disable_reason(&self, feature: ExtensionPlatformFeature) -> Option<String>;
}

/// Platform readiness a gate depends on but does not own — the sandbox self-test being the case
/// this change ships with.
pub(crate) trait FeaturePrerequisitePort: Send + Sync {
    fn unsatisfied_prerequisite(
        &self,
        feature: ExtensionPlatformFeature,
    ) -> Option<PrerequisiteReason>;
}

pub(crate) trait FeatureGateClock: Send + Sync {
    fn now_rfc3339(&self) -> String;
}

/// No override configured. The default in every build that has not wired a safety kill.
pub(crate) struct NoForcedDisables;

impl FeatureForcedDisablePort for NoForcedDisables {
    fn forced_disable_reason(&self, _feature: ExtensionPlatformFeature) -> Option<String> {
        None
    }
}

/// Current platform readiness.
///
/// No sandbox provider exists yet, so the sidecar gate is honestly reported as blocked rather
/// than enabled. A separate process is not a sandbox, and this is where that distinction is kept
/// from quietly disappearing.
pub(crate) struct DefaultPrerequisites;

impl FeaturePrerequisitePort for DefaultPrerequisites {
    fn unsatisfied_prerequisite(
        &self,
        feature: ExtensionPlatformFeature,
    ) -> Option<PrerequisiteReason> {
        match feature {
            ExtensionPlatformFeature::SidecarRuntime => {
                Some(PrerequisiteReason::SandboxSelfTestUnavailable)
            }
            _ => None,
        }
    }
}
