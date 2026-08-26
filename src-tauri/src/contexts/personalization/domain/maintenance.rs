use chrono::{DateTime, Utc};

use super::memory::{MemoryId, MemoryStatus};
use super::query::MemoryScopeFilter;

/// The exact phrase a user must type to execute a reset. Matched literally and case-sensitively:
/// a confirmation a user can pass by pressing Enter is not a confirmation.
pub(crate) const RESET_CONFIRMATION_PHRASE: &str = "DELETE";

/// How long a reset preview's counts stay usable. Short enough that the number a user reads is
/// still the number that will be deleted.
pub(crate) const RESET_TOKEN_TTL_SECONDS: i64 = 120;

/// Issued by the preview, required by the execute. Its purpose is to stop the UI from confirming
/// against counts that have since changed, so it is bound to the preview's filters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResetConfirmationToken {
    pub(crate) value: String,
    pub(crate) issued_at: DateTime<Utc>,
    pub(crate) scope: MemoryScopeFilter,
    pub(crate) statuses: Vec<MemoryStatus>,
}

impl ResetConfirmationToken {
    pub(crate) fn is_expired_at(&self, now: DateTime<Utc>) -> bool {
        (now - self.issued_at).num_seconds() >= RESET_TOKEN_TTL_SECONDS
    }

    /// A token issued for one filter must not authorize a different, broader deletion.
    pub(crate) fn authorizes(&self, scope: &MemoryScopeFilter, statuses: &[MemoryStatus]) -> bool {
        &self.scope == scope && self.statuses == statuses
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResetMemoryPreview {
    pub(crate) matched: usize,
    pub(crate) matched_global: usize,
    pub(crate) matched_workspace: usize,
    pub(crate) matched_candidates: usize,
    /// Owned files that will not parse. Counted separately because they are exactly what the old
    /// capped, parse-dependent scan lost track of.
    pub(crate) matched_malformed: usize,
    pub(crate) token: ResetConfirmationToken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResetMemoryRequest {
    pub(crate) scope: MemoryScopeFilter,
    pub(crate) statuses: Vec<MemoryStatus>,
    pub(crate) token: ResetConfirmationToken,
    pub(crate) typed_phrase: String,
}

/// Why a reset was refused before anything was deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResetRefusal {
    PhraseMismatch,
    TokenExpired,
    /// The token was issued for a different scope or status set than the one being executed.
    TokenScopeMismatch,
}

impl ResetMemoryRequest {
    pub(crate) fn authorize(&self, now: DateTime<Utc>) -> Result<(), ResetRefusal> {
        if self.typed_phrase != RESET_CONFIRMATION_PHRASE {
            return Err(ResetRefusal::PhraseMismatch);
        }
        if self.token.is_expired_at(now) {
            return Err(ResetRefusal::TokenExpired);
        }
        if !self.token.authorizes(&self.scope, &self.statuses) {
            return Err(ResetRefusal::TokenScopeMismatch);
        }
        Ok(())
    }
}

/// What a maintenance operation could not do. Carries the memory id, never a filesystem path — the
/// outcome is rendered in the UI and written to logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MaintenanceFailure {
    pub(crate) memory_id: Option<MemoryId>,
    pub(crate) phase: MaintenancePhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MaintenancePhase {
    AuthoritativeFile,
    SqliteProjection,
    DerivedIndex,
    RetrievalIndex,
    Quarantine,
    /// A malformed owned file whose scope could not be established, encountered by a scope-limited
    /// reset. Guessing a scope would delete across a boundary the user did not authorize.
    UnclassifiableEntry,
}

impl MaintenancePhase {
    /// A stable code, because this reaches a screen and a log. `Debug` would name the variant
    /// today and something else the moment it is renamed.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::AuthoritativeFile => "authoritative-file",
            Self::SqliteProjection => "sqlite-projection",
            Self::DerivedIndex => "derived-index",
            Self::RetrievalIndex => "retrieval-index",
            Self::Quarantine => "quarantine",
            Self::UnclassifiableEntry => "unclassifiable-entry",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ResetMemoryOutcome {
    pub(crate) matched: usize,
    pub(crate) deleted_files: usize,
    pub(crate) deleted_projection_rows: usize,
    pub(crate) revoked_retrieval_entries: usize,
    pub(crate) removed_quarantine_entries: usize,
    pub(crate) failures: Vec<MaintenanceFailure>,
}

impl ResetMemoryOutcome {
    /// Consistency is uncertain the moment any phase fails, so the caller must set repair-required
    /// rather than reporting a clean reset with a footnote.
    pub(crate) fn requires_repair(&self) -> bool {
        !self.failures.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReconcileMemoryOutcome {
    pub(crate) scanned_entries: usize,
    pub(crate) rebuilt_projection_rows: usize,
    pub(crate) rebuilt_index_lines: usize,
    pub(crate) revoked_orphan_retrieval_entries: usize,
    pub(crate) quarantined_entries: usize,
    pub(crate) failures: Vec<MaintenanceFailure>,
}

/// How an entry in the memory directory is classified by complete enumeration.
///
/// Classification is by explicit rule, not by whether parsing happened to succeed. That distinction
/// is the whole fix: the previous implementation silently skipped anything it could not parse, and
/// destructive work inherited that blindness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnedEntryClassification {
    ValidV2,
    MalformedV2,
    /// A pre-v2, path-addressed memory file awaiting migration.
    LegacyV1,
    /// `MEMORY.md` and anything else generated from authoritative records.
    Derived,
    Quarantined,
    /// In-flight temporary or lock file from an atomic write.
    Transient,
    /// Present in the directory but not owned by this application.
    Foreign,
}

impl OwnedEntryClassification {
    /// Whether a reset is allowed to delete this entry. Derived files are rebuilt rather than
    /// deleted, and foreign files are not ours to remove.
    pub(crate) fn is_resettable(self) -> bool {
        matches!(
            self,
            Self::ValidV2 | Self::MalformedV2 | Self::LegacyV1 | Self::Quarantined
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StorageEntry {
    pub(crate) file_name: String,
    pub(crate) classification: OwnedEntryClassification,
    /// Absent for entries whose frontmatter could not be read — which is precisely why enumeration
    /// cannot depend on parsing.
    pub(crate) memory_id: Option<MemoryId>,
}

/// How far startup maintenance has got, as persisted.
///
/// Deliberately smaller than [`MemoryRuntimeHealth`]: this is what one process wrote down, while
/// health is what a caller in *this* process may conclude, which also depends on whether another
/// process currently holds maintenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MigrationPhase {
    NotStarted,
    /// Legacy policy, legacy rows, and legacy files are being converted.
    Migrating,
    /// Conversion finished; the projection, the index, and the retrieval entries are being rebuilt
    /// from the authoritative files.
    RebuildingDerived,
    /// Every phase completed and a generation was committed.
    Ready,
    /// A phase failed in a way that leaves memory unusable until it is retried.
    Failed,
}

impl MigrationPhase {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::Migrating => "migrating",
            Self::RebuildingDerived => "rebuilding_derived",
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }

    /// An unreadable value reads as `NotStarted` rather than failing.
    ///
    /// The alternative is refusing to start because a marker this build does not recognize exists,
    /// and `NotStarted` is the safe reading: it keeps memory unavailable and schedules the work.
    pub(crate) fn parse(value: &str) -> Self {
        match value {
            "migrating" => Self::Migrating,
            "rebuilding_derived" => Self::RebuildingDerived,
            "ready" => Self::Ready,
            "failed" => Self::Failed,
            _ => Self::NotStarted,
        }
    }
}

/// Whether stored memory may be used, and if not, why.
///
/// One value, checked in one place, by every path that would read or write a governed memory. The
/// question "is this data trustworthy" has exactly one answer per process at any moment, and making
/// it an enum rather than a boolean is what lets a surface say *why* without a second source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemoryRuntimeHealth {
    NotStarted,
    /// Another process holds maintenance. Transient by construction: re-reading the persisted state
    /// is what resolves it, so nothing stays here once the holder finishes.
    Busy,
    Migrating,
    RebuildingDerived,
    Ready {
        generation: u64,
    },
    /// Authoritative data is intact but a derived view is not. Memory stays unavailable, because a
    /// derived view that disagrees with the files is how a deleted memory stays recallable.
    RepairRequired,
    Failed,
}

impl MemoryRuntimeHealth {
    /// The single question every runtime path asks. Only a committed generation answers yes.
    pub(crate) fn allows_memory_use(self) -> bool {
        matches!(self, Self::Ready { .. })
    }

    /// A stable code for diagnostics and for the UI. Never a message, never a path.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::Busy => "busy",
            Self::Migrating => "migrating",
            Self::RebuildingDerived => "rebuilding_derived",
            Self::Ready { .. } => "ready",
            Self::RepairRequired => "repair_required",
            Self::Failed => "failed",
        }
    }

    /// Whether this value came from durable state rather than from what one process observed.
    ///
    /// A persisted conclusion always wins over a local one: a process that gave up on a held lock
    /// must see `Ready` as soon as the holder commits, or it would stay `Busy` forever.
    pub(crate) fn is_settled(self) -> bool {
        matches!(
            self,
            Self::Ready { .. } | Self::RepairRequired | Self::Failed
        )
    }
}

/// Durable record of how far personalization data migration has got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MigrationState {
    pub(crate) generation: u64,
    pub(crate) phase: MigrationPhase,
    pub(crate) started_at: Option<DateTime<Utc>>,
    pub(crate) completed_at: Option<DateTime<Utc>>,
    /// When the pre-file row store was converted. A dedicated marker rather than the presence of a
    /// derived index file: `MEMORY.md` is rebuilt from v2 records too, so treating its existence as
    /// "the rows already migrated" would make a v2-only installation look mid-migration, and a
    /// rebuilt index would silently re-authorize a conversion that had already run.
    pub(crate) legacy_rows_migrated_at: Option<DateTime<Utc>>,
    /// A stable code, never a message: this is persisted and surfaced.
    pub(crate) last_error_code: Option<String>,
    pub(crate) repair_required: bool,
    /// When a rebuild last completed. `None` means one has never run, which is a different answer
    /// from "it ran and found nothing" and the two must not render the same.
    pub(crate) last_reconciled_at: Option<DateTime<Utc>>,
}

impl MigrationState {
    pub(crate) fn not_started() -> Self {
        Self {
            generation: 0,
            phase: MigrationPhase::NotStarted,
            started_at: None,
            completed_at: None,
            legacy_rows_migrated_at: None,
            last_error_code: None,
            repair_required: false,
            last_reconciled_at: None,
        }
    }

    /// Memory stays unavailable until a generation actually completed. An interrupted migration
    /// looks identical to a completed one from the outside unless this is checked.
    pub(crate) fn is_complete(&self) -> bool {
        self.completed_at.is_some() && matches!(self.phase, MigrationPhase::Ready)
    }

    /// What this durable row alone says about whether memory may be used.
    ///
    /// `repair_required` outranks everything, including a committed generation: a generation says
    /// the files were converted, and repair says a derived view no longer agrees with them.
    pub(crate) fn health(&self) -> MemoryRuntimeHealth {
        if self.repair_required {
            return MemoryRuntimeHealth::RepairRequired;
        }
        match self.phase {
            MigrationPhase::Failed => MemoryRuntimeHealth::Failed,
            MigrationPhase::Migrating => MemoryRuntimeHealth::Migrating,
            MigrationPhase::RebuildingDerived => MemoryRuntimeHealth::RebuildingDerived,
            // A `Ready` phase with no completion timestamp is not ready. Trusting the phase alone
            // would let a half-written commit answer the one question that must never be guessed.
            MigrationPhase::Ready if self.completed_at.is_some() => MemoryRuntimeHealth::Ready {
                generation: self.generation,
            },
            MigrationPhase::Ready | MigrationPhase::NotStarted => MemoryRuntimeHealth::NotStarted,
        }
    }
}
