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

fn timed_out_evidence() -> AgentExecutionEvidence {
    AgentExecutionEvidence {
        completed: false,
        timed_out: true,
        ..evidence()
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

/// An empty check list is absence of evidence, not evidence of zero failures. Before the outcome
/// tier existed, `failed_checks` compared 0 against 1 and ranked an Agent that crashed before
/// writing a line ahead of one that produced a patch failing a single test.
#[test]
fn recorded_failure_outranks_an_outcome_that_recorded_nothing() {
    let task_failed = aggregate_evaluation(
        Ok(evidence()),
        vec![Ok(check(false)), Ok(check(false))],
        None,
    );
    let succeeded = aggregate_evaluation(Ok(evidence()), vec![Ok(check(true))], None);
    for empty in [
        aggregate_evaluation(Err("dispatch".into()), vec![], None),
        aggregate_evaluation(Ok(timed_out_evidence()), vec![], None),
    ] {
        assert!(empty.checks.is_empty(), "the fixture recorded checks");
        assert_eq!(
            compare_aggregates(&task_failed, &empty),
            Ordering::Less,
            "a task failure with recorded checks must rank ahead of {:?}",
            empty.outcome,
        );
        assert_eq!(compare_aggregates(&succeeded, &empty), Ordering::Less);
    }
    assert_eq!(compare_aggregates(&succeeded, &task_failed), Ordering::Less);
}

/// Recording why dispatch failed must not cost the attempt its place. `EvaluationApi::execute`
/// appends a failed `agent-dispatch` check to an `agent_failed` attempt; if that counted as a
/// failed acceptance result it would sort below a timeout that recorded nothing.
#[test]
fn a_dispatch_diagnostic_does_not_rank_below_a_failure_that_recorded_nothing() {
    let mut diagnosed = aggregate_evaluation(Err("no configured model".into()), vec![], None);
    diagnosed.checks.push(EvaluationCheck {
        check_id: "agent-dispatch".into(),
        passed: false,
        summary: "API agent is missing a configured model".into(),
    });
    let silent = aggregate_evaluation(Ok(timed_out_evidence()), vec![], None);
    assert!(silent.checks.is_empty());
    assert_eq!(compare_aggregates(&diagnosed, &silent), Ordering::Equal);

    // And it still ranks behind an attempt whose work was actually verified.
    let task_failed = aggregate_evaluation(Ok(evidence()), vec![Ok(check(false))], None);
    assert_eq!(compare_aggregates(&task_failed, &diagnosed), Ordering::Less);
}

/// Within one tier the existing metric keys still decide, so the tier cannot flatten the ordering
/// it sits in front of.
#[test]
fn ordering_inside_the_non_completion_tier_still_falls_through_to_metrics() {
    let mut intervened = timed_out_evidence();
    intervened.interventions = 1;
    let autonomous = timed_out_evidence();
    let left = aggregate_evaluation(Ok(autonomous), vec![], None);
    let right = aggregate_evaluation(Ok(intervened), vec![], None);
    assert_eq!(left.outcome, right.outcome);
    // `aggregate_error` records no metrics either, so two non-completion outcomes are
    // indistinguishable here -- which is the honest answer, not a hidden ordering.
    assert_eq!(compare_aggregates(&left, &right), Ordering::Equal);

    let mut busy = evidence();
    busy.tool_calls = 50;
    assert_eq!(
        compare_aggregates(
            &aggregate_evaluation(Ok(busy), vec![Ok(check(false))], None),
            &aggregate_evaluation(Ok(evidence()), vec![Ok(check(false))], None),
        ),
        Ordering::Greater,
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
