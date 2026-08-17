use super::*;

fn check(passed: bool) -> EvaluationCheck {
    EvaluationCheck {
        check_id: "tests".into(),
        passed,
        summary: "bounded".into(),
    }
}

fn evidence() -> AgentExecutionEvidence {
    AgentExecutionEvidence {
        completed: true,
        timed_out: false,
        stuck: false,
        cancelled: false,
        tool_calls: 2,
        duration_ms: 10,
        retries: 0,
        replans: 0,
        recoveries: 0,
        interventions: 0,
        reported_input_tokens: None,
        reported_output_tokens: None,
        context_evidence_manifest_id: None,
        pricing: None,
    }
}

#[test]
fn deterministic_failure_and_missing_values_are_preserved() {
    let result = aggregate_evaluation(Ok(evidence()), vec![Ok(check(false))], None);
    assert_eq!(result.outcome, EvaluationOutcome::TaskFailed);
    assert!(result
        .metrics
        .iter()
        .filter(|metric| metric.name.contains("tokens"))
        .all(|metric| metric.value.is_none() && metric.quality == MetricQuality::Unavailable));
}

#[test]
fn harness_timeout_stuck_and_agent_failures_are_distinct() {
    assert_eq!(
        aggregate_evaluation(Ok(evidence()), vec![Err("harness".into())], None).outcome,
        EvaluationOutcome::BenchmarkError
    );
    let mut timed_out = evidence();
    timed_out.timed_out = true;
    let mut stuck = evidence();
    stuck.stuck = true;
    let mut cancelled = evidence();
    cancelled.cancelled = true;
    assert_eq!(
        aggregate_evaluation(Ok(timed_out), vec![], None).outcome,
        EvaluationOutcome::TimedOut
    );
    assert_eq!(
        aggregate_evaluation(Ok(stuck), vec![], None).outcome,
        EvaluationOutcome::Stuck
    );
    assert_eq!(
        aggregate_evaluation(Ok(cancelled), vec![], None).outcome,
        EvaluationOutcome::Cancelled
    );
    assert_eq!(
        aggregate_evaluation(Err("agent".into()), vec![], None).outcome,
        EvaluationOutcome::AgentFailed
    );
}

#[test]
fn flaky_and_ranking_precedence_are_deterministic() {
    let success = aggregate_evaluation(Ok(evidence()), vec![Ok(check(true))], None);
    let failed = aggregate_evaluation(Ok(evidence()), vec![Ok(check(false))], None);
    assert_eq!(compare_aggregates(&success, &failed), Ordering::Less);
    let flaky = aggregate_evaluation(Ok(evidence()), vec![Ok(check(true))], Some(&[check(false)]));
    assert!(flaky.flaky);
    let mut intervened = evidence();
    intervened.interventions = 1;
    intervened.tool_calls = 1;
    let mut autonomous = evidence();
    autonomous.tool_calls = 50;
    assert_eq!(
        compare_aggregates(
            &aggregate_evaluation(Ok(autonomous), vec![], None),
            &aggregate_evaluation(Ok(intervened), vec![], None)
        ),
        Ordering::Less
    );
}

#[test]
fn fake_agent_is_deterministic_and_never_invents_tokens() {
    let first = DeterministicFakeAgent
        .execute("fix-null-auth-token", "a")
        .expect("fake");
    let second = DeterministicFakeAgent
        .execute("fix-null-auth-token", "b")
        .expect("fake");
    assert_eq!(first, second);
    assert_eq!(first.reported_input_tokens, None);
}

#[test]
fn pricing_and_context_keep_snapshot_provenance() {
    let mut measured = evidence();
    measured.pricing = Some(PricingSnapshot {
        id: "pricing-2026-08".into(),
        currency: "USD".into(),
        input_per_million: 1.0,
        output_per_million: 2.0,
    });
    assert_eq!(
        aggregate_evaluation(Ok(measured.clone()), vec![], None)
            .metrics
            .iter()
            .find(|metric| metric.name == "cost")
            .and_then(|metric| metric.value),
        None
    );
    measured.reported_input_tokens = Some(1_000_000);
    measured.reported_output_tokens = Some(500_000);
    measured.context_evidence_manifest_id = Some("manifest-1".into());
    let result = aggregate_evaluation(Ok(measured), vec![], None);
    let cost = result
        .metrics
        .iter()
        .find(|metric| metric.name == "cost")
        .expect("cost");
    assert_eq!(
        (cost.value, cost.source.as_str(), cost.unit.as_str()),
        (Some(2.0), "pricing-2026-08", "USD")
    );
    assert_eq!(
        result
            .metrics
            .iter()
            .find(|metric| metric.name == "context_evidence")
            .map(|metric| metric.source.as_str()),
        Some("manifest-1")
    );
}
