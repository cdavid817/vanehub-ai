#![allow(unused_imports)]
// The delegation gate remains off until provider-network and child-action isolation are composed.
#![allow(dead_code)]

mod apply_exact;
mod apply_preflight;
mod apply_recovery;
mod apply_staging;
mod apply_verification;
mod changeset_capture;
mod changeset_policy;
mod changeset_review;
mod changeset_sealing;
mod circuit_breaker;
mod claude_invocation;
mod claude_protocol;
mod codex_invocation;
mod codex_protocol;
mod environment;
mod execution;
mod materialization;
mod network_policy;
mod readiness;
mod readiness_support;
mod report;
mod report_comparison;
mod restart_recovery;
mod scheduling;
mod state_machine;
mod workspace;

#[cfg(test)]
#[path = "compatibility_tests.rs"]
mod compatibility_tests;

pub(crate) use apply_exact::{
    DelegationExactApplyError, DelegationExactApplyPort, DelegationExactApplyRequest,
    DelegationExactApplyService, DelegationExactApplyWitness,
};
pub(crate) use apply_preflight::{
    DelegationApplyArtifactEvidence, DelegationApplyArtifactPort, DelegationApplyOncePort,
    DelegationApplyPlan, DelegationApplyPreflightError, DelegationApplyPreflightRequest,
    DelegationApplyPreflightService, DelegationApplyTargetPort, DelegationApplyTargetWitness,
};
pub(crate) use apply_recovery::{
    DelegationApplyRecoveryError, DelegationApplyRecoveryPort, DelegationApplyRecoveryService,
    DelegationRecoveryOutcome,
};
pub(crate) use apply_staging::{
    DelegationApplyPathExpectation, DelegationApplyPathWitness, DelegationApplyStagingError,
    DelegationApplyStagingPort, DelegationApplyStagingService, DelegationRecoveryCapsule,
};
pub(crate) use apply_verification::{
    DelegationPostApplyVerificationError, DelegationPostApplyVerificationPort,
    DelegationPostApplyVerificationService, DelegationPostApplyWitness,
};
pub(crate) use changeset_capture::{
    DelegationChangeFile, DelegationChangeKind, DelegationChangeSetCapture,
    DelegationChangeSetCaptureError, DelegationChangeSetCapturePort,
};
pub(crate) use changeset_policy::{
    DelegationChangeSetLimits, DelegationChangeSetPolicy, DelegationChangeSetPolicyError,
};
pub(crate) use changeset_review::{
    DelegationChangeSetPayload, DelegationChangeSetReview, DelegationChangeSetReviewError,
    DelegationChangeSetReviewPort, DelegationChangeSetReviewRequest, DelegationChangeSetReviewer,
    DelegationDiffEncoding,
};
pub(crate) use changeset_sealing::{
    DelegationChangeSetArtifact, DelegationChangeSetArtifactPort, DelegationChangeSetSealError,
    DelegationChangeSetSealRequest, DelegationChangeSetSealer,
};
pub(crate) use circuit_breaker::{
    DelegationCircuitBreaker, DelegationCircuitFailure, DelegationCircuitKey,
    DelegationCircuitState,
};
pub(crate) use claude_invocation::{
    ClaudeDelegationInvocation, ClaudeDelegationInvocationBuilder, ClaudeInvocationError,
    ClaudeInvocationProfile, ClaudeInvocationRequest,
};
pub(crate) use claude_protocol::{
    ClaudeDelegationAdapter, ClaudeProtocolError, DelegationProviderEvent,
};
pub(crate) use codex_invocation::{
    CodexDelegationInvocation, CodexDelegationInvocationBuilder, CodexInvocationError,
    CodexInvocationProfile, CodexInvocationRequest,
};
pub(crate) use codex_protocol::{CodexDelegationAdapter, CodexProtocolError};
pub(crate) use environment::{DelegationEnvironmentBuilder, DelegationEnvironmentError};
pub(crate) use execution::{
    DelegationExecutionError, DelegationExecutionLimits, DelegationExecutionObservation,
    DelegationExecutionRequest, DelegationExecutionRunner, DelegationOwnedProcess,
    DelegationProcessLauncher,
};
pub(crate) use materialization::{
    DelegationArtifactInput, DelegationArtifactPort, DelegationMaterializationError,
    DelegationMaterializationPort, DelegationMaterializationRequest, DelegationMaterializer,
};
pub(crate) use network_policy::{
    DelegationChildNetworkPort, DelegationNetworkError, DelegationNetworkPolicy,
    DelegationProviderConnection,
};
pub(crate) use readiness::{
    DelegationAuthentication, DelegationCapabilityDependencies, DelegationMode,
    DelegationProbeObservation, DelegationProbePort, DelegationReadiness,
    DelegationReadinessReason, DelegationReadinessService, DelegationReadinessState,
    DelegationTarget,
};
pub(crate) use report::{
    DelegationAgentReportV1, DelegationEvidenceRole, DelegationHostEvidence, DelegationReportError,
    DelegationReportNormalizer, DelegationReportOutcome, DelegationVerificationClaim,
};
pub(crate) use report_comparison::{DelegationEvidenceWarning, DelegationReportComparator};
pub(crate) use restart_recovery::{
    DelegationInterruptedApply, DelegationRestartRecoveryError, DelegationRestartRecoveryPort,
    DelegationRestartRecoveryService, DelegationRestartResolution, DelegationRestartWitness,
};
pub(crate) use scheduling::{
    DelegationAdmission, DelegationLimitError, DelegationLimitProfile, DelegationObservedUsage,
    DelegationQueueSnapshot, DelegationScheduler,
};
pub(crate) use state_machine::{
    Delegation, DelegationAttempt, DelegationAttemptStatus, DelegationError,
    DelegationRequestSnapshot, DelegationResult, DelegationStatus,
};
pub(crate) use workspace::{
    DelegationRepositoryBaseline, DelegationWorkspace, DelegationWorkspaceError,
    DelegationWorkspacePort,
};
