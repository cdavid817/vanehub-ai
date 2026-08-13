use super::{
    FailureClass, OperationClass, SignalPolarity, SignalSeverity, SkillLifecycleAnomaly,
    UtilityOutcome, VerificationClass, VerificationOutcome,
};

pub(super) fn operation_name(value: OperationClass) -> &'static str {
    match value {
        OperationClass::Generation => "generation",
        OperationClass::Tool => "tool",
        OperationClass::Permission => "permission",
        OperationClass::Provider => "provider",
        OperationClass::Process => "process",
    }
}

pub(super) fn failure_name(value: FailureClass) -> &'static str {
    match value {
        FailureClass::Agent => "agent",
        FailureClass::Provider => "provider",
        FailureClass::Process => "process",
        FailureClass::Tool => "tool",
        FailureClass::Permission => "permission",
        FailureClass::Timeout => "timeout",
        FailureClass::Limit => "limit",
        FailureClass::Sandbox => "sandbox",
    }
}

pub(super) fn failure_severity(value: FailureClass) -> SignalSeverity {
    match value {
        FailureClass::Permission | FailureClass::Limit | FailureClass::Agent => {
            SignalSeverity::Medium
        }
        _ => SignalSeverity::High,
    }
}

pub(super) fn verifier_name(value: VerificationClass) -> &'static str {
    match value {
        VerificationClass::Test => "test",
        VerificationClass::Build => "build",
        VerificationClass::Lint => "lint",
        VerificationClass::Review => "review",
        VerificationClass::Type => "type",
        VerificationClass::Security => "security",
        VerificationClass::Specification => "specification",
        VerificationClass::Acceptance => "acceptance",
        VerificationClass::Plan => "plan",
    }
}

pub(super) fn verification_name(value: VerificationOutcome) -> &'static str {
    match value {
        VerificationOutcome::Passed => "passed",
        VerificationOutcome::Failed => "failed",
        VerificationOutcome::Inconclusive => "inconclusive",
        VerificationOutcome::Skipped => "skipped",
    }
}

pub(super) fn verification_polarity(value: VerificationOutcome) -> SignalPolarity {
    match value {
        VerificationOutcome::Passed => SignalPolarity::Positive,
        VerificationOutcome::Failed => SignalPolarity::Negative,
        VerificationOutcome::Inconclusive | VerificationOutcome::Skipped => SignalPolarity::Neutral,
    }
}

pub(super) fn utility_name(value: UtilityOutcome) -> &'static str {
    match value {
        UtilityOutcome::Succeeded => "succeeded",
        UtilityOutcome::Failed => "failed",
        UtilityOutcome::Cancelled => "cancelled",
        UtilityOutcome::TimedOut => "timed-out",
        UtilityOutcome::Limited => "limited",
        UtilityOutcome::Refused => "refused",
        UtilityOutcome::Incomplete => "incomplete",
    }
}

pub(super) fn anomaly_name(value: SkillLifecycleAnomaly) -> &'static str {
    match value {
        SkillLifecycleAnomaly::LoadRefusal => "load_refusal",
        SkillLifecycleAnomaly::DependencyUnavailable => "dependency_unavailable",
        SkillLifecycleAnomaly::OverlayConflict => "overlay_conflict",
        SkillLifecycleAnomaly::PromptBudgetOmission => "prompt_budget_omission",
    }
}

pub(super) fn anomaly_threshold(value: SkillLifecycleAnomaly) -> u32 {
    match value {
        SkillLifecycleAnomaly::OverlayConflict => 2,
        SkillLifecycleAnomaly::LoadRefusal
        | SkillLifecycleAnomaly::DependencyUnavailable
        | SkillLifecycleAnomaly::PromptBudgetOmission => 3,
    }
}
