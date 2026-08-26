/// What sessions is willing to say about accounting it recorded.
///
/// A reference to a usage observation, never the observation. Sessions owns the token dimensions
/// and the invocation detail; duplicating them into the journal would create two totals that drift
/// and no way to tell which is right. What crosses is the id to join on and how the accounting was
/// arrived at, so a reader can weigh it without the journal restating it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionEvidenceSignal {
    UsageObserved {
        session_id: String,
        invocation_id: String,
        run_id: Option<String>,
        quality: SessionUsageEvidenceQuality,
        occurred_at: String,
    },
    /// A decision recorded about a whole review.
    ///
    /// File Viewed state is still not here: its store exists, but nothing writes it until 13.5,
    /// and deriving it from a decision would mean asserting that deciding about a file is the same
    /// as having read it.
    ReviewDecisionRecorded {
        session_id: String,
        review_id: String,
        decision: SessionReviewDecision,
        /// The snapshot the decision was made against. Without it a reader cannot tell a decision
        /// about the current diff from one about a diff that has since changed.
        witness_fingerprint: String,
        occurred_at: String,
    },
    /// A decision recorded about one hunk.
    ///
    /// Separate from the review-level decision rather than derived from it. The two are
    /// independent, so a journal that inferred one from the other would report decisions nobody
    /// made — which is precisely what a journal exists not to do.
    ReviewHunkDecisionRecorded {
        session_id: String,
        review_id: String,
        /// Which hunk, by fingerprint. The path stays in the review store: a journal entry does
        /// not need workspace content to identify the hunk a decision was about.
        hunk_fingerprint: String,
        decision: SessionReviewDecision,
        /// The snapshot the decision was made against, for the same reason the review-level one
        /// carries it.
        witness_fingerprint: String,
        occurred_at: String,
    },
    /// An automated check that finished. The counts are what it produced; what it said about any
    /// particular line stays in the finding store.
    VerificationCompleted {
        session_id: String,
        run_id: Option<String>,
        verification_run_id: String,
        name: String,
        outcome: SessionVerificationOutcome,
        passed_count: Option<u32>,
        failed_count: Option<u32>,
        occurred_at: String,
    },
}

/// What a reviewer concluded. `Pending` is not published: it is the absence of a decision, and
/// recording it would put "nobody has decided yet" in a journal of things that happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionReviewDecision {
    Accepted,
    ChangesRequested,
}

/// No `Skipped`: a check that did not run produces no outcome at all, and an event saying it was
/// skipped would be an observation of a non-event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionVerificationOutcome {
    Passed,
    Failed,
}

/// How the numbers were arrived at. A consumer that cannot tell a provider-reported total from an
/// estimate will present both as fact, so the distinction travels with the reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionUsageEvidenceQuality {
    Reported,
    ReportedDerived,
    Estimated,
}

/// Where sessions hands an observation off.
///
/// Synchronous and infallible from the caller's side. Accounting that was recorded stays recorded
/// whether or not the journal accepted a pointer to it.
pub(crate) trait SessionEvidencePort: Send + Sync {
    fn try_publish(&self, signal: SessionEvidenceSignal);
}

/// A publisher that records nothing.
///
/// Test-only. Production takes its publisher as a constructor argument, so an assembly that
/// forgets one fails to compile rather than running and quietly recording nothing — which used to
/// surface as a panel reporting that a session did no work.
#[cfg(test)]
pub(crate) struct NoSessionEvidence;

#[cfg(test)]
impl SessionEvidencePort for NoSessionEvidence {
    fn try_publish(&self, _signal: SessionEvidenceSignal) {}
}
