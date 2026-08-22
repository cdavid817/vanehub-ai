//! Consuming-side contracts for capability-gate state and publisher trust.

use super::developer_mode::DeveloperModeView;
use crate::contexts::tooling::extension_platform::domain::{
    ContentPublication, DeveloperMode, DeveloperModeError, ExtensionId, ExtensionPlatformFeature,
    FeatureGateDegradation, FeatureGateError, PackageHash, PrerequisiteReason,
    PublisherKeyFingerprint, PublisherKeyRecord, SnapshotPointer, SnapshotPublicationError,
    SnapshotRecord, TrustedPublisherKey,
};
use std::path::Path;

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

/// Where a publisher key is looked up by the fingerprint an envelope names.
///
/// By fingerprint and nothing else. Looking up by publisher id would let a package choose which of
/// a publisher's keys to be checked against, and looking up every trusted key and trying each
/// would turn "which key signed this" — a fact the evidence has to record — into a guess.
///
/// A failure is a storage failure. It is deliberately not `Option`, because "the store is
/// unreachable" and "the key is not trusted" must never collapse into the same answer: one of them
/// would then read as a definite refusal when nothing was actually checked.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) trait PublisherKeyDirectory: Send + Sync {
    fn find(
        &self,
        fingerprint: &PublisherKeyFingerprint,
    ) -> Result<Option<PublisherKeyRecord>, String>;
}

/// Managing trusted publisher keys.
///
/// Separate from `PublisherKeyDirectory` on purpose. The verification path takes the narrow
/// read-only port and therefore cannot write, so no future change to the verifier can reach a
/// mutation by accident.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) trait TrustedPublisherKeyRepository: Send + Sync {
    fn list(&self) -> Result<Vec<TrustedPublisherKey>, String>;

    fn find(
        &self,
        fingerprint: &PublisherKeyFingerprint,
    ) -> Result<Option<TrustedPublisherKey>, String>;

    /// Inserts a new key, or refreshes `last_seen_at`, `label`, and `source` on one already filed
    /// under this fingerprint. Never changes `publisher`, `first_seen_at`, or trust state — those
    /// are decided before the call and are not the repository's to reinterpret.
    fn upsert(&self, key: &TrustedPublisherKey) -> Result<(), String>;

    /// Withdraws trust. Idempotent: a key already revoked keeps the timestamp and reason of the
    /// first revocation, because when trust was withdrawn is the fact worth keeping.
    fn revoke(
        &self,
        fingerprint: &PublisherKeyFingerprint,
        revoked_at: &str,
        reason: Option<&str>,
    ) -> Result<(), String>;
}

/// An append-only record of one Developer Mode change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeveloperModeAuditEntry {
    pub(crate) previous_enabled: bool,
    pub(crate) new_enabled: bool,
    pub(crate) revision: i64,
    pub(crate) recorded_at: String,
    pub(crate) actor: String,
    pub(crate) reason: Option<String>,
}

/// Where the Developer Mode switch is kept.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) trait DeveloperModeRepository: Send + Sync {
    /// The stored switch. A build with nothing stored reports `Off` at revision 0, which is also
    /// what a fresh install must have.
    fn load(&self) -> Result<DeveloperModeView, DeveloperModeError>;

    fn store(
        &self,
        mode: DeveloperMode,
        revision: i64,
        updated_at: &str,
        updated_by: &str,
        reason: Option<&str>,
    ) -> Result<DeveloperModeView, DeveloperModeError>;
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) trait DeveloperModeAuditSink: Send + Sync {
    fn record(&self, entry: &DeveloperModeAuditEntry) -> Result<(), DeveloperModeError>;
}

/// Where immutable package content is kept, addressed by its own digest.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) trait SnapshotContentStore: Send + Sync {
    /// Moves `staged` into the content-addressed store under `hash`.
    ///
    /// A destination that already exists is `AlreadyPresent` rather than an error: content is
    /// addressed by its own digest, so what is there is what would have been written — including
    /// when it is there because a concurrent install of the same package won the race.
    fn publish(&self, staged: &Path, hash: &PackageHash) -> Result<ContentPublication, String>;

    /// Removes staged content that will not be published.
    fn discard_staged(&self, staged: &Path) -> Result<(), String>;
}

/// Which snapshot each installation is running.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) trait SnapshotPointerRepository: Send + Sync {
    fn active(&self, extension: &ExtensionId) -> Result<Option<SnapshotPointer>, String>;

    /// Records the snapshot and moves the pointer to it, in one guarded write.
    ///
    /// The previous active snapshot becomes the rollback target. On any failure the pointer is
    /// left exactly where it was, because a half-moved pointer is an installation nobody can
    /// describe.
    fn point_at(
        &self,
        record: &SnapshotRecord,
        expected_revision: i64,
    ) -> Result<SnapshotPointer, SnapshotPublicationError>;
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
