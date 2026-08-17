use super::super::domain::{EvaluationCheck, EvaluationJudgeAssessment, EvaluationOutcome};
use std::path::{Component, Path};

const MAX_SUMMARY_CHARS: usize = 1_000;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct VerificationAggregate {
    pub(crate) outcome: EvaluationOutcome,
    pub(crate) checks: Vec<EvaluationCheck>,
    pub(crate) flaky: bool,
    pub(crate) judge: Option<EvaluationJudgeAssessment>,
}

pub(crate) fn aggregate_verification(
    checks: Vec<EvaluationCheck>,
    repeated_checks: Option<&[EvaluationCheck]>,
    judge: Option<EvaluationJudgeAssessment>,
) -> VerificationAggregate {
    let flaky = repeated_checks.is_some_and(|repeated| repeated != checks.as_slice());
    let deterministic_passed = checks.iter().all(|check| check.passed) && !flaky;
    VerificationAggregate {
        outcome: if deterministic_passed {
            EvaluationOutcome::Succeeded
        } else {
            EvaluationOutcome::TaskFailed
        },
        checks,
        flaky,
        judge: judge.map(bound_judge),
    }
}

pub(crate) fn verify_diff_rules(changed_paths: &[String]) -> EvaluationCheck {
    let bounded = !changed_paths.is_empty()
        && changed_paths.len() <= 256
        && changed_paths
            .iter()
            .all(|path| bounded_child(Path::new("."), path).is_ok() && path.len() <= 240);
    EvaluationCheck {
        check_id: "diff-rules".to_string(),
        passed: bounded,
        summary: if bounded {
            format!("{} bounded changed paths", changed_paths.len())
        } else {
            "diff is empty, oversized, or contains an escaping path".to_string()
        },
    }
}

fn bound_judge(mut judge: EvaluationJudgeAssessment) -> EvaluationJudgeAssessment {
    judge.confidence = judge.confidence.map(|value| value.clamp(0.0, 1.0));
    judge.notes = judge.notes.chars().take(MAX_SUMMARY_CHARS).collect();
    judge.evidence_ids.truncate(32);
    judge
}

fn bounded_child(root: &Path, relative: &str) -> Result<std::path::PathBuf, String> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("evaluation assertion path escapes workspace".to_string());
    }
    Ok(root.join(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn judge_never_overrides_deterministic_failure_or_flaky_result() {
        let failed = EvaluationCheck {
            check_id: "tests".into(),
            passed: false,
            summary: "failed".into(),
        };
        let judge = EvaluationJudgeAssessment {
            model_id: "judge".into(),
            rubric_version: "v1".into(),
            prompt_version: "v1".into(),
            seed: Some(7),
            temperature: Some(0.0),
            passed: true,
            confidence: Some(2.0),
            evidence_ids: vec!["artifact-1".into()],
            notes: "ok".into(),
        };
        let result = aggregate_verification(vec![failed], None, Some(judge));
        assert_eq!(result.outcome, EvaluationOutcome::TaskFailed);
        assert_eq!(result.judge.and_then(|value| value.confidence), Some(1.0));
    }

    #[test]
    fn diff_rules_reject_empty_escape_and_oversized_change_sets() {
        assert!(verify_diff_rules(&["src/lib.rs".into()]).passed);
        assert!(!verify_diff_rules(&[]).passed);
        assert!(!verify_diff_rules(&["../escape".into()]).passed);
        assert!(!verify_diff_rules(&vec!["safe.rs".into(); 257]).passed);
    }
}
