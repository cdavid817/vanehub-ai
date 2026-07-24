//! Domain model for validated Multi-Agent coordination plans and durable runs.
//!
//! Plans are directed acyclic graphs keyed by stable node and Agent ids. Run transitions preserve
//! attempt provenance, bound persisted output, allow ordered fallback only for retryable failures,
//! and deterministically skip dependents whose prerequisites cannot succeed.

use super::AgentRuntimeDomainError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Maximum persisted output retained for a successful node attempt.
pub(crate) const COORDINATION_OUTPUT_LIMIT_BYTES: usize = 64 * 1024;
/// Maximum combined prerequisite context accepted by a dependent node.
pub(crate) const COORDINATION_CONTEXT_LIMIT_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Unvalidated node input accepted at the domain boundary.
pub(crate) struct CoordinationNodeInput {
    pub(crate) id: String,
    pub(crate) primary_agent_id: String,
    pub(crate) fallback_agent_ids: Vec<String>,
    pub(crate) instruction: String,
    pub(crate) depends_on: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Complete unvalidated plan input.
pub(crate) struct CoordinationPlanInput {
    pub(crate) name: String,
    pub(crate) project_path: Option<String>,
    pub(crate) nodes: Vec<CoordinationNodeInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Validated immutable node definition stored in a plan snapshot.
pub(crate) struct CoordinationNodeDefinition {
    pub(crate) id: String,
    pub(crate) primary_agent_id: String,
    pub(crate) fallback_agent_ids: Vec<String>,
    pub(crate) instruction: String,
    pub(crate) depends_on: Vec<String>,
}

impl CoordinationNodeDefinition {
    pub(crate) fn candidates(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.primary_agent_id.as_str())
            .chain(self.fallback_agent_ids.iter().map(String::as_str))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Validated plan snapshot with deterministic topological order.
pub(crate) struct CoordinationPlan {
    pub(crate) name: String,
    pub(crate) project_path: Option<String>,
    pub(crate) nodes: Vec<CoordinationNodeDefinition>,
    pub(crate) topological_order: Vec<String>,
}

impl CoordinationPlan {
    pub(crate) fn new(
        input: CoordinationPlanInput,
        known_agent_ids: &BTreeSet<String>,
    ) -> Result<Self, AgentRuntimeDomainError> {
        let name = required(input.name, "coordination name")?;
        if input.nodes.is_empty() {
            return Err(invalid("requires at least one node"));
        }
        let project_path = input
            .project_path
            .map(|value| required(value, "coordination project path"))
            .transpose()?;
        let mut nodes = BTreeMap::new();
        for input_node in input.nodes {
            let id = stable_id(input_node.id, "coordination node id")?;
            if nodes.contains_key(&id) {
                return Err(invalid(format!("duplicate node id '{id}'")));
            }
            let instruction = required(input_node.instruction, "coordination instruction")?;
            let primary_agent_id = stable_id(input_node.primary_agent_id, "primary Agent id")?;
            let fallback_agent_ids = input_node
                .fallback_agent_ids
                .into_iter()
                .map(|id| stable_id(id, "fallback Agent id"))
                .collect::<Result<Vec<_>, _>>()?;
            let mut candidates = BTreeSet::new();
            for agent_id in std::iter::once(&primary_agent_id).chain(fallback_agent_ids.iter()) {
                if !known_agent_ids.contains(agent_id) {
                    return Err(invalid(format!(
                        "node '{id}' references unknown Agent '{agent_id}'"
                    )));
                }
                if !candidates.insert(agent_id.clone()) {
                    return Err(invalid(format!(
                        "node '{id}' repeats primary or fallback Agent '{agent_id}'"
                    )));
                }
            }
            let depends_on = input_node
                .depends_on
                .into_iter()
                .map(|dependency| stable_id(dependency, "coordination dependency id"))
                .collect::<Result<Vec<_>, _>>()?;
            if depends_on.iter().collect::<BTreeSet<_>>().len() != depends_on.len() {
                return Err(invalid(format!("node '{id}' repeats a dependency")));
            }
            nodes.insert(
                id.clone(),
                CoordinationNodeDefinition {
                    id,
                    primary_agent_id,
                    fallback_agent_ids,
                    instruction,
                    depends_on,
                },
            );
        }
        for node in nodes.values() {
            for dependency in &node.depends_on {
                if dependency == &node.id {
                    return Err(invalid(format!(
                        "node '{}' cannot depend on itself",
                        node.id
                    )));
                }
                if !nodes.contains_key(dependency) {
                    return Err(invalid(format!(
                        "node '{}' references missing dependency '{dependency}'",
                        node.id
                    )));
                }
            }
        }
        let topological_order = topological_order(&nodes)?;
        Ok(Self {
            name,
            project_path,
            nodes: nodes.into_values().collect(),
            topological_order,
        })
    }

    pub(crate) fn rehydrate(
        name: String,
        project_path: Option<String>,
        nodes: Vec<CoordinationNodeDefinition>,
        known_agent_ids: &BTreeSet<String>,
    ) -> Result<Self, AgentRuntimeDomainError> {
        Self::new(
            CoordinationPlanInput {
                name,
                project_path,
                nodes: nodes
                    .into_iter()
                    .map(|node| CoordinationNodeInput {
                        id: node.id,
                        primary_agent_id: node.primary_agent_id,
                        fallback_agent_ids: node.fallback_agent_ids,
                        instruction: node.instruction,
                        depends_on: node.depends_on,
                    })
                    .collect(),
            },
            known_agent_ids,
        )
    }

    pub(crate) fn node(&self, id: &str) -> Option<&CoordinationNodeDefinition> {
        self.nodes.iter().find(|node| node.id == id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
/// Lifecycle state of a persisted coordination run.
pub(crate) enum CoordinationRunStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl CoordinationRunStatus {
    pub(crate) fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
/// Lifecycle state of one node in a coordination run.
pub(crate) enum CoordinationNodeStatus {
    Blocked,
    Queued,
    Running,
    Succeeded,
    Failed,
    Skipped,
    Cancelled,
}

impl CoordinationNodeStatus {
    pub(crate) fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Skipped | Self::Cancelled
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CoordinationAttemptStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
/// Failure classification that determines whether ordered fallback is permitted.
pub(crate) enum CoordinationFailureKind {
    Retryable,
    NonRetryable,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CoordinationCandidateRole {
    Primary,
    Fallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoordinationOutput {
    pub(crate) source_node_id: String,
    pub(crate) agent_id: String,
    pub(crate) attempt: u32,
    pub(crate) content: String,
    pub(crate) byte_count: usize,
    pub(crate) truncated: bool,
}

impl CoordinationOutput {
    #[cfg(test)]
    pub(crate) fn bounded(
        source_node_id: String,
        agent_id: String,
        attempt: u32,
        content: String,
    ) -> Self {
        let byte_count = content.len();
        Self::from_bounded(
            source_node_id,
            agent_id,
            attempt,
            content,
            byte_count,
            false,
        )
    }

    pub(crate) fn from_bounded(
        source_node_id: String,
        agent_id: String,
        attempt: u32,
        content: String,
        byte_count: usize,
        truncated: bool,
    ) -> Self {
        let received_bytes = content.len();
        let (content, bounded_here) = truncate_utf8(content, COORDINATION_OUTPUT_LIMIT_BYTES);
        Self {
            source_node_id,
            agent_id,
            attempt,
            content,
            byte_count: byte_count.max(received_bytes),
            truncated: truncated || bounded_here || byte_count > received_bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoordinationAttempt {
    pub(crate) attempt: u32,
    pub(crate) agent_id: String,
    pub(crate) candidate_role: CoordinationCandidateRole,
    pub(crate) status: CoordinationAttemptStatus,
    pub(crate) failure_kind: Option<CoordinationFailureKind>,
    pub(crate) error: Option<String>,
    pub(crate) started_at: String,
    pub(crate) completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoordinationNodeRun {
    pub(crate) definition: CoordinationNodeDefinition,
    pub(crate) status: CoordinationNodeStatus,
    pub(crate) actual_agent_id: Option<String>,
    pub(crate) output: Option<CoordinationOutput>,
    pub(crate) attempts: Vec<CoordinationAttempt>,
    pub(crate) error: Option<String>,
    pub(crate) started_at: Option<String>,
    pub(crate) completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Durable coordination run aggregate and its node/attempt history.
pub(crate) struct CoordinationRun {
    pub(crate) id: String,
    pub(crate) operation_id: String,
    pub(crate) plan: CoordinationPlan,
    pub(crate) status: CoordinationRunStatus,
    pub(crate) nodes: Vec<CoordinationNodeRun>,
    pub(crate) cancel_requested: bool,
    pub(crate) created_at: String,
    pub(crate) started_at: Option<String>,
    pub(crate) updated_at: String,
    pub(crate) completed_at: Option<String>,
}

impl CoordinationRun {
    pub(crate) fn new(
        id: String,
        operation_id: String,
        plan: CoordinationPlan,
        now: String,
    ) -> Result<Self, AgentRuntimeDomainError> {
        let id = required(id, "coordination run id")?;
        let operation_id = required(operation_id, "coordination operation id")?;
        let nodes = plan
            .nodes
            .iter()
            .map(|definition| CoordinationNodeRun {
                status: if definition.depends_on.is_empty() {
                    CoordinationNodeStatus::Queued
                } else {
                    CoordinationNodeStatus::Blocked
                },
                definition: definition.clone(),
                actual_agent_id: None,
                output: None,
                attempts: Vec::new(),
                error: None,
                started_at: None,
                completed_at: None,
            })
            .collect();
        Ok(Self {
            id,
            operation_id,
            plan,
            status: CoordinationRunStatus::Queued,
            nodes,
            cancel_requested: false,
            created_at: now.clone(),
            started_at: None,
            updated_at: now,
            completed_at: None,
        })
    }

    pub(crate) fn request_cancel(&mut self, now: &str) {
        if self.status.is_terminal() {
            return;
        }
        self.cancel_requested = true;
        self.updated_at = now.to_string();
        if self.status == CoordinationRunStatus::Queued {
            self.finish_cancelled(now);
        }
    }

    pub(crate) fn finish_cancelled(&mut self, now: &str) {
        self.cancel_requested = true;
        for node in &mut self.nodes {
            if !node.status.is_terminal() {
                node.status = if node.status == CoordinationNodeStatus::Blocked {
                    CoordinationNodeStatus::Skipped
                } else {
                    CoordinationNodeStatus::Cancelled
                };
                node.error = Some("Coordination was cancelled.".to_string());
                node.completed_at = Some(now.to_string());
            }
        }
        self.status = CoordinationRunStatus::Cancelled;
        self.started_at.get_or_insert_with(|| now.to_string());
        self.updated_at = now.to_string();
        self.completed_at = Some(now.to_string());
    }

    pub(crate) fn node(&self, id: &str) -> Option<&CoordinationNodeRun> {
        self.nodes.iter().find(|node| node.definition.id == id)
    }

    pub(crate) fn node_mut(&mut self, id: &str) -> Option<&mut CoordinationNodeRun> {
        self.nodes.iter_mut().find(|node| node.definition.id == id)
    }

    pub(crate) fn finalize(&mut self, now: &str) {
        if self.cancel_requested {
            self.finish_cancelled(now);
            return;
        }
        self.status = if self
            .nodes
            .iter()
            .all(|node| node.status == CoordinationNodeStatus::Succeeded)
        {
            CoordinationRunStatus::Succeeded
        } else {
            CoordinationRunStatus::Failed
        };
        self.updated_at = now.to_string();
        self.completed_at = Some(now.to_string());
    }
}

fn topological_order(
    nodes: &BTreeMap<String, CoordinationNodeDefinition>,
) -> Result<Vec<String>, AgentRuntimeDomainError> {
    let mut indegree = nodes
        .iter()
        .map(|(id, node)| (id.clone(), node.depends_on.len()))
        .collect::<BTreeMap<_, _>>();
    let mut dependents = BTreeMap::<String, Vec<String>>::new();
    for node in nodes.values() {
        for dependency in &node.depends_on {
            dependents
                .entry(dependency.clone())
                .or_default()
                .push(node.id.clone());
        }
    }
    let mut ready = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(id.clone()))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(nodes.len());
    while let Some(id) = ready.pop_first() {
        order.push(id.clone());
        if let Some(children) = dependents.get(&id) {
            for child in children {
                let Some(degree) = indegree.get_mut(child) else {
                    return Err(invalid(format!(
                        "dependency graph lost referenced node '{child}'"
                    )));
                };
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(child.clone());
                }
            }
        }
    }
    if order.len() != nodes.len() {
        return Err(invalid("dependency graph contains a cycle"));
    }
    Ok(order)
}

fn stable_id(value: String, label: &'static str) -> Result<String, AgentRuntimeDomainError> {
    let value = required(value, label)?;
    let valid = value.split('-').all(|part| {
        !part.is_empty()
            && part
                .chars()
                .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
    });
    if !valid {
        return Err(invalid(format!(
            "{label} '{value}' is not a stable kebab-case id"
        )));
    }
    Ok(value)
}

fn required(value: String, label: &'static str) -> Result<String, AgentRuntimeDomainError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(invalid(format!("{label} cannot be empty")));
    }
    if value.chars().any(char::is_control) {
        return Err(invalid(format!("{label} contains control characters")));
    }
    Ok(value)
}

fn invalid(message: impl Into<String>) -> AgentRuntimeDomainError {
    AgentRuntimeDomainError::InvalidCoordination(message.into())
}

fn truncate_utf8(mut content: String, limit: usize) -> (String, bool) {
    if content.len() <= limit {
        return (content, false);
    }
    let mut boundary = limit;
    while !content.is_char_boundary(boundary) {
        boundary -= 1;
    }
    content.truncate(boundary);
    (content, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known_agents() -> BTreeSet<String> {
        ["claude-code", "codex-cli", "gemini-cli"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    fn node(id: &str, agent_id: &str, dependencies: &[&str]) -> CoordinationNodeInput {
        CoordinationNodeInput {
            id: id.to_string(),
            primary_agent_id: agent_id.to_string(),
            fallback_agent_ids: Vec::new(),
            instruction: format!("Execute {id}"),
            depends_on: dependencies
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
        }
    }

    #[test]
    fn graph_validation_produces_deterministic_order() {
        let plan = CoordinationPlan::new(
            CoordinationPlanInput {
                name: "pipeline".to_string(),
                project_path: None,
                nodes: vec![
                    node("review", "claude-code", &["test", "implement"]),
                    node("test", "gemini-cli", &["implement"]),
                    node("docs", "claude-code", &[]),
                    node("implement", "codex-cli", &[]),
                ],
            },
            &known_agents(),
        )
        .expect("valid plan");
        assert_eq!(
            plan.topological_order,
            vec!["docs", "implement", "test", "review"]
        );
    }

    #[test]
    fn graph_validation_rejects_cycles_missing_dependencies_and_agent_reuse() {
        let cycle = CoordinationPlan::new(
            CoordinationPlanInput {
                name: "cycle".to_string(),
                project_path: None,
                nodes: vec![
                    node("alpha", "codex-cli", &["beta"]),
                    node("beta", "claude-code", &["alpha"]),
                ],
            },
            &known_agents(),
        );
        assert!(cycle
            .expect_err("cycle")
            .to_string()
            .contains("contains a cycle"));

        let mut repeated = node("alpha", "codex-cli", &[]);
        repeated.fallback_agent_ids = vec!["codex-cli".to_string()];
        assert!(CoordinationPlan::new(
            CoordinationPlanInput {
                name: "repeat".to_string(),
                project_path: None,
                nodes: vec![repeated],
            },
            &known_agents(),
        )
        .expect_err("repeat")
        .to_string()
        .contains("repeats primary or fallback"));
    }

    #[test]
    fn output_is_utf8_safe_and_run_cancellation_is_idempotent() {
        let output = CoordinationOutput::bounded(
            "alpha".to_string(),
            "codex-cli".to_string(),
            1,
            "界".repeat(COORDINATION_OUTPUT_LIMIT_BYTES),
        );
        assert!(output.truncated);
        assert!(output.content.len() <= COORDINATION_OUTPUT_LIMIT_BYTES);

        let plan = CoordinationPlan::new(
            CoordinationPlanInput {
                name: "cancel".to_string(),
                project_path: None,
                nodes: vec![node("alpha", "codex-cli", &[])],
            },
            &known_agents(),
        )
        .expect("plan");
        let mut run = CoordinationRun::new(
            "run-1".to_string(),
            "operation-1".to_string(),
            plan,
            "now".to_string(),
        )
        .expect("run");
        run.request_cancel("later");
        run.request_cancel("latest");
        assert_eq!(run.status, CoordinationRunStatus::Cancelled);
        assert_eq!(run.nodes[0].status, CoordinationNodeStatus::Cancelled);
    }
}
