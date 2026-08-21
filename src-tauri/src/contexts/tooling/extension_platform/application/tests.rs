//! Capability-gate service behavior against deterministic port doubles.

use super::feature_gates::FeatureGateService;
use super::ports::{
    FeatureForcedDisablePort, FeatureGateAuditEntry, FeatureGateAuditSink, FeatureGateClock,
    FeatureGateRepository, FeatureGateWrite, FeaturePrerequisitePort, PersistedFeatureGate,
};
use crate::contexts::tooling::extension_platform::domain::{
    ExtensionPlatformFeature, FeatureGateError, FeatureGateStatus, PrerequisiteReason, ALL_FEATURES,
};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct InMemoryRepository {
    rows: Mutex<BTreeMap<ExtensionPlatformFeature, PersistedFeatureGate>>,
    fail_reads: Mutex<bool>,
}

impl FeatureGateRepository for InMemoryRepository {
    fn load_all(&self) -> Result<Vec<PersistedFeatureGate>, FeatureGateError> {
        if *self
            .fail_reads
            .lock()
            .unwrap_or_else(|error| error.into_inner())
        {
            return Err(FeatureGateError::Storage("read failed".to_string()));
        }
        let rows = self.rows.lock().unwrap_or_else(|error| error.into_inner());
        Ok(rows.values().cloned().collect())
    }

    fn upsert(&self, write: &FeatureGateWrite) -> Result<PersistedFeatureGate, FeatureGateError> {
        let mut rows = self.rows.lock().unwrap_or_else(|error| error.into_inner());
        let current_revision = rows.get(&write.feature).map_or(0, |row| row.revision);
        if current_revision != write.expected_revision {
            return Err(FeatureGateError::StaleRevision {
                feature: write.feature,
                expected: write.expected_revision,
                actual: current_revision,
            });
        }
        let stored = PersistedFeatureGate {
            feature: write.feature,
            desired_enabled: write.desired_enabled,
            revision: current_revision + 1,
            updated_at: write.updated_at.clone(),
            updated_by: write.updated_by.clone(),
            reason: write.reason.clone(),
        };
        rows.insert(write.feature, stored.clone());
        Ok(stored)
    }
}

#[derive(Default)]
struct RecordingAudit {
    entries: Mutex<Vec<FeatureGateAuditEntry>>,
}

impl FeatureGateAuditSink for RecordingAudit {
    fn record(&self, entry: &FeatureGateAuditEntry) -> Result<(), FeatureGateError> {
        self.entries
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(entry.clone());
        Ok(())
    }
}

struct NoForced;

impl FeatureForcedDisablePort for NoForced {
    fn forced_disable_reason(&self, _feature: ExtensionPlatformFeature) -> Option<String> {
        None
    }
}

struct ForceAll(&'static str);

impl FeatureForcedDisablePort for ForceAll {
    fn forced_disable_reason(&self, _feature: ExtensionPlatformFeature) -> Option<String> {
        Some(self.0.to_string())
    }
}

struct NoPrerequisites;

impl FeaturePrerequisitePort for NoPrerequisites {
    fn unsatisfied_prerequisite(
        &self,
        _feature: ExtensionPlatformFeature,
    ) -> Option<PrerequisiteReason> {
        None
    }
}

struct BlockCatalog;

impl FeaturePrerequisitePort for BlockCatalog {
    fn unsatisfied_prerequisite(
        &self,
        feature: ExtensionPlatformFeature,
    ) -> Option<PrerequisiteReason> {
        (feature == ExtensionPlatformFeature::Catalog)
            .then_some(PrerequisiteReason::SandboxSelfTestUnavailable)
    }
}

struct FixedClock;

impl FeatureGateClock for FixedClock {
    fn now_rfc3339(&self) -> String {
        "2026-08-22T00:00:00Z".to_string()
    }
}

struct Harness {
    service: FeatureGateService,
    repository: Arc<InMemoryRepository>,
    audit: Arc<RecordingAudit>,
}

fn harness_with(
    forced: Arc<dyn FeatureForcedDisablePort>,
    prerequisites: Arc<dyn FeaturePrerequisitePort>,
) -> Harness {
    let repository = Arc::new(InMemoryRepository::default());
    let audit = Arc::new(RecordingAudit::default());
    let service = FeatureGateService::new(
        Arc::clone(&repository) as Arc<dyn FeatureGateRepository>,
        Arc::clone(&audit) as Arc<dyn FeatureGateAuditSink>,
        forced,
        prerequisites,
        Arc::new(FixedClock),
    );
    Harness {
        service,
        repository,
        audit,
    }
}

fn harness() -> Harness {
    harness_with(Arc::new(NoForced), Arc::new(NoPrerequisites))
}

#[test]
fn every_gate_starts_disabled() {
    let harness = harness();
    let snapshot = harness.service.snapshot();

    for feature in ALL_FEATURES {
        assert!(!snapshot.is_enabled(feature), "{feature} started enabled");
        assert_eq!(snapshot.get(feature).revision, 0);
        assert!(snapshot.get(feature).updated_at.is_none());
    }
}

#[test]
fn the_snapshot_lists_every_gate_in_stable_order() {
    let harness = harness();
    let snapshot = harness.service.snapshot();

    let listed: Vec<_> = snapshot.views().map(|view| view.feature).collect();
    assert_eq!(listed, ALL_FEATURES.to_vec());
}

#[test]
fn a_missing_row_is_disabled_rather_than_an_error() {
    let harness = harness();
    harness
        .service
        .set_desired_state(ExtensionPlatformFeature::Catalog, true, 0, "operator", None)
        .expect("enable should succeed");

    // Only one gate has a row; the other six must still resolve, as disabled.
    let snapshot = harness.service.snapshot();
    assert!(snapshot.is_enabled(ExtensionPlatformFeature::Catalog));
    assert!(!snapshot.is_enabled(ExtensionPlatformFeature::Connectors));
    assert_eq!(
        snapshot.get(ExtensionPlatformFeature::Connectors).status,
        FeatureGateStatus::RuntimeDisabled
    );
}

#[test]
fn a_storage_read_failure_fails_closed_without_clearing_the_previous_snapshot() {
    let harness = harness();
    harness
        .service
        .set_desired_state(ExtensionPlatformFeature::Catalog, true, 0, "operator", None)
        .expect("enable should succeed");
    assert!(harness
        .service
        .is_enabled(ExtensionPlatformFeature::Catalog));

    *harness
        .repository
        .fail_reads
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = true;

    let error = harness.service.reload().expect_err("reload should fail");
    assert_eq!(error.code(), "storage");
    // The last known-good snapshot survives; nothing silently becomes more permissive, and
    // nothing that was on flickers off because one read failed.
    assert!(harness
        .service
        .is_enabled(ExtensionPlatformFeature::Catalog));
}

#[test]
fn a_service_built_on_unreadable_storage_reports_everything_disabled() {
    let repository = Arc::new(InMemoryRepository::default());
    *repository
        .fail_reads
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = true;
    let service = FeatureGateService::new(
        repository as Arc<dyn FeatureGateRepository>,
        Arc::new(RecordingAudit::default()) as Arc<dyn FeatureGateAuditSink>,
        Arc::new(NoForced),
        Arc::new(NoPrerequisites),
        Arc::new(FixedClock),
    );

    for feature in ALL_FEATURES {
        assert!(
            !service.is_enabled(feature),
            "{feature} enabled on a failed read"
        );
    }
}

#[test]
fn enabling_a_gate_this_build_cannot_serve_is_refused_and_persists_nothing() {
    // Both runtime-bearing gates are off in the default build. If a build ever turns one on,
    // this assertion has nothing to prove and is skipped rather than silently inverted.
    if ExtensionPlatformFeature::SidecarRuntime.build_available() {
        return;
    }
    let harness = harness();

    let error = harness
        .service
        .set_desired_state(
            ExtensionPlatformFeature::SidecarRuntime,
            true,
            0,
            "operator",
            None,
        )
        .expect_err("enabling an uncompiled gate must fail");

    assert_eq!(error.code(), "feature_unavailable_in_build");
    assert!(harness
        .repository
        .rows
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .is_empty());
    assert!(harness
        .audit
        .entries
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .is_empty());
}

#[test]
fn an_uncompiled_gate_reports_not_compiled_not_runtime_disabled() {
    if ExtensionPlatformFeature::SidecarRuntime.build_available() {
        return;
    }
    let harness = harness();
    let snapshot = harness.service.snapshot();
    let view = snapshot.get(ExtensionPlatformFeature::SidecarRuntime);

    assert_eq!(view.status, FeatureGateStatus::NotCompiled);
    assert!(!view.build_available);
    assert_ne!(view.status, FeatureGateStatus::RuntimeDisabled);
}

#[test]
fn disabling_an_uncompiled_gate_is_allowed_and_recorded() {
    if ExtensionPlatformFeature::SidecarRuntime.build_available() {
        return;
    }
    let harness = harness();

    harness
        .service
        .set_desired_state(
            ExtensionPlatformFeature::SidecarRuntime,
            false,
            0,
            "operator",
            Some("not ready".to_string()),
        )
        .expect("recording an operator's 'off' is always meaningful");

    let entries = harness
        .audit
        .entries
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    assert_eq!(entries.len(), 1);
}

#[test]
fn a_stale_revision_is_rejected_without_overwriting() {
    let harness = harness();
    harness
        .service
        .set_desired_state(ExtensionPlatformFeature::Catalog, true, 0, "first", None)
        .expect("first write should succeed");

    let error = harness
        .service
        .set_desired_state(ExtensionPlatformFeature::Catalog, false, 0, "second", None)
        .expect_err("a second writer holding revision 0 must be rejected");

    assert_eq!(error.code(), "stale_revision");
    assert!(harness
        .service
        .is_enabled(ExtensionPlatformFeature::Catalog));
}

#[test]
fn every_accepted_mutation_is_audited_with_its_transition() {
    let harness = harness();
    harness
        .service
        .set_desired_state(
            ExtensionPlatformFeature::Catalog,
            true,
            0,
            "operator",
            Some("gate 1".to_string()),
        )
        .expect("enable should succeed");
    harness
        .service
        .set_desired_state(
            ExtensionPlatformFeature::Catalog,
            false,
            1,
            "operator",
            None,
        )
        .expect("disable should succeed");

    let entries = harness
        .audit
        .entries
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    assert_eq!(entries.len(), 2);

    assert_eq!(entries[0].feature, ExtensionPlatformFeature::Catalog);
    assert!(!entries[0].previous_enabled);
    assert!(entries[0].new_enabled);
    assert_eq!(entries[0].revision, 1);
    assert_eq!(entries[0].actor, "operator");
    assert_eq!(entries[0].reason.as_deref(), Some("gate 1"));

    assert!(entries[1].previous_enabled);
    assert!(!entries[1].new_enabled);
    assert_eq!(entries[1].revision, 2);
}

#[test]
fn a_forced_disable_overrides_an_enabled_gate_without_editing_desired_state() {
    let harness = harness_with(Arc::new(ForceAll("incident")), Arc::new(NoPrerequisites));
    harness
        .service
        .set_desired_state(ExtensionPlatformFeature::Catalog, true, 0, "operator", None)
        .expect("enable should succeed");

    let snapshot = harness.service.snapshot();
    let view = snapshot.get(ExtensionPlatformFeature::Catalog);
    assert_eq!(
        view.status,
        FeatureGateStatus::ForcedDisabled {
            reason: "incident".to_string()
        }
    );
    // Desired state is untouched, so lifting the override restores exactly what the operator had.
    assert!(view.desired_enabled);
    assert!(!snapshot.is_enabled(ExtensionPlatformFeature::Catalog));
}

#[test]
fn an_unsatisfied_prerequisite_is_reported_rather_than_silently_failing() {
    let harness = harness_with(Arc::new(NoForced), Arc::new(BlockCatalog));
    harness
        .service
        .set_desired_state(ExtensionPlatformFeature::Catalog, true, 0, "operator", None)
        .expect("enable should succeed");

    let snapshot = harness.service.snapshot();
    assert_eq!(
        snapshot.get(ExtensionPlatformFeature::Catalog).status,
        FeatureGateStatus::BlockedByPrerequisite(PrerequisiteReason::SandboxSelfTestUnavailable)
    );
    assert!(!snapshot.is_enabled(ExtensionPlatformFeature::Catalog));
}

#[test]
fn a_captured_snapshot_does_not_change_underneath_its_holder() {
    let harness = harness();
    let before = harness.service.snapshot();

    harness
        .service
        .set_desired_state(ExtensionPlatformFeature::Catalog, true, 0, "operator", None)
        .expect("enable should succeed");

    assert!(!before.is_enabled(ExtensionPlatformFeature::Catalog));
    assert!(harness
        .service
        .snapshot()
        .is_enabled(ExtensionPlatformFeature::Catalog));
}
