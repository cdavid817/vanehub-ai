use std::sync::Arc;

use super::error::PersonalizationApplicationError;
use super::manage_memory::MemoryApplicationService;
use super::models::{CreateMemoryInput, UpdateMemoryPatch};
use super::ports::{CandidateRepository, ClockPort};
use crate::contexts::personalization::domain::{
    CandidateReviewStatus, MemoryCandidate, MemoryCandidateOperation, MemoryId, MemoryProvenance,
    MemorySensitivity, MemoryStatus, ReviewAction, ReviewOutcome,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// Reviewed candidates kept before their proposed text is dropped.
///
/// A rejected proposal is content the user declined to keep, so it must not linger as a second
/// copy of itself. Enough survive to answer "what did I just reject" and to make a queue's recent
/// history legible; beyond that only the audit metadata remains.
const REVIEWED_CANDIDATE_RETENTION: usize = 50;

/// One decision about one proposal.
#[derive(Debug, Clone)]
pub(crate) struct ReviewRequest {
    pub(crate) candidate_id: MemoryId,
    pub(crate) action: ReviewAction,
}

/// The only path from a proposal to an active memory.
///
/// Extraction and the model's tool can produce candidates and nothing else; this is where a human
/// decision turns one into a record. Splitting it this way is what makes "a model cannot decide
/// what the user remembers" structural rather than a rule somebody has to keep applying.
///
/// Every action that touches an existing record carries the revision the proposal was written
/// against, and a target that moved since is a typed conflict rather than an overwrite. A proposal
/// written minutes ago must not silently replace an edit the user made in between.
pub(crate) struct CandidateReviewService {
    candidates: Arc<dyn CandidateRepository>,
    memories: Arc<MemoryApplicationService>,
    clock: Arc<dyn ClockPort>,
}

impl CandidateReviewService {
    pub(crate) fn new(
        candidates: Arc<dyn CandidateRepository>,
        memories: Arc<MemoryApplicationService>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            candidates,
            memories,
            clock,
        }
    }

    pub(crate) fn pending(&self, limit: usize) -> Result<Vec<MemoryCandidate>> {
        self.candidates.list_pending(limit)
    }

    pub(crate) fn pending_count(&self) -> Result<usize> {
        self.candidates.count_pending()
    }

    /// Applies one decision.
    ///
    /// The active record changes first and the candidate is marked reviewed afterwards. A crash
    /// between the two leaves a pending candidate whose change already landed, which a second
    /// review resolves as a conflict — the opposite order would leave a candidate marked approved
    /// with nothing to show for it, and nothing left to retry from.
    pub(crate) fn review(&self, request: ReviewRequest) -> Result<ReviewOutcome> {
        let candidate = self
            .candidates
            .get(&request.candidate_id)?
            .ok_or(PersonalizationApplicationError::NotFound)?;
        if !candidate.is_pending() {
            // Reported rather than reapplied. A second approval would create a second record from
            // one proposal, and a second rejection would say something happened that did not.
            return Err(PersonalizationApplicationError::Domain(
                crate::contexts::personalization::domain::PersonalizationDomainError::
                    CandidateAlreadyReviewed,
            ));
        }

        let now = self.clock.now();
        if !request.action.mutates_active_state() {
            self.candidates
                .mark_reviewed(&candidate.id, CandidateReviewStatus::Rejected, now)?;
            self.candidates
                .prune_reviewed(REVIEWED_CANDIDATE_RETENTION)?;
            return Ok(ReviewOutcome {
                candidate_id: candidate.id,
                status: CandidateReviewStatus::Rejected,
                resulting_memory_id: None,
                retained_content: false,
            });
        }

        let resulting = self.apply(&candidate, &request.action)?;
        self.candidates
            .mark_reviewed(&candidate.id, CandidateReviewStatus::Approved, now)?;
        self.candidates
            .prune_reviewed(REVIEWED_CANDIDATE_RETENTION)?;
        Ok(ReviewOutcome {
            candidate_id: candidate.id,
            status: CandidateReviewStatus::Approved,
            resulting_memory_id: Some(resulting),
            retained_content: true,
        })
    }

    fn apply(&self, candidate: &MemoryCandidate, action: &ReviewAction) -> Result<MemoryId> {
        match action {
            ReviewAction::MergeInto {
                target_id,
                expected_target_revision,
            } => {
                // The reviewer chose a different target from the one the proposal named, so the
                // revision they are acting on is theirs, not the proposal's.
                let content = proposed_content(candidate);
                let record = self.memories.update(
                    target_id,
                    *expected_target_revision,
                    UpdateMemoryPatch {
                        content,
                        ..UpdateMemoryPatch::default()
                    },
                )?;
                Ok(record.record.id)
            }
            ReviewAction::MarkSensitiveAndArchive => {
                let target = candidate
                    .operation
                    .target_id()
                    .ok_or(PersonalizationApplicationError::NotFound)?;
                let expected = self.current_revision(target)?;
                candidate.check_target_revision(expected)?;
                let record = self.memories.update(
                    target,
                    expected,
                    UpdateMemoryPatch {
                        status: Some(MemoryStatus::Archived),
                        sensitivity: Some(MemorySensitivity::Sensitive),
                        ..UpdateMemoryPatch::default()
                    },
                )?;
                Ok(record.record.id)
            }
            ReviewAction::Approve | ReviewAction::ApproveWithEdits { .. } => {
                self.apply_operation(candidate, action)
            }
            // Handled before this function is reached; a rejection touches no active state.
            ReviewAction::Reject => Err(PersonalizationApplicationError::NotFound),
        }
    }

    fn apply_operation(
        &self,
        candidate: &MemoryCandidate,
        action: &ReviewAction,
    ) -> Result<MemoryId> {
        let edits = match action {
            ReviewAction::ApproveWithEdits {
                name,
                description,
                content,
                memory_type,
                scope,
                audience,
            } => Some((name, description, content, memory_type, scope, audience)),
            _ => None,
        };
        match &candidate.operation {
            MemoryCandidateOperation::Create(create) => {
                let record = self.memories.create(CreateMemoryInput {
                    name: edits
                        .and_then(|(name, ..)| name.clone())
                        .unwrap_or_else(|| create.name.clone()),
                    description: edits
                        .and_then(|(_, description, ..)| description.clone())
                        .unwrap_or_else(|| create.description.clone()),
                    memory_type: edits
                        .and_then(|(_, _, _, memory_type, ..)| *memory_type)
                        .unwrap_or(create.memory_type),
                    content: edits
                        .and_then(|(_, _, content, ..)| content.clone())
                        .unwrap_or_else(|| create.content.clone()),
                    scope: edits
                        .and_then(|(_, _, _, _, scope, _)| scope.clone())
                        .unwrap_or_else(|| create.scope.clone()),
                    audience: edits
                        .and_then(|(_, _, _, _, _, audience)| audience.clone())
                        .unwrap_or_else(|| create.audience.clone()),
                    status: MemoryStatus::Active,
                    // The source records how the record came to exist, and approval does not
                    // change that: a memory the model proposed is still a memory the model
                    // proposed, and a user reading its provenance later needs to see so.
                    source: candidate.source,
                    provenance: MemoryProvenance {
                        ..candidate.provenance.clone()
                    },
                    sensitivity: MemorySensitivity::Normal,
                })?;
                Ok(record.record.id)
            }
            MemoryCandidateOperation::Update(update) => {
                let expected = self.current_revision(&update.target_id)?;
                candidate.check_target_revision(expected)?;
                let record = self.memories.update(
                    &update.target_id,
                    expected,
                    UpdateMemoryPatch {
                        name: edits.and_then(|(name, ..)| name.clone()).or(None),
                        description: edits
                            .and_then(|(_, description, ..)| description.clone())
                            .or_else(|| update.description.clone()),
                        content: edits
                            .and_then(|(_, _, content, ..)| content.clone())
                            .or_else(|| update.content.clone()),
                        ..UpdateMemoryPatch::default()
                    },
                )?;
                Ok(record.record.id)
            }
            MemoryCandidateOperation::Archive(archive) => {
                let expected = self.current_revision(&archive.target_id)?;
                candidate.check_target_revision(expected)?;
                let record = self.memories.update(
                    &archive.target_id,
                    expected,
                    UpdateMemoryPatch {
                        status: Some(MemoryStatus::Archived),
                        ..UpdateMemoryPatch::default()
                    },
                )?;
                Ok(record.record.id)
            }
        }
    }

    /// The revision the target is at right now.
    ///
    /// Read here rather than taken from the proposal, so `check_target_revision` compares what the
    /// proposal expected against what is actually stored. Comparing the proposal to itself would
    /// make the check pass by construction.
    fn current_revision(&self, id: &MemoryId) -> Result<u64> {
        self.memories
            .detail(id)?
            .map(|record| record.revision)
            .ok_or(PersonalizationApplicationError::NotFound)
    }
}

/// What a proposal is offering as text, for an action that redirects it somewhere else.
fn proposed_content(candidate: &MemoryCandidate) -> Option<String> {
    match &candidate.operation {
        MemoryCandidateOperation::Create(create) => Some(create.content.clone()),
        MemoryCandidateOperation::Update(update) => update.content.clone(),
        MemoryCandidateOperation::Archive(_) => None,
    }
}
