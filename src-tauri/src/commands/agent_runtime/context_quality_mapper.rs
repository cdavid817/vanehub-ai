use super::dto;

pub(super) fn history_to_dto(
    page: crate::contexts::agent_runtime::api::ContextQualityAssessmentPage,
) -> dto::ContextQualityHistoryPage {
    dto::ContextQualityHistoryPage {
        items: page.items.into_iter().map(assessment_to_dto).collect(),
        next_cursor: page.next_cursor,
    }
}

fn assessment_to_dto(
    record: crate::contexts::agent_runtime::domain::ContextQualityAssessmentRecord,
) -> dto::ContextQualityAssessment {
    let assessment = record.assessment;
    dto::ContextQualityAssessment {
        version: assessment.version,
        attempt_id: assessment.attempt_id,
        session_correlation: record.session_correlation,
        decision_sequence: assessment.decision_sequence,
        recorded_at: record.recorded_at,
        outcome: assessment.outcome.as_str().to_string(),
        path: assessment.path.map(|value| value.as_str().to_string()),
        reason: assessment.reason.map(|value| value.as_str().to_string()),
        trigger_source: assessment
            .trigger_source
            .map(|value| value.as_str().to_string()),
        before_characters: assessment.before_characters,
        after_characters: assessment.after_characters,
        saved_characters: assessment.saved_characters,
        before_tokens: assessment.before_tokens,
        after_tokens: assessment.after_tokens,
        saved_tokens: assessment.saved_tokens,
        measurement_quality: assessment.measurement_quality.as_str().to_string(),
        invariants: assessment
            .invariants
            .map(|value| dto::ContextQualityInvariants {
                protocol_complete: value.protocol_complete,
                protected_retained: value.protected_retained,
                verbatim_retained: value.verbatim_retained,
                reinjection_complete: value.reinjection_complete,
            }),
        context_policy_version: assessment.context_policy_version,
        optimizer_version: assessment.optimizer_version,
        verifier_version: assessment.verifier_version,
    }
}

pub(super) fn summary_to_dto(
    range_days: u32,
    summary: crate::contexts::agent_runtime::api::ContextQualitySummary,
) -> dto::ContextQualitySummary {
    let measured = summary.token_measurement_count.min(summary.evaluated);
    let token_coverage_basis_points = measured
        .saturating_mul(10_000)
        .checked_div(summary.evaluated)
        .unwrap_or(0);
    dto::ContextQualitySummary {
        range_days,
        evaluated: summary.evaluated,
        saved_characters: summary.saved_characters,
        saved_tokens: summary.saved_tokens,
        token_measurement_count: measured,
        quality_coverage: dto::ContextQualityCoverage {
            measured_with_tokens: measured,
            characters_only: summary.evaluated.saturating_sub(measured),
            token_coverage_basis_points,
        },
        outcomes: summary.outcomes,
        paths: summary.paths,
        qualities: summary.qualities,
        reasons: summary.reasons,
        policy_versions: summary.policy_versions,
        earliest_recorded_at: summary.earliest_recorded_at,
        latest_recorded_at: summary.latest_recorded_at,
    }
}
