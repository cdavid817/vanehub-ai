use std::sync::{Arc, Mutex};

use super::error::PersonalizationApplicationError;
use super::manage_memory::MemoryApplicationService;
use super::migrate_legacy_memories::LegacyMemoryMigrationService;
use super::migrate_legacy_policy::map_legacy_settings;
use super::ports::{
    ClockPort, LegacyPersonalizationSettingsPort, LegacyPolicyMigrationPort,
    LegacyRowMigrationPort, MaintenanceLockPort, MemoryHealthPort, MigrationStatePort,
    PolicyRepository,
};
use crate::contexts::personalization::domain::{
    MemoryRuntimeHealth, MigrationPhase, MigrationState, PersonalizationPolicyScope,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// Brings personalization data to a state a runtime may use, once per installation, and reports
/// whether it got there.
///
/// # Why every phase runs before anything is `Ready`
///
/// A journal entry reaching `Completed` says one legacy source became one v2 record. It says
/// nothing about whether the projection can answer a list query, whether `MEMORY.md` names the
/// records that exist, or whether the retrieval index still points at files that are gone. Those
/// are what a runtime actually reads, so `Ready` is a statement about all of them together:
///
/// 1. a validated dedicated policy exists;
/// 2. the pre-file rows have been converted;
/// 3. every safe legacy source has completed, failed, or been quarantined — accounted for, not
///    merely attempted;
/// 4. the projection has been rebuilt from the authoritative files and its orphans removed;
/// 5. `MEMORY.md` has been rebuilt;
/// 6. the retrieval index has been reconciled;
/// 7. a generation has been committed.
///
/// Anything less leaves a derived view disagreeing with the files, which is the shape that keeps a
/// deleted memory recallable and an unmigrated one invisible.
///
/// # Why fail-closed rather than a previous generation
///
/// A validated last-known-good generation would need the derived views from that generation to
/// still be intact, and the failure that makes this run stop is usually the reason they are not.
/// Serving a prior generation would mean answering "which memories exist" from state this build has
/// just failed to verify. Unavailable is a worse experience and a correct answer; stale is a better
/// experience and a guess.
pub(crate) struct StartupMaintenanceService {
    lock: Arc<dyn MaintenanceLockPort>,
    state: Arc<dyn MigrationStatePort>,
    policies: Arc<dyn PolicyRepository>,
    policy_migration: Arc<dyn LegacyPolicyMigrationPort>,
    legacy_settings: Arc<dyn LegacyPersonalizationSettingsPort>,
    rows: Arc<dyn LegacyRowMigrationPort>,
    memories: Arc<LegacyMemoryMigrationService>,
    derived: Arc<MemoryApplicationService>,
    clock: Arc<dyn ClockPort>,
    /// What this process concluded, when the durable row cannot say.
    ///
    /// Only ever holds `Busy`: every other conclusion is written down. A local value is never
    /// allowed to outrank a settled durable one, which is what stops a process that gave up on a
    /// held lock from reporting busy forever after the holder finished.
    observed: Mutex<Option<MemoryRuntimeHealth>>,
}

pub(crate) struct StartupMaintenancePorts {
    pub(crate) lock: Arc<dyn MaintenanceLockPort>,
    pub(crate) state: Arc<dyn MigrationStatePort>,
    pub(crate) policies: Arc<dyn PolicyRepository>,
    pub(crate) policy_migration: Arc<dyn LegacyPolicyMigrationPort>,
    pub(crate) legacy_settings: Arc<dyn LegacyPersonalizationSettingsPort>,
    pub(crate) rows: Arc<dyn LegacyRowMigrationPort>,
    pub(crate) memories: Arc<LegacyMemoryMigrationService>,
    pub(crate) derived: Arc<MemoryApplicationService>,
    pub(crate) clock: Arc<dyn ClockPort>,
}

impl StartupMaintenanceService {
    pub(crate) fn new(ports: StartupMaintenancePorts) -> Self {
        Self {
            lock: ports.lock,
            state: ports.state,
            policies: ports.policies,
            policy_migration: ports.policy_migration,
            legacy_settings: ports.legacy_settings,
            rows: ports.rows,
            memories: ports.memories,
            derived: ports.derived,
            clock: ports.clock,
            observed: Mutex::new(None),
        }
    }

    /// Whether memory may be used right now, and if not, why.
    ///
    /// Re-reads the durable row on every call. That is the whole reason a second process never gets
    /// stuck on `Busy`: the moment the holder commits a generation, this answers `Ready` without
    /// anything having to notify it.
    pub(crate) fn health(&self) -> MemoryRuntimeHealth {
        let persisted = match self.state.load() {
            Ok(state) => state.health(),
            // An unreadable marker is not an argument for using the data behind it.
            Err(_) => MemoryRuntimeHealth::Failed,
        };
        if persisted.is_settled() {
            return persisted;
        }
        self.observed
            .lock()
            .ok()
            .and_then(|observed| *observed)
            .unwrap_or(persisted)
    }

    /// Runs the whole sequence, or reports why it could not.
    ///
    /// Returns rather than panics on every failure: this is called from a background thread at
    /// startup, and an unavailable memory subsystem must not take the application with it.
    pub(crate) fn run(&self) -> MemoryRuntimeHealth {
        let lease = match self.lock.try_acquire() {
            Ok(Some(lease)) => lease,
            Ok(None) => return self.observe(MemoryRuntimeHealth::Busy),
            Err(error) => return self.fail(failure_code(&error)),
        };

        let health = match self.run_locked() {
            Ok(health) => health,
            Err(error) => self.fail(failure_code(&error)),
        };
        drop(lease);
        health
    }

    fn run_locked(&self) -> Result<MemoryRuntimeHealth> {
        let mut state = self
            .state
            .load()
            .unwrap_or_else(|_| MigrationState::not_started());
        if state.health().allows_memory_use() {
            // Every startup runs this. A completed installation must be a no-op rather than a
            // re-import, so the durable generation is what decides, not the presence of any file.
            self.clear_observation();
            return Ok(state.health());
        }

        let now = self.clock.now();
        state.phase = MigrationPhase::Migrating;
        state.started_at = Some(now);
        state.last_error_code = None;
        state.repair_required = false;
        self.state.save(&state)?;

        self.migrate_policy(now)?;
        self.migrate_rows(&mut state, now)?;

        let memories = self.memories.run()?;
        // Contention on the memory directory means an ordinary writer is mid-save. Nothing is
        // half-applied and the journal says exactly where each source stopped, so the honest answer
        // is "not yet", not "failed".
        if memories.deferred > 0 {
            return Ok(self.observe(MemoryRuntimeHealth::Busy));
        }

        state.phase = MigrationPhase::RebuildingDerived;
        self.state.save(&state)?;
        let derived = self.derived.reconcile()?;

        // Only now is every view known to agree with the files behind it.
        if memories.requires_repair() || !derived.failures.is_empty() {
            state.repair_required = true;
            state.last_error_code = Some(
                memories
                    .failure_codes
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "derived_rebuild_incomplete".to_string()),
            );
            self.state.save(&state)?;
            self.clear_observation();
            return Ok(MemoryRuntimeHealth::RepairRequired);
        }

        state.generation = state.generation.saturating_add(1);
        state.phase = MigrationPhase::Ready;
        state.completed_at = Some(self.clock.now());
        state.last_error_code = None;
        self.state.save(&state)?;
        self.clear_observation();
        Ok(MemoryRuntimeHealth::Ready {
            generation: state.generation,
        })
    }

    /// Moves the legacy `AppSettings` personalization fields into the dedicated policy, once.
    ///
    /// Fails closed on a policy that cannot be read back afterwards: a runtime with no validated
    /// policy has nothing to resolve against, and resolving against defaults would silently grant
    /// whatever the defaults happen to allow.
    fn migrate_policy(&self, now: chrono::DateTime<chrono::Utc>) -> Result<()> {
        if !self.policy_migration.is_complete()? {
            let legacy = self.legacy_settings.load()?;
            let migrated = map_legacy_settings(&legacy)?;
            // One transaction for the rows and the marker. A marker without rows would make the
            // next startup skip a migration that never happened; rows without a marker would run it
            // again over data that had already moved.
            self.policy_migration.commit(&migrated, now)?;
        }
        // Idempotent, and the reason a fresh installation needs no special case.
        self.policies.seed_default_global(now)?;
        self.policies
            .load(&PersonalizationPolicyScope::Global)?
            .ok_or_else(|| {
                PersonalizationApplicationError::Storage("policy_unavailable".to_string())
            })?;
        Ok(())
    }

    fn migrate_rows(
        &self,
        state: &mut MigrationState,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        if state.legacy_rows_migrated_at.is_some() {
            return Ok(());
        }
        self.rows.convert_rows_to_legacy_files()?;
        // Recorded before the file migration runs over the results, so a crash between the two
        // resumes into the file migration rather than converting the rows a second time — which
        // would resurrect memories the user has since deleted.
        state.legacy_rows_migrated_at = Some(now);
        self.state.save(state)
    }

    fn observe(&self, health: MemoryRuntimeHealth) -> MemoryRuntimeHealth {
        if let Ok(mut observed) = self.observed.lock() {
            *observed = Some(health);
        }
        health
    }

    fn clear_observation(&self) {
        if let Ok(mut observed) = self.observed.lock() {
            *observed = None;
        }
    }

    /// Records a terminal failure and reports it. Never leaves the durable row saying `Migrating`,
    /// which would be indistinguishable from a run still in progress.
    fn fail(&self, code: String) -> MemoryRuntimeHealth {
        let mut state = self
            .state
            .load()
            .unwrap_or_else(|_| MigrationState::not_started());
        state.phase = MigrationPhase::Failed;
        state.last_error_code = Some(code);
        let _ = self.state.save(&state);
        self.clear_observation();
        MemoryRuntimeHealth::Failed
    }
}

impl MemoryHealthPort for StartupMaintenanceService {
    fn health(&self) -> MemoryRuntimeHealth {
        Self::health(self)
    }
}

/// Codes only. This is persisted and surfaced, so no path and no message may travel in it.
fn failure_code(error: &PersonalizationApplicationError) -> String {
    match error {
        PersonalizationApplicationError::Storage(message)
            if message
                .chars()
                .all(|character| character.is_ascii_lowercase() || character == '_') =>
        {
            message.clone()
        }
        PersonalizationApplicationError::MaintenanceBusy => "maintenance_busy".to_string(),
        PersonalizationApplicationError::Domain(_) => "domain_validation_failed".to_string(),
        PersonalizationApplicationError::NotFound => "policy_unavailable".to_string(),
        _ => "startup_maintenance_failed".to_string(),
    }
}
