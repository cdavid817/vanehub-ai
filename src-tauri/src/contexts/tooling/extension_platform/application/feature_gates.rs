//! Capability-gate application service and its immutable published snapshot.

use super::ports::{
    FeatureForcedDisablePort, FeatureGateAuditEntry, FeatureGateAuditSink, FeatureGateClock,
    FeatureGateDegradationEntry, FeatureGateRepository, FeatureGateWrite, FeaturePrerequisitePort,
    PersistedFeatureGate,
};
use crate::contexts::tooling::extension_platform::domain::{
    evaluate_gate, ExtensionPlatformFeature, FeatureGateDegradation, FeatureGateError,
    FeatureGateEvaluation, FeatureGateFreshness, FeatureGateStatus, PrerequisiteReason,
    ALL_FEATURES,
};
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

/// One gate as published to other contexts and the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureGateView {
    pub(crate) feature: ExtensionPlatformFeature,
    pub(crate) status: FeatureGateStatus,
    pub(crate) build_available: bool,
    pub(crate) desired_enabled: bool,
    pub(crate) revision: i64,
    pub(crate) updated_at: Option<String>,
    pub(crate) updated_by: Option<String>,
    pub(crate) reason: Option<String>,
}

/// An immutable point-in-time view of every gate.
///
/// Handed out as `Arc` so a caller that captured it keeps a coherent set even while another
/// thread publishes a newer one — the same pinning discipline the contribution registry needs
/// later. Backed by a fixed-size array indexed by `ExtensionPlatformFeature::index`, which makes
/// lookup total: there is no absent-gate case to handle or panic on.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FeatureGateSnapshot {
    entries: [FeatureGateView; ALL_FEATURES.len()],
    freshness: FeatureGateFreshness,
}

impl FeatureGateSnapshot {
    pub(crate) fn get(&self, feature: ExtensionPlatformFeature) -> &FeatureGateView {
        &self.entries[feature.index()]
    }

    /// Whether these values came from a successful read. A degraded snapshot still answers every
    /// gate definitively; it just says the answer may be stale or fail-closed rather than current.
    pub(crate) fn freshness(&self) -> FeatureGateFreshness {
        self.freshness
    }

    pub(crate) fn is_enabled(&self, feature: ExtensionPlatformFeature) -> bool {
        self.get(feature).status.is_enabled()
    }

    /// Stable order, matching `ALL_FEATURES`.
    pub(crate) fn views(&self) -> impl Iterator<Item = &FeatureGateView> {
        self.entries.iter()
    }
}

pub(crate) struct FeatureGateService {
    repository: Arc<dyn FeatureGateRepository>,
    audit: Arc<dyn FeatureGateAuditSink>,
    forced: Arc<dyn FeatureForcedDisablePort>,
    prerequisites: Arc<dyn FeaturePrerequisitePort>,
    clock: Arc<dyn FeatureGateClock>,
    cached: RwLock<Arc<FeatureGateSnapshot>>,
}

impl FeatureGateService {
    pub(crate) fn new(
        repository: Arc<dyn FeatureGateRepository>,
        audit: Arc<dyn FeatureGateAuditSink>,
        forced: Arc<dyn FeatureForcedDisablePort>,
        prerequisites: Arc<dyn FeaturePrerequisitePort>,
        clock: Arc<dyn FeatureGateClock>,
    ) -> Self {
        let service = Self {
            repository,
            audit,
            forced,
            prerequisites,
            clock,
            cached: RwLock::new(Arc::new(fail_closed_snapshot(
                FeatureGateDegradation::NeverLoaded,
            ))),
        };
        // A failed initial read must not leave the process without a snapshot. With no
        // last-known-good set to fall back to, every gate stays fail-closed and the snapshot is
        // marked `NeverLoaded` — distinct from `ReloadFailed`, because "we never read anything"
        // and "we are showing you the last good read" are different claims.
        let _ = service.reload();
        service
    }

    /// Re-reads persisted state and republishes the snapshot.
    ///
    /// A read failure keeps the previously published values rather than clearing them, republishes
    /// them marked degraded, records an audit entry, and returns the error. Callers that cannot
    /// act on it (startup, background refresh) ignore the result: the published state already
    /// says what happened.
    pub(crate) fn reload(&self) -> Result<Arc<FeatureGateSnapshot>, FeatureGateError> {
        let persisted = match self.repository.load_all() {
            Ok(persisted) => persisted,
            Err(error) => {
                self.publish_degraded(error.code());
                return Err(error);
            }
        };
        let by_feature = persisted
            .into_iter()
            .map(|record| (record.feature, record))
            .collect::<BTreeMap<_, _>>();
        let snapshot = Arc::new(build_snapshot(
            &by_feature,
            self.forced.as_ref(),
            self.prerequisites.as_ref(),
            FeatureGateFreshness::Current,
        ));
        self.publish(Arc::clone(&snapshot));
        Ok(snapshot)
    }

    /// Republishes the current values marked degraded and records the failure.
    ///
    /// A set that has never loaded stays `NeverLoaded` rather than being relabelled
    /// `ReloadFailed`: the second says a good read exists to fall back to, and it does not.
    fn publish_degraded(&self, code: &'static str) {
        let current = self.snapshot();
        let degradation = match current.freshness() {
            FeatureGateFreshness::Degraded(FeatureGateDegradation::NeverLoaded) => {
                FeatureGateDegradation::NeverLoaded
            }
            _ => FeatureGateDegradation::ReloadFailed,
        };
        self.publish(Arc::new(FeatureGateSnapshot {
            entries: current.entries.clone(),
            freshness: FeatureGateFreshness::Degraded(degradation),
        }));
        // Best effort: the sink that records the failure may be backed by the store that just
        // failed. Losing the record must not also lose the degraded state above.
        let _ = self.audit.record_degradation(&FeatureGateDegradationEntry {
            degradation,
            code,
            recorded_at: self.clock.now_rfc3339(),
        });
    }

    /// The current snapshot, without touching storage. This is the hot path other contexts use.
    pub(crate) fn snapshot(&self) -> Arc<FeatureGateSnapshot> {
        match self.cached.read() {
            Ok(guard) => Arc::clone(&guard),
            // A poisoned lock means another thread panicked mid-publish. An all-disabled snapshot
            // is the fail-closed answer, not a second panic — and it is honestly degraded, since
            // nothing was read to produce it.
            Err(_) => Arc::new(fail_closed_snapshot(FeatureGateDegradation::NeverLoaded)),
        }
    }

    pub(crate) fn is_enabled(&self, feature: ExtensionPlatformFeature) -> bool {
        self.snapshot().is_enabled(feature)
    }

    /// Changes one gate's desired state.
    ///
    /// Refuses to persist an enabled state for a gate this build cannot serve: a stored "on" that
    /// the binary will never honour is indistinguishable from a working gate at every later read.
    /// Disabling an uncompiled gate is allowed, because recording that the operator does not want
    /// it is always meaningful.
    pub(crate) fn set_desired_state(
        &self,
        feature: ExtensionPlatformFeature,
        desired_enabled: bool,
        expected_revision: i64,
        actor: &str,
        reason: Option<String>,
    ) -> Result<Arc<FeatureGateSnapshot>, FeatureGateError> {
        if desired_enabled && !feature.build_available() {
            return Err(FeatureGateError::FeatureUnavailableInBuild { feature });
        }

        let previous_enabled = self.snapshot().get(feature).desired_enabled;
        let now = self.clock.now_rfc3339();
        let stored = self.repository.upsert(&FeatureGateWrite {
            feature,
            desired_enabled,
            expected_revision,
            updated_at: now.clone(),
            updated_by: actor.to_string(),
            reason: reason.clone(),
        })?;

        self.audit.record(&FeatureGateAuditEntry {
            feature,
            previous_enabled,
            new_enabled: stored.desired_enabled,
            revision: stored.revision,
            recorded_at: now,
            actor: actor.to_string(),
            reason,
        })?;

        self.reload()
    }

    fn publish(&self, snapshot: Arc<FeatureGateSnapshot>) {
        if let Ok(mut guard) = self.cached.write() {
            *guard = snapshot;
        }
    }
}

/// Every gate disabled, with no persisted state and no overrides. The answer whenever gate state
/// is unknown or unreadable.
fn fail_closed_snapshot(degradation: FeatureGateDegradation) -> FeatureGateSnapshot {
    build_snapshot(
        &BTreeMap::new(),
        &NoOverrides,
        &NoOverrides,
        FeatureGateFreshness::Degraded(degradation),
    )
}

fn build_snapshot(
    persisted: &BTreeMap<ExtensionPlatformFeature, PersistedFeatureGate>,
    forced: &dyn FeatureForcedDisablePort,
    prerequisites: &dyn FeaturePrerequisitePort,
    freshness: FeatureGateFreshness,
) -> FeatureGateSnapshot {
    FeatureGateSnapshot {
        freshness,
        entries: ALL_FEATURES.map(|feature| {
            let record = persisted.get(&feature);
            // No row means never configured, which is disabled — not an error, and not a reason
            // to omit the gate from the snapshot.
            let desired_enabled = record.is_some_and(|record| record.desired_enabled);
            let status = evaluate_gate(
                feature,
                FeatureGateEvaluation {
                    desired_enabled,
                    forced_disable_reason: forced.forced_disable_reason(feature),
                    unsatisfied_prerequisite: prerequisites.unsatisfied_prerequisite(feature),
                },
            );
            FeatureGateView {
                feature,
                status,
                build_available: feature.build_available(),
                desired_enabled,
                revision: record.map_or(0, |record| record.revision),
                updated_at: record.map(|record| record.updated_at.clone()),
                updated_by: record.map(|record| record.updated_by.clone()),
                reason: record.and_then(|record| record.reason.clone()),
            }
        }),
    }
}

/// Neutral element for both override ports, used for the fail-closed empty snapshot.
struct NoOverrides;

impl FeatureForcedDisablePort for NoOverrides {
    fn forced_disable_reason(&self, _feature: ExtensionPlatformFeature) -> Option<String> {
        None
    }
}

impl FeaturePrerequisitePort for NoOverrides {
    fn unsatisfied_prerequisite(
        &self,
        _feature: ExtensionPlatformFeature,
    ) -> Option<PrerequisiteReason> {
        None
    }
}
