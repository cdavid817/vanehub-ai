use super::{DependencyEdge, PlanDraft};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

const MAX_SUBTASKS: usize = 10;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub(crate) enum GraphValidationError {
    #[error("a Plan must contain between one and ten SubTasks")]
    InvalidTaskCount,
    #[error("SubTask `{0}` must contain between one and three acceptance criteria")]
    InvalidAcceptanceCriteria(String),
    #[error("SubTask id `{0}` is duplicated")]
    DuplicateTask(String),
    #[error("dependency endpoint `{0}` does not exist")]
    UnknownEndpoint(String),
    #[error("SubTask `{0}` cannot depend on itself")]
    SelfDependency(String),
    #[error("dependency `{0}` -> `{1}` is duplicated")]
    DuplicateDependency(String, String),
    #[error("dependency graph contains a cycle involving: {0}")]
    Cycle(String),
    #[error("Plan field validation failed: {0}")]
    InvalidPlan(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphValidation {
    pub(crate) topological_order: Vec<String>,
    pub(crate) ranks: BTreeMap<String, u16>,
}

pub(crate) fn validate_plan_graph(
    draft: &PlanDraft,
) -> Result<GraphValidation, GraphValidationError> {
    draft
        .validate_required_fields()
        .map_err(|error| GraphValidationError::InvalidPlan(error.to_string()))?;
    if draft.subtasks.is_empty() || draft.subtasks.len() > MAX_SUBTASKS {
        return Err(GraphValidationError::InvalidTaskCount);
    }

    let mut ordinals = BTreeMap::new();
    for task in &draft.subtasks {
        if !(1..=3).contains(&task.acceptance_criteria.len()) {
            return Err(GraphValidationError::InvalidAcceptanceCriteria(
                task.id.clone(),
            ));
        }
        if ordinals.insert(task.id.clone(), task.ordinal).is_some() {
            return Err(GraphValidationError::DuplicateTask(task.id.clone()));
        }
    }

    let mut outgoing: BTreeMap<String, Vec<String>> =
        ordinals.keys().map(|id| (id.clone(), Vec::new())).collect();
    let mut indegree: BTreeMap<String, usize> = ordinals.keys().map(|id| (id.clone(), 0)).collect();
    let mut unique_edges = BTreeSet::new();
    for DependencyEdge {
        predecessor_id,
        successor_id,
    } in &draft.dependencies
    {
        if !ordinals.contains_key(predecessor_id) {
            return Err(GraphValidationError::UnknownEndpoint(
                predecessor_id.clone(),
            ));
        }
        if !ordinals.contains_key(successor_id) {
            return Err(GraphValidationError::UnknownEndpoint(successor_id.clone()));
        }
        if predecessor_id == successor_id {
            return Err(GraphValidationError::SelfDependency(predecessor_id.clone()));
        }
        if !unique_edges.insert((predecessor_id.clone(), successor_id.clone())) {
            return Err(GraphValidationError::DuplicateDependency(
                predecessor_id.clone(),
                successor_id.clone(),
            ));
        }
        outgoing
            .get_mut(predecessor_id)
            .expect("validated predecessor")
            .push(successor_id.clone());
        *indegree.get_mut(successor_id).expect("validated successor") += 1;
    }

    let sort_key = |id: &String| (ordinals[id], id.clone());
    for targets in outgoing.values_mut() {
        targets.sort_by_key(&sort_key);
    }
    let mut ready: BTreeSet<(u16, String)> = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| (ordinals[id], id.clone()))
        .collect();
    let mut order = Vec::with_capacity(draft.subtasks.len());
    let mut ranks = BTreeMap::new();
    while let Some((_, id)) = ready.pop_first() {
        let rank = ranks.get(&id).copied().unwrap_or(0);
        order.push(id.clone());
        for successor in &outgoing[&id] {
            let successor_rank = ranks.entry(successor.clone()).or_insert(0);
            *successor_rank = (*successor_rank).max(rank + 1);
            let degree = indegree.get_mut(successor).expect("known successor");
            *degree -= 1;
            if *degree == 0 {
                ready.insert((ordinals[successor], successor.clone()));
            }
        }
        ranks.entry(id).or_insert(rank);
    }

    if order.len() != draft.subtasks.len() {
        let cycle = indegree
            .into_iter()
            .filter(|(_, degree)| *degree > 0)
            .map(|(id, _)| id)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(GraphValidationError::Cycle(cycle));
    }
    Ok(GraphValidation {
        topological_order: order,
        ranks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::task_orchestration::domain::{
        CriterionEvidenceBinding, CriterionEvidenceKind, PlanDiscoveryMetadata,
        PlanExecutionPolicy, ResourceLimits, SubTaskSpec, VerificationCommand,
    };

    fn task(id: &str, ordinal: u16) -> SubTaskSpec {
        SubTaskSpec {
            id: id.into(),
            title: id.into(),
            description: format!("Implement {id}"),
            acceptance_criteria: vec![format!("{id} passes")],
            criterion_evidence: vec![CriterionEvidenceBinding {
                criterion_index: 0,
                kind: CriterionEvidenceKind::Automated,
                command_id: Some(format!("verify-{id}")),
            }],
            ordinal,
            assigned_role: "worker".into(),
            limits: ResourceLimits {
                token_budget: Some(1_000),
                tool_call_limit: Some(10),
                timeout_seconds: Some(300),
            },
            validation_commands: vec![VerificationCommand {
                id: format!("verify-{id}"),
                program: "cargo".into(),
                args: vec!["test".into()],
                working_directory: None,
                timeout_seconds: 300,
                required: true,
            }],
        }
    }

    fn draft(edges: Vec<(&str, &str)>) -> PlanDraft {
        PlanDraft {
            id: "plan-1".into(),
            version_id: "version-1".into(),
            version: 1,
            goal: "Ship the feature".into(),
            project_path: "C:\\code\\app".into(),
            base_ref: "main".into(),
            planner_profile_id: None,
            discovery: PlanDiscoveryMetadata::default(),
            execution_policy: PlanExecutionPolicy::default(),
            subtasks: vec![task("a", 2), task("b", 1), task("c", 3)],
            dependencies: edges
                .into_iter()
                .map(|(predecessor_id, successor_id)| DependencyEdge {
                    predecessor_id: predecessor_id.into(),
                    successor_id: successor_id.into(),
                })
                .collect(),
        }
    }

    #[test]
    fn produces_stable_topological_order_and_ranks() {
        let result =
            validate_plan_graph(&draft(vec![("a", "c"), ("b", "c")])).expect("valid graph");
        assert_eq!(result.topological_order, vec!["b", "a", "c"]);
        assert_eq!(result.ranks["a"], 0);
        assert_eq!(result.ranks["b"], 0);
        assert_eq!(result.ranks["c"], 1);
    }

    #[test]
    fn rejects_cycles_and_invalid_edges() {
        assert!(matches!(
            validate_plan_graph(&draft(vec![("a", "b"), ("b", "a")])),
            Err(GraphValidationError::Cycle(_))
        ));
        assert_eq!(
            validate_plan_graph(&draft(vec![("a", "missing")])),
            Err(GraphValidationError::UnknownEndpoint("missing".into()))
        );
        assert_eq!(
            validate_plan_graph(&draft(vec![("a", "a")])),
            Err(GraphValidationError::SelfDependency("a".into()))
        );
    }

    #[test]
    fn rejects_duplicate_edges_and_bad_acceptance_counts() {
        assert_eq!(
            validate_plan_graph(&draft(vec![("a", "c"), ("a", "c")])),
            Err(GraphValidationError::DuplicateDependency(
                "a".into(),
                "c".into()
            ))
        );
        let mut invalid = draft(Vec::new());
        invalid.subtasks[0].acceptance_criteria.clear();
        assert_eq!(
            validate_plan_graph(&invalid),
            Err(GraphValidationError::InvalidAcceptanceCriteria("a".into()))
        );
        invalid.subtasks[0].acceptance_criteria =
            vec!["1".into(), "2".into(), "3".into(), "4".into()];
        assert_eq!(
            validate_plan_graph(&invalid),
            Err(GraphValidationError::InvalidAcceptanceCriteria("a".into()))
        );
    }

    #[test]
    fn rejects_task_counts_outside_the_one_to_ten_bound() {
        let mut empty = draft(Vec::new());
        empty.subtasks.clear();
        assert_eq!(
            validate_plan_graph(&empty),
            Err(GraphValidationError::InvalidTaskCount)
        );

        let mut oversized = draft(Vec::new());
        oversized.subtasks = (0_u16..11)
            .map(|ordinal| task(&format!("task-{ordinal}"), ordinal))
            .collect();
        assert_eq!(
            validate_plan_graph(&oversized),
            Err(GraphValidationError::InvalidTaskCount)
        );
    }
}
