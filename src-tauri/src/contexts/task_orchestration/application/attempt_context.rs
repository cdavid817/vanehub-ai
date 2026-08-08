use super::PlanApplicationError;
use crate::contexts::task_orchestration::domain::SubTaskSpec;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PredecessorContextSource {
    pub(crate) subtask_id: String,
    pub(crate) topological_rank: u16,
    pub(crate) ordinal: u16,
    pub(crate) outcome: String,
    pub(crate) result_summary: Option<String>,
    pub(crate) changed_files: Vec<String>,
    pub(crate) verification_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttemptContextRequest {
    pub(crate) plan_run_id: String,
    pub(crate) subtask_run_id: String,
    pub(crate) task: SubTaskSpec,
    pub(crate) direct_predecessor_ids: Vec<String>,
    pub(crate) predecessor_sources: Vec<PredecessorContextSource>,
    pub(crate) character_budget: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BoundedAttemptContext {
    pub(crate) prompt: String,
    pub(crate) predecessor_ids: Vec<String>,
    pub(crate) truncated: bool,
}

pub(crate) fn build_attempt_context(
    request: &AttemptContextRequest,
) -> Result<BoundedAttemptContext, PlanApplicationError> {
    let direct = request
        .direct_predecessor_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut predecessors = request
        .predecessor_sources
        .iter()
        .filter(|source| direct.contains(source.subtask_id.as_str()))
        .collect::<Vec<_>>();
    predecessors.sort_by_key(|source| {
        (
            source.topological_rank,
            source.ordinal,
            source.subtask_id.as_str(),
        )
    });

    let criteria = request
        .task
        .acceptance_criteria
        .iter()
        .map(|criterion| format!("- {criterion}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut prompt = format!(
        "# Plan SubTask attempt\nplanRunId: {}\nsubtaskRunId: {}\nsubtaskId: {}\ntitle: {}\ndescription: {}\nacceptanceCriteria:\n{}\n\n# Direct predecessor evidence\n",
        request.plan_run_id,
        request.subtask_run_id,
        request.task.id,
        request.task.title,
        request.task.description,
        criteria
    );
    for predecessor in &predecessors {
        prompt.push_str(&format!(
            "- subtaskId: {}; outcome: {}; verification: {}\n",
            predecessor.subtask_id,
            predecessor.outcome,
            predecessor
                .verification_summary
                .as_deref()
                .unwrap_or("none")
        ));
    }
    let essential_length = prompt.len() + "contextTruncated: true\n".len();
    if essential_length > request.character_budget {
        return Err(PlanApplicationError::Validation(format!(
            "attempt context budget {} is smaller than required identity and evidence metadata {}",
            request.character_budget, essential_length
        )));
    }

    let mut truncated = false;
    for predecessor in &predecessors {
        let optional = format!(
            "  resultSummary: {}\n  changedFiles: {}\n",
            predecessor.result_summary.as_deref().unwrap_or("none"),
            if predecessor.changed_files.is_empty() {
                "none".to_string()
            } else {
                predecessor.changed_files.join(", ")
            }
        );
        if prompt.len() + optional.len() + "contextTruncated: true\n".len()
            <= request.character_budget
        {
            prompt.push_str(&optional);
        } else {
            truncated = true;
        }
    }
    prompt.push_str(&format!("contextTruncated: {truncated}\n"));
    Ok(BoundedAttemptContext {
        prompt,
        predecessor_ids: predecessors
            .into_iter()
            .map(|source| source.subtask_id.clone())
            .collect(),
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::task_orchestration::domain::{ResourceLimits, VerificationCommand};

    fn task() -> SubTaskSpec {
        SubTaskSpec {
            id: "target".into(),
            title: "Implement".into(),
            description: "Implement the bounded change".into(),
            acceptance_criteria: vec!["Tests pass".into()],
            ordinal: 3,
            assigned_role: "worker".into(),
            limits: ResourceLimits {
                token_budget: Some(2_000),
                tool_call_limit: Some(20),
                timeout_seconds: Some(300),
            },
            validation_commands: Vec::<VerificationCommand>::new(),
        }
    }

    fn source(id: &str, rank: u16, ordinal: u16) -> PredecessorContextSource {
        PredecessorContextSource {
            subtask_id: id.into(),
            topological_rank: rank,
            ordinal,
            outcome: "succeeded".into(),
            result_summary: Some(format!("summary-{id}")),
            changed_files: vec![format!("src/{id}.rs")],
            verification_summary: Some(format!("verified-{id}")),
        }
    }

    #[test]
    fn selects_only_direct_predecessors_in_deterministic_order() {
        let result = build_attempt_context(&AttemptContextRequest {
            plan_run_id: "run-1".into(),
            subtask_run_id: "task-run-1".into(),
            task: task(),
            direct_predecessor_ids: vec!["b".into(), "a".into()],
            predecessor_sources: vec![
                source("unrelated", 0, 0),
                source("b", 1, 2),
                source("a", 1, 1),
            ],
            character_budget: 4_000,
        })
        .expect("context");
        assert_eq!(result.predecessor_ids, vec!["a", "b"]);
        assert!(!result.prompt.contains("unrelated"));
        assert!(result.prompt.find("subtaskId: a") < result.prompt.find("subtaskId: b"));
    }

    #[test]
    fn truncation_preserves_identity_outcome_and_verification_before_details() {
        let mut large = source("a", 0, 0);
        large.result_summary = Some("optional-description".repeat(200));
        let baseline = build_attempt_context(&AttemptContextRequest {
            plan_run_id: "run-1".into(),
            subtask_run_id: "task-run-1".into(),
            task: task(),
            direct_predecessor_ids: vec!["a".into()],
            predecessor_sources: vec![large.clone()],
            character_budget: 10_000,
        })
        .expect("baseline");
        let budget = baseline
            .prompt
            .find("  resultSummary")
            .expect("optional boundary")
            + 30;
        let result = build_attempt_context(&AttemptContextRequest {
            plan_run_id: "run-1".into(),
            subtask_run_id: "task-run-1".into(),
            task: task(),
            direct_predecessor_ids: vec!["a".into()],
            predecessor_sources: vec![large],
            character_budget: budget,
        })
        .expect("truncated context");
        assert!(result.truncated);
        assert!(result
            .prompt
            .contains("subtaskId: a; outcome: succeeded; verification: verified-a"));
        assert!(!result.prompt.contains("optional-description"));
    }

    #[test]
    fn prompt_model_has_no_channel_for_raw_transcripts_prompts_tools_or_credentials() {
        let result = build_attempt_context(&AttemptContextRequest {
            plan_run_id: "run-1".into(),
            subtask_run_id: "task-run-1".into(),
            task: task(),
            direct_predecessor_ids: Vec::new(),
            predecessor_sources: Vec::new(),
            character_budget: 2_000,
        })
        .expect("context");
        for prohibited in [
            "rawTranscript",
            "rawPrompt",
            "toolArguments",
            "toolResults",
            "credential",
        ] {
            assert!(!result.prompt.contains(prohibited));
        }
    }
}
