use super::{
    anomaly_name, anomaly_threshold, draft, failure_name, failure_severity, operation_name,
    utility_name, verification_name, verification_polarity, verifier_name, EvidenceSourceEnvelope,
    ExtractorFamily, FailureClass, FeedbackState, OperationClass, SanitizationResult,
    SignalCategory, SignalDraft, SignalPolarity, SignalSeverity, TerminalOutcome, UtilityOutcome,
    VerificationOutcome,
};

pub(crate) const EXTRACTOR_VERSION_V1: u16 = 1;

type ExtractorFn = fn(&EvidenceSourceEnvelope, Option<&SanitizationResult>) -> Vec<SignalDraft>;

#[derive(Clone, Copy)]
pub(crate) struct ExtractorRegistration {
    pub(crate) family: ExtractorFamily,
    pub(crate) version: u16,
    extract: ExtractorFn,
}

pub(crate) const REGISTERED_EXTRACTORS: [ExtractorRegistration; 6] = [
    registration(ExtractorFamily::ExplicitFeedback, extract_feedback),
    registration(ExtractorFamily::ExecutionFailure, extract_execution_failure),
    registration(ExtractorFamily::VerificationOutcome, extract_verification),
    registration(ExtractorFamily::RetryRecovery, extract_retry_recovery),
    registration(ExtractorFamily::DelegationOutcome, extract_delegation),
    registration(
        ExtractorFamily::SkillLifecycleAnomaly,
        extract_lifecycle_anomaly,
    ),
];

const fn registration(family: ExtractorFamily, extract: ExtractorFn) -> ExtractorRegistration {
    ExtractorRegistration {
        family,
        version: EXTRACTOR_VERSION_V1,
        extract,
    }
}

pub(crate) fn extract_registered_signals(
    envelope: &EvidenceSourceEnvelope,
    sanitized_text: Option<&SanitizationResult>,
) -> Vec<SignalDraft> {
    REGISTERED_EXTRACTORS
        .iter()
        .flat_map(|registration| (registration.extract)(envelope, sanitized_text))
        .collect()
}

fn extract_feedback(
    envelope: &EvidenceSourceEnvelope,
    sanitized_text: Option<&SanitizationResult>,
) -> Vec<SignalDraft> {
    let EvidenceSourceEnvelope::ExplicitFeedback {
        feedback,
        feedback_revision,
        correction_note,
        ..
    } = envelope
    else {
        return Vec::new();
    };
    let (polarity, severity, summary) = match feedback {
        FeedbackState::Helpful => (
            SignalPolarity::Positive,
            SignalSeverity::Low,
            "helpful".into(),
        ),
        FeedbackState::Unhelpful => (
            SignalPolarity::Negative,
            SignalSeverity::Medium,
            "unhelpful".into(),
        ),
        FeedbackState::Corrected => {
            let Some(sanitized) = sanitized_text.filter(|_| correction_note.is_some()) else {
                return Vec::new();
            };
            (
                SignalPolarity::Negative,
                SignalSeverity::High,
                format!("corrected: {}", sanitized.text()),
            )
        }
    };
    vec![draft(
        envelope,
        ExtractorFamily::ExplicitFeedback,
        SignalCategory::ExplicitFeedback,
        polarity,
        severity,
        summary,
        format!("feedback:{feedback_revision}"),
    )]
}

fn extract_execution_failure(
    envelope: &EvidenceSourceEnvelope,
    _: Option<&SanitizationResult>,
) -> Vec<SignalDraft> {
    let failure = match envelope {
        EvidenceSourceEnvelope::NativeExecution {
            operation_class,
            outcome: TerminalOutcome::Failed,
            failure_class,
            safe_counts,
            ..
        } => Some((
            *operation_class,
            failure_class.unwrap_or(FailureClass::Agent),
            safe_counts.attempts,
        )),
        EvidenceSourceEnvelope::ManagedCli {
            outcome: TerminalOutcome::Failed,
            failure_class,
            ..
        } => Some((
            OperationClass::Process,
            failure_class.unwrap_or(FailureClass::Process),
            1,
        )),
        EvidenceSourceEnvelope::InteractiveCli {
            outcome: TerminalOutcome::Failed,
            ..
        } => Some((OperationClass::Process, FailureClass::Process, 1)),
        _ => None,
    };
    let Some((operation, class, attempts)) = failure else {
        return Vec::new();
    };
    vec![draft(
        envelope,
        ExtractorFamily::ExecutionFailure,
        SignalCategory::ExecutionFailure,
        SignalPolarity::Negative,
        failure_severity(class),
        format!(
            "{} failed with {} classification in {attempts} attempts",
            operation_name(operation),
            failure_name(class)
        ),
        format!(
            "failure:{}:{}",
            operation_name(operation),
            failure_name(class)
        ),
    )]
}

fn extract_verification(
    envelope: &EvidenceSourceEnvelope,
    _: Option<&SanitizationResult>,
) -> Vec<SignalDraft> {
    let EvidenceSourceEnvelope::PlanVerification {
        verifier,
        outcome,
        passed_count,
        failed_count,
        ..
    } = envelope
    else {
        return Vec::new();
    };
    let polarity = verification_polarity(*outcome);
    vec![draft(
        envelope,
        ExtractorFamily::VerificationOutcome,
        SignalCategory::VerificationOutcome,
        polarity,
        if polarity == SignalPolarity::Negative {
            SignalSeverity::High
        } else {
            SignalSeverity::Low
        },
        format!(
            "{} verification {}; passed={} failed={}",
            verifier_name(*verifier),
            verification_name(*outcome),
            passed_count,
            failed_count
        ),
        format!("verification:{}", verifier_name(*verifier)),
    )]
}

fn extract_retry_recovery(
    envelope: &EvidenceSourceEnvelope,
    _: Option<&SanitizationResult>,
) -> Vec<SignalDraft> {
    let EvidenceSourceEnvelope::PlanVerification {
        outcome,
        predecessor_attempt_id: Some(predecessor),
        ..
    } = envelope
    else {
        return Vec::new();
    };
    let (polarity, summary) = match outcome {
        VerificationOutcome::Passed => (SignalPolarity::Positive, "retry recovered"),
        VerificationOutcome::Failed => (SignalPolarity::Negative, "retry repeated failure"),
        VerificationOutcome::Inconclusive | VerificationOutcome::Skipped => return Vec::new(),
    };
    vec![draft(
        envelope,
        ExtractorFamily::RetryRecovery,
        SignalCategory::RetryRecovery,
        polarity,
        SignalSeverity::High,
        format!("{summary}; predecessor={predecessor}"),
        format!("retry:{predecessor}"),
    )]
}

fn extract_delegation(
    envelope: &EvidenceSourceEnvelope,
    _: Option<&SanitizationResult>,
) -> Vec<SignalDraft> {
    let EvidenceSourceEnvelope::DelegatedUtility {
        outcome,
        tool_count,
        approval_count,
        ..
    } = envelope
    else {
        return Vec::new();
    };
    let (polarity, severity) = match outcome {
        UtilityOutcome::Succeeded => (SignalPolarity::Positive, SignalSeverity::Low),
        UtilityOutcome::Failed | UtilityOutcome::TimedOut | UtilityOutcome::Limited => {
            (SignalPolarity::Negative, SignalSeverity::High)
        }
        UtilityOutcome::Refused => (SignalPolarity::Neutral, SignalSeverity::Medium),
        UtilityOutcome::Cancelled | UtilityOutcome::Incomplete => {
            (SignalPolarity::Neutral, SignalSeverity::Medium)
        }
    };
    let outcome = utility_name(*outcome);
    vec![draft(
        envelope,
        ExtractorFamily::DelegationOutcome,
        SignalCategory::DelegationOutcome,
        polarity,
        severity,
        format!("Utility {outcome}; tools={tool_count} approvals={approval_count}"),
        format!("delegation:{outcome}"),
    )]
}

fn extract_lifecycle_anomaly(
    envelope: &EvidenceSourceEnvelope,
    _: Option<&SanitizationResult>,
) -> Vec<SignalDraft> {
    let (anomaly, observation_count) = match envelope {
        EvidenceSourceEnvelope::SkillLoading {
            anomaly: Some(anomaly),
            observation_count,
            ..
        } if *observation_count >= anomaly_threshold(*anomaly) => {
            (anomaly_name(*anomaly), Some(*observation_count))
        }
        EvidenceSourceEnvelope::NativeExecution {
            outcome: TerminalOutcome::Cancelled,
            ..
        }
        | EvidenceSourceEnvelope::ManagedCli {
            outcome: TerminalOutcome::Cancelled,
            ..
        }
        | EvidenceSourceEnvelope::InteractiveCli {
            outcome: TerminalOutcome::Cancelled,
            ..
        } => ("cancelled", None),
        _ => return Vec::new(),
    };
    let count_suffix = observation_count
        .map(|count| format!("; observations={count}"))
        .unwrap_or_default();
    vec![draft(
        envelope,
        ExtractorFamily::SkillLifecycleAnomaly,
        SignalCategory::SkillLifecycleAnomaly,
        SignalPolarity::Neutral,
        SignalSeverity::Medium,
        format!("Skill lifecycle anomaly: {anomaly}{count_suffix}"),
        format!("lifecycle:{anomaly}"),
    )]
}
