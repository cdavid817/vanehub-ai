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

pub(crate) struct NoSessionEvidence;

impl SessionEvidencePort for NoSessionEvidence {
    fn try_publish(&self, _signal: SessionEvidenceSignal) {}
}
