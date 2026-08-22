/// What operations is willing to say about an operation that did not complete.
///
/// A failure reference, not a failure report. The log store already holds the message, the stack,
/// and whatever the operation printed; this carries the ids needed to find that entry plus a
/// reason code, so the journal can say "something failed here" without becoming a second copy of
/// the log — a second copy is a second place redaction has to be right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OperationsEvidenceSignal {
    OperationFailed {
        session_id: String,
        operation_id: String,
        run_id: Option<String>,
        /// One of the operations context's own stable codes. Free text would defeat the point:
        /// a message is what the log holds, and a code is what a UI can group and translate.
        reason_code: String,
        occurred_at: String,
    },
}

/// Where operations hands an observation off.
///
/// Synchronous and infallible from the caller's side. An operation that failed has failed, and the
/// journal being full must not turn that into a different failure.
pub(crate) trait OperationsEvidencePort: Send + Sync {
    fn try_publish(&self, signal: OperationsEvidenceSignal);
}

pub(crate) struct NoOperationsEvidence;

impl OperationsEvidencePort for NoOperationsEvidence {
    fn try_publish(&self, _signal: OperationsEvidenceSignal) {}
}
