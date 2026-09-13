//! Phase-boundary scope evidence: complete manifests compared against the sealed baseline and
//! classified against the frozen scope. Violations are sticky rows; anything that could not be
//! established is recorded as unverifiable and never passes.

use super::{
    AgentClockPort, AgentRuntimeApplicationError, LoopEvidenceView, LoopIterationRepository,
    LoopIterationView, LoopManifestChangeView, LoopScopeBinding, LoopScopePlatformPort,
    LoopVerificationCancellation,
};
use crate::contexts::agent_runtime::domain::{
    CaseRule, LoopScopeConfig, LoopScopePath, ScopeClassification,
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

pub(crate) const SCOPE_EVIDENCE_KIND: &str = "scope-evidence";
const MAX_RECORDED_VIOLATIONS: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PhaseEvidenceOutcome {
    Passed {
        manifest_id: String,
        digest: String,
        changes: usize,
    },
    Violation {
        violations: Vec<String>,
    },
    Unverifiable {
        reason: String,
    },
}

/// Captures a stable manifest at a phase boundary, diffs it against the baseline and records
/// one evidence row. The caller decides the run transition from the returned outcome.
#[allow(clippy::too_many_arguments)]
pub(crate) fn seal_phase_evidence(
    platform: &Arc<dyn LoopScopePlatformPort>,
    iterations: &Arc<dyn LoopIterationRepository>,
    clock: &Arc<dyn AgentClockPort>,
    binding: &LoopScopeBinding,
    run_id: &str,
    iteration: &LoopIterationView,
    worktree_path: &str,
    phase: &str,
    operation_id: Option<&str>,
    cancellation: &LoopVerificationCancellation,
) -> Result<PhaseEvidenceOutcome, AgentRuntimeApplicationError> {
    let manifest_id = format!("{phase}-{}-{}", iteration.sequence, Uuid::new_v4());
    let outcome = capture_and_classify(
        platform,
        binding,
        run_id,
        worktree_path,
        &manifest_id,
        cancellation,
    );
    let (status, summary, details) = match &outcome {
        PhaseEvidenceOutcome::Passed {
            manifest_id,
            digest,
            changes,
        } => (
            "passed",
            format!("Scope evidence for the {phase} phase is complete with no violations."),
            json!({
                "phase": phase,
                "manifestId": manifest_id,
                "manifestDigest": digest,
                "baselineDigest": binding.baseline_digest,
                "scopeDigest": binding.scope_digest,
                "changes": changes,
                "requestedMode": binding.requested_mode,
            }),
        ),
        PhaseEvidenceOutcome::Violation { violations } => (
            "violation",
            format!(
                "Scope evidence for the {phase} phase found {} protected or out-of-scope change(s).",
                violations.len()
            ),
            json!({
                "phase": phase,
                "baselineDigest": binding.baseline_digest,
                "scopeDigest": binding.scope_digest,
                "violations": violations,
                "sticky": true,
                "requestedMode": binding.requested_mode,
            }),
        ),
        PhaseEvidenceOutcome::Unverifiable { reason } => (
            "unverifiable",
            format!("Scope evidence for the {phase} phase could not be completed."),
            json!({
                "phase": phase,
                "reason": reason,
                "scopeDigest": binding.scope_digest,
                "requestedMode": binding.requested_mode,
            }),
        ),
    };
    iterations.append_evidence(&LoopEvidenceView {
        id: format!("loop-evidence-{}", Uuid::new_v4()),
        run_id: run_id.to_string(),
        iteration_id: Some(iteration.id.clone()),
        kind: SCOPE_EVIDENCE_KIND.to_string(),
        status: status.to_string(),
        summary,
        operation_id: operation_id.map(str::to_string),
        command_id: None,
        exit_code: None,
        duration_ms: None,
        details: Some(details),
        created_at: clock.now(),
    })?;
    Ok(outcome)
}

fn capture_and_classify(
    platform: &Arc<dyn LoopScopePlatformPort>,
    binding: &LoopScopeBinding,
    run_id: &str,
    worktree_path: &str,
    manifest_id: &str,
    cancellation: &LoopVerificationCancellation,
) -> PhaseEvidenceOutcome {
    if let Err(failure) = platform.verify_root(worktree_path, &binding.root) {
        return PhaseEvidenceOutcome::Unverifiable {
            reason: format!("{}: {}", failure.code, failure.message),
        };
    }
    let scope = match binding.scope() {
        Ok(scope) => scope,
        Err(error) => {
            return PhaseEvidenceOutcome::Unverifiable {
                reason: error.to_string(),
            }
        }
    };
    let manifest = match platform.capture_manifest(run_id, worktree_path, manifest_id, cancellation)
    {
        Ok(manifest) => manifest,
        Err(failure) => {
            return PhaseEvidenceOutcome::Unverifiable {
                reason: format!("{}: {}", failure.code, failure.message),
            }
        }
    };
    let changes = match platform.diff_manifests(run_id, &binding.baseline_manifest_id, manifest_id)
    {
        Ok(changes) => changes,
        Err(failure) => {
            return PhaseEvidenceOutcome::Unverifiable {
                reason: format!("{}: {}", failure.code, failure.message),
            }
        }
    };
    let case = if binding.root.case_rule == CaseRule::InsensitiveAscii.as_str() {
        CaseRule::InsensitiveAscii
    } else {
        CaseRule::Sensitive
    };
    let violations = classify_changes(&scope, &changes, case);
    if violations.is_empty() {
        PhaseEvidenceOutcome::Passed {
            manifest_id: manifest.manifest_id,
            digest: manifest.digest,
            changes: changes.len(),
        }
    } else {
        PhaseEvidenceOutcome::Violation { violations }
    }
}

/// Every changed entry must be an allowed mutation. Paths come from the scanner rather than
/// from configuration syntax, so they are split literally instead of re-validated.
pub(crate) fn classify_changes(
    scope: &LoopScopeConfig,
    changes: &[LoopManifestChangeView],
    case: CaseRule,
) -> Vec<String> {
    let mut violations = Vec::new();
    for change in changes {
        let path = LoopScopePath::from_components(
            change
                .path
                .split('/')
                .filter(|component| !component.is_empty())
                .map(str::to_string)
                .collect(),
        );
        let classification = scope.classify(&path, case);
        if classification != ScopeClassification::Allowed {
            if violations.len() < MAX_RECORDED_VIOLATIONS {
                violations.push(format!(
                    "{} [{}] is {}",
                    change.path,
                    change.kind,
                    classification.as_str()
                ));
            } else {
                break;
            }
        }
    }
    violations
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScopeEvidenceState {
    Complete { verifier_manifest_digest: String },
    Violation,
    Unverifiable,
    Incomplete,
}

/// Reads the sticky state of one iteration's scope evidence. A violation anywhere in the
/// iteration dominates; completeness requires a passed row for every phase.
pub(crate) fn scope_evidence_state(iteration: &LoopIterationView) -> ScopeEvidenceState {
    let rows: Vec<&LoopEvidenceView> = iteration
        .evidence
        .iter()
        .filter(|item| item.kind == SCOPE_EVIDENCE_KIND)
        .collect();
    if rows.iter().any(|item| item.status == "violation") {
        return ScopeEvidenceState::Violation;
    }
    let latest_for = |phase: &str| {
        rows.iter()
            .rev()
            .find(|item| phase_of(item) == Some(phase))
            .copied()
    };
    let phases = ["worker", "verification", "verifier"];
    let mut verifier_digest = None;
    for phase in phases {
        match latest_for(phase) {
            Some(item) if item.status == "passed" => {
                if phase == "verifier" {
                    verifier_digest = item
                        .details
                        .as_ref()
                        .and_then(|details| details.get("manifestDigest"))
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string);
                }
            }
            Some(item) if item.status == "unverifiable" => return ScopeEvidenceState::Unverifiable,
            _ => return ScopeEvidenceState::Incomplete,
        }
    }
    match verifier_digest {
        Some(digest) => ScopeEvidenceState::Complete {
            verifier_manifest_digest: digest,
        },
        None => ScopeEvidenceState::Incomplete,
    }
}

pub(crate) fn latest_passed_manifest(
    iteration: &LoopIterationView,
    phase: &str,
) -> Option<(String, String)> {
    iteration
        .evidence
        .iter()
        .rev()
        .filter(|item| item.kind == SCOPE_EVIDENCE_KIND && item.status == "passed")
        .find(|item| phase_of(item) == Some(phase))
        .and_then(|item| {
            let details = item.details.as_ref()?;
            Some((
                details.get("manifestId")?.as_str()?.to_string(),
                details.get("manifestDigest")?.as_str()?.to_string(),
            ))
        })
}

fn phase_of(item: &LoopEvidenceView) -> Option<&str> {
    item.details
        .as_ref()
        .and_then(|details| details.get("phase"))
        .and_then(serde_json::Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::domain::LoopRunStatus;

    fn scope() -> LoopScopeConfig {
        LoopScopeConfig::parse(&["src".to_string()], &["src/generated".to_string()]).expect("scope")
    }

    fn change(path: &str) -> LoopManifestChangeView {
        LoopManifestChangeView {
            path: path.to_string(),
            kind: "modified".to_string(),
        }
    }

    #[test]
    fn ignored_hidden_and_reserved_changes_are_violations_when_outside_scope() {
        let violations = classify_changes(
            &scope(),
            &[
                change("src/app.ts"),
                change("src/generated/out.js"),
                change("node_modules/.cache/x"),
                change(".git/index"),
                change("src/file:with:colon.txt"),
            ],
            CaseRule::Sensitive,
        );
        assert_eq!(violations.len(), 3);
        assert!(violations[0].contains("src/generated/out.js"));
        assert!(violations[0].contains("protected"));
        assert!(violations[1].contains("node_modules/.cache/x"));
        assert!(violations[2].contains(".git/index"));
        assert!(violations[2].contains("reserved"));
    }

    fn evidence(phase: &str, status: &str, digest: &str) -> LoopEvidenceView {
        LoopEvidenceView {
            id: format!("{phase}-{status}"),
            run_id: "run".to_string(),
            iteration_id: Some("iteration".to_string()),
            kind: SCOPE_EVIDENCE_KIND.to_string(),
            status: status.to_string(),
            summary: String::new(),
            operation_id: None,
            command_id: None,
            exit_code: None,
            duration_ms: None,
            details: Some(json!({"phase": phase, "manifestDigest": digest, "manifestId": "m"})),
            created_at: "t".to_string(),
        }
    }

    fn iteration(rows: Vec<LoopEvidenceView>) -> LoopIterationView {
        LoopIterationView {
            id: "iteration".to_string(),
            run_id: "run".to_string(),
            sequence: 1,
            status: LoopRunStatus::Running,
            worker_session_id: None,
            verifier_session_id: None,
            worker_summary: None,
            verifier_recommendation: None,
            verifier_findings: Vec::new(),
            decision_reason: None,
            diff_fingerprint: None,
            check_failure_fingerprint: None,
            user_feedback: None,
            evidence: rows,
            started_at: "t".to_string(),
            completed_at: None,
        }
    }

    #[test]
    fn a_recorded_violation_is_sticky_and_completeness_needs_every_phase() {
        assert_eq!(
            scope_evidence_state(&iteration(vec![
                evidence("worker", "violation", "a"),
                evidence("worker", "passed", "b"),
                evidence("verification", "passed", "c"),
                evidence("verifier", "passed", "d"),
            ])),
            ScopeEvidenceState::Violation
        );
        assert_eq!(
            scope_evidence_state(&iteration(vec![
                evidence("worker", "passed", "a"),
                evidence("verification", "passed", "b"),
            ])),
            ScopeEvidenceState::Incomplete
        );
        assert_eq!(
            scope_evidence_state(&iteration(vec![
                evidence("worker", "passed", "a"),
                evidence("verification", "unverifiable", "b"),
                evidence("verifier", "passed", "c"),
            ])),
            ScopeEvidenceState::Unverifiable
        );
        assert_eq!(
            scope_evidence_state(&iteration(vec![
                evidence("worker", "passed", "a"),
                evidence("verification", "passed", "b"),
                evidence("verifier", "passed", "c"),
            ])),
            ScopeEvidenceState::Complete {
                verifier_manifest_digest: "c".to_string()
            }
        );
    }
}
