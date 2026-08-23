use chrono::{Duration, TimeZone, Utc};

use super::candidate::{
    ArchiveMemoryCandidate, CandidateReviewStatus, MemoryCandidate, MemoryCandidateOperation,
    ReviewAction, UpdateMemoryCandidate,
};
use super::maintenance::{
    MaintenanceFailure, MaintenancePhase, MigrationState, OwnedEntryClassification,
    ResetConfirmationToken, ResetMemoryOutcome, ResetMemoryRequest, ResetRefusal,
    RESET_CONFIRMATION_PHRASE, RESET_TOKEN_TTL_SECONDS,
};
use super::memory::{MemoryId, MemoryProvenance, MemorySource, MemoryStatus};
use super::policy::RevisionConflict;
use super::query::{
    MemoryOrder, MemoryQuery, MemoryScopeFilter, MEMORY_PAGE_DEFAULT_SIZE, MEMORY_PAGE_MAX_SIZE,
};
use super::scope::WorkspaceKey;

fn issued_at() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 24, 12, 0, 0).unwrap()
}

fn memory_id(value: &str) -> MemoryId {
    MemoryId::parse(value).expect("memory id")
}

fn token(scope: MemoryScopeFilter, statuses: Vec<MemoryStatus>) -> ResetConfirmationToken {
    ResetConfirmationToken {
        value: "tok_01K2ABCDEF".to_string(),
        issued_at: issued_at(),
        scope,
        statuses,
    }
}

fn request(
    scope: MemoryScopeFilter,
    statuses: Vec<MemoryStatus>,
    token: ResetConfirmationToken,
    phrase: &str,
) -> ResetMemoryRequest {
    ResetMemoryRequest {
        scope,
        statuses,
        token,
        typed_phrase: phrase.to_string(),
    }
}

#[test]
fn a_reset_needs_the_exact_typed_phrase() {
    let subject = request(
        MemoryScopeFilter::Any,
        vec![MemoryStatus::Active],
        token(MemoryScopeFilter::Any, vec![MemoryStatus::Active]),
        "delete",
    );
    assert_eq!(
        subject.authorize(issued_at()),
        Err(ResetRefusal::PhraseMismatch),
        "the phrase is matched case-sensitively"
    );

    let subject = request(
        MemoryScopeFilter::Any,
        vec![MemoryStatus::Active],
        token(MemoryScopeFilter::Any, vec![MemoryStatus::Active]),
        RESET_CONFIRMATION_PHRASE,
    );
    assert_eq!(subject.authorize(issued_at()), Ok(()));
}

#[test]
fn a_reset_token_expires_so_the_confirmed_counts_stay_current() {
    let subject = request(
        MemoryScopeFilter::Any,
        vec![MemoryStatus::Active],
        token(MemoryScopeFilter::Any, vec![MemoryStatus::Active]),
        RESET_CONFIRMATION_PHRASE,
    );
    let just_inside = issued_at() + Duration::seconds(RESET_TOKEN_TTL_SECONDS - 1);
    assert_eq!(subject.authorize(just_inside), Ok(()));

    let expired = issued_at() + Duration::seconds(RESET_TOKEN_TTL_SECONDS);
    assert_eq!(subject.authorize(expired), Err(ResetRefusal::TokenExpired));
}

#[test]
fn a_token_issued_for_one_scope_cannot_authorize_a_broader_delete() {
    // The concrete attack this closes: preview a workspace-only reset, then execute an all-memory
    // reset with the same token.
    let workspace = MemoryScopeFilter::Workspace {
        workspace_key: WorkspaceKey::parse("ws_1").expect("workspace"),
    };
    let subject = request(
        MemoryScopeFilter::Any,
        vec![MemoryStatus::Active],
        token(workspace, vec![MemoryStatus::Active]),
        RESET_CONFIRMATION_PHRASE,
    );
    assert_eq!(
        subject.authorize(issued_at()),
        Err(ResetRefusal::TokenScopeMismatch)
    );
}

#[test]
fn a_token_issued_for_one_status_set_cannot_authorize_another() {
    let subject = request(
        MemoryScopeFilter::Any,
        vec![MemoryStatus::Active, MemoryStatus::Archived],
        token(MemoryScopeFilter::Any, vec![MemoryStatus::Active]),
        RESET_CONFIRMATION_PHRASE,
    );
    assert_eq!(
        subject.authorize(issued_at()),
        Err(ResetRefusal::TokenScopeMismatch)
    );
}

#[test]
fn any_maintenance_failure_marks_the_outcome_as_needing_repair() {
    let clean = ResetMemoryOutcome {
        matched: 3,
        deleted_files: 3,
        ..ResetMemoryOutcome::default()
    };
    assert!(!clean.requires_repair());

    let partial = ResetMemoryOutcome {
        matched: 3,
        deleted_files: 2,
        failures: vec![MaintenanceFailure {
            memory_id: Some(memory_id("01K2ABCDEFGHJKMNPQRSTVWXYZ")),
            phase: MaintenancePhase::RetrievalIndex,
        }],
        ..ResetMemoryOutcome::default()
    };
    assert!(
        partial.requires_repair(),
        "a reset that could not revoke an index entry is not a clean reset"
    );
}

#[test]
fn only_application_owned_entries_are_resettable() {
    // Malformed and legacy entries are exactly what the old parse-dependent scan lost; derived and
    // foreign entries are exactly what a reset must not delete.
    for classification in [
        OwnedEntryClassification::ValidV2,
        OwnedEntryClassification::MalformedV2,
        OwnedEntryClassification::LegacyV1,
        OwnedEntryClassification::Quarantined,
    ] {
        assert!(
            classification.is_resettable(),
            "{classification:?} must be reachable by reset"
        );
    }
    for classification in [
        OwnedEntryClassification::Derived,
        OwnedEntryClassification::Transient,
        OwnedEntryClassification::Foreign,
    ] {
        assert!(
            !classification.is_resettable(),
            "{classification:?} must not be deleted by reset"
        );
    }
}

#[test]
fn migration_is_incomplete_until_a_generation_finishes() {
    let mut state = MigrationState::not_started();
    assert!(!state.is_complete());

    state.started_at = Some(issued_at());
    assert!(
        !state.is_complete(),
        "an interrupted migration must not read as complete"
    );

    state.completed_at = Some(issued_at());
    assert!(state.is_complete());
}

#[test]
fn a_list_page_size_is_clamped_rather_than_trusted() {
    assert_eq!(MemoryQuery::default().page_size(), MEMORY_PAGE_DEFAULT_SIZE);
    assert_eq!(MemoryQuery::default().with_page_size(10).page_size(), 10);
    assert_eq!(
        MemoryQuery::default()
            .with_page_size(MEMORY_PAGE_MAX_SIZE + 5_000)
            .page_size(),
        MEMORY_PAGE_MAX_SIZE,
        "the list endpoint must not become a way to pull the whole store"
    );
    assert_eq!(MemoryQuery::default().with_page_size(0).page_size(), 1);
}

#[test]
fn the_default_query_orders_newest_first() {
    let query = MemoryQuery::default();
    assert_eq!(query.order, MemoryOrder::UpdatedDescending);
    assert_eq!(query.scope, MemoryScopeFilter::Any);
    assert!(query.cursor.is_none());
    assert!(
        query.statuses.is_empty(),
        "an unfiltered query must not silently exclude statuses"
    );
}

#[test]
fn a_scope_filter_never_matches_across_workspaces() {
    let mine = super::memory::MemoryScope::Workspace {
        workspace_key: WorkspaceKey::parse("ws_1").expect("workspace"),
    };
    let theirs = super::memory::MemoryScope::Workspace {
        workspace_key: WorkspaceKey::parse("ws_2").expect("workspace"),
    };
    let filter = MemoryScopeFilter::Workspace {
        workspace_key: WorkspaceKey::parse("ws_1").expect("workspace"),
    };
    assert!(filter.matches(&mine));
    assert!(!filter.matches(&theirs));
    assert!(!filter.matches(&super::memory::MemoryScope::Global));

    assert!(MemoryScopeFilter::GlobalOnly.matches(&super::memory::MemoryScope::Global));
    assert!(!MemoryScopeFilter::GlobalOnly.matches(&mine));
    assert!(MemoryScopeFilter::Any.matches(&mine));
}

#[test]
fn approving_a_proposal_whose_target_moved_is_a_conflict() {
    let candidate = MemoryCandidate {
        id: memory_id("01K9CANDIDATE0000000000000"),
        operation: MemoryCandidateOperation::Update(UpdateMemoryCandidate {
            target_id: memory_id("01K2ABCDEFGHJKMNPQRSTVWXYZ"),
            expected_target_revision: 4,
            name: None,
            description: None,
            content: Some("corrected".to_string()),
        }),
        source: MemorySource::OnePieceAutomatic,
        provenance: MemoryProvenance::default(),
        status: CandidateReviewStatus::Pending,
        created_at: issued_at(),
    };
    assert!(candidate.is_pending());
    assert_eq!(candidate.check_target_revision(4), Ok(()));
    assert_eq!(
        candidate.check_target_revision(5),
        Err(RevisionConflict {
            expected: 4,
            current: 5,
        }),
        "an edit made since the proposal must not be silently overwritten"
    );
}

#[test]
fn an_archive_proposal_also_carries_its_target_revision() {
    let candidate = MemoryCandidate {
        id: memory_id("01K9CANDIDATE0000000000001"),
        operation: MemoryCandidateOperation::Archive(ArchiveMemoryCandidate {
            target_id: memory_id("01K2ABCDEFGHJKMNPQRSTVWXYZ"),
            expected_target_revision: 2,
        }),
        source: MemorySource::CliAutomatic,
        provenance: MemoryProvenance::default(),
        status: CandidateReviewStatus::Pending,
        created_at: issued_at(),
    };
    assert_eq!(candidate.operation.kind_str(), "archive");
    assert_eq!(
        candidate.operation.target_id().map(MemoryId::as_str),
        Some("01K2ABCDEFGHJKMNPQRSTVWXYZ")
    );
    assert_eq!(
        candidate.check_target_revision(3),
        Err(RevisionConflict {
            expected: 2,
            current: 3,
        })
    );
}

#[test]
fn rejection_is_the_only_review_action_that_touches_no_active_state() {
    assert!(!ReviewAction::Reject.mutates_active_state());
    assert!(ReviewAction::Approve.mutates_active_state());
    assert!(ReviewAction::MarkSensitiveAndArchive.mutates_active_state());
    assert!(ReviewAction::MergeInto {
        target_id: memory_id("01K2ABCDEFGHJKMNPQRSTVWXYZ"),
        expected_target_revision: 1,
    }
    .mutates_active_state());
}
