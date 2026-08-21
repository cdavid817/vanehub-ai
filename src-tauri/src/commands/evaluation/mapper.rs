use super::dto;
use crate::contexts::execution_observability::domain as model;

pub(crate) fn task(value: model::EvaluationManifest) -> dto::EvaluationTask {
    dto::EvaluationTask {
        id: value.id,
        version: value.version,
        category: json_name(&value.category),
        prompt: value.prompt,
        timeout_seconds: value.timeout_seconds,
        verifier_profiles: value.acceptance.verifier_profiles,
    }
}

pub(crate) fn arena(value: model::EvaluationArena) -> dto::EvaluationArena {
    dto::EvaluationArena {
        id: value.id,
        operation_id: value.operation_id,
        task_id: value.task_id,
        task_version: value.task_version,
        ranking_version: value.ranking_version,
        attempts: value.attempts.into_iter().map(attempt).collect(),
    }
}

pub(crate) fn attempt(value: model::EvaluationAttempt) -> dto::EvaluationAttempt {
    let mut timeline = vec![serde_json::json!({
        "id": format!("{}-lifecycle", value.id),
        "kind": "lifecycle",
        "label": "Canonical evaluation attempt",
        "status": json_name(&value.outcome),
    })];
    timeline.extend(value.checks.iter().map(|check| {
        serde_json::json!({
            "id": format!("{}-{}", value.id, check.check_id),
            "kind": "verification",
            "label": check.check_id,
            "status": if check.passed { "passed" } else { "failed" },
        })
    }));
    if let Some(manifest_id) = value.context_evidence_manifest_id.as_deref() {
        timeline.push(serde_json::json!({
            "id": format!("{}-context", value.id), "kind": "context",
            "label": manifest_id, "status": "linked",
        }));
    }
    dto::EvaluationAttempt {
        id: value.id,
        arena_id: value.arena_id,
        canonical_run_id: value.canonical_run_id,
        task_id: value.task_id,
        task_version: value.task_version,
        agent: dto::EvaluationAgentSnapshot {
            agent_id: value.agent.agent_id,
            provider_id: value.agent.provider_id,
            model_id: value.agent.model_id,
            interaction_mode: value.agent.interaction_mode,
            configuration_fingerprint: value.agent.configuration_fingerprint,
        },
        outcome: json_name(&value.outcome),
        checks: value
            .checks
            .into_iter()
            .map(|check| dto::EvaluationCheck {
                check_id: check.check_id,
                passed: check.passed,
                summary: check.summary,
            })
            .collect(),
        judge: value
            .judge
            .and_then(|judge| serde_json::to_value(judge).ok()),
        metrics: value
            .metrics
            .into_iter()
            .map(|metric| dto::EvaluationMetric {
                name: metric.name,
                value: metric.value,
                unit: metric.unit,
                quality: json_name(&metric.quality),
                source: metric.source,
            })
            .collect(),
        context_evidence_manifest_id: value.context_evidence_manifest_id,
        artifact_ids: value.artifact_ids,
        timeline,
    }
}

pub(crate) fn export(value: model::EvaluationExport) -> dto::EvaluationExport {
    dto::EvaluationExport {
        schema_version: value.schema_version,
        arena: arena(value.arena),
    }
}

fn json_name<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|item| item.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Delegates to the domain rule so a failure redacts identically whether it leaves through a
/// command's `Err` or through a diagnostic check recorded on the attempt.
pub(crate) fn safe_error(error: String) -> String {
    model::safe_evaluation_error(error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::execution_observability::domain::{
        EvaluationAgentSnapshot, EvaluationAttempt, EvaluationOutcome,
    };

    #[test]
    fn command_dto_is_camel_case_and_error_mapping_hides_paths() {
        let mapped = attempt(EvaluationAttempt {
            id: "attempt-1".into(),
            arena_id: "arena-1".into(),
            canonical_run_id: "run-1".into(),
            task_id: "task".into(),
            task_version: 1,
            agent: EvaluationAgentSnapshot {
                agent_id: "fake".into(),
                provider_id: "local".into(),
                model_id: None,
                interaction_mode: "fake".into(),
                configuration_fingerprint: "hash".into(),
            },
            outcome: EvaluationOutcome::Queued,
            checks: Vec::new(),
            judge: None,
            metrics: Vec::new(),
            context_evidence_manifest_id: None,
            artifact_ids: Vec::new(),
        });
        let json = serde_json::to_value(mapped).expect("serialize");
        assert_eq!(json["canonicalRunId"], "run-1");
        assert!(json.get("canonical_run_id").is_none());
        assert!(!safe_error("database failed at /home/user/private.db".into()).contains("/home"));
    }
}
