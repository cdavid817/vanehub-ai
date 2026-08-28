//! What a decision about a proposal actually does, over the real store.

use super::compatibility_tests::{fixture, mark_ready, seed, Fixture};
use crate::contexts::personalization::application::{
    CandidateSubmission, PersonalizationApplicationError, ReviewRequest, UpdateMemoryPatch,
};
use crate::contexts::personalization::domain::{
    ArchiveMemoryCandidate, CandidateReviewStatus, CreateMemoryCandidate, MemoryAudience, MemoryId,
    MemoryProvenance, MemoryScope, MemorySource, MemoryStatus, MemoryType,
    PersonalizationDomainError, ReviewAction, UpdateMemoryCandidate,
};

fn submit(
    fixture: &Fixture,
    operation: crate::contexts::personalization::domain::MemoryCandidateOperation,
    eligible: Vec<MemoryId>,
) -> MemoryId {
    let outcome = fixture
        .api
        .submit_memory_candidates(CandidateSubmission {
            proposals: vec![operation],
            source: MemorySource::OnePieceAutomatic,
            provenance: MemoryProvenance::default(),
            eligible_targets: eligible,
        })
        .expect("submission");
    assert_eq!(outcome.accepted_count(), 1);
    outcome.accepted[0].clone()
}

fn create_proposal(
    name: &str,
) -> crate::contexts::personalization::domain::MemoryCandidateOperation {
    crate::contexts::personalization::domain::MemoryCandidateOperation::Create(
        CreateMemoryCandidate {
            name: name.to_string(),
            description: format!("About {name}"),
            memory_type: MemoryType::Project,
            content: "Never pnpm in this repo.".to_string(),
            scope: MemoryScope::Global,
            audience: MemoryAudience::AllAgents,
        },
    )
}

/// Approval is the only thing that turns a proposal into a memory.
#[test]
fn approving_a_create_produces_one_active_record_and_nothing_stays_pending() {
    let fixture = fixture("review-approve-create");
    mark_ready(&fixture);
    let candidate_id = submit(&fixture, create_proposal("npm-only"), Vec::new());

    let outcome = fixture
        .api
        .review_memory_candidate(ReviewRequest {
            candidate_id,
            action: ReviewAction::Approve,
        })
        .expect("review");

    assert_eq!(outcome.status, CandidateReviewStatus::Approved);
    let active = fixture.api.compatibility_memories().expect("listing");
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].name, "npm-only");
    assert_eq!(active[0].content, "Never pnpm in this repo.");
    assert_eq!(
        fixture
            .api
            .pending_memory_candidate_count()
            .expect("pending count"),
        0
    );
}

/// The source records how a record came to exist, and approval does not change that.
///
/// A memory the model proposed is still a memory the model proposed. A user reading its provenance
/// months later needs to see that, not a claim that they wrote it themselves.
#[test]
fn approval_keeps_the_provenance_the_proposal_carried() {
    let fixture = fixture("review-provenance");
    mark_ready(&fixture);
    let candidate_id = submit(&fixture, create_proposal("npm-only"), Vec::new());

    fixture
        .api
        .review_memory_candidate(ReviewRequest {
            candidate_id,
            action: ReviewAction::Approve,
        })
        .expect("review");

    let active = fixture.api.compatibility_memories().expect("listing");
    assert!(
        active[0].is_automatic,
        "an approved proposal must not read as a user's own note"
    );
}

/// Rejection touches no active state and leaves nothing waiting.
#[test]
fn rejecting_a_proposal_writes_no_memory_and_clears_the_queue() {
    let fixture = fixture("review-reject");
    mark_ready(&fixture);
    let candidate_id = submit(&fixture, create_proposal("npm-only"), Vec::new());

    let outcome = fixture
        .api
        .review_memory_candidate(ReviewRequest {
            candidate_id,
            action: ReviewAction::Reject,
        })
        .expect("review");

    assert_eq!(outcome.status, CandidateReviewStatus::Rejected);
    assert_eq!(outcome.resulting_memory_id, None);
    assert!(!outcome.retained_content);
    assert!(fixture
        .api
        .compatibility_memories()
        .expect("listing")
        .is_empty());
    assert_eq!(
        fixture
            .api
            .pending_memory_candidate_count()
            .expect("pending count"),
        0
    );
}

/// A proposal is decided once.
///
/// A second approval would create a second record from one proposal; a second rejection would
/// report something that did not happen. Both are refused rather than reapplied.
#[test]
fn a_proposal_cannot_be_decided_twice() {
    let fixture = fixture("review-twice");
    mark_ready(&fixture);
    let candidate_id = submit(&fixture, create_proposal("npm-only"), Vec::new());
    fixture
        .api
        .review_memory_candidate(ReviewRequest {
            candidate_id: candidate_id.clone(),
            action: ReviewAction::Approve,
        })
        .expect("first review");

    let again = fixture.api.review_memory_candidate(ReviewRequest {
        candidate_id,
        action: ReviewAction::Reject,
    });

    assert!(matches!(
        again,
        Err(PersonalizationApplicationError::Domain(
            PersonalizationDomainError::CandidateAlreadyReviewed
        ))
    ));
    assert_eq!(
        fixture.api.compatibility_memories().expect("listing").len(),
        1
    );
}

/// An update proposal corrects the record it named, at the revision it was written against.
#[test]
fn approving_an_update_corrects_the_record_it_named() {
    let fixture = fixture("review-update");
    mark_ready(&fixture);
    let record = seed(
        &fixture,
        "npm-only",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    let candidate_id = submit(
        &fixture,
        crate::contexts::personalization::domain::MemoryCandidateOperation::Update(
            UpdateMemoryCandidate {
                target_id: record.id.clone(),
                expected_target_revision: record.revision,
                name: None,
                description: None,
                content: Some("Uses pnpm after all.".to_string()),
            },
        ),
        vec![record.id.clone()],
    );

    fixture
        .api
        .review_memory_candidate(ReviewRequest {
            candidate_id,
            action: ReviewAction::Approve,
        })
        .expect("review");

    let active = fixture.api.compatibility_memories().expect("listing");
    assert_eq!(active.len(), 1, "a correction must not add a second record");
    assert_eq!(active[0].content, "Uses pnpm after all.");
}

/// A proposal written minutes ago must not silently replace an edit made in between.
///
/// The revision is read from the store at review time and compared against what the proposal
/// expected. Comparing the proposal to itself would make the check pass by construction, which is
/// the failure mode this exists to prevent.
#[test]
fn approving_an_update_whose_target_moved_is_a_conflict_rather_than_an_overwrite() {
    let fixture = fixture("review-update-conflict");
    mark_ready(&fixture);
    let record = seed(
        &fixture,
        "npm-only",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    let candidate_id = submit(
        &fixture,
        crate::contexts::personalization::domain::MemoryCandidateOperation::Update(
            UpdateMemoryCandidate {
                target_id: record.id.clone(),
                expected_target_revision: record.revision,
                name: None,
                description: None,
                content: Some("Uses pnpm after all.".to_string()),
            },
        ),
        vec![record.id.clone()],
    );
    // The user edits the same memory before getting to the queue.
    fixture
        .service
        .update(
            &record.id,
            record.revision,
            UpdateMemoryPatch {
                content: Some("Corrected by hand.".to_string()),
                ..UpdateMemoryPatch::default()
            },
        )
        .expect("user edit");

    let refused = fixture.api.review_memory_candidate(ReviewRequest {
        candidate_id,
        action: ReviewAction::Approve,
    });

    assert!(matches!(
        refused,
        Err(PersonalizationApplicationError::RevisionConflict(_))
    ));
    let active = fixture.api.compatibility_memories().expect("listing");
    assert_eq!(active[0].content, "Corrected by hand.");
}

/// Archive rather than delete: a model proposing removal is not evidence strong enough to destroy
/// a record, and an archived memory is still there to restore.
#[test]
fn approving_an_archive_retires_the_record_without_destroying_it() {
    let fixture = fixture("review-archive");
    mark_ready(&fixture);
    let record = seed(
        &fixture,
        "stale-note",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    let candidate_id = submit(
        &fixture,
        crate::contexts::personalization::domain::MemoryCandidateOperation::Archive(
            ArchiveMemoryCandidate {
                target_id: record.id.clone(),
                expected_target_revision: record.revision,
            },
        ),
        vec![record.id.clone()],
    );

    fixture
        .api
        .review_memory_candidate(ReviewRequest {
            candidate_id,
            action: ReviewAction::Approve,
        })
        .expect("review");

    let stored = fixture
        .service
        .detail(&record.id)
        .expect("detail")
        .expect("record still exists");
    assert_eq!(stored.status, MemoryStatus::Archived);
    // Archived records leave the view a runtime reads from, which is what "stop using this" means.
    assert!(fixture
        .api
        .compatibility_memories()
        .expect("listing")
        .is_empty());
}

/// A reviewer may correct what a proposal said before keeping it.
#[test]
fn approving_with_edits_stores_what_the_reviewer_wrote_rather_than_what_was_proposed() {
    let fixture = fixture("review-edits");
    mark_ready(&fixture);
    let candidate_id = submit(&fixture, create_proposal("npm-only"), Vec::new());

    fixture
        .api
        .review_memory_candidate(ReviewRequest {
            candidate_id,
            action: ReviewAction::ApproveWithEdits {
                name: Some("package-manager".to_string()),
                description: Some("Which package manager this repo uses".to_string()),
                content: Some("npm, never pnpm.".to_string()),
                memory_type: None,
                scope: None,
                audience: None,
            },
        })
        .expect("review");

    let active = fixture.api.compatibility_memories().expect("listing");
    assert_eq!(active[0].name, "package-manager");
    assert_eq!(active[0].content, "npm, never pnpm.");
}

/// An approval that landed while migration owned the directory would be written into a store whose
/// derived views are mid-rebuild, and the rebuild would either miss it or resurrect what it was
/// removing.
#[test]
fn reviewing_is_refused_while_migration_owns_the_directory() {
    let fixture = fixture("review-maintenance");
    mark_ready(&fixture);
    let candidate_id = submit(&fixture, create_proposal("npm-only"), Vec::new());
    let reopened = super::compatibility_tests::fixture("review-maintenance-not-ready");

    let refused = reopened.api.review_memory_candidate(ReviewRequest {
        candidate_id,
        action: ReviewAction::Approve,
    });

    assert!(matches!(
        refused,
        Err(PersonalizationApplicationError::MaintenanceRequired)
    ));
}
