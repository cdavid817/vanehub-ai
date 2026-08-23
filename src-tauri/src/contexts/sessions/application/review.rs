use crate::contexts::sessions::domain::{
    ReviewAnchor, ReviewComment, ReviewDecision, ReviewDomainError, ReviewFile, ReviewFinding,
    ReviewSession,
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

#[derive(Clone)]
pub(crate) struct ReviewApplicationService {
    repository: Arc<dyn ReviewRepository>,
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
        clock: Arc<dyn ReviewClockPort>,
        ids: Arc<dyn ReviewIdPort>,
        feedback: Arc<dyn ReviewFeedbackPort>,
        snapshots: Arc<dyn ReviewSnapshotPort>,
        operations: Arc<dyn ReviewOperationPort>,
        logging: Arc<dyn ReviewLoggingPort>,
    ) -> Self {
        Self::new(
            repository,
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
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(Fixed),
            Arc::new(CapturingLog::default()),
        )
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
