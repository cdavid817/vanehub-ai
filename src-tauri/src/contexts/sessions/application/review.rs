use crate::contexts::sessions::domain::{
    ReviewAnchor, ReviewComment, ReviewCommentStatus, ReviewDecision, ReviewDomainError,
    ReviewFile, ReviewFileViewState, ReviewFinding, ReviewHunkDecision, ReviewSession,
};
use std::fmt;
use std::sync::Arc;

pub(crate) trait ReviewRepository: Send + Sync {
    fn find_active_by_session(
        &self,
        session_id: &str,
    ) -> Result<Option<ReviewSession>, ReviewApplicationError>;
    fn find(&self, review_id: &str) -> Result<Option<ReviewSession>, ReviewApplicationError>;
    fn save(&self, review: &ReviewSession) -> Result<(), ReviewApplicationError>;
}

/// Where hunk decisions are written, separately from the review they belong to.
///
/// A port of its own rather than two more methods on `ReviewRepository`, because the distinction
/// is the requirement: `ReviewRepository::save` rewrites the aggregate, and a hunk decision that
/// travelled through it would rewrite the review's decision along with everything else. Two traits
/// make "this write touches one row" something the type system says rather than something a
/// reviewer has to notice.
pub(crate) trait ReviewDecisionRepository: Send + Sync {
    /// Records a decision for one hunk, replacing whatever that hunk's decision was.
    fn upsert_hunk_decision(
        &self,
        review_id: &str,
        decision: &ReviewHunkDecision,
    ) -> Result<(), ReviewApplicationError>;

    /// Records whether one file has been read, replacing whatever it said before.
    fn upsert_file_view_state(
        &self,
        review_id: &str,
        state: &ReviewFileViewState,
    ) -> Result<(), ReviewApplicationError>;

    /// Every file view state recorded for a review, in path order.
    fn list_file_view_states(
        &self,
        review_id: &str,
    ) -> Result<Vec<ReviewFileViewState>, ReviewApplicationError>;

    /// Every hunk decision recorded for a review, in path then fingerprint order.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the write path is live; reading decisions back is what 13.10's per-hunk \
                 controls render"
        )
    )]
    fn list_hunk_decisions(
        &self,
        review_id: &str,
    ) -> Result<Vec<ReviewHunkDecision>, ReviewApplicationError>;
}

/// What the current diff still contains, for one file.
///
/// A decision is about a hunk, and a hunk is a range of a diff that changes whenever anybody
/// writes to the file. Nothing in the review store can answer whether the hunk a reviewer is
/// looking at is still there — the review holds which files changed, not what the change is now —
/// so this reaches the workspace that owns the diff.
pub(crate) trait ReviewHunkWitnessPort: Send + Sync {
    /// The fingerprints the current bounded diff holds for one file, in the diff's own order.
    fn hunk_fingerprints(
        &self,
        session_id: &str,
        path: &str,
        expected_snapshot: &str,
    ) -> Result<Vec<String>, ReviewApplicationError>;
}

pub(crate) trait ReviewClockPort: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait ReviewIdPort: Send + Sync {
    fn next_id(&self, kind: &'static str) -> String;
}

pub(crate) trait ReviewFeedbackPort: Send + Sync {
    fn send(
        &self,
        session_id: &str,
        feedback: &PreparedReviewFeedback,
    ) -> Result<String, ReviewApplicationError>;
}

pub(crate) trait ReviewSnapshotPort: Send + Sync {
    fn snapshot(&self, session_id: &str) -> Result<CreateReviewRequest, ReviewApplicationError>;
}

pub(crate) trait ReviewOperationPort: Send + Sync {
    fn start(
        &self,
        review_id: &str,
        action: ReviewAction,
    ) -> Result<String, ReviewApplicationError>;
}

pub(crate) trait ReviewLoggingPort: Send + Sync {
    fn record(&self, event: ReviewLogEvent);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewLogEvent {
    pub(crate) kind: &'static str,
    pub(crate) review_id: String,
    pub(crate) item_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReviewAction {
    ReviewAgent,
    Tests,
    Security,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewActionFindingInput {
    pub(crate) title: String,
    pub(crate) severity: String,
    pub(crate) anchor: Option<ReviewAnchor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreateReviewRequest {
    pub(crate) session_id: String,
    pub(crate) workspace_id: String,
    pub(crate) base_revision: Option<String>,
    pub(crate) head_revision: Option<String>,
    pub(crate) fingerprint: String,
    pub(crate) files: Vec<ReviewFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AddReviewCommentRequest {
    pub(crate) review_id: String,
    pub(crate) anchor: ReviewAnchor,
    pub(crate) body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewFeedbackComment {
    pub(crate) comment_id: String,
    pub(crate) file_path: String,
    pub(crate) side: String,
    pub(crate) start_line: u32,
    pub(crate) end_line: u32,
    pub(crate) hunk_fingerprint: String,
    pub(crate) stale: bool,
    pub(crate) body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedReviewFeedback {
    pub(crate) review_id: String,
    pub(crate) decision: ReviewDecision,
    pub(crate) comments: Vec<ReviewFeedbackComment>,
    pub(crate) prepared_at: String,
}

/// What a caller is asking to mark, and the diff they were looking at when they asked.
///
/// The expectation is the caller's, and it is the whole point: a reviewer decides about the diff
/// on their screen, which may be several writes behind the one on disk. Without it the service
/// would record a decision about whatever the diff happens to be when the click arrives — a
/// decision the reviewer never made, indistinguishable from one they did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SetHunkDecisionRequest {
    pub(crate) path: String,
    pub(crate) hunk_fingerprint: String,
    /// The snapshot the reviewer was looking at.
    pub(crate) expected_snapshot_fingerprint: String,
    pub(crate) decision: ReviewDecision,
}

/// What the Review header counts.
///
/// Derived on every read rather than stored. A stored count is a second answer to a question the
/// rows already answer, and the two disagree the first time anything writes without updating both
/// — at which point the header is confidently wrong and nothing says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReviewSummary {
    /// Files the review currently holds as changed.
    pub(crate) changed_files: usize,
    /// How many of those the reviewer has read *at their current version*.
    pub(crate) viewed_files: usize,
    /// Comments nobody has resolved.
    pub(crate) unresolved_comments: usize,
    /// Automated findings nobody has resolved.
    pub(crate) unresolved_findings: usize,
}

/// A review and what its header says about it.
///
/// Paired rather than folded into the aggregate: the counts are a projection over two stores, and
/// an aggregate that carried them would have to be reloaded to stay honest after every write to
/// either one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewView {
    pub(crate) session: ReviewSession,
    pub(crate) summary: ReviewSummary,
}

/// What a caller is asking to mark as read, and the diff they read.
///
/// The witness is not supplied by the caller: it is derived from the review's own copy of the
/// file, so a mark can never claim to be about a version of the file the review does not hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SetFileViewedRequest {
    pub(crate) path: String,
    /// The snapshot the reviewer was looking at.
    pub(crate) expected_snapshot_fingerprint: String,
    pub(crate) viewed: bool,
}

/// Which witness stopped a decision.
///
/// One reason code crosses the boundary, because a caller's next move is the same for all three:
/// reload and look again. The distinction is kept for the reviewer-facing message 13.10 renders —
/// "this file is no longer in the review" and "this hunk moved" are different things to be told —
/// and for a log that would otherwise say only that something was stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StaleReviewWitness {
    /// The review has been reconciled against a newer diff than the caller saw.
    Snapshot,
    /// The file is no longer among the review's changed files.
    File,
    /// The file is still changed, but not in the way the caller was looking at.
    Hunk,
}

#[derive(Clone)]
pub(crate) struct ReviewApplicationService {
    repository: Arc<dyn ReviewRepository>,
    decisions: Arc<dyn ReviewDecisionRepository>,
    hunk_witnesses: Arc<dyn ReviewHunkWitnessPort>,
    clock: Arc<dyn ReviewClockPort>,
    ids: Arc<dyn ReviewIdPort>,
    feedback: Arc<dyn ReviewFeedbackPort>,
    snapshots: Arc<dyn ReviewSnapshotPort>,
    operations: Arc<dyn ReviewOperationPort>,
    logging: Arc<dyn ReviewLoggingPort>,
    evidence: Arc<dyn super::SessionEvidencePort>,
}

impl ReviewApplicationService {
    // The publisher is the eighth port rather than a builder step on purpose: a review service
    // assembled without one would compile, run, and record nothing.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        repository: Arc<dyn ReviewRepository>,
        decisions: Arc<dyn ReviewDecisionRepository>,
        hunk_witnesses: Arc<dyn ReviewHunkWitnessPort>,
        clock: Arc<dyn ReviewClockPort>,
        ids: Arc<dyn ReviewIdPort>,
        feedback: Arc<dyn ReviewFeedbackPort>,
        snapshots: Arc<dyn ReviewSnapshotPort>,
        operations: Arc<dyn ReviewOperationPort>,
        logging: Arc<dyn ReviewLoggingPort>,
        evidence: Arc<dyn super::SessionEvidencePort>,
    ) -> Self {
        Self {
            repository,
            decisions,
            hunk_witnesses,
            clock,
            ids,
            feedback,
            snapshots,
            operations,
            logging,
            evidence,
        }
    }

    /// A service that records nothing, for tests whose subject is not evidence.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_for_test_without_evidence(
        repository: Arc<dyn ReviewRepository>,
        decisions: Arc<dyn ReviewDecisionRepository>,
        hunk_witnesses: Arc<dyn ReviewHunkWitnessPort>,
        clock: Arc<dyn ReviewClockPort>,
        ids: Arc<dyn ReviewIdPort>,
        feedback: Arc<dyn ReviewFeedbackPort>,
        snapshots: Arc<dyn ReviewSnapshotPort>,
        operations: Arc<dyn ReviewOperationPort>,
        logging: Arc<dyn ReviewLoggingPort>,
    ) -> Self {
        Self::new(
            repository,
            decisions,
            hunk_witnesses,
            clock,
            ids,
            feedback,
            snapshots,
            operations,
            logging,
            Arc::new(super::NoSessionEvidence),
        )
    }

    pub(crate) fn start_action(
        &self,
        review_id: &str,
        action: ReviewAction,
    ) -> Result<String, ReviewApplicationError> {
        self.find(review_id)?;
        let operation_id = self.operations.start(review_id, action)?;
        self.logging.record(ReviewLogEvent {
            kind: "action-started",
            review_id: review_id.to_string(),
            item_count: 0,
        });
        Ok(operation_id)
    }

    pub(crate) fn open(&self, session_id: &str) -> Result<ReviewSession, ReviewApplicationError> {
        self.create_or_recover(self.snapshots.snapshot(session_id)?)
    }

    pub(crate) fn create_or_recover(
        &self,
        request: CreateReviewRequest,
    ) -> Result<ReviewSession, ReviewApplicationError> {
        if let Some(mut existing) = self
            .repository
            .find_active_by_session(&request.session_id)?
        {
            if existing.fingerprint != request.fingerprint {
                existing.reconcile_snapshot(request.fingerprint, request.files);
                existing.updated_at = self.clock.now();
                self.repository.save(&existing)?;
                self.logging.record(ReviewLogEvent {
                    kind: "anchors-reconciled",
                    review_id: existing.id.clone(),
                    item_count: existing.comments().len(),
                });
            }
            return Ok(existing);
        }
        let mut review = ReviewSession::try_new(
            self.ids.next_id("review"),
            request.session_id,
            request.workspace_id,
            request.base_revision,
            request.head_revision,
            request.fingerprint,
            request.files,
        )?;
        let now = self.clock.now();
        review.set_timestamps(now.clone(), now);
        self.repository.save(&review)?;
        self.logging.record(ReviewLogEvent {
            kind: "review-created",
            review_id: review.id.clone(),
            item_count: review.files().len(),
        });
        Ok(review)
    }

    /// The session's active review, if one already exists.
    ///
    /// Distinct from `open` in the one way that matters to a reader: `open` takes a workspace
    /// snapshot and creates or reconciles a review, which is a write. A report must not create the
    /// thing it reports on — a session with no review would acquire one by being looked at, and its
    /// change section would then describe a review that the act of reporting brought into being.
    pub(crate) fn find_active(
        &self,
        session_id: &str,
    ) -> Result<Option<ReviewSession>, ReviewApplicationError> {
        self.repository.find_active_by_session(session_id)
    }

    pub(crate) fn find(&self, review_id: &str) -> Result<ReviewSession, ReviewApplicationError> {
        self.repository
            .find(review_id)?
            .ok_or_else(|| ReviewApplicationError::NotFound(review_id.to_string()))
    }

    pub(crate) fn add_comment(
        &self,
        request: AddReviewCommentRequest,
    ) -> Result<ReviewComment, ReviewApplicationError> {
        let mut review = self.find(&request.review_id)?;
        let comment =
            ReviewComment::try_new(self.ids.next_id("comment"), request.anchor, request.body)?;
        review.add_comment(comment.clone())?;
        review.updated_at = self.clock.now();
        self.repository.save(&review)?;
        self.logging.record(ReviewLogEvent {
            kind: "comment-added",
            review_id: review.id.clone(),
            item_count: 1,
        });
        Ok(comment)
    }

    pub(crate) fn resolve_comment(
        &self,
        review_id: &str,
        comment_id: &str,
    ) -> Result<ReviewSession, ReviewApplicationError> {
        let mut review = self.find(review_id)?;
        let comment = review
            .comment_mut(comment_id)
            .ok_or_else(|| ReviewApplicationError::CommentNotFound(comment_id.to_string()))?;
        comment.resolve();
        review.updated_at = self.clock.now();
        self.repository.save(&review)?;
        self.logging.record(ReviewLogEvent {
            kind: "comment-resolved",
            review_id: review.id.clone(),
            item_count: 1,
        });
        self.logging.record(ReviewLogEvent {
            kind: "findings-projected",
            review_id: review.id.clone(),
            item_count: review.findings().len(),
        });
        Ok(review)
    }

    pub(crate) fn select_comment(
        &self,
        review_id: &str,
        comment_id: &str,
        selected: bool,
    ) -> Result<ReviewSession, ReviewApplicationError> {
        let mut review = self.find(review_id)?;
        let comment = review
            .comment_mut(comment_id)
            .ok_or_else(|| ReviewApplicationError::CommentNotFound(comment_id.to_string()))?;
        comment.set_selected(selected);
        review.updated_at = self.clock.now();
        self.repository.save(&review)?;
        Ok(review)
    }

    pub(crate) fn set_decision(
        &self,
        review_id: &str,
        decision: ReviewDecision,
    ) -> Result<ReviewSession, ReviewApplicationError> {
        let mut review = self.find(review_id)?;
        review.set_decision(decision);
        review.updated_at = self.clock.now();
        self.repository.save(&review)?;
        self.logging.record(ReviewLogEvent {
            kind: "decision-changed",
            review_id: review.id.clone(),
            item_count: 1,
        });
        // After `save` commits. A decision reported first would survive a rollback as a record of
        // something nobody decided. `Pending` publishes nothing: it is the absence of a decision.
        if let Some(decision) = review_evidence_decision(review.decision) {
            self.evidence
                .try_publish(super::SessionEvidenceSignal::ReviewDecisionRecorded {
                    session_id: review.session_id.clone(),
                    review_id: review.id.clone(),
                    decision,
                    // The snapshot fingerprint, which is what makes a decision about the current
                    // diff distinguishable from one about a diff that has since moved on.
                    witness_fingerprint: review.fingerprint.clone(),
                    occurred_at: review.updated_at.clone(),
                });
        }
        Ok(review)
    }

    /// Records a decision about one hunk, leaving the review's own decision alone.
    ///
    /// The two are independent in both directions: accepting a review does not accept its hunks,
    /// and accepting every hunk does not accept the review. Deriving either from the other was the
    /// shortcut this whole group exists to remove — a reviewer who accepted three hunks out of
    /// twenty had that rendered as an accepted review.
    pub(crate) fn set_hunk_decision(
        &self,
        review_id: &str,
        request: SetHunkDecisionRequest,
    ) -> Result<ReviewHunkDecision, ReviewApplicationError> {
        let review = self.find(review_id)?;
        // Every witness before anything is written. A refusal that had already stored something
        // would be a decision recorded against a diff the service just declared it cannot vouch
        // for.
        self.assert_witnesses(&review, &request)?;
        // The review's fingerprint rather than the request's, even though the check above proved
        // them equal. What is recorded is the diff the decision was made about, and the authority
        // on that is the review — the caller's copy is what was verified, not what is stored.
        let decision = ReviewHunkDecision::try_new(
            request.path,
            request.hunk_fingerprint,
            review.fingerprint.clone(),
            request.decision,
            self.clock.now(),
        )?;
        self.decisions.upsert_hunk_decision(&review.id, &decision)?;
        self.logging.record(ReviewLogEvent {
            kind: "hunk-decision-changed",
            review_id: review.id.clone(),
            item_count: 1,
        });
        // After the upsert commits, for the same reason the review-level decision publishes after
        // its save: a reference to a decision that then rolled back is a record of something
        // nobody decided. `Pending` publishes nothing — it is the absence of a decision.
        if let Some(value) = review_evidence_decision(decision.decision) {
            self.evidence
                .try_publish(super::SessionEvidenceSignal::ReviewHunkDecisionRecorded {
                    session_id: review.session_id.clone(),
                    review_id: review.id.clone(),
                    hunk_fingerprint: decision.hunk_fingerprint.clone(),
                    decision: value,
                    witness_fingerprint: decision.snapshot_fingerprint.clone(),
                    occurred_at: decision.decided_at.clone(),
                });
        }
        Ok(decision)
    }

    /// Records that a reviewer has read a file, or that they have not.
    ///
    /// Witnessed to the file rather than to the review, so an agent writing to one file does not
    /// clear the marks on the other eleven. A file whose own content moved becomes unviewed on its
    /// next read, because the witness stored with the mark no longer matches the file — nothing
    /// sweeps, and nothing needs to.
    pub(crate) fn set_file_viewed(
        &self,
        review_id: &str,
        request: SetFileViewedRequest,
    ) -> Result<ReviewFileViewState, ReviewApplicationError> {
        let review = self.find(review_id)?;
        if review.fingerprint != request.expected_snapshot_fingerprint {
            return Err(ReviewApplicationError::StaleWitness(
                StaleReviewWitness::Snapshot,
            ));
        }
        let file = review
            .files()
            .iter()
            .find(|file| file.path == request.path)
            .ok_or(ReviewApplicationError::StaleWitness(
                StaleReviewWitness::File,
            ))?;
        let now = self.clock.now();
        let state = ReviewFileViewState::try_new(
            request.path.clone(),
            review.fingerprint.clone(),
            file.witness(),
            request.viewed,
            // Present exactly when the file is viewed. Keeping the previous mark's time on an
            // unviewed row would leave a moment attached to something that is no longer true.
            request.viewed.then_some(now),
        )?;
        self.decisions.upsert_file_view_state(&review.id, &state)?;
        self.logging.record(ReviewLogEvent {
            kind: "file-viewed-changed",
            review_id: review.id.clone(),
            item_count: 1,
        });
        // After the upsert commits, for the same reason every other reference here does. Only a
        // file that was read is published: unviewing is a reviewer taking a claim back, and a
        // journal entry saying somebody stopped having read something is not an observation.
        if state.viewed {
            self.evidence
                .try_publish(super::SessionEvidenceSignal::ReviewFileViewedRecorded {
                    session_id: review.session_id.clone(),
                    review_id: review.id.clone(),
                    file_witness: state.file_witness.clone(),
                    witness_fingerprint: state.snapshot_fingerprint.clone(),
                    occurred_at: state
                        .viewed_at
                        .clone()
                        .unwrap_or_else(|| review.updated_at.clone()),
                });
        }
        Ok(state)
    }

    /// A review together with the counts its header shows.
    ///
    /// The viewed count is the interesting one. It walks the review's *current* files and asks
    /// whether each has a mark made against the version that is there now — so a file that changed
    /// since it was read counts as unread without anything having swept the store. That is the
    /// reset, and deriving it rather than writing it is why it is also correct for changes nobody
    /// published.
    pub(crate) fn view(
        &self,
        session: ReviewSession,
    ) -> Result<ReviewView, ReviewApplicationError> {
        let states = self.decisions.list_file_view_states(&session.id)?;
        let viewed_files = session
            .files()
            .iter()
            .filter(|file| {
                let witness = file.witness();
                states.iter().any(|state| {
                    state.viewed && state.path == file.path && state.file_witness == witness
                })
            })
            .count();
        let summary = ReviewSummary {
            changed_files: session.files().len(),
            viewed_files,
            unresolved_comments: session
                .comments()
                .iter()
                .filter(|comment| comment.status == ReviewCommentStatus::Active)
                .count(),
            unresolved_findings: session
                .findings()
                .iter()
                .filter(|finding| !finding.resolved)
                .count(),
        };
        Ok(ReviewView { session, summary })
    }

    /// Every file view state this review holds, whether or not it still applies.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the summary reads through the view; the raw list is what 13.10's per-file \
                 controls render"
        )
    )]
    pub(crate) fn file_view_states(
        &self,
        review_id: &str,
    ) -> Result<Vec<ReviewFileViewState>, ReviewApplicationError> {
        self.find(review_id)?;
        self.decisions.list_file_view_states(review_id)
    }

    /// Whether the review, the file, and the hunk are all still what the caller saw.
    ///
    /// In that order, cheapest first, and each is a different way of being out of date. The
    /// snapshot moved, so everything the caller saw is suspect; the file is no longer part of the
    /// review, so the hunk cannot be either; the file is still changing but not there any more.
    /// Collapsing them into one check would answer the reviewer with the least specific of the
    /// three.
    fn assert_witnesses(
        &self,
        review: &ReviewSession,
        request: &SetHunkDecisionRequest,
    ) -> Result<(), ReviewApplicationError> {
        if review.fingerprint != request.expected_snapshot_fingerprint {
            return Err(ReviewApplicationError::StaleWitness(
                StaleReviewWitness::Snapshot,
            ));
        }
        if !review.files().iter().any(|file| file.path == request.path) {
            return Err(ReviewApplicationError::StaleWitness(
                StaleReviewWitness::File,
            ));
        }
        // Last because it is the only one that reads the workspace. The two above are answered
        // from the review already in memory, so a stale snapshot never costs a diff read.
        let current = self.hunk_witnesses.hunk_fingerprints(
            &review.session_id,
            &request.path,
            &review.fingerprint,
        )?;
        if !current
            .iter()
            .any(|fingerprint| fingerprint == &request.hunk_fingerprint)
        {
            return Err(ReviewApplicationError::StaleWitness(
                StaleReviewWitness::Hunk,
            ));
        }
        Ok(())
    }

    /// Every hunk decision this review holds.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the write path is live; reading decisions back is what 13.10's per-hunk \
                 controls render"
        )
    )]
    pub(crate) fn hunk_decisions(
        &self,
        review_id: &str,
    ) -> Result<Vec<ReviewHunkDecision>, ReviewApplicationError> {
        self.find(review_id)?;
        self.decisions.list_hunk_decisions(review_id)
    }

    pub(crate) fn add_findings(
        &self,
        review_id: &str,
        findings: Vec<ReviewFinding>,
    ) -> Result<ReviewSession, ReviewApplicationError> {
        let mut review = self.find(review_id)?;
        for finding in findings {
            review.add_finding(finding)?;
        }
        review.updated_at = self.clock.now();
        self.repository.save(&review)?;
        Ok(review)
    }

    pub(crate) fn project_action_findings(
        &self,
        review_id: &str,
        action: ReviewAction,
        operation_id: &str,
        inputs: Vec<ReviewActionFindingInput>,
    ) -> Result<ReviewSession, ReviewApplicationError> {
        if inputs.len() > 100 {
            return Err(ReviewApplicationError::InvalidActionOutput);
        }
        let source = match action {
            ReviewAction::ReviewAgent => "review-agent",
            ReviewAction::Tests => "tests",
            ReviewAction::Security => "security",
        };
        let findings = inputs
            .into_iter()
            .map(|input| {
                let severity = match input.severity.as_str() {
                    "info" => crate::contexts::sessions::domain::ReviewFindingSeverity::Info,
                    "warning" => crate::contexts::sessions::domain::ReviewFindingSeverity::Warning,
                    "error" => crate::contexts::sessions::domain::ReviewFindingSeverity::Error,
                    _ => return Err(ReviewApplicationError::InvalidActionOutput),
                };
                ReviewFinding::try_new(
                    self.ids.next_id("finding"),
                    source.to_string(),
                    input.title,
                    severity,
                    input.anchor,
                    operation_id.to_string(),
                )
                .map_err(ReviewApplicationError::Domain)
            })
            .collect::<Result<Vec<_>, _>>()?;
        // Errors are what an automated check failing means here; anything less severe is an
        // observation it made, not a verdict against it.
        let failed = findings
            .iter()
            .filter(|finding| {
                finding.severity == crate::contexts::sessions::domain::ReviewFindingSeverity::Error
            })
            .count() as u32;
        let total = findings.len() as u32;
        let review = self.add_findings(review_id, findings)?;
        // After `add_findings` commits. A check reported before its findings persist would claim
        // an outcome nobody can look up.
        self.evidence
            .try_publish(super::SessionEvidenceSignal::VerificationCompleted {
                session_id: review.session_id.clone(),
                run_id: None,
                // The operation is the check's own run. It is what makes a re-run of the same
                // check a second observation and a replayed callback the same one.
                verification_run_id: operation_id.to_string(),
                name: source.to_string(),
                outcome: if failed > 0 {
                    super::SessionVerificationOutcome::Failed
                } else {
                    super::SessionVerificationOutcome::Passed
                },
                // Counts, never the findings. A finding's title quotes the code it is about, and
                // the finding store already holds it behind rules that can render it safely.
                passed_count: Some(total.saturating_sub(failed)),
                failed_count: Some(failed),
                occurred_at: review.updated_at.clone(),
            });
        Ok(review)
    }

    pub(crate) fn prepare_feedback(
        &self,
        review_id: &str,
        acknowledge_stale: bool,
    ) -> Result<PreparedReviewFeedback, ReviewApplicationError> {
        let review = self.find(review_id)?;
        let comments = review
            .comments()
            .iter()
            .filter(|comment| comment.selected)
            .map(|comment| ReviewFeedbackComment {
                comment_id: comment.id.clone(),
                file_path: comment.anchor.file_path.clone(),
                side: comment.anchor.side.clone(),
                start_line: comment.anchor.start_line,
                end_line: comment.anchor.end_line,
                hunk_fingerprint: comment.anchor.hunk_fingerprint.clone(),
                stale: comment.anchor.state
                    == crate::contexts::sessions::domain::ReviewAnchorState::Stale,
                body: comment.body.clone(),
            })
            .collect::<Vec<_>>();
        if comments.is_empty() {
            return Err(ReviewApplicationError::NoSelectedComments);
        }
        if !acknowledge_stale && comments.iter().any(|comment| comment.stale) {
            return Err(ReviewApplicationError::StaleAcknowledgementRequired);
        }
        Ok(PreparedReviewFeedback {
            review_id: review.id.clone(),
            decision: review.decision,
            comments,
            prepared_at: self.clock.now(),
        })
    }

    pub(crate) fn send_feedback(
        &self,
        review_id: &str,
        acknowledge_stale: bool,
    ) -> Result<String, ReviewApplicationError> {
        let review = self.find(review_id)?;
        let feedback = self.prepare_feedback(review_id, acknowledge_stale)?;
        let message_id = self.feedback.send(&review.session_id, &feedback)?;
        self.logging.record(ReviewLogEvent {
            kind: "feedback-sent",
            review_id: review.id,
            item_count: feedback.comments.len(),
        });
        Ok(message_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReviewApplicationError {
    Domain(ReviewDomainError),
    Repository(String),
    Feedback(String),
    NotFound(String),
    CommentNotFound(String),
    NoSelectedComments,
    StaleAcknowledgementRequired,
    /// The diff the caller decided about is not the diff the review holds.
    StaleWitness(StaleReviewWitness),
    InvalidActionOutput,
}

impl From<ReviewDomainError> for ReviewApplicationError {
    fn from(value: ReviewDomainError) -> Self {
        Self::Domain(value)
    }
}

impl fmt::Display for ReviewApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ReviewApplicationError {}

/// `Pending` is not a decision. Recording it would put "nobody has decided yet" into a journal of
/// things that happened, and a reader counting decisions would count it.
fn review_evidence_decision(decision: ReviewDecision) -> Option<super::SessionReviewDecision> {
    match decision {
        ReviewDecision::Accepted => Some(super::SessionReviewDecision::Accepted),
        ReviewDecision::ChangesRequested => Some(super::SessionReviewDecision::ChangesRequested),
        ReviewDecision::Pending => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::SessionReviewDecision;
    use super::*;
    use crate::contexts::sessions::domain::{ReviewAnchorState, ReviewFindingSeverity};
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryRepository(Mutex<Vec<ReviewSession>>);
    impl ReviewRepository for MemoryRepository {
        fn find_active_by_session(
            &self,
            session_id: &str,
        ) -> Result<Option<ReviewSession>, ReviewApplicationError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .find(|review| {
                    review.session_id == session_id
                        && review.status == crate::contexts::sessions::domain::ReviewStatus::Active
                })
                .cloned())
        }
        fn find(&self, review_id: &str) -> Result<Option<ReviewSession>, ReviewApplicationError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .find(|review| review.id == review_id)
                .cloned())
        }
        fn save(&self, review: &ReviewSession) -> Result<(), ReviewApplicationError> {
            let mut rows = self.0.lock().unwrap();
            rows.retain(|row| row.id != review.id);
            rows.push(review.clone());
            Ok(())
        }
    }
    /// Hunk decisions, keyed the way the table keys them.
    #[derive(Default)]
    struct MemoryDecisions {
        rows: Mutex<Vec<(String, ReviewHunkDecision)>>,
        views: Mutex<Vec<(String, ReviewFileViewState)>>,
        /// When set, every write fails. Used to prove nothing is published for a write that did
        /// not land.
        refuse: Mutex<bool>,
    }
    impl ReviewDecisionRepository for MemoryDecisions {
        fn upsert_file_view_state(
            &self,
            review_id: &str,
            state: &ReviewFileViewState,
        ) -> Result<(), ReviewApplicationError> {
            if *self.refuse.lock().unwrap() {
                return Err(ReviewApplicationError::Repository("refused".into()));
            }
            let mut rows = self.views.lock().unwrap();
            rows.retain(|(id, row)| id != review_id || row.path != state.path);
            rows.push((review_id.to_string(), state.clone()));
            Ok(())
        }
        fn list_file_view_states(
            &self,
            review_id: &str,
        ) -> Result<Vec<ReviewFileViewState>, ReviewApplicationError> {
            let mut rows: Vec<ReviewFileViewState> = self
                .views
                .lock()
                .unwrap()
                .iter()
                .filter(|(id, _)| id == review_id)
                .map(|(_, row)| row.clone())
                .collect();
            rows.sort_by(|left, right| left.path.cmp(&right.path));
            Ok(rows)
        }
        fn upsert_hunk_decision(
            &self,
            review_id: &str,
            decision: &ReviewHunkDecision,
        ) -> Result<(), ReviewApplicationError> {
            if *self.refuse.lock().unwrap() {
                return Err(ReviewApplicationError::Repository("refused".into()));
            }
            let mut rows = self.rows.lock().unwrap();
            rows.retain(|(id, row)| {
                id != review_id
                    || row.path != decision.path
                    || row.hunk_fingerprint != decision.hunk_fingerprint
            });
            rows.push((review_id.to_string(), decision.clone()));
            Ok(())
        }
        fn list_hunk_decisions(
            &self,
            review_id: &str,
        ) -> Result<Vec<ReviewHunkDecision>, ReviewApplicationError> {
            let mut rows: Vec<ReviewHunkDecision> = self
                .rows
                .lock()
                .unwrap()
                .iter()
                .filter(|(id, _)| id == review_id)
                .map(|(_, row)| row.clone())
                .collect();
            rows.sort_by(|left, right| {
                (&left.path, &left.hunk_fingerprint).cmp(&(&right.path, &right.hunk_fingerprint))
            });
            Ok(rows)
        }
    }

    /// The hunks the workspace would report, and whether it can be reached at all.
    struct MemoryWitnesses {
        fingerprints: Mutex<Vec<String>>,
        /// When set, the workspace read fails. A witness check that cannot be answered must not
        /// read as a witness that passed.
        unreachable: Mutex<bool>,
    }
    impl Default for MemoryWitnesses {
        fn default() -> Self {
            Self {
                fingerprints: Mutex::new(vec!["hunk-1".into(), "hunk-2".into()]),
                unreachable: Mutex::new(false),
            }
        }
    }
    impl ReviewHunkWitnessPort for MemoryWitnesses {
        fn hunk_fingerprints(
            &self,
            _session_id: &str,
            _path: &str,
            _expected_snapshot: &str,
        ) -> Result<Vec<String>, ReviewApplicationError> {
            if *self.unreachable.lock().unwrap() {
                return Err(ReviewApplicationError::Repository("unreachable".into()));
            }
            Ok(self.fingerprints.lock().unwrap().clone())
        }
    }

    #[derive(Default)]
    struct CapturingEvidence(Mutex<Vec<super::super::SessionEvidenceSignal>>);
    impl super::super::SessionEvidencePort for CapturingEvidence {
        fn try_publish(&self, signal: super::super::SessionEvidenceSignal) {
            self.0.lock().unwrap().push(signal);
        }
    }

    struct Fixed;
    impl ReviewClockPort for Fixed {
        fn now(&self) -> String {
            "2026-08-16T00:00:00Z".into()
        }
    }
    impl ReviewIdPort for Fixed {
        fn next_id(&self, kind: &'static str) -> String {
            format!("{kind}-1")
        }
    }
    impl ReviewFeedbackPort for Fixed {
        fn send(
            &self,
            _: &str,
            _: &PreparedReviewFeedback,
        ) -> Result<String, ReviewApplicationError> {
            Ok("message-1".into())
        }
    }
    impl ReviewSnapshotPort for Fixed {
        fn snapshot(
            &self,
            session_id: &str,
        ) -> Result<CreateReviewRequest, ReviewApplicationError> {
            Ok(CreateReviewRequest {
                session_id: session_id.into(),
                workspace_id: "workspace-1".into(),
                base_revision: None,
                head_revision: None,
                fingerprint: "snapshot".into(),
                files: vec![file()],
            })
        }
    }
    impl ReviewOperationPort for Fixed {
        fn start(&self, _: &str, _: ReviewAction) -> Result<String, ReviewApplicationError> {
            Ok("operation-1".into())
        }
    }
    #[derive(Default)]
    struct CapturingLog(Mutex<Vec<ReviewLogEvent>>);
    impl ReviewLoggingPort for CapturingLog {
        fn record(&self, event: ReviewLogEvent) {
            self.0.lock().unwrap().push(event);
        }
    }

    fn service() -> ReviewApplicationService {
        ReviewApplicationService::new_for_test_without_evidence(
            Arc::new(MemoryRepository::default()),
            Arc::new(MemoryDecisions::default()),
            Arc::new(MemoryWitnesses::default()),
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(CapturingLog::default()),
        )
    }

    /// A service whose decision store, workspace witnesses, and publisher the caller can inspect.
    fn reviewing() -> (
        ReviewApplicationService,
        Arc<MemoryDecisions>,
        Arc<MemoryWitnesses>,
        Arc<CapturingEvidence>,
    ) {
        let decisions = Arc::new(MemoryDecisions::default());
        let witnesses = Arc::new(MemoryWitnesses::default());
        let evidence = Arc::new(CapturingEvidence::default());
        let service = ReviewApplicationService::new(
            Arc::new(MemoryRepository::default()),
            decisions.clone(),
            witnesses.clone(),
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(CapturingLog::default()),
            evidence.clone(),
        );
        (service, decisions, witnesses, evidence)
    }

    fn opened(service: &ReviewApplicationService) -> ReviewSession {
        service.open("session-1").unwrap()
    }

    /// A request that agrees with the fixture review, so a case that is not about staleness is
    /// not accidentally about staleness.
    fn mark(path: &str, hunk: &str, decision: ReviewDecision) -> SetHunkDecisionRequest {
        SetHunkDecisionRequest {
            path: path.into(),
            hunk_fingerprint: hunk.into(),
            expected_snapshot_fingerprint: "snapshot".into(),
            decision,
        }
    }

    fn published_hunk_decisions(
        evidence: &CapturingEvidence,
    ) -> Vec<(String, SessionReviewDecision, String)> {
        evidence
            .0
            .lock()
            .unwrap()
            .iter()
            .filter_map(|signal| match signal {
                super::super::SessionEvidenceSignal::ReviewHunkDecisionRecorded {
                    hunk_fingerprint,
                    decision,
                    witness_fingerprint,
                    ..
                } => Some((
                    hunk_fingerprint.clone(),
                    *decision,
                    witness_fingerprint.clone(),
                )),
                _ => None,
            })
            .collect()
    }
    fn file() -> ReviewFile {
        ReviewFile::try_new("src/a.rs".into(), None, "modified".into(), None, None).unwrap()
    }
    fn anchor() -> ReviewAnchor {
        ReviewAnchor::try_new(
            "src/a.rs".into(),
            "new".into(),
            2,
            2,
            "hunk".into(),
            "context".into(),
        )
        .unwrap()
    }

    #[test]
    fn creates_recovers_and_mutates_review() {
        let service = service();
        let request = CreateReviewRequest {
            session_id: "session-1".into(),
            workspace_id: "workspace-1".into(),
            base_revision: None,
            head_revision: None,
            fingerprint: "snapshot".into(),
            files: vec![file()],
        };
        let created = service.create_or_recover(request.clone()).unwrap();
        assert_eq!(service.create_or_recover(request).unwrap().id, created.id);
        let comment = service
            .add_comment(AddReviewCommentRequest {
                review_id: created.id.clone(),
                anchor: anchor(),
                body: "Please fix".into(),
            })
            .unwrap();
        assert_eq!(
            service
                .resolve_comment(&created.id, &comment.id)
                .unwrap()
                .comments()[0]
                .status,
            crate::contexts::sessions::domain::ReviewCommentStatus::Resolved
        );
        assert_eq!(
            service.send_feedback(&created.id, false).unwrap(),
            "message-1"
        );
    }

    #[test]
    fn requires_stale_acknowledgement_and_projects_findings() {
        let service = service();
        let review = service
            .create_or_recover(CreateReviewRequest {
                session_id: "session-1".into(),
                workspace_id: "workspace-1".into(),
                base_revision: None,
                head_revision: None,
                fingerprint: "snapshot".into(),
                files: vec![file()],
            })
            .unwrap();
        let mut stale = anchor();
        stale.mark_stale();
        assert_eq!(stale.state, ReviewAnchorState::Stale);
        service
            .add_comment(AddReviewCommentRequest {
                review_id: review.id.clone(),
                anchor: stale,
                body: "Outdated".into(),
            })
            .unwrap();
        assert_eq!(
            service.prepare_feedback(&review.id, false),
            Err(ReviewApplicationError::StaleAcknowledgementRequired)
        );
        let finding = ReviewFinding::try_new(
            "finding-1".into(),
            "tests".into(),
            "Failure".into(),
            ReviewFindingSeverity::Error,
            None,
            "operation-1".into(),
        )
        .unwrap();
        assert_eq!(
            service
                .add_findings(&review.id, vec![finding])
                .unwrap()
                .findings()
                .len(),
            1
        );
        assert_eq!(
            service.project_action_findings(
                &review.id,
                ReviewAction::Security,
                "operation-2",
                vec![ReviewActionFindingInput {
                    title: "Unsafe".into(),
                    severity: "critical".into(),
                    anchor: None
                }],
            ),
            Err(ReviewApplicationError::InvalidActionOutput)
        );
    }

    // The whole reason this group exists. A reviewer who accepted three hunks out of twenty had
    // that rendered as an accepted review, and a reviewer who accepted the review had every hunk
    // reported as accepted. Neither inference is available once the two are stored apart.
    #[test]
    fn a_review_decision_and_a_hunk_decision_do_not_move_each_other() {
        let (service, _decisions, _witnesses, _evidence) = reviewing();
        let review = opened(&service);

        service
            .set_hunk_decision(
                &review.id,
                mark("src/a.rs", "hunk-1", ReviewDecision::Accepted),
            )
            .unwrap();
        assert_eq!(
            service.find(&review.id).unwrap().decision,
            ReviewDecision::Pending,
            "accepting a hunk decided the review"
        );

        service
            .set_decision(&review.id, ReviewDecision::ChangesRequested)
            .unwrap();
        let recorded = service.hunk_decisions(&review.id).unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(
            recorded[0].decision,
            ReviewDecision::Accepted,
            "the review's decision overwrote the hunk's"
        );
    }

    #[test]
    fn a_second_decision_for_the_same_hunk_replaces_the_first() {
        let (service, _decisions, _witnesses, _evidence) = reviewing();
        let review = opened(&service);

        service
            .set_hunk_decision(
                &review.id,
                mark("src/a.rs", "hunk-1", ReviewDecision::Accepted),
            )
            .unwrap();
        service
            .set_hunk_decision(
                &review.id,
                mark("src/a.rs", "hunk-1", ReviewDecision::ChangesRequested),
            )
            .unwrap();

        // One answer per hunk. Two rows would leave a reader with two decisions and nothing saying
        // which one the reviewer meant.
        let recorded = service.hunk_decisions(&review.id).unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].decision, ReviewDecision::ChangesRequested);
    }

    #[test]
    fn a_hunk_decision_is_witnessed_to_the_review_s_own_snapshot() {
        let (service, _decisions, _witnesses, evidence) = reviewing();
        let review = opened(&service);

        let recorded = service
            .set_hunk_decision(
                &review.id,
                mark("src/a.rs", "hunk-1", ReviewDecision::Accepted),
            )
            .unwrap();

        // Taken from the review rather than from the caller, so a stored decision is never a claim
        // about a diff that was not the one under review. 13.3 adds the caller's expectation and
        // refuses when the two disagree; until then there is nothing for them to disagree about.
        assert_eq!(recorded.snapshot_fingerprint, review.fingerprint);
        assert_eq!(
            published_hunk_decisions(&evidence),
            vec![(
                "hunk-1".to_string(),
                SessionReviewDecision::Accepted,
                review.fingerprint.clone()
            )]
        );
    }

    #[test]
    fn a_pending_hunk_is_recorded_and_published_as_nothing() {
        let (service, _decisions, _witnesses, evidence) = reviewing();
        let review = opened(&service);

        service
            .set_hunk_decision(
                &review.id,
                mark("src/a.rs", "hunk-1", ReviewDecision::Pending),
            )
            .unwrap();

        // Stored, because clearing a decision is something a reviewer does. Not published, because
        // "nobody has decided yet" is the absence of a decision and a journal of things that
        // happened has no entry for it.
        assert_eq!(service.hunk_decisions(&review.id).unwrap().len(), 1);
        assert_eq!(published_hunk_decisions(&evidence), vec![]);
    }

    #[test]
    fn a_write_that_did_not_land_publishes_nothing() {
        let (service, decisions, _witnesses, evidence) = reviewing();
        let review = opened(&service);
        *decisions.refuse.lock().unwrap() = true;

        assert!(service
            .set_hunk_decision(
                &review.id,
                mark("src/a.rs", "hunk-1", ReviewDecision::Accepted)
            )
            .is_err());

        // A reference to a decision that rolled back is a record of something nobody decided, and
        // it is indistinguishable from one that stuck.
        assert_eq!(published_hunk_decisions(&evidence), vec![]);
    }

    #[test]
    fn a_hunk_decision_for_a_review_that_does_not_exist_is_refused() {
        let (service, decisions, _witnesses, evidence) = reviewing();

        assert!(matches!(
            service.set_hunk_decision(
                "review-missing",
                mark("src/a.rs", "hunk-1", ReviewDecision::Accepted)
            ),
            Err(ReviewApplicationError::NotFound(_))
        ));
        assert!(decisions.rows.lock().unwrap().is_empty());
        assert_eq!(published_hunk_decisions(&evidence), vec![]);
    }

    #[test]
    fn a_hunk_decision_refuses_a_path_that_leaves_the_workspace() {
        let (service, decisions, _witnesses, _evidence) = reviewing();
        let review = opened(&service);

        assert!(service
            .set_hunk_decision(
                &review.id,
                mark("../outside.rs", "hunk-1", ReviewDecision::Accepted)
            )
            .is_err());
        assert!(decisions.rows.lock().unwrap().is_empty());
    }

    // Three ways to be looking at a diff that no longer exists, and each is refused before
    // anything is written. A decision recorded against a diff the service cannot vouch for is
    // indistinguishable from one recorded against the diff the reviewer actually read.
    #[test]
    fn a_decision_about_an_older_snapshot_is_refused_without_writing() {
        let (service, decisions, _witnesses, evidence) = reviewing();
        let review = opened(&service);

        let result = service.set_hunk_decision(
            &review.id,
            SetHunkDecisionRequest {
                path: "src/a.rs".into(),
                hunk_fingerprint: "hunk-1".into(),
                expected_snapshot_fingerprint: "an-older-snapshot".into(),
                decision: ReviewDecision::Accepted,
            },
        );

        assert_eq!(
            result,
            Err(ReviewApplicationError::StaleWitness(
                StaleReviewWitness::Snapshot
            ))
        );
        assert!(decisions.rows.lock().unwrap().is_empty());
        assert_eq!(published_hunk_decisions(&evidence), vec![]);
    }

    #[test]
    fn a_decision_about_a_file_the_review_does_not_hold_is_refused_without_writing() {
        let (service, decisions, _witnesses, evidence) = reviewing();
        let review = opened(&service);

        let result = service.set_hunk_decision(
            &review.id,
            mark("src/gone.rs", "hunk-1", ReviewDecision::Accepted),
        );

        assert_eq!(
            result,
            Err(ReviewApplicationError::StaleWitness(
                StaleReviewWitness::File
            ))
        );
        assert!(decisions.rows.lock().unwrap().is_empty());
        assert_eq!(published_hunk_decisions(&evidence), vec![]);
    }

    #[test]
    fn a_decision_about_a_hunk_the_diff_no_longer_has_is_refused_without_writing() {
        let (service, decisions, witnesses, evidence) = reviewing();
        let review = opened(&service);
        // The file is still changed; the change is just not the one the reviewer was reading.
        *witnesses.fingerprints.lock().unwrap() = vec!["hunk-rewritten".into()];

        let result = service.set_hunk_decision(
            &review.id,
            mark("src/a.rs", "hunk-1", ReviewDecision::Accepted),
        );

        assert_eq!(
            result,
            Err(ReviewApplicationError::StaleWitness(
                StaleReviewWitness::Hunk
            ))
        );
        assert!(decisions.rows.lock().unwrap().is_empty());
        assert_eq!(published_hunk_decisions(&evidence), vec![]);
    }

    // Cheapest first, and the ordering is worth holding: a reviewer whose snapshot has moved on
    // would otherwise pay a workspace diff read to be told something the review already knew.
    #[test]
    fn a_stale_snapshot_is_answered_without_reading_the_workspace() {
        let (service, _decisions, witnesses, _evidence) = reviewing();
        let review = opened(&service);
        *witnesses.unreachable.lock().unwrap() = true;

        let result = service.set_hunk_decision(
            &review.id,
            SetHunkDecisionRequest {
                path: "src/a.rs".into(),
                hunk_fingerprint: "hunk-1".into(),
                expected_snapshot_fingerprint: "an-older-snapshot".into(),
                decision: ReviewDecision::Accepted,
            },
        );

        // The workspace was unreachable throughout, so reaching it would have produced a
        // repository error rather than this one.
        assert_eq!(
            result,
            Err(ReviewApplicationError::StaleWitness(
                StaleReviewWitness::Snapshot
            ))
        );
    }

    #[test]
    fn a_witness_that_cannot_be_checked_is_not_a_witness_that_passed() {
        let (service, decisions, witnesses, _evidence) = reviewing();
        let review = opened(&service);
        *witnesses.unreachable.lock().unwrap() = true;

        // Treating an unanswerable check as a pass would record decisions about a diff nobody
        // could read — which is exactly the state the check exists to refuse.
        assert!(matches!(
            service.set_hunk_decision(
                &review.id,
                mark("src/a.rs", "hunk-1", ReviewDecision::Accepted)
            ),
            Err(ReviewApplicationError::Repository(_))
        ));
        assert!(decisions.rows.lock().unwrap().is_empty());
    }

    #[test]
    fn a_refused_decision_leaves_the_one_already_recorded_alone() {
        let (service, _decisions, witnesses, _evidence) = reviewing();
        let review = opened(&service);
        service
            .set_hunk_decision(
                &review.id,
                mark("src/a.rs", "hunk-1", ReviewDecision::Accepted),
            )
            .unwrap();

        *witnesses.fingerprints.lock().unwrap() = vec!["hunk-rewritten".into()];
        assert!(service
            .set_hunk_decision(
                &review.id,
                mark("src/a.rs", "hunk-1", ReviewDecision::ChangesRequested)
            )
            .is_err());

        // The prior decision survives. A refusal that cleared it would lose a reviewer's work to
        // an edit somebody else made.
        let recorded = service.hunk_decisions(&review.id).unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].decision, ReviewDecision::Accepted);
    }

    fn read(path: &str, viewed: bool) -> SetFileViewedRequest {
        SetFileViewedRequest {
            path: path.into(),
            expected_snapshot_fingerprint: "snapshot".into(),
            viewed,
        }
    }

    fn published_file_views(evidence: &CapturingEvidence) -> Vec<String> {
        evidence
            .0
            .lock()
            .unwrap()
            .iter()
            .filter_map(|signal| match signal {
                super::super::SessionEvidenceSignal::ReviewFileViewedRecorded {
                    file_witness,
                    ..
                } => Some(file_witness.clone()),
                _ => None,
            })
            .collect()
    }

    // The behaviour the whole per-file witness exists for. A review snapshot covers every changed
    // file, so a mark witnessed to it would be cleared by an agent writing to a different file —
    // and a progress count that resets on unrelated work is a count nobody can act on.
    #[test]
    fn a_viewed_mark_is_witnessed_to_its_own_file_not_to_the_review() {
        let (service, _decisions, _witnesses, _evidence) = reviewing();
        let review = opened(&service);

        let state = service
            .set_file_viewed(&review.id, read("src/a.rs", true))
            .unwrap();

        let file = review
            .files()
            .iter()
            .find(|file| file.path == "src/a.rs")
            .expect("the fixture review holds this file");
        assert_eq!(state.file_witness, file.witness());
        assert_ne!(state.file_witness, review.fingerprint);
    }

    #[test]
    fn a_file_that_changed_no_longer_matches_the_mark_that_was_made_about_it() {
        let unchanged =
            ReviewFile::try_new("src/a.rs".into(), None, "modified".into(), None, None).unwrap();
        let rewritten = ReviewFile::try_new(
            "src/a.rs".into(),
            None,
            "modified".into(),
            None,
            Some("a-new-blob".into()),
        )
        .unwrap();

        // Nothing sweeps and nothing needs to: the mark stops applying because the witness stored
        // with it stops matching the file.
        assert_ne!(unchanged.witness(), rewritten.witness());
        // And a file nobody touched keeps its mark, which is the half that makes the count usable.
        let same =
            ReviewFile::try_new("src/a.rs".into(), None, "modified".into(), None, None).unwrap();
        assert_eq!(unchanged.witness(), same.witness());
    }

    #[test]
    fn a_file_that_was_deleted_is_not_the_file_that_was_read() {
        let modified = ReviewFile::try_new(
            "src/a.rs".into(),
            None,
            "modified".into(),
            Some("before".into()),
            Some("after".into()),
        )
        .unwrap();
        let deleted = ReviewFile::try_new(
            "src/a.rs".into(),
            None,
            "deleted".into(),
            Some("before".into()),
            Some("after".into()),
        )
        .unwrap();

        // Same hashes, different change. A witness made only of the content hash would call these
        // the same file and leave a deleted file marked as read.
        assert_ne!(modified.witness(), deleted.witness());
    }

    #[test]
    fn unmarking_a_file_leaves_no_moment_at_which_it_was_read() {
        let (service, _decisions, _witnesses, evidence) = reviewing();
        let review = opened(&service);

        let viewed = service
            .set_file_viewed(&review.id, read("src/a.rs", true))
            .unwrap();
        assert!(viewed.viewed_at.is_some());

        let unviewed = service
            .set_file_viewed(&review.id, read("src/a.rs", false))
            .unwrap();
        assert_eq!(unviewed.viewed_at, None);

        // One publication, not two. Reading a file is an observation; deciding you had not read it
        // is a reviewer taking a claim back, and a journal has no entry for that.
        assert_eq!(published_file_views(&evidence).len(), 1);
    }

    #[test]
    fn a_mark_replaces_the_previous_one_for_the_same_file() {
        let (service, _decisions, _witnesses, _evidence) = reviewing();
        let review = opened(&service);

        service
            .set_file_viewed(&review.id, read("src/a.rs", true))
            .unwrap();
        service
            .set_file_viewed(&review.id, read("src/a.rs", false))
            .unwrap();

        let states = service.file_view_states(&review.id).unwrap();
        assert_eq!(states.len(), 1);
        assert!(!states[0].viewed);
    }

    #[test]
    fn a_mark_about_an_older_snapshot_or_an_absent_file_is_refused_without_writing() {
        let (service, decisions, _witnesses, evidence) = reviewing();
        let review = opened(&service);

        assert_eq!(
            service.set_file_viewed(
                &review.id,
                SetFileViewedRequest {
                    path: "src/a.rs".into(),
                    expected_snapshot_fingerprint: "an-older-snapshot".into(),
                    viewed: true,
                }
            ),
            Err(ReviewApplicationError::StaleWitness(
                StaleReviewWitness::Snapshot
            ))
        );
        assert_eq!(
            service.set_file_viewed(&review.id, read("src/gone.rs", true)),
            Err(ReviewApplicationError::StaleWitness(
                StaleReviewWitness::File
            ))
        );
        assert!(decisions.views.lock().unwrap().is_empty());
        assert_eq!(published_file_views(&evidence), Vec::<String>::new());
    }

    #[test]
    fn a_mark_that_did_not_land_publishes_nothing() {
        let (service, decisions, _witnesses, evidence) = reviewing();
        let review = opened(&service);
        *decisions.refuse.lock().unwrap() = true;

        assert!(service
            .set_file_viewed(&review.id, read("src/a.rs", true))
            .is_err());
        assert_eq!(published_file_views(&evidence), Vec::<String>::new());
    }

    #[test]
    fn reading_a_file_is_not_deciding_about_it() {
        let (service, _decisions, _witnesses, _evidence) = reviewing();
        let review = opened(&service);

        service
            .set_file_viewed(&review.id, read("src/a.rs", true))
            .unwrap();

        // Neither the review's decision nor any hunk's. A surface that conflated them would report
        // a reviewer who scrolled through a diff as having approved it.
        assert_eq!(
            service.find(&review.id).unwrap().decision,
            ReviewDecision::Pending
        );
        assert!(service.hunk_decisions(&review.id).unwrap().is_empty());
    }

    #[test]
    fn the_summary_counts_a_file_as_read_only_while_it_is_the_file_that_was_read() {
        let (service, decisions, _witnesses, _evidence) = reviewing();
        let review = opened(&service);
        service
            .set_file_viewed(&review.id, read("src/a.rs", true))
            .unwrap();

        assert_eq!(
            service.view(review.clone()).unwrap().summary.viewed_files,
            1
        );

        // The same path, marked against a version of the file that is not the one the review
        // holds. Nothing swept the store; the mark simply stops matching.
        decisions
            .upsert_file_view_state(
                &review.id,
                &ReviewFileViewState::try_new(
                    "src/a.rs".into(),
                    review.fingerprint.clone(),
                    "a-witness-from-an-older-version".into(),
                    true,
                    Some("2026-08-27T00:00:00Z".into()),
                )
                .unwrap(),
            )
            .unwrap();

        assert_eq!(service.view(review).unwrap().summary.viewed_files, 0);
    }

    #[test]
    fn the_summary_counts_the_review_s_current_files_not_the_marks_it_holds() {
        let (service, decisions, _witnesses, _evidence) = reviewing();
        let review = opened(&service);
        // A mark for a file that is no longer in the review — the reviewer read it, then the agent
        // reverted it. Counting marks instead of files would report 1 of 1 read on a review whose
        // one changed file nobody has opened.
        decisions
            .upsert_file_view_state(
                &review.id,
                &ReviewFileViewState::try_new(
                    "src/gone.rs".into(),
                    review.fingerprint.clone(),
                    "witness".into(),
                    true,
                    Some("2026-08-27T00:00:00Z".into()),
                )
                .unwrap(),
            )
            .unwrap();

        let summary = service.view(review).unwrap().summary;
        assert_eq!(summary.changed_files, 1);
        assert_eq!(summary.viewed_files, 0);
    }

    #[test]
    fn the_summary_counts_what_is_unresolved_rather_than_what_exists() {
        let (service, _decisions, _witnesses, _evidence) = reviewing();
        let review = opened(&service);
        let comment = service
            .add_comment(AddReviewCommentRequest {
                review_id: review.id.clone(),
                anchor: anchor(),
                body: "Please fix".into(),
            })
            .unwrap();

        assert_eq!(
            service
                .view(service.find(&review.id).unwrap())
                .unwrap()
                .summary
                .unresolved_comments,
            1
        );

        let resolved = service.resolve_comment(&review.id, &comment.id).unwrap();
        // The comment is still there. What changed is whether anybody still has to act on it, and
        // that is the number a header is for.
        assert_eq!(resolved.comments().len(), 1);
        assert_eq!(
            service.view(resolved).unwrap().summary.unresolved_comments,
            0
        );
    }

    #[test]
    fn persisted_log_contract_cannot_carry_sensitive_review_content() {
        let event = ReviewLogEvent {
            kind: "comment-added",
            review_id: "review-1".into(),
            item_count: 1,
        };
        let rendered = format!("{event:?}");
        for forbidden in ["secret-token", "src/private.rs", "Please fix", "@@ -1"] {
            assert!(!rendered.contains(forbidden));
        }
    }
}
