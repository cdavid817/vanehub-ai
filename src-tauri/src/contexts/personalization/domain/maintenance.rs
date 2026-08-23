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

/// Durable record of how far personalization data migration has got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MigrationState {
    pub(crate) generation: u64,
    pub(crate) started_at: Option<DateTime<Utc>>,
    pub(crate) completed_at: Option<DateTime<Utc>>,
    /// A stable code, never a message: this is persisted and surfaced.
    pub(crate) last_error_code: Option<String>,
    pub(crate) repair_required: bool,
}

impl MigrationState {
    pub(crate) fn not_started() -> Self {
        Self {
            generation: 0,
            started_at: None,
            completed_at: None,
            last_error_code: None,
            repair_required: false,
        }
    }

    /// Memory stays unavailable until a generation actually completed. An interrupted migration
    /// looks identical to a completed one from the outside unless this is checked.
    pub(crate) fn is_complete(&self) -> bool {
        self.completed_at.is_some()
    }
}
