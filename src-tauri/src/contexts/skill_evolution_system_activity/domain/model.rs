use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const ACTIVITY_SCHEMA_VERSION_V1: u16 = 1;
pub(crate) const MAX_ENVELOPE_BYTES: usize = 16 * 1024;
pub(crate) const MAX_PAYLOAD_BYTES: usize = 8 * 1024;
pub(crate) const MAX_SAFE_IDENTITIES: usize = 16;
pub(crate) const MAX_METRICS: usize = 24;
pub(crate) const MAX_REASON_CODES: usize = 16;

macro_rules! activity_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub(crate) enum $name { $($variant),+ }
    };
}

activity_enum!(ActivityKind { SkillEvolution });
activity_enum!(ActivityScopeKind { Global, Workspace });
activity_enum!(ActivitySeverity {
    Info,
    Warning,
    Error,
    Critical
});
activity_enum!(ActivityStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Blocked,
    Cancelled,
    Superseded
});
activity_enum!(ActivityAttentionKind {
    None,
    Review,
    Security,
    Integrity,
    Regression,
    ApplicationFailure,
    Breaker
});
activity_enum!(ActivityActorKind {
    System,
    User,
    Model,
    Curator,
    Scheduler
});
activity_enum!(ActivityTargetKind {
    SystemTimeline,
    SkillDashboard,
    UnreadState,
    Notification
});
activity_enum!(ActivityDeliveryStatus {
    Delivered,
    Suppressed,
    Failed
});
activity_enum!(ActivityDigestCadence { Off, Hourly, Daily });
activity_enum!(ActivityRebuildStatus {
    Pending,
    Running,
    Validating,
    Ready,
    Active,
    Failed,
    Cancelled
});
activity_enum!(ActivityExportFormat { Json, Markdown });
activity_enum!(ActivityNavigationKind {
    Run,
    Evidence,
    Assessment,
    Dossier,
    GenerationJob,
    CuratorCandidate,
    OverlayHistory,
    Skill,
    Probation,
    Breaker
});
activity_enum!(ActivityEventCode {
    RunStarted,
    RunCompleted,
    RunFailed,
    StageStarted,
    StageCompleted,
    StageFailed,
    EvidenceReady,
    SeedReady,
    AssessmentCompleted,
    AssessmentNeedsReview,
    GenerationStarted,
    GenerationCompleted,
    GenerationFailed,
    DossierCompleted,
    CuratorQueued,
    CuratorApproved,
    CuratorRejected,
    CuratorDeferred,
    OverlayPreviewed,
    OverlayApplied,
    OverlayReverted,
    AutomaticEligible,
    AutomaticApplied,
    AutomaticBlocked,
    ProbationStarted,
    ProbationPassed,
    ProbationRegressed,
    BreakerOpened,
    BreakerClosed,
    SkillCreated,
    RecoveryCompleted,
    ReconciliationFailed,
    RetentionApplied,
    SourcePurged
});
impl ActivityEventCode {
    /// Every registered event code, for safe alias search over the closed registry.
    pub(crate) const ALL: &'static [Self] = &[
        Self::RunStarted,
        Self::RunCompleted,
        Self::RunFailed,
        Self::StageStarted,
        Self::StageCompleted,
        Self::StageFailed,
        Self::EvidenceReady,
        Self::SeedReady,
        Self::AssessmentCompleted,
        Self::AssessmentNeedsReview,
        Self::GenerationStarted,
        Self::GenerationCompleted,
        Self::GenerationFailed,
        Self::DossierCompleted,
        Self::CuratorQueued,
        Self::CuratorApproved,
        Self::CuratorRejected,
        Self::CuratorDeferred,
        Self::OverlayPreviewed,
        Self::OverlayApplied,
        Self::OverlayReverted,
        Self::AutomaticEligible,
        Self::AutomaticApplied,
        Self::AutomaticBlocked,
        Self::ProbationStarted,
        Self::ProbationPassed,
        Self::ProbationRegressed,
        Self::BreakerOpened,
        Self::BreakerClosed,
        Self::SkillCreated,
        Self::RecoveryCompleted,
        Self::ReconciliationFailed,
        Self::RetentionApplied,
        Self::SourcePurged,
    ];
}

activity_enum!(ActivitySafeIdentityKind {
    Workspace,
    Skill,
    Run,
    Evidence,
    Seed,
    Assessment,
    Dossier,
    GenerationJob,
    CuratorCandidate,
    Application,
    Probation,
    Breaker
});
activity_enum!(ActivityReasonCode {
    Started,
    Completed,
    Partial,
    Failed,
    Cancelled,
    BudgetExhausted,
    EvidenceReady,
    SeedReady,
    ReviewRequired,
    PolicyBlocked,
    ValidationFailed,
    ApplicationFailed,
    RegressionDetected,
    BreakerOpened,
    IntegrityFailed,
    SecurityBlocked,
    Recovered,
    RetentionApplied,
    SourcePurged
});
activity_enum!(ActivityMetricCode {
    CandidateCount,
    EvidenceCount,
    PassedCheckCount,
    FailedCheckCount,
    ReviewCheckCount,
    AppliedCount,
    RejectedCount,
    PurgedCount,
    DurationMs
});
activity_enum!(ActivityLabelCode {
    Outcome,
    CurrentStage,
    GovernanceDecision,
    ApplicationStatus,
    RetentionOutcome
});
activity_enum!(ActivityValueCode {
    Started,
    Pending,
    Running,
    Ready,
    Succeeded,
    Completed,
    Failed,
    Blocked,
    Cancelled,
    Superseded,
    Eligible,
    Ineligible,
    Approved,
    Rejected,
    Deferred,
    Applied,
    Reverted,
    Healthy,
    Regressed,
    Open,
    Closed,
    Created,
    Purged
});
activity_enum!(ActivityStageCode {
    Recover,
    MaintainEvidence,
    BuildSeeds,
    Assess,
    RouteGovernance,
    EvaluateAutoApply,
    ProjectResults,
    Notify
});
activity_enum!(ActivityGapCode {
    MissingSequence,
    RetentionFloorAdvanced,
    SourceHashMismatch
});
activity_enum!(ActivityProjectionFailureCode {
    SourceUnavailable,
    InvalidCursor,
    InvalidEnvelope,
    IntegrityFailed,
    StorageFailed,
    UnsupportedVersion
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SafeIdentity {
    pub(crate) kind: ActivitySafeIdentityKind,
    pub(crate) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ActivityNavigation {
    pub(crate) kind: ActivityNavigationKind,
    pub(crate) stable_id: String,
    pub(crate) child_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "schema",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum ActivityPayloadV1 {
    StatusCard {
        label_code: ActivityLabelCode,
        value_code: ActivityValueCode,
    },
    StageTimeline {
        stages: Vec<ActivityStage>,
    },
    CheckSummary {
        passed: u32,
        failed: u32,
        review: u32,
    },
    MetricSummary {
        metrics: BTreeMap<ActivityMetricCode, i64>,
    },
    NavigationList {
        links: Vec<ActivityNavigation>,
    },
    SupersessionNotice {
        prior_event_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ActivityStage {
    pub(crate) code: ActivityStageCode,
    pub(crate) status: ActivityStatus,
}
