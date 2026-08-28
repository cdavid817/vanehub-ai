use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeZone, Utc};

use super::error::PersonalizationApplicationError;
use super::ports::{CandidateRepository, ClockPort, MemoryIdGeneratorPort};
use super::submit_candidates::{
    CandidateSubmission, CandidateSubmissionService, MAX_CANDIDATES_PER_SUBMISSION,
};
use crate::contexts::personalization::domain::{
    ArchiveMemoryCandidate, CandidateReviewStatus, CreateMemoryCandidate, MemoryAudience,
    MemoryCandidate, MemoryCandidateOperation, MemoryId, MemoryProvenance, MemoryScope,
    MemorySource, MemoryType, UpdateMemoryCandidate,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 25, 12, 0, 0).unwrap()
}

struct FixedClock;

impl ClockPort for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        now()
    }
}

#[derive(Default)]
struct SequentialIds {
    next: AtomicUsize,
}

impl MemoryIdGeneratorPort for SequentialIds {
    fn generate(&self) -> MemoryId {
        let index = self.next.fetch_add(1, Ordering::SeqCst);
        MemoryId::parse(&format!("01K2CAND{index:018}")).expect("candidate id")
    }
}

#[derive(Default)]
struct RecordingCandidates {
    inserted: Mutex<Vec<MemoryCandidate>>,
    fail_from: Option<usize>,
}

impl RecordingCandidates {
    fn failing_from(index: usize) -> Self {
        Self {
            fail_from: Some(index),
            ..Self::default()
        }
    }
}

impl CandidateRepository for RecordingCandidates {
    fn insert(&self, candidate: &MemoryCandidate) -> Result<()> {
        let mut inserted = self.inserted.lock().expect("inserted");
        if self.fail_from.is_some_and(|from| inserted.len() >= from) {
            return Err(PersonalizationApplicationError::Storage(
                "candidate table unavailable".to_string(),
            ));
        }
        inserted.push(candidate.clone());
        Ok(())
    }

    fn get(&self, candidate_id: &MemoryId) -> Result<Option<MemoryCandidate>> {
        Ok(self
            .inserted
            .lock()
            .expect("inserted")
            .iter()
            .find(|candidate| &candidate.id == candidate_id)
            .cloned())
    }

    fn list_pending(&self, limit: usize) -> Result<Vec<MemoryCandidate>> {
        Ok(self
            .inserted
            .lock()
            .expect("inserted")
            .iter()
            .filter(|candidate| candidate.is_pending())
            .take(limit)
            .cloned()
            .collect())
    }

    fn count_pending(&self) -> Result<usize> {
        Ok(self
            .inserted
            .lock()
            .expect("inserted")
            .iter()
            .filter(|candidate| candidate.is_pending())
            .count())
    }

    fn mark_reviewed(
        &self,
        _candidate_id: &MemoryId,
        _status: CandidateReviewStatus,
        _reviewed_at: DateTime<Utc>,
    ) -> Result<()> {
        Ok(())
    }

    fn prune_reviewed(&self, _retain: usize) -> Result<usize> {
        Ok(0)
    }
}

fn service(candidates: Arc<RecordingCandidates>) -> CandidateSubmissionService {
    CandidateSubmissionService::new(
        candidates,
        Arc::new(SequentialIds::default()),
        Arc::new(FixedClock),
    )
}

fn memory_id(label: &str) -> MemoryId {
    MemoryId::parse(&format!("01K2MEM{label:0>19}")).expect("memory id")
}

fn create(name: &str) -> MemoryCandidateOperation {
    MemoryCandidateOperation::Create(CreateMemoryCandidate {
        name: name.to_string(),
        description: format!("About {name}"),
        memory_type: MemoryType::Project,
        content: "Never pnpm in this repo.".to_string(),
        scope: MemoryScope::Global,
        audience: MemoryAudience::AllAgents,
    })
}

fn update(target: &MemoryId) -> MemoryCandidateOperation {
    MemoryCandidateOperation::Update(UpdateMemoryCandidate {
        target_id: target.clone(),
        expected_target_revision: 3,
        name: None,
        description: None,
        content: Some("Corrected.".to_string()),
    })
}

fn archive(target: &MemoryId) -> MemoryCandidateOperation {
    MemoryCandidateOperation::Archive(ArchiveMemoryCandidate {
        target_id: target.clone(),
        expected_target_revision: 3,
    })
}

fn submission(
    proposals: Vec<MemoryCandidateOperation>,
    eligible_targets: Vec<MemoryId>,
) -> CandidateSubmission {
    CandidateSubmission {
        proposals,
        source: MemorySource::OnePieceAutomatic,
        provenance: MemoryProvenance::default(),
        eligible_targets,
    }
}

#[test]
fn accepted_proposals_become_pending_candidates_and_nothing_else() {
    let candidates = Arc::new(RecordingCandidates::default());
    let target = memory_id("1");

    let outcome = service(candidates.clone())
        .submit(submission(
            vec![create("npm-only"), update(&target), archive(&target)],
            vec![target.clone()],
        ))
        .expect("submission");

    assert_eq!(outcome.accepted_count(), 3);
    assert_eq!(outcome.rejected_count(), 0);
    let inserted = candidates.inserted.lock().expect("inserted");
    assert!(inserted.iter().all(MemoryCandidate::is_pending));
    assert_eq!(
        inserted
            .iter()
            .map(|candidate| candidate.operation.kind_str())
            .collect::<Vec<_>>(),
        vec!["create", "update", "archive"]
    );
}

/// The one way scope could otherwise be escaped.
///
/// A proposal names a target the submitter chose. If the service accepted any id that happened to
/// exist, a model shown three memories could correct or archive a fourth it had only guessed at —
/// including one belonging to a workspace this session may not read.
#[test]
fn a_proposal_naming_a_target_outside_the_eligible_set_is_rejected() {
    let candidates = Arc::new(RecordingCandidates::default());
    let offered = memory_id("1");
    let unseen = memory_id("2");

    let outcome = service(candidates.clone())
        .submit(submission(
            vec![update(&unseen), archive(&unseen), create("kept")],
            vec![offered],
        ))
        .expect("submission");

    assert_eq!(outcome.accepted_count(), 1);
    assert_eq!(
        outcome
            .rejected
            .iter()
            .map(|rejection| rejection.reason)
            .collect::<Vec<_>>(),
        vec!["target-outside-eligible-set", "target-outside-eligible-set"]
    );
    assert_eq!(candidates.inserted.lock().expect("inserted").len(), 1);
}

/// One bad proposal must not discard the good ones beside it.
#[test]
fn an_invalid_proposal_is_dropped_while_its_neighbours_survive() {
    let candidates = Arc::new(RecordingCandidates::default());
    let mut blank = create("blank");
    if let MemoryCandidateOperation::Create(create) = &mut blank {
        create.content = "   ".to_string();
    }

    let outcome = service(candidates.clone())
        .submit(submission(
            vec![create("first"), blank, create("second")],
            Vec::new(),
        ))
        .expect("submission");

    assert_eq!(outcome.accepted_count(), 2);
    assert_eq!(
        outcome.rejected.first().map(|rejection| rejection.reason),
        Some("candidate-content")
    );
    assert_eq!(
        outcome.rejected.first().map(|rejection| rejection.index),
        Some(1)
    );
}

/// An update proposing no change would bump a revision and alter no text.
#[test]
fn an_update_that_changes_nothing_is_rejected() {
    let candidates = Arc::new(RecordingCandidates::default());
    let target = memory_id("1");
    let empty = MemoryCandidateOperation::Update(UpdateMemoryCandidate {
        target_id: target.clone(),
        expected_target_revision: 3,
        name: None,
        description: None,
        content: None,
    });

    let outcome = service(candidates.clone())
        .submit(submission(vec![empty], vec![target]))
        .expect("submission");

    assert_eq!(outcome.accepted_count(), 0);
    assert_eq!(
        outcome.rejected.first().map(|rejection| rejection.reason),
        Some("candidate-empty-update")
    );
}

/// `Untyped` exists for records migrated from a store that had no types. Nothing proposed today
/// may claim it: there is no pre-governance file to attribute it to.
#[test]
fn a_proposal_claiming_the_legacy_untyped_type_is_rejected() {
    let candidates = Arc::new(RecordingCandidates::default());
    let mut untyped = create("untyped");
    if let MemoryCandidateOperation::Create(create) = &mut untyped {
        create.memory_type = MemoryType::Untyped;
    }

    let outcome = service(candidates.clone())
        .submit(submission(vec![untyped], Vec::new()))
        .expect("submission");

    assert_eq!(outcome.accepted_count(), 0);
    assert_eq!(
        outcome.rejected.first().map(|rejection| rejection.reason),
        Some("candidate-untyped")
    );
}

/// A submitter proposing dozens of writes is malfunctioning, and the queue is a human surface.
#[test]
fn proposals_past_the_submission_bound_are_rejected_rather_than_queued() {
    let candidates = Arc::new(RecordingCandidates::default());
    let proposals: Vec<MemoryCandidateOperation> = (0..MAX_CANDIDATES_PER_SUBMISSION + 3)
        .map(|index| create(&format!("memory-{index}")))
        .collect();

    let outcome = service(candidates.clone())
        .submit(submission(proposals, Vec::new()))
        .expect("submission");

    assert_eq!(outcome.accepted_count(), MAX_CANDIDATES_PER_SUBMISSION);
    assert_eq!(outcome.rejected_count(), 3);
    assert!(outcome
        .rejected
        .iter()
        .all(|rejection| rejection.reason == "submission-limit"));
}

/// Persistence failing for one proposal is recorded, not raised: this runs behind a generation
/// that has already answered.
#[test]
fn a_proposal_that_cannot_be_persisted_is_counted_rather_than_raised() {
    let candidates = Arc::new(RecordingCandidates::failing_from(1));

    let outcome = service(candidates.clone())
        .submit(submission(
            vec![create("first"), create("second"), create("third")],
            Vec::new(),
        ))
        .expect("submission must not fail the caller");

    assert_eq!(outcome.accepted_count(), 1);
    assert_eq!(outcome.rejected_count(), 2);
    assert!(outcome
        .rejected
        .iter()
        .all(|rejection| rejection.reason == "not-persisted"));
}

/// A rejection reaches the unified log, and a rejected proposal is exactly the text nobody
/// approved. Reasons are codes for that reason.
#[test]
fn rejection_reasons_never_carry_the_rejected_text() {
    let candidates = Arc::new(RecordingCandidates::default());
    let mut secret = create("secret");
    if let MemoryCandidateOperation::Create(create) = &mut secret {
        create.content = String::new();
        create.description = "sk-live-should-never-appear".to_string();
    }

    let outcome = service(candidates)
        .submit(submission(vec![secret], Vec::new()))
        .expect("submission");

    for rejection in &outcome.rejected {
        assert!(!rejection.reason.contains("sk-live"));
        assert!(!rejection.reason.contains("secret"));
    }
}

#[test]
fn an_empty_submission_persists_nothing() {
    let candidates = Arc::new(RecordingCandidates::default());

    let outcome = service(candidates.clone())
        .submit(submission(Vec::new(), Vec::new()))
        .expect("submission");

    assert_eq!(outcome, Default::default());
    assert!(candidates.inserted.lock().expect("inserted").is_empty());
}
