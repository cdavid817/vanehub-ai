#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AssessmentQueueLane {
    Deterministic,
    OptionalModel,
}

impl AssessmentQueueLane {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Deterministic => "deterministic",
            Self::OptionalModel => "optional_model",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssessmentQueueRequest {
    pub(crate) queue_id: String,
    pub(crate) seed_id: String,
    pub(crate) witness_hash: String,
    pub(crate) lane: AssessmentQueueLane,
    pub(crate) priority: i32,
    pub(crate) available_at_ms: i64,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum QueueEnqueueOutcome {
    Scheduled { queue_id: String },
    Coalesced { queue_id: String },
    OptionalFallback,
    Saturated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AssessmentQueueError {
    InvalidInput,
    LineageUnavailable,
    LeaseUnavailable,
    DatabaseLock,
    WorkerPanic,
    Storage,
}

pub(crate) trait AssessmentQueuePersistence: Send + Sync {
    fn enqueue(
        &self,
        request: &AssessmentQueueRequest,
    ) -> Result<QueueEnqueueOutcome, AssessmentQueueError>;
}
