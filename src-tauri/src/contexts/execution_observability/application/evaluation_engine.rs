use super::super::domain::{EvaluationCheck, EvaluationMetric, EvaluationOutcome, MetricQuality};
use std::cmp::Ordering;

pub(crate) const MAX_ARENA_ATTEMPTS: usize = 8;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AgentExecutionEvidence {
    pub(crate) completed: bool,
    pub(crate) timed_out: bool,
    pub(crate) stuck: bool,
    pub(crate) cancelled: bool,
    pub(crate) tool_calls: u32,
    pub(crate) duration_ms: u64,
    pub(crate) retries: u32,
    pub(crate) replans: u32,
    pub(crate) recoveries: u32,
    pub(crate) interventions: u32,
    pub(crate) reported_input_tokens: Option<u64>,
    pub(crate) reported_output_tokens: Option<u64>,
    pub(crate) context_evidence_manifest_id: Option<String>,
    pub(crate) pricing: Option<PricingSnapshot>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PricingSnapshot {
    pub(crate) id: String,
    pub(crate) currency: String,
    pub(crate) input_per_million: f64,
    pub(crate) output_per_million: f64,
}

pub(crate) trait EvaluationVerifierPort: Send + Sync {
    fn verify(&self, profile: &str, workspace: &str) -> Result<EvaluationCheck, String>;
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EvaluationAggregate {
    pub(crate) outcome: EvaluationOutcome,
    pub(crate) checks: Vec<EvaluationCheck>,
    pub(crate) metrics: Vec<EvaluationMetric>,
    pub(crate) flaky: bool,
}

pub(crate) fn aggregate_evaluation(
    execution: Result<AgentExecutionEvidence, String>,
    checks: Vec<Result<EvaluationCheck, String>>,
    repeated_checks: Option<&[EvaluationCheck]>,
) -> EvaluationAggregate {
    let evidence = match execution {
        Ok(evidence) => evidence,
        Err(_) => return aggregate_error(EvaluationOutcome::AgentFailed),
    };
    if evidence.timed_out {
        return aggregate_error(EvaluationOutcome::TimedOut);
    }
    if evidence.stuck {
        return aggregate_error(EvaluationOutcome::Stuck);
    }
    if evidence.cancelled {
        return aggregate_error(EvaluationOutcome::Cancelled);
    }
    if !evidence.completed {
        return aggregate_error(EvaluationOutcome::AgentFailed);
    }
    let checks: Vec<_> = match checks.into_iter().collect::<Result<Vec<_>, _>>() {
        Ok(checks) => checks,
        Err(_) => return aggregate_error(EvaluationOutcome::BenchmarkError),
    };
    let flaky = repeated_checks.is_some_and(|repeat| repeat != checks.as_slice());
    let passed = checks.iter().all(|check| check.passed) && !flaky;
    EvaluationAggregate {
        outcome: if passed {
            EvaluationOutcome::Succeeded
        } else {
            EvaluationOutcome::TaskFailed
        },
        metrics: vec![
            metric(
                "tool_calls",
                Some(f64::from(evidence.tool_calls)),
                MetricQuality::Reported,
                "runtime",
                "count",
            ),
            token_metric("input_tokens", evidence.reported_input_tokens),
            token_metric("output_tokens", evidence.reported_output_tokens),
            metric(
                "duration",
                Some(evidence.duration_ms as f64),
                MetricQuality::Reported,
                "runtime",
                "ms",
            ),
            metric(
                "retries",
                Some(f64::from(evidence.retries)),
                MetricQuality::Reported,
                "canonical_run",
                "count",
            ),
            metric(
                "replans",
                Some(f64::from(evidence.replans)),
                MetricQuality::Reported,
                "canonical_run",
                "count",
            ),
            metric(
                "recoveries",
                Some(f64::from(evidence.recoveries)),
                MetricQuality::Reported,
                "canonical_run",
                "count",
            ),
            metric(
                "interventions",
                Some(f64::from(evidence.interventions)),
                MetricQuality::Reported,
                "canonical_run",
                "count",
            ),
            cost_metric(&evidence),
            context_metric(evidence.context_evidence_manifest_id.as_deref()),
        ],
        checks,
        flaky,
    }
}

pub(crate) fn compare_aggregates(
    left: &EvaluationAggregate,
    right: &EvaluationAggregate,
) -> Ordering {
    success_rank(&right.outcome)
        .cmp(&success_rank(&left.outcome))
        .then_with(|| failed_checks(left).cmp(&failed_checks(right)))
        .then_with(|| comparable_metric(left, right, "interventions"))
        .then_with(|| comparable_metric(left, right, "tool_calls"))
}

fn aggregate_error(outcome: EvaluationOutcome) -> EvaluationAggregate {
    EvaluationAggregate {
        outcome,
        checks: Vec::new(),
        metrics: Vec::new(),
        flaky: false,
    }
}

fn metric(
    name: &str,
    value: Option<f64>,
    quality: MetricQuality,
    source: &str,
    unit: &str,
) -> EvaluationMetric {
    EvaluationMetric {
        name: name.into(),
        value,
        unit: unit.into(),
        quality,
        source: source.into(),
    }
}

fn token_metric(name: &str, value: Option<u64>) -> EvaluationMetric {
    metric(
        name,
        value.map(|item| item as f64),
        if value.is_some() {
            MetricQuality::Reported
        } else {
            MetricQuality::Unavailable
        },
        "provider",
        "tokens",
    )
}

fn cost_metric(evidence: &AgentExecutionEvidence) -> EvaluationMetric {
    let value = evidence.pricing.as_ref().and_then(|pricing| {
        Some(
            (evidence.reported_input_tokens? as f64 * pricing.input_per_million
                + evidence.reported_output_tokens? as f64 * pricing.output_per_million)
                / 1_000_000.0,
        )
    });
    let source = evidence
        .pricing
        .as_ref()
        .map(|value| value.id.as_str())
        .unwrap_or("pricing-unavailable");
    let unit = evidence
        .pricing
        .as_ref()
        .map(|value| value.currency.as_str())
        .unwrap_or("currency");
    metric(
        "cost",
        value,
        if value.is_some() {
            MetricQuality::Estimated
        } else {
            MetricQuality::Unavailable
        },
        source,
        unit,
    )
}

fn context_metric(manifest_id: Option<&str>) -> EvaluationMetric {
    metric(
        "context_evidence",
        manifest_id.map(|_| 1.0),
        if manifest_id.is_some() {
            MetricQuality::Reported
        } else {
            MetricQuality::Unavailable
        },
        manifest_id.unwrap_or("context-evidence-unavailable"),
        "manifest",
    )
}

fn success_rank(outcome: &EvaluationOutcome) -> u8 {
    u8::from(matches!(outcome, EvaluationOutcome::Succeeded))
}
fn failed_checks(result: &EvaluationAggregate) -> usize {
    result.checks.iter().filter(|check| !check.passed).count()
}
fn comparable_metric(
    left: &EvaluationAggregate,
    right: &EvaluationAggregate,
    name: &str,
) -> Ordering {
    let value = |result: &EvaluationAggregate| {
        result
            .metrics
            .iter()
            .find(|metric| metric.name == name)
            .and_then(|metric| metric.value)
    };
    match (value(left), value(right)) {
        (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
        _ => Ordering::Equal,
    }
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct DeterministicFakeAgent;

#[cfg(test)]
impl DeterministicFakeAgent {
    pub(crate) fn execute(
        &self,
        task_id: &str,
        _workspace: &str,
    ) -> Result<AgentExecutionEvidence, String> {
        Ok(AgentExecutionEvidence {
            completed: !task_id.contains("agent-failure"),
            timed_out: task_id.contains("timeout"),
            stuck: task_id.contains("stuck"),
            cancelled: task_id.contains("cancelled"),
            tool_calls: 3,
            duration_ms: 10,
            retries: 0,
            replans: 0,
            recoveries: 0,
            interventions: 0,
            reported_input_tokens: None,
            reported_output_tokens: None,
            context_evidence_manifest_id: None,
            pricing: None,
        })
    }
}

#[cfg(test)]
#[path = "evaluation_engine_tests.rs"]
mod tests;
