use crate::contexts::skill_evolution_evidence::domain::{
    AttributionRationale, AttributionStrength, ExtractorFamily, FeedbackState, SeedReadiness,
    SignalCategory, SignalPolarity, SignalSeverity, SignalSourceLinkKind, SkillAssociationKind,
    SourceFamily, SourceFidelity, TargetingEligibility,
};

pub(super) fn extractor(value: ExtractorFamily) -> &'static str {
    match value {
        ExtractorFamily::ExplicitFeedback => "explicit_feedback",
        ExtractorFamily::ExecutionFailure => "execution_failure",
        ExtractorFamily::VerificationOutcome => "verification_outcome",
        ExtractorFamily::RetryRecovery => "retry_recovery",
        ExtractorFamily::DelegationOutcome => "delegation_outcome",
        ExtractorFamily::SkillLifecycleAnomaly => "skill_lifecycle_anomaly",
    }
}

pub(super) fn feedback(value: FeedbackState) -> &'static str {
    match value {
        FeedbackState::Helpful => "helpful",
        FeedbackState::Unhelpful => "unhelpful",
        FeedbackState::Corrected => "corrected",
    }
}

pub(super) fn category(value: SignalCategory) -> &'static str {
    match value {
        SignalCategory::ExplicitFeedback => "explicit_feedback",
        SignalCategory::ExecutionFailure => "execution_failure",
        SignalCategory::VerificationOutcome => "verification_outcome",
        SignalCategory::RetryRecovery => "retry_recovery",
        SignalCategory::DelegationOutcome => "delegation_outcome",
        SignalCategory::SkillLifecycleAnomaly => "skill_lifecycle_anomaly",
    }
}

pub(super) fn polarity(value: SignalPolarity) -> &'static str {
    match value {
        SignalPolarity::Positive => "positive",
        SignalPolarity::Negative => "negative",
        SignalPolarity::Neutral => "neutral",
    }
}

pub(super) fn severity(value: SignalSeverity) -> &'static str {
    match value {
        SignalSeverity::Low => "low",
        SignalSeverity::Medium => "medium",
        SignalSeverity::High => "high",
    }
}

pub(super) fn attribution(value: AttributionStrength) -> &'static str {
    match value {
        AttributionStrength::Verified => "verified",
        AttributionStrength::Correlated => "correlated",
        AttributionStrength::Weak => "weak",
        AttributionStrength::Unattributed => "unattributed",
    }
}

pub(super) fn rationale(value: AttributionRationale) -> &'static str {
    match value {
        AttributionRationale::ExactNativeObservation => "exact_native_observation",
        AttributionRationale::ActiveCliMountSnapshot => "active_cli_mount_snapshot",
        AttributionRationale::ConfiguredBindingOnly => "configured_binding_only",
        AttributionRationale::NoObservedSkillParticipation => "no_observed_skill_participation",
    }
}

pub(super) fn targeting(value: TargetingEligibility) -> &'static str {
    match value {
        TargetingEligibility::AutomatedConsideration => "automated_consideration",
        TargetingEligibility::HumanReviewOnly => "human_review_only",
        TargetingEligibility::Ineligible => "ineligible",
    }
}

pub(super) fn fidelity(value: SourceFidelity) -> &'static str {
    match value {
        SourceFidelity::Native => "native",
        SourceFidelity::Proxied => "proxied",
        SourceFidelity::Inferred => "inferred",
        SourceFidelity::Opaque => "opaque",
    }
}

pub(super) fn source_family(value: SourceFamily) -> &'static str {
    match value {
        SourceFamily::NativeExecution => "native_execution",
        SourceFamily::SkillLoading => "skill_loading",
        SourceFamily::DelegatedUtility => "delegated_utility",
        SourceFamily::PlanVerification => "plan_verification",
        SourceFamily::ManagedCli => "managed_cli",
        SourceFamily::InteractiveCli => "interactive_cli",
        SourceFamily::ExplicitFeedback => "explicit_feedback",
    }
}

pub(super) fn association_kind(value: SkillAssociationKind) -> &'static str {
    match value {
        SkillAssociationKind::Injected => "injected",
        SkillAssociationKind::Loaded => "loaded",
        SkillAssociationKind::Delegated => "delegated",
        SkillAssociationKind::Mounted => "mounted",
        SkillAssociationKind::Configured => "configured",
    }
}

pub(super) fn source_link(value: SignalSourceLinkKind) -> &'static str {
    match value {
        SignalSourceLinkKind::Session => "session",
        SignalSourceLinkKind::Message => "message",
        SignalSourceLinkKind::Run => "run",
        SignalSourceLinkKind::Attempt => "attempt",
    }
}

#[allow(dead_code)]
pub(super) fn readiness(value: SeedReadiness) -> &'static str {
    match value {
        SeedReadiness::Collecting => "collecting",
        SeedReadiness::Ready => "ready",
        SeedReadiness::HumanReviewOnly => "human_review_only",
        SeedReadiness::Ineligible => "ineligible",
    }
}
