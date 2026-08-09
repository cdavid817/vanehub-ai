use super::PlanApplicationError;
use crate::contexts::task_orchestration::domain::{
    DependencyEdge, PlanDraft, ResourceLimits, SubTaskSpec, VerificationCommand,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

const PLANNER_INSTRUCTIONS: &str = include_str!("../assets/plan_planner_system.md");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GeneratePlanDraftRequest {
    pub(crate) plan_id: Option<String>,
    pub(crate) version: u32,
    pub(crate) goal: String,
    pub(crate) project_path: String,
    pub(crate) base_ref: String,
    pub(crate) available_tools: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlanGenerationRequest {
    pub(crate) prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlanGenerationResponse {
    pub(crate) content: String,
    pub(crate) active_profile_id: String,
}

pub(crate) trait PlanGenerationPort: Send + Sync {
    fn generate(
        &self,
        request: &PlanGenerationRequest,
    ) -> Result<PlanGenerationResponse, PlanApplicationError>;
}

pub(crate) fn build_planner_prompt(request: &GeneratePlanDraftRequest) -> String {
    let tools = if request.available_tools.is_empty() {
        "No execution tools are available.".to_string()
    } else {
        request
            .available_tools
            .iter()
            .map(|tool| format!("- {tool}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "{PLANNER_INSTRUCTIONS}\n\n## Available execution tools\n{tools}\n\n## User goal\n{}",
        request.goal.trim()
    )
}

pub(crate) fn parse_planner_response(
    request: &GeneratePlanDraftRequest,
    response: PlanGenerationResponse,
) -> Result<PlanDraft, PlanApplicationError> {
    let value: PlannerOutput =
        serde_json::from_str(strip_json_fence(&response.content)).map_err(|error| {
            PlanApplicationError::Validation(format!("invalid planner JSON: {error}"))
        })?;
    let plan_id = request
        .plan_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let version_id = Uuid::new_v4().to_string();
    let mut ids = BTreeMap::new();
    let mut subtasks = Vec::with_capacity(value.subtasks.len());
    for (ordinal, proposal) in value.subtasks.into_iter().enumerate() {
        if ids.contains_key(&proposal.id) {
            return Err(PlanApplicationError::Validation(format!(
                "planner SubTask id `{}` is duplicated",
                proposal.id
            )));
        }
        let id = Uuid::new_v4().to_string();
        ids.insert(proposal.id, id.clone());
        subtasks.push(SubTaskSpec {
            id,
            title: proposal.title,
            description: proposal.description,
            acceptance_criteria: proposal.acceptance_criteria,
            ordinal: u16::try_from(ordinal).map_err(|_| {
                PlanApplicationError::Validation("too many planner SubTasks".to_string())
            })?,
            assigned_role: "worker".to_string(),
            limits: ResourceLimits {
                token_budget: proposal.token_budget,
                tool_call_limit: proposal.tool_call_limit,
                timeout_seconds: proposal.timeout_seconds,
            },
            validation_commands: proposal.validation_commands,
        });
    }
    let dependencies = value
        .dependencies
        .into_iter()
        .map(|edge| {
            let predecessor_id = ids.get(&edge.predecessor_id).cloned().ok_or_else(|| {
                PlanApplicationError::Validation(format!(
                    "unknown planner dependency endpoint `{}`",
                    edge.predecessor_id
                ))
            })?;
            let successor_id = ids.get(&edge.successor_id).cloned().ok_or_else(|| {
                PlanApplicationError::Validation(format!(
                    "unknown planner dependency endpoint `{}`",
                    edge.successor_id
                ))
            })?;
            Ok(DependencyEdge {
                predecessor_id,
                successor_id,
            })
        })
        .collect::<Result<Vec<_>, PlanApplicationError>>()?;
    Ok(PlanDraft {
        id: plan_id,
        version_id,
        version: request.version,
        goal: request.goal.trim().to_string(),
        project_path: request.project_path.trim().to_string(),
        base_ref: request.base_ref.trim().to_string(),
        planner_profile_id: Some(response.active_profile_id),
        subtasks,
        dependencies,
    })
}

fn strip_json_fence(content: &str) -> &str {
    let trimmed = content.trim();
    let trimmed = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed)
        .trim();
    trimmed.strip_suffix("```").unwrap_or(trimmed).trim()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PlannerOutput {
    subtasks: Vec<PlannerSubTask>,
    dependencies: Vec<PlannerDependency>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PlannerSubTask {
    id: String,
    title: String,
    description: String,
    acceptance_criteria: Vec<String>,
    #[serde(default)]
    token_budget: Option<u32>,
    #[serde(default)]
    tool_call_limit: Option<u32>,
    #[serde(default)]
    timeout_seconds: Option<u64>,
    #[serde(default)]
    validation_commands: Vec<VerificationCommand>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PlannerDependency {
    predecessor_id: String,
    successor_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::task_orchestration::domain::validate_plan_graph;

    fn request() -> GeneratePlanDraftRequest {
        GeneratePlanDraftRequest {
            plan_id: Some("plan-1".into()),
            version: 1,
            goal: "Build Plan execution".into(),
            project_path: "C:\\code\\app".into(),
            base_ref: "main".into(),
            available_tools: vec!["shell: run validation commands".into()],
        }
    }

    #[test]
    fn prompt_contains_bounds_tools_and_goal() {
        let prompt = build_planner_prompt(&request());
        assert!(prompt.contains("at most 10"));
        assert!(prompt.contains("shell: run validation commands"));
        assert!(prompt.contains("Build Plan execution"));
    }

    #[test]
    fn strict_response_maps_local_ids_and_validates() {
        let response = PlanGenerationResponse {
            active_profile_id: "profile-1".into(),
            content: r#"{
                "subtasks": [
                    {"id":"a","title":"Design","description":"Design it","acceptanceCriteria":["Design accepted"]},
                    {"id":"b","title":"Build","description":"Build it","acceptanceCriteria":["Tests pass"]}
                ],
                "dependencies": [{"predecessorId":"a","successorId":"b"}]
            }"#
            .into(),
        };
        let draft = parse_planner_response(&request(), response).expect("parse");
        assert_eq!(draft.planner_profile_id.as_deref(), Some("profile-1"));
        assert_ne!(draft.subtasks[0].id, "a");
        assert_eq!(draft.dependencies[0].predecessor_id, draft.subtasks[0].id);
        validate_plan_graph(&draft).expect("valid graph");
    }

    #[test]
    fn response_rejects_unknown_fields_and_endpoints() {
        let unknown = PlanGenerationResponse {
            active_profile_id: "profile-1".into(),
            content: r#"{"subtasks":[],"dependencies":[],"secret":"no"}"#.into(),
        };
        assert!(parse_planner_response(&request(), unknown).is_err());

        let missing = PlanGenerationResponse {
            active_profile_id: "profile-1".into(),
            content: r#"{
                "subtasks":[{"id":"a","title":"A","description":"A","acceptanceCriteria":["ok"]}],
                "dependencies":[{"predecessorId":"a","successorId":"missing"}]
            }"#
            .into(),
        };
        assert!(parse_planner_response(&request(), missing).is_err());
    }
}
