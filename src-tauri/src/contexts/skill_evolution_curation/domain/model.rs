use serde::{Deserialize, Serialize};

pub(crate) const CURATOR_SCHEMA_VERSION_V1: u16 = 1;
pub(crate) const DEFAULT_OPEN_RETENTION_DAYS: u16 = 180;
pub(crate) const DEFAULT_TERMINAL_RETENTION_DAYS: u16 = 365;

macro_rules! curator_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub(crate) enum $name { $($variant),+ }
    };
}

curator_enum!(CuratorCandidateState {
    Pending,
    AwaitingDraft,
    ReadyForReview,
    Deferred,
    Rejected,
    Applying,
    Applied,
    ApplyFailed,
    Superseded
});
curator_enum!(CuratorStalenessReason {
    AssessmentChanged,
    TargetChanged,
    EvidencePurged,
    DraftChanged,
    BaseChanged,
    OverlayChanged,
    PinChanged,
    TrustChanged,
    ConflictChanged,
    PolicyChanged,
    PreviewExpired
});
curator_enum!(CuratorDraftKind {
    LearnBlock,
    ExactPatch
});
curator_enum!(CuratorDecisionKind {
    Approve,
    Reject,
    Defer,
    Resume
});
curator_enum!(CuratorRejectionReason {
    IncorrectTarget,
    UnsupportedLesson,
    Duplicate,
    TooRisky,
    NotUseful,
    Other
});
curator_enum!(CuratorDeferReason {
    NeedMoreEvidence,
    NeedExpertReview,
    WaitingForChange,
    LowerPriority,
    Other
});
curator_enum!(CuratorApplicationStatus {
    IntentRecorded,
    Applying,
    Applied,
    Failed,
    Reconciled
});
curator_enum!(CuratorActorClass {
    LocalInteractiveUser,
    System,
    WebMockInteractiveUser
});
curator_enum!(CuratorRoute {
    Advance,
    NeedsHumanReview
});
curator_enum!(CuratorAssessmentRoute {
    Advance,
    Drop,
    RecordMemoryOnly,
    MergeDuplicate,
    NeedsHumanReview
});
curator_enum!(CuratorRisk { Low, Medium, High });
curator_enum!(CuratorConfidence { Low, Medium, High });
curator_enum!(CuratorCheckResult {
    Pass,
    Fail,
    Review,
    NotApplicable
});
curator_enum!(CuratorEventKind {
    Intake,
    DraftRejected,
    DraftRevised,
    DraftAssessed,
    Previewed,
    PreviewInvalidated,
    Deferred,
    Resumed,
    Rejected,
    Approved,
    ApplicationStarted,
    Applied,
    ApplicationFailed,
    Superseded,
    PolicyChanged,
    ContentPurged
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorPolicyV1 {
    pub(crate) schema_version: u16,
    pub(crate) workspace_id: String,
    pub(crate) enqueue_routes: Vec<CuratorRoute>,
    pub(crate) require_rejection_reason: bool,
    pub(crate) require_defer_reason: bool,
    pub(crate) maximum_defer_days: u16,
    pub(crate) open_retention_days: u16,
    pub(crate) terminal_retention_days: u16,
    pub(crate) notifications_enabled: bool,
    pub(crate) digest_enabled: bool,
    pub(crate) draft_display_limit_bytes: u32,
    pub(crate) diff_display_limit_bytes: u32,
    pub(crate) revision: u64,
}

impl CuratorPolicyV1 {
    pub(crate) fn manual_default(workspace_id: String) -> Self {
        Self {
            schema_version: CURATOR_SCHEMA_VERSION_V1,
            workspace_id,
            enqueue_routes: vec![CuratorRoute::Advance, CuratorRoute::NeedsHumanReview],
            require_rejection_reason: true,
            require_defer_reason: true,
            maximum_defer_days: 180,
            open_retention_days: DEFAULT_OPEN_RETENTION_DAYS,
            terminal_retention_days: DEFAULT_TERMINAL_RETENTION_DAYS,
            notifications_enabled: true,
            digest_enabled: false,
            draft_display_limit_bytes: 16 * 1024,
            diff_display_limit_bytes: 64 * 1024,
            revision: 1,
        }
    }
}
