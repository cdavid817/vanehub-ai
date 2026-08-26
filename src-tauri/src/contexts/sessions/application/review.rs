use crate::contexts::sessions::domain::{
    ReviewAnchor, ReviewComment, ReviewDecision, ReviewDomainError, ReviewFile, ReviewFinding,
    ReviewHunkDecision, ReviewSession,
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
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "reachable from 13.4's setCodeReviewHunkDecision command; \
                 an expect rather than an allow so wiring it removes this line"
        )
    )]
    fn upsert_hunk_decision(
        &self,
        review_id: &str,
        decision: &ReviewHunkDecision,
    ) -> Result<(), ReviewApplicationError>;

    /// Every hunk decision recorded for a review, in path then fingerprint order.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "reachable from 13.4's setCodeReviewHunkDecision command; \
                 an expect rather than an allow so wiring it removes this line"
        )
    )]
    fn list_hunk_decisions(
        &self,
        review_id: &str,
    ) -> Result<Vec<ReviewHunkDecision>, ReviewApplicationError>;
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

/// What a caller is asking to mark.
///
/// No expected snapshot yet. The service records the review's own current fingerprint, so nothing
/// stored is ever a claim about a diff that was not the one being reviewed; 13.3 adds the caller's
/// expectation and the refusal when the two disagree. Accepting an expectation here and ignoring
/// it would be a parameter that documents a check nobody performs.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reachable from 13.4's setCodeReviewHunkDecision command; \
                 an expect rather than an allow so wiring it removes this line"
    )
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SetHunkDecisionRequest {
    pub(crate) path: String,
    pub(crate) hunk_fingerprint: String,
    pub(crate) decision: ReviewDecision,
}

#[derive(Clone)]
pub(crate) struct ReviewApplicationService {
    repository: Arc<dyn ReviewRepository>,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "reachable from 13.4's setCodeReviewHunkDecision command; \
                 an expect rather than an allow so wiring it removes this line"
        )
    )]
    decisions: Arc<dyn ReviewDecisionRepository>,
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
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "reachable from 13.4's setCodeReviewHunkDecision command; \
                 an expect rather than an allow so wiring it removes this line"
        )
    )]
    pub(crate) fn set_hunk_decision(
        &self,
        review_id: &str,
        request: SetHunkDecisionRequest,
    ) -> Result<ReviewHunkDecision, ReviewApplicationError> {
        let review = self.find(review_id)?;
        // The review's fingerprint, not one supplied by the caller. What is being recorded is the
        // diff this decision was made about, and the only authority on that is the review.
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

    /// Every hunk decision this review holds.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "reachable from 13.4's setCodeReviewHunkDecision command; \
                 an expect rather than an allow so wiring it removes this line"
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
        /// When set, every write fails. Used to prove nothing is published for a write that did
        /// not land.
        refuse: Mutex<bool>,
    }
    impl ReviewDecisionRepository for MemoryDecisions {
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
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(CapturingLog::default()),
        )
    }

    /// A service whose decision store and evidence publisher the caller can inspect.
    fn reviewing() -> (
        ReviewApplicationService,
        Arc<MemoryDecisions>,
        Arc<CapturingEvidence>,
    ) {
        let decisions = Arc::new(MemoryDecisions::default());
        let evidence = Arc::new(CapturingEvidence::default());
        let service = ReviewApplicationService::new(
            Arc::new(MemoryRepository::default()),
            decisions.clone(),
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(CapturingLog::default()),
            evidence.clone(),
        );
        (service, decisions, evidence)
    }

    fn opened(service: &ReviewApplicationService) -> ReviewSession {
        service.open("session-1").unwrap()
    }

    fn mark(path: &str, hunk: &str, decision: ReviewDecision) -> SetHunkDecisionRequest {
        SetHunkDecisionRequest {
            path: path.into(),
            hunk_fingerprint: hunk.into(),
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
        let (service, _decisions, _evidence) = reviewing();
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
        let (service, _decisions, _evidence) = reviewing();
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
        let (service, _decisions, evidence) = reviewing();
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
        let (service, _decisions, evidence) = reviewing();
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
        let (service, decisions, evidence) = reviewing();
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
        let (service, decisions, evidence) = reviewing();

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
        let (service, decisions, _evidence) = reviewing();
        let review = opened(&service);

        assert!(service
            .set_hunk_decision(
                &review.id,
                mark("../outside.rs", "hunk-1", ReviewDecision::Accepted)
            )
            .is_err());
        assert!(decisions.rows.lock().unwrap().is_empty());
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
