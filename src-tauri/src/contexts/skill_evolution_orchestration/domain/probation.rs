use std::collections::BTreeSet;

use super::{AutoApplyProbationV1, ProbationObservationV1, ProbationStatus};

pub(crate) const PROBATION_REGRESSION_POLICY_V1: &str = "probation-regression-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProbationEvaluation {
    Active,
    Healthy,
    Regressed,
}

pub(crate) fn evaluate_probation(
    probation: &AutoApplyProbationV1,
    observations: &[ProbationObservationV1],
    now_ms: i64,
) -> Result<ProbationEvaluation, ProbationEvaluationError> {
    if probation.status != ProbationStatus::Active || now_ms < probation.starts_at_ms {
        return Err(ProbationEvaluationError::InvalidState);
    }
    // Only evidence observed inside the probation window counts: an observation recorded after
    // ends_at_ms — but before the expiry sweep runs — must not regress a probation that already
    // survived its window, so the upper bound is the window end, never the sweep time.
    let window_end = now_ms.min(probation.ends_at_ms);
    let relevant = observations.iter().filter(|observation| {
        observation.probation_id == probation.probation_id
            && observation.verified
            && observation.negative
            && (observation.baseline_exceeded || observation.harmful_correction)
            && (probation.starts_at_ms..=window_end).contains(&observation.observed_at_ms)
    });
    let mut independent_negatives = BTreeSet::new();
    let mut harmful_correction = false;
    for observation in relevant {
        let compatible = probation
            .evidence_categories
            .iter()
            .any(|category| category == &observation.safe_category);
        if compatible {
            independent_negatives.insert((
                observation.source_kind.as_str(),
                observation.source_id.as_str(),
            ));
        }
        harmful_correction |=
            observation.harmful_correction && observation.source_kind == "explicit_correction";
    }
    if harmful_correction || independent_negatives.len() >= 2 {
        Ok(ProbationEvaluation::Regressed)
    } else if now_ms >= probation.ends_at_ms {
        Ok(ProbationEvaluation::Healthy)
    } else {
        Ok(ProbationEvaluation::Active)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProbationEvaluationError {
    InvalidState,
}
