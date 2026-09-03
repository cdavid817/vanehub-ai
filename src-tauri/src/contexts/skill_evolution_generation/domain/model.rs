use serde::{Deserialize, Serialize};

pub(crate) const GENERATION_SCHEMA_VERSION_V1: u16 = 1;

macro_rules! generation_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub(crate) enum $name { $($variant),+ }
    };
}

generation_enum!(GenerationConsentState {
    Disabled,
    Enabled,
    Revoked,
    DisclosureStale
});
generation_enum!(GenerationJobStatus {
    Requested,
    BlockedConsent,
    Queued,
    Running,
    CancelRequested,
    Cancelled,
    Failed,
    Completed,
    Superseded
});
generation_enum!(GenerationStageKind {
    FreezeInput,
    InspectTarget,
    BuildDossier,
    PlanMutation,
    SynthesizeStructuredDraft,
    ValidateAndSimulate,
    PackageForGovernance
});
generation_enum!(GenerationStageStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Superseded
});
generation_enum!(DossierSectionKind {
    IdentityAndProvenance,
    ExecutiveSummary,
    CandidateSeed,
    SourceSignalInventory,
    AttributionAndTargetSelection,
    AssessmentAndQualityGates,
    CurrentEffectiveSkillSnapshot,
    RelevantGuidanceAndResourceContext,
    FailureRecoveryAndVerificationTimeline,
    PrivacyAndRedactionReport,
    ProposedMutationRationale,
    VerificationPlan,
    LineageAndVersionWitnesses
});
generation_enum!(DossierSectionStatus {
    Complete,
    Partial,
    NotApplicable,
    Unavailable,
    Redacted
});
generation_enum!(GeneratedArtifactKind {
    OverlayLearnBlock,
    OverlayExactPatch,
    NewSkill
});
generation_enum!(GenerationModelOutcome {
    Valid,
    ProviderUnavailable,
    Timeout,
    RateLimited,
    MalformedJson,
    InvalidSchema,
    OversizedOutput,
    ConsentLost,
    ProviderFailure
});
generation_enum!(GenerationToolOutcome {
    Succeeded,
    StaleWitness,
    InvalidArgument,
    ResultTooLarge,
    BudgetExceeded,
    PolicyDenied,
    Failed
});
generation_enum!(GenerationValidationStatus {
    Pending,
    Passed,
    Failed,
    Repairable,
    Superseded
});
generation_enum!(GenerationQuarantineStatus {
    PendingValidation,
    Quarantined,
    Reviewable,
    Rejected,
    Applied,
    Purged,
    Superseded
});
generation_enum!(GenerationHandoffStatus {
    Pending,
    Delivered,
    Duplicate,
    Failed,
    Superseded
});
