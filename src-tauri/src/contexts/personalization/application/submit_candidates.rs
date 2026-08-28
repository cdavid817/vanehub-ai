use std::collections::HashSet;
use std::sync::Arc;

use super::error::PersonalizationApplicationError;
use super::ports::{CandidateRepository, ClockPort, MemoryIdGeneratorPort};
use crate::contexts::personalization::domain::{
    MemoryCandidate, MemoryCandidateOperation, MemoryId, MemoryProvenance, MemorySource,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// Proposals honoured from one submission.
///
/// A submitter proposing dozens of writes at once is malfunctioning, not productive, and the
/// review queue is a human surface. Bounded here rather than trusting the caller's own bound: a
/// caller's limit is the caller's guarantee, and this service answers to several of them.
pub(crate) const MAX_CANDIDATES_PER_SUBMISSION: usize = 10;

/// One batch of proposals from one source.
///
/// `eligible_targets` is what the submitter was allowed to see, not what exists. An update or
/// archive naming anything outside it is rejected, so a proposal cannot reach a memory the policy
/// that produced it never surfaced — which is the only way scope could otherwise be escaped by
/// proposing against a memory the model guessed at.
#[derive(Debug, Clone)]
pub(crate) struct CandidateSubmission {
    pub(crate) proposals: Vec<MemoryCandidateOperation>,
    pub(crate) source: MemorySource,
    pub(crate) provenance: MemoryProvenance,
    pub(crate) eligible_targets: Vec<MemoryId>,
}

/// Why one proposal was dropped.
///
/// Content-free by construction — a position and a reason code, never the rejected text — because
/// these reach the unified log and a rejected proposal is exactly the text nobody approved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CandidateRejection {
    pub(crate) index: usize,
    pub(crate) reason: &'static str,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CandidateSubmissionOutcome {
    pub(crate) accepted: Vec<MemoryId>,
    pub(crate) rejected: Vec<CandidateRejection>,
}

impl CandidateSubmissionOutcome {
    pub(crate) fn accepted_count(&self) -> usize {
        self.accepted.len()
    }

    pub(crate) fn rejected_count(&self) -> usize {
        self.rejected.len()
    }
}

/// Turns proposals into reviewable candidates, and nothing else.
///
/// It cannot write an active memory. That is the point of the split: extraction and the model's
/// own tool reach this service, the active store is reached only through review, and there is no
/// argument either of them could pass that would cross from one to the other.
///
/// Every proposal is judged on its own. One invalid element must not discard the valid ones beside
/// it, and none of them may fail the generation that produced the batch.
pub(crate) struct CandidateSubmissionService {
    candidates: Arc<dyn CandidateRepository>,
    ids: Arc<dyn MemoryIdGeneratorPort>,
    clock: Arc<dyn ClockPort>,
}

impl CandidateSubmissionService {
    pub(crate) fn new(
        candidates: Arc<dyn CandidateRepository>,
        ids: Arc<dyn MemoryIdGeneratorPort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            candidates,
            ids,
            clock,
        }
    }

    pub(crate) fn submit(
        &self,
        submission: CandidateSubmission,
    ) -> Result<CandidateSubmissionOutcome> {
        let eligible: HashSet<&MemoryId> = submission.eligible_targets.iter().collect();
        let now = self.clock.now();
        let mut outcome = CandidateSubmissionOutcome::default();

        for (index, operation) in submission.proposals.iter().enumerate() {
            if outcome.accepted.len() >= MAX_CANDIDATES_PER_SUBMISSION {
                outcome.rejected.push(CandidateRejection {
                    index,
                    reason: "submission-limit",
                });
                continue;
            }
            if let Some(target) = operation.target_id() {
                if !eligible.contains(target) {
                    outcome.rejected.push(CandidateRejection {
                        index,
                        reason: "target-outside-eligible-set",
                    });
                    continue;
                }
            }
            if let Err(reason) = operation.validate() {
                outcome.rejected.push(CandidateRejection { index, reason });
                continue;
            }

            let candidate = MemoryCandidate {
                id: self.ids.generate(),
                operation: operation.clone(),
                source: submission.source,
                provenance: submission.provenance.clone(),
                status: crate::contexts::personalization::domain::CandidateReviewStatus::Pending,
                created_at: now,
            };
            match self.candidates.insert(&candidate) {
                Ok(()) => outcome.accepted.push(candidate.id),
                // Persistence failing for one proposal is recorded and stepped over for the same
                // reason a malformed one is: this runs behind a generation that has already
                // answered, and the batch is best-effort background work either way.
                Err(_) => outcome.rejected.push(CandidateRejection {
                    index,
                    reason: "not-persisted",
                }),
            }
        }
        Ok(outcome)
    }
}
