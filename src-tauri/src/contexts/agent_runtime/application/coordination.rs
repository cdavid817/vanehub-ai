use super::{
    AgentClockPort, AgentLog, AgentLogLevel, AgentLoggingPort, AgentRegistryRepository,
    AgentRuntimeApplicationError,
};
use crate::contexts::agent_runtime::domain::{
    CoordinationAttempt, CoordinationAttemptStatus, CoordinationCandidateRole,
    CoordinationFailureKind, CoordinationNodeInput, CoordinationNodeStatus, CoordinationOutput,
    CoordinationPlan, CoordinationPlanInput, CoordinationRun, CoordinationRunStatus,
    COORDINATION_CONTEXT_LIMIT_BYTES,
};
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartCoordinationRequest {
    pub(crate) name: String,
    pub(crate) project_path: Option<String>,
    pub(crate) nodes: Vec<CoordinationNodeInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartCoordinationResultView {
    pub(crate) run_id: String,
    pub(crate) operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CoordinationExecutionRequest {
    pub(crate) run_id: String,
    pub(crate) node_id: String,
    pub(crate) agent_id: String,
    pub(crate) attempt: u32,
    pub(crate) candidate_role: CoordinationCandidateRole,
    pub(crate) instruction: String,
    pub(crate) prerequisite_context: String,
    pub(crate) project_path: Option<String>,
    pub(crate) operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CoordinationExecutionResult {
    Succeeded(CoordinationExecutionOutput),
    Failed {
        kind: CoordinationFailureKind,
        error: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CoordinationExecutionOutput {
    pub(crate) content: String,
    pub(crate) byte_count: usize,
    pub(crate) truncated: bool,
}

pub(crate) trait CoordinationRepository: Send + Sync {
    fn insert(&self, run: &CoordinationRun) -> Result<(), AgentRuntimeApplicationError>;
    fn save(&self, run: &CoordinationRun) -> Result<(), AgentRuntimeApplicationError>;
    fn find(&self, run_id: &str) -> Result<Option<CoordinationRun>, AgentRuntimeApplicationError>;
    fn list(&self) -> Result<Vec<CoordinationRun>, AgentRuntimeApplicationError>;
    fn request_cancel(
        &self,
        run_id: &str,
        updated_at: &str,
    ) -> Result<CoordinationRun, AgentRuntimeApplicationError>;
    fn is_cancel_requested(&self, run_id: &str) -> Result<bool, AgentRuntimeApplicationError>;
    fn list_recoverable(&self) -> Result<Vec<CoordinationRun>, AgentRuntimeApplicationError>;
}

pub(crate) trait CoordinationNodeExecutor: Send + Sync {
    fn start_coordination(
        &self,
        run_id: &str,
        operation_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError>;
    fn execute(
        &self,
        request: CoordinationExecutionRequest,
    ) -> Result<CoordinationExecutionResult, AgentRuntimeApplicationError>;
    fn cancel(&self, run_id: &str) -> Result<(), AgentRuntimeApplicationError>;
    fn finish_coordination(
        &self,
        run_id: &str,
        status: CoordinationRunStatus,
    ) -> Result<(), AgentRuntimeApplicationError>;
}

pub(crate) trait CoordinationIdPort: Send + Sync {
    fn next_id(&self, prefix: &str) -> String;
}

pub(crate) trait CoordinationOperationPort: Send + Sync {
    fn start(&self, run_id: &str, name: &str) -> Result<String, AgentRuntimeApplicationError>;
    fn append_log(
        &self,
        operation_id: &str,
        message: String,
    ) -> Result<(), AgentRuntimeApplicationError>;
    fn complete(&self, operation_id: &str) -> Result<(), AgentRuntimeApplicationError>;
    fn fail(&self, operation_id: &str, error: String) -> Result<(), AgentRuntimeApplicationError>;
    fn cancel(&self, operation_id: &str) -> Result<(), AgentRuntimeApplicationError>;
}

#[derive(Clone)]
pub(crate) struct CoordinationApplicationPorts {
    pub(crate) repository: Arc<dyn CoordinationRepository>,
    pub(crate) registry: Arc<dyn AgentRegistryRepository>,
    pub(crate) executor: Arc<dyn CoordinationNodeExecutor>,
    pub(crate) ids: Arc<dyn CoordinationIdPort>,
    pub(crate) operations: Arc<dyn CoordinationOperationPort>,
    pub(crate) logging: Arc<dyn AgentLoggingPort>,
    pub(crate) clock: Arc<dyn AgentClockPort>,
}

#[derive(Clone)]
pub(crate) struct CoordinationApplicationService {
    ports: CoordinationApplicationPorts,
}

impl CoordinationApplicationService {
    pub(crate) fn new(ports: CoordinationApplicationPorts) -> Self {
        Self { ports }
    }

    pub(crate) fn start(
        &self,
        request: StartCoordinationRequest,
    ) -> Result<StartCoordinationResultView, AgentRuntimeApplicationError> {
        let known_agent_ids = self
            .ports
            .registry
            .list()?
            .into_iter()
            .map(|agent| agent.id().as_str().to_string())
            .collect::<BTreeSet<_>>();
        let plan = CoordinationPlan::new(
            CoordinationPlanInput {
                name: request.name,
                project_path: request.project_path,
                nodes: request.nodes,
            },
            &known_agent_ids,
        )?;
        let run_id = self.ports.ids.next_id("coordination-run");
        let operation_id = self.ports.operations.start(&run_id, &plan.name)?;
        let run = CoordinationRun::new(
            run_id.clone(),
            operation_id.clone(),
            plan,
            self.ports.clock.now(),
        )?;
        if let Err(error) = self.ports.repository.insert(&run) {
            let _ = self.ports.operations.fail(&operation_id, error.to_string());
            return Err(error);
        }
        let _ = self
            .ports
            .executor
            .start_coordination(&run_id, &operation_id);
        self.log(
            &run,
            AgentLogLevel::Info,
            "coordination.started",
            "Multi-Agent coordination accepted".to_string(),
            None,
        );
        Ok(StartCoordinationResultView {
            run_id,
            operation_id,
        })
    }

    pub(crate) fn list(&self) -> Result<Vec<CoordinationRun>, AgentRuntimeApplicationError> {
        self.ports.repository.list()
    }

    pub(crate) fn get(
        &self,
        run_id: &str,
    ) -> Result<CoordinationRun, AgentRuntimeApplicationError> {
        self.ports.repository.find(run_id)?.ok_or_else(|| {
            AgentRuntimeApplicationError::Coordination(format!("run not found: {run_id}"))
        })
    }

    pub(crate) fn cancel(
        &self,
        run_id: &str,
    ) -> Result<CoordinationRun, AgentRuntimeApplicationError> {
        let now = self.ports.clock.now();
        let mut run = self.ports.repository.request_cancel(run_id, &now)?;
        if run.status.is_terminal() {
            if run.status == CoordinationRunStatus::Cancelled {
                let _ = self.ports.executor.cancel(run_id);
                let _ = self.ports.operations.cancel(&run.operation_id);
                let _ = self.ports.executor.finish_coordination(&run.id, run.status);
            }
            return Ok(run);
        }
        let _ = self.ports.executor.cancel(run_id);
        run.request_cancel(&now);
        if run.status == CoordinationRunStatus::Queued {
            self.ports.repository.save(&run)?;
            let _ = self.ports.operations.cancel(&run.operation_id);
        }
        self.log(
            &run,
            AgentLogLevel::Info,
            "coordination.cancel",
            "Multi-Agent coordination cancellation requested".to_string(),
            None,
        );
        Ok(run)
    }

    pub(crate) fn execute(&self, run_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        let mut run = self.get(run_id)?;
        if run.status.is_terminal() {
            return Ok(());
        }
        if self.ports.repository.is_cancel_requested(run_id)? {
            return self.finish_cancelled(&mut run);
        }
        let started_at = self.ports.clock.now();
        run.status = CoordinationRunStatus::Running;
        run.started_at.get_or_insert_with(|| started_at.clone());
        run.updated_at = started_at;
        self.ports.repository.save(&run)?;
        let order = run.plan.topological_order.clone();

        for node_id in order {
            if self.ports.repository.is_cancel_requested(run_id)? {
                return self.finish_cancelled(&mut run);
            }
            if run
                .node(&node_id)
                .is_some_and(|node| node.status.is_terminal())
            {
                continue;
            }
            let dependencies = run
                .plan
                .node(&node_id)
                .map(|node| node.depends_on.clone())
                .unwrap_or_default();
            if dependencies.iter().any(|dependency| {
                run.node(dependency)
                    .is_none_or(|node| node.status != CoordinationNodeStatus::Succeeded)
            }) {
                let now = self.ports.clock.now();
                if let Some(node) = run.node_mut(&node_id) {
                    node.status = CoordinationNodeStatus::Skipped;
                    node.error = Some("A prerequisite node did not succeed.".to_string());
                    node.completed_at = Some(now.clone());
                }
                run.updated_at = now;
                self.ports.repository.save(&run)?;
                continue;
            }
            let context = self.assemble_context(&run, &dependencies)?;
            if context.len() > COORDINATION_CONTEXT_LIMIT_BYTES {
                let now = self.ports.clock.now();
                if let Some(node) = run.node_mut(&node_id) {
                    node.status = CoordinationNodeStatus::Failed;
                    node.error = Some(format!(
                        "Prerequisite context exceeds {COORDINATION_CONTEXT_LIMIT_BYTES} bytes."
                    ));
                    node.completed_at = Some(now.clone());
                }
                run.updated_at = now;
                self.ports.repository.save(&run)?;
                continue;
            }
            self.execute_node(&mut run, &node_id, context)?;
        }

        let now = self.ports.clock.now();
        run.finalize(&now);
        self.ports.repository.save(&run)?;
        match run.status {
            CoordinationRunStatus::Succeeded => {
                let _ = self.ports.operations.complete(&run.operation_id);
            }
            CoordinationRunStatus::Cancelled => {
                let _ = self.ports.operations.cancel(&run.operation_id);
            }
            CoordinationRunStatus::Failed => {
                let _ = self.ports.operations.fail(
                    &run.operation_id,
                    "One or more coordination nodes failed.".to_string(),
                );
            }
            CoordinationRunStatus::Queued | CoordinationRunStatus::Running => {}
        }
        let _ = self.ports.executor.finish_coordination(&run.id, run.status);
        self.log(
            &run,
            if run.status == CoordinationRunStatus::Succeeded {
                AgentLogLevel::Info
            } else {
                AgentLogLevel::Warn
            },
            "coordination.completed",
            format!("Multi-Agent coordination ended with {:?}", run.status),
            None,
        );
        Ok(())
    }

    pub(crate) fn handle_scheduler_failure(
        &self,
        run_id: &str,
        error: &AgentRuntimeApplicationError,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let classification = scheduler_error_classification(error);
        self.record_scheduler_log(
            run_id,
            "coordination.scheduler.failure",
            format!("Coordination scheduler execution failed classification={classification}"),
        );
        let mut run = self.get(run_id)?;
        if run.status.is_terminal() {
            return Ok(());
        }
        let now = self.ports.clock.now();
        let failed_node_id = run
            .nodes
            .iter()
            .find(|node| node.status == CoordinationNodeStatus::Running)
            .or_else(|| run.nodes.iter().find(|node| !node.status.is_terminal()))
            .map(|node| node.definition.id.clone());
        for node in &mut run.nodes {
            if node.status.is_terminal() {
                continue;
            }
            if failed_node_id.as_deref() == Some(node.definition.id.as_str()) {
                node.status = CoordinationNodeStatus::Failed;
                node.error = Some(format!(
                    "Coordination scheduler stopped after a non-retryable {classification} failure."
                ));
                if let Some(attempt) = node
                    .attempts
                    .last_mut()
                    .filter(|attempt| attempt.status == CoordinationAttemptStatus::Running)
                {
                    attempt.status = CoordinationAttemptStatus::Failed;
                    attempt.failure_kind = Some(CoordinationFailureKind::NonRetryable);
                    attempt.error = node.error.clone();
                    attempt.completed_at = Some(now.clone());
                }
            } else {
                node.status = CoordinationNodeStatus::Skipped;
                node.error = Some(
                    "Coordination stopped after a scheduler infrastructure failure.".to_string(),
                );
            }
            node.completed_at = Some(now.clone());
        }
        run.status = CoordinationRunStatus::Failed;
        run.updated_at = now.clone();
        run.completed_at = Some(now);
        self.ports.repository.save(&run)?;
        let _ = self.ports.executor.cancel(&run.id);
        let _ = self.ports.operations.fail(
            &run.operation_id,
            format!("Coordination scheduler failed classification={classification}"),
        );
        let _ = self.ports.executor.finish_coordination(&run.id, run.status);
        self.log(
            &run,
            AgentLogLevel::Error,
            "coordination.scheduler.terminal",
            format!("Coordination scheduler settled run classification={classification}"),
            None,
        );
        Ok(())
    }

    pub(crate) fn record_scheduler_settlement_failure(&self, run_id: &str) {
        self.record_scheduler_log(
            run_id,
            "coordination.scheduler.settlement_failure",
            "Coordination scheduler could not persist its terminal failure state.".to_string(),
        );
    }

    pub(crate) fn recover_startup(&self) -> Result<Vec<String>, AgentRuntimeApplicationError> {
        let mut recovered = Vec::new();
        for mut run in self.ports.repository.list_recoverable()? {
            if run.cancel_requested {
                self.finish_cancelled(&mut run)?;
                continue;
            }
            let now = self.ports.clock.now();
            if let Some(node) = run
                .nodes
                .iter_mut()
                .find(|node| node.status == CoordinationNodeStatus::Running)
            {
                if let Some(attempt) = node
                    .attempts
                    .iter_mut()
                    .find(|attempt| attempt.status == CoordinationAttemptStatus::Running)
                {
                    attempt.status = CoordinationAttemptStatus::Failed;
                    attempt.failure_kind = Some(CoordinationFailureKind::Retryable);
                    attempt.error = Some("Runtime interrupted before completion.".to_string());
                    attempt.completed_at = Some(now.clone());
                }
                node.status = CoordinationNodeStatus::Queued;
                node.error = Some("Runtime interrupted; resuming with next candidate.".to_string());
            }
            run.status = CoordinationRunStatus::Queued;
            run.updated_at = now;
            self.ports.repository.save(&run)?;
            recovered.push(run.id);
        }
        Ok(recovered)
    }

    fn execute_node(
        &self,
        run: &mut CoordinationRun,
        node_id: &str,
        prerequisite_context: String,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let definition = run.plan.node(node_id).cloned().ok_or_else(|| {
            AgentRuntimeApplicationError::Coordination(format!("node not found in plan: {node_id}"))
        })?;
        let candidates = definition
            .candidates()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let already_attempted = run
            .node(node_id)
            .map(|node| node.attempts.len())
            .unwrap_or_default();
        if already_attempted >= candidates.len() {
            let now = self.ports.clock.now();
            if let Some(node) = run.node_mut(node_id) {
                node.status = CoordinationNodeStatus::Failed;
                node.error = Some("No unused Agent candidate remains.".to_string());
                node.completed_at = Some(now.clone());
            }
            run.updated_at = now;
            self.ports.repository.save(run)?;
            return Ok(());
        }
        let started_at = self.ports.clock.now();
        if let Some(node) = run.node_mut(node_id) {
            node.status = CoordinationNodeStatus::Running;
            node.started_at.get_or_insert_with(|| started_at.clone());
        }
        run.updated_at = started_at;
        self.ports.repository.save(run)?;

        for (candidate_index, agent_id) in candidates.iter().enumerate().skip(already_attempted) {
            if self.ports.repository.is_cancel_requested(&run.id)? {
                return self.finish_cancelled(run);
            }
            let attempt_number = u32::try_from(candidate_index + 1)
                .map_err(|error| AgentRuntimeApplicationError::Coordination(error.to_string()))?;
            let candidate_role = if candidate_index == 0 {
                CoordinationCandidateRole::Primary
            } else {
                CoordinationCandidateRole::Fallback
            };
            let now = self.ports.clock.now();
            if let Some(node) = run.node_mut(node_id) {
                node.attempts.push(CoordinationAttempt {
                    attempt: attempt_number,
                    agent_id: agent_id.clone(),
                    candidate_role,
                    status: CoordinationAttemptStatus::Running,
                    failure_kind: None,
                    error: None,
                    started_at: now.clone(),
                    completed_at: None,
                });
            }
            run.updated_at = now;
            self.ports.repository.save(run)?;
            let mut result = self.ports.executor.execute(CoordinationExecutionRequest {
                run_id: run.id.clone(),
                node_id: node_id.to_string(),
                agent_id: agent_id.clone(),
                attempt: attempt_number,
                candidate_role,
                instruction: definition.instruction.clone(),
                prerequisite_context: prerequisite_context.clone(),
                project_path: run.plan.project_path.clone(),
                operation_id: run.operation_id.clone(),
            })?;
            if self.ports.repository.is_cancel_requested(&run.id)? {
                result = CoordinationExecutionResult::Failed {
                    kind: CoordinationFailureKind::Cancelled,
                    error: "Coordination was cancelled.".to_string(),
                };
            }
            let completed_at = self.ports.clock.now();
            match result {
                CoordinationExecutionResult::Succeeded(output) => {
                    if let Some(node) = run.node_mut(node_id) {
                        if let Some(attempt) = node.attempts.last_mut() {
                            attempt.status = CoordinationAttemptStatus::Succeeded;
                            attempt.completed_at = Some(completed_at.clone());
                        }
                        node.status = CoordinationNodeStatus::Succeeded;
                        node.actual_agent_id = Some(agent_id.clone());
                        node.output = Some(CoordinationOutput::from_bounded(
                            node_id.to_string(),
                            agent_id.clone(),
                            attempt_number,
                            output.content,
                            output.byte_count,
                            output.truncated,
                        ));
                        node.error = None;
                        node.completed_at = Some(completed_at.clone());
                    }
                    run.updated_at = completed_at;
                    self.ports.repository.save(run)?;
                    return Ok(());
                }
                CoordinationExecutionResult::Failed { kind, error } => {
                    if let Some(node) = run.node_mut(node_id) {
                        if let Some(attempt) = node.attempts.last_mut() {
                            attempt.status = if kind == CoordinationFailureKind::Cancelled {
                                CoordinationAttemptStatus::Cancelled
                            } else {
                                CoordinationAttemptStatus::Failed
                            };
                            attempt.failure_kind = Some(kind);
                            attempt.error = Some(error.clone());
                            attempt.completed_at = Some(completed_at.clone());
                        }
                        node.error = Some(error.clone());
                    }
                    run.updated_at = completed_at.clone();
                    self.ports.repository.save(run)?;
                    self.log(
                        run,
                        AgentLogLevel::Warn,
                        "coordination.attempt",
                        format!(
                            "Agent attempt failed node={node_id} agent={agent_id} attempt={attempt_number} classification={kind:?}"
                        ),
                        Some(agent_id),
                    );
                    let has_fallback = candidate_index + 1 < candidates.len();
                    if kind != CoordinationFailureKind::Retryable || !has_fallback {
                        if let Some(node) = run.node_mut(node_id) {
                            node.status = if kind == CoordinationFailureKind::Cancelled {
                                CoordinationNodeStatus::Cancelled
                            } else {
                                CoordinationNodeStatus::Failed
                            };
                            node.completed_at = Some(completed_at);
                        }
                        if kind == CoordinationFailureKind::Cancelled {
                            run.cancel_requested = true;
                        }
                        self.ports.repository.save(run)?;
                        return Ok(());
                    }
                    let _ = self.ports.operations.append_log(
                        &run.operation_id,
                        format!(
                            "Retryable Agent failure for node {node_id}; advancing to fallback"
                        ),
                    );
                }
            }
        }
        Ok(())
    }

    fn assemble_context(
        &self,
        run: &CoordinationRun,
        dependencies: &[String],
    ) -> Result<String, AgentRuntimeApplicationError> {
        let mut blocks = Vec::new();
        for dependency_id in dependencies {
            let output = run
                .node(dependency_id)
                .and_then(|node| node.output.as_ref())
                .ok_or_else(|| {
                    AgentRuntimeApplicationError::Coordination(format!(
                        "successful dependency '{dependency_id}' has no output"
                    ))
                })?;
            blocks.push(format!(
                "--- prerequisite node={} agent={} attempt={} ---\n{}\n--- end prerequisite ---",
                output.source_node_id, output.agent_id, output.attempt, output.content
            ));
        }
        Ok(blocks.join("\n\n"))
    }

    fn finish_cancelled(
        &self,
        run: &mut CoordinationRun,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let now = self.ports.clock.now();
        run.finish_cancelled(&now);
        self.ports.repository.save(run)?;
        let _ = self.ports.operations.cancel(&run.operation_id);
        let _ = self.ports.executor.finish_coordination(&run.id, run.status);
        Ok(())
    }

    fn log(
        &self,
        run: &CoordinationRun,
        level: AgentLogLevel,
        category: &str,
        message: String,
        agent_id: Option<&str>,
    ) {
        let _ = self.ports.logging.record(AgentLog {
            level,
            category: category.to_string(),
            message,
            agent_id: agent_id.map(str::to_string),
            session_id: None,
            operation_id: Some(run.operation_id.clone()),
            run_id: Some(run.id.clone()),
            trace_id: None,
            span_id: None,
            occurred_at: self.ports.clock.now(),
        });
    }

    fn record_scheduler_log(&self, run_id: &str, category: &str, message: String) {
        let _ = self.ports.logging.record(AgentLog {
            level: AgentLogLevel::Error,
            category: category.to_string(),
            message,
            agent_id: None,
            session_id: None,
            operation_id: None,
            run_id: Some(run_id.to_string()),
            trace_id: None,
            span_id: None,
            occurred_at: self.ports.clock.now(),
        });
    }
}

fn scheduler_error_classification(error: &AgentRuntimeApplicationError) -> &'static str {
    match error {
        AgentRuntimeApplicationError::Validation(_)
        | AgentRuntimeApplicationError::Domain(_)
        | AgentRuntimeApplicationError::AgentNotFound(_)
        | AgentRuntimeApplicationError::SessionNotFound(_)
        | AgentRuntimeApplicationError::MessageNotFound(_)
        | AgentRuntimeApplicationError::NoActiveAgent
        | AgentRuntimeApplicationError::UnsupportedInteractionMode(_)
        | AgentRuntimeApplicationError::GenerationConflict(_)
        | AgentRuntimeApplicationError::PolicyDenied { .. }
        | AgentRuntimeApplicationError::VerificationPolicy(_) => "validation_or_policy",
        AgentRuntimeApplicationError::Registry(_)
        | AgentRuntimeApplicationError::Workflow(_)
        | AgentRuntimeApplicationError::Session(_)
        | AgentRuntimeApplicationError::Coordination(_) => "persistence_or_runtime",
        AgentRuntimeApplicationError::CliProfile(_) | AgentRuntimeApplicationError::Prompt(_) => {
            "configuration"
        }
        AgentRuntimeApplicationError::Process(_)
        | AgentRuntimeApplicationError::VerificationProcess(_)
        | AgentRuntimeApplicationError::Event(_)
        | AgentRuntimeApplicationError::Generation(_) => "execution",
        AgentRuntimeApplicationError::Operation(_) => "operation",
        AgentRuntimeApplicationError::Loop(_) => "loop",
        AgentRuntimeApplicationError::Logging(_) => "logging",
        AgentRuntimeApplicationError::AgentUnavailable(_) => "agent_unavailable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::domain::{
        AgentAvailability, AgentDefinition, AgentDefinitionInput, AvailabilityAssessment,
        InteractionMode, LaunchMetadata,
    };
    use std::collections::BTreeMap;
    use std::sync::{Condvar, Mutex};
    use std::time::Duration;

    struct World {
        run: Mutex<Option<CoordinationRun>>,
        executions: Mutex<Vec<CoordinationExecutionRequest>>,
        outcomes: Mutex<Vec<CoordinationExecutionResult>>,
        logs: Mutex<Vec<AgentLog>>,
        block_execution: bool,
        execution_started: Mutex<bool>,
        execution_started_signal: Condvar,
        execution_released: Mutex<bool>,
        execution_release_signal: Condvar,
        cancel_calls: Mutex<usize>,
    }

    impl World {
        fn new(outcomes: Vec<CoordinationExecutionResult>) -> Self {
            Self {
                run: Mutex::new(None),
                executions: Mutex::new(Vec::new()),
                outcomes: Mutex::new(outcomes),
                logs: Mutex::new(Vec::new()),
                block_execution: false,
                execution_started: Mutex::new(false),
                execution_started_signal: Condvar::new(),
                execution_released: Mutex::new(false),
                execution_release_signal: Condvar::new(),
                cancel_calls: Mutex::new(0),
            }
        }

        fn blocking(outcomes: Vec<CoordinationExecutionResult>) -> Self {
            Self {
                block_execution: true,
                ..Self::new(outcomes)
            }
        }

        fn wait_for_execution_start(&self) {
            let started = self.execution_started.lock().expect("execution started");
            let (started, timeout) = self
                .execution_started_signal
                .wait_timeout_while(started, Duration::from_secs(5), |started| !*started)
                .expect("wait for execution");
            assert!(*started, "executor did not start before timeout");
            assert!(!timeout.timed_out(), "executor start timed out");
        }
    }

    impl CoordinationRepository for World {
        fn insert(&self, run: &CoordinationRun) -> Result<(), AgentRuntimeApplicationError> {
            *self.run.lock().map_err(coordination_error)? = Some(run.clone());
            Ok(())
        }
        fn save(&self, run: &CoordinationRun) -> Result<(), AgentRuntimeApplicationError> {
            *self.run.lock().map_err(coordination_error)? = Some(run.clone());
            Ok(())
        }
        fn find(
            &self,
            _run_id: &str,
        ) -> Result<Option<CoordinationRun>, AgentRuntimeApplicationError> {
            Ok(self.run.lock().map_err(coordination_error)?.clone())
        }
        fn list(&self) -> Result<Vec<CoordinationRun>, AgentRuntimeApplicationError> {
            Ok(CoordinationRepository::find(self, "run")?
                .into_iter()
                .collect())
        }
        fn request_cancel(
            &self,
            _run_id: &str,
            updated_at: &str,
        ) -> Result<CoordinationRun, AgentRuntimeApplicationError> {
            let mut run = CoordinationRepository::find(self, "run")?
                .ok_or_else(|| coordination_error("missing"))?;
            run.cancel_requested = true;
            run.updated_at = updated_at.to_string();
            self.save(&run)?;
            Ok(run)
        }
        fn is_cancel_requested(&self, _run_id: &str) -> Result<bool, AgentRuntimeApplicationError> {
            Ok(CoordinationRepository::find(self, "run")?.is_some_and(|run| run.cancel_requested))
        }
        fn list_recoverable(&self) -> Result<Vec<CoordinationRun>, AgentRuntimeApplicationError> {
            CoordinationRepository::list(self)
        }
    }

    impl CoordinationNodeExecutor for World {
        fn start_coordination(
            &self,
            _run_id: &str,
            _operation_id: &str,
        ) -> Result<(), AgentRuntimeApplicationError> {
            Ok(())
        }

        fn execute(
            &self,
            request: CoordinationExecutionRequest,
        ) -> Result<CoordinationExecutionResult, AgentRuntimeApplicationError> {
            self.executions
                .lock()
                .map_err(coordination_error)?
                .push(request);
            if self.block_execution {
                *self.execution_started.lock().map_err(coordination_error)? = true;
                self.execution_started_signal.notify_all();
                let released = self.execution_released.lock().map_err(coordination_error)?;
                let (released, timeout) = self
                    .execution_release_signal
                    .wait_timeout_while(released, Duration::from_secs(5), |released| !*released)
                    .map_err(coordination_error)?;
                if timeout.timed_out() || !*released {
                    return Err(coordination_error("execution release timed out"));
                }
            }
            let mut outcomes = self.outcomes.lock().map_err(coordination_error)?;
            if outcomes.is_empty() {
                return Ok(success("default output"));
            }
            Ok(outcomes.remove(0))
        }
        fn cancel(&self, _run_id: &str) -> Result<(), AgentRuntimeApplicationError> {
            *self.cancel_calls.lock().map_err(coordination_error)? += 1;
            *self.execution_released.lock().map_err(coordination_error)? = true;
            self.execution_release_signal.notify_all();
            Ok(())
        }
        fn finish_coordination(
            &self,
            _run_id: &str,
            _status: CoordinationRunStatus,
        ) -> Result<(), AgentRuntimeApplicationError> {
            Ok(())
        }
    }

    impl AgentRegistryRepository for World {
        fn list(&self) -> Result<Vec<AgentDefinition>, AgentRuntimeApplicationError> {
            ["claude-code", "codex-cli", "gemini-cli"]
                .into_iter()
                .map(agent)
                .collect()
        }
        fn find(
            &self,
            agent_id: &str,
        ) -> Result<Option<AgentDefinition>, AgentRuntimeApplicationError> {
            Ok(Some(agent(agent_id)?))
        }
    }

    impl CoordinationIdPort for World {
        fn next_id(&self, prefix: &str) -> String {
            format!("{prefix}-1")
        }
    }

    impl CoordinationOperationPort for World {
        fn start(
            &self,
            _run_id: &str,
            _name: &str,
        ) -> Result<String, AgentRuntimeApplicationError> {
            Ok("operation-1".to_string())
        }
        fn append_log(
            &self,
            _operation_id: &str,
            _message: String,
        ) -> Result<(), AgentRuntimeApplicationError> {
            Ok(())
        }
        fn complete(&self, _operation_id: &str) -> Result<(), AgentRuntimeApplicationError> {
            Ok(())
        }
        fn fail(
            &self,
            _operation_id: &str,
            _error: String,
        ) -> Result<(), AgentRuntimeApplicationError> {
            Ok(())
        }
        fn cancel(&self, _operation_id: &str) -> Result<(), AgentRuntimeApplicationError> {
            Ok(())
        }
    }

    impl AgentLoggingPort for World {
        fn record(&self, log: AgentLog) -> Result<(), AgentRuntimeApplicationError> {
            self.logs.lock().map_err(coordination_error)?.push(log);
            Ok(())
        }
    }

    impl AgentClockPort for World {
        fn now(&self) -> String {
            "2026-07-23T00:00:00Z".to_string()
        }
    }

    fn service(world: Arc<World>) -> CoordinationApplicationService {
        CoordinationApplicationService::new(CoordinationApplicationPorts {
            repository: world.clone(),
            registry: world.clone(),
            executor: world.clone(),
            ids: world.clone(),
            operations: world.clone(),
            logging: world.clone(),
            clock: world,
        })
    }

    fn start_request() -> StartCoordinationRequest {
        StartCoordinationRequest {
            name: "pipeline".to_string(),
            project_path: Some("D:\\project".to_string()),
            nodes: vec![
                CoordinationNodeInput {
                    id: "implement".to_string(),
                    primary_agent_id: "codex-cli".to_string(),
                    fallback_agent_ids: vec!["claude-code".to_string()],
                    instruction: "implement".to_string(),
                    depends_on: Vec::new(),
                },
                CoordinationNodeInput {
                    id: "review".to_string(),
                    primary_agent_id: "gemini-cli".to_string(),
                    fallback_agent_ids: Vec::new(),
                    instruction: "review".to_string(),
                    depends_on: vec!["implement".to_string()],
                },
            ],
        }
    }

    #[test]
    fn retryable_failure_uses_fallback_and_passes_output_to_dependent() {
        let world = Arc::new(World::new(vec![
            CoordinationExecutionResult::Failed {
                kind: CoordinationFailureKind::Retryable,
                error: "process failed".to_string(),
            },
            success("implementation"),
            success("review"),
        ]));
        let service = service(world.clone());
        let started = service.start(start_request()).expect("start");
        service.execute(&started.run_id).expect("execute");
        service
            .execute(&started.run_id)
            .expect("terminal execution is idempotent");

        let run = service.get(&started.run_id).expect("run");
        assert_eq!(run.status, CoordinationRunStatus::Succeeded);
        assert_eq!(
            run.node("implement")
                .and_then(|node| node.actual_agent_id.as_deref()),
            Some("claude-code")
        );
        let executions = world.executions.lock().expect("executions");
        assert_eq!(executions.len(), 3);
        assert!(executions[2]
            .prerequisite_context
            .contains("node=implement agent=claude-code attempt=2"));
        assert!(executions[2]
            .prerequisite_context
            .contains("implementation"));
    }

    #[test]
    fn non_retryable_failure_skips_dependent_without_using_fallback() {
        let world = Arc::new(World::new(vec![CoordinationExecutionResult::Failed {
            kind: CoordinationFailureKind::NonRetryable,
            error: "policy rejected".to_string(),
        }]));
        let service = service(world.clone());
        let mut request = start_request();
        request.nodes.push(CoordinationNodeInput {
            id: "z-docs".to_string(),
            primary_agent_id: "gemini-cli".to_string(),
            fallback_agent_ids: Vec::new(),
            instruction: "write docs".to_string(),
            depends_on: Vec::new(),
        });
        let started = service.start(request).expect("start");
        service.execute(&started.run_id).expect("execute");

        let run = service.get(&started.run_id).expect("run");
        assert_eq!(run.status, CoordinationRunStatus::Failed);
        assert_eq!(
            run.node("review").map(|node| node.status),
            Some(CoordinationNodeStatus::Skipped)
        );
        assert_eq!(
            run.node("z-docs").map(|node| node.status),
            Some(CoordinationNodeStatus::Succeeded)
        );
        assert_eq!(world.executions.lock().expect("executions").len(), 2);
    }

    #[test]
    fn startup_recovery_closes_orphan_attempt_and_resumes_with_fallback() {
        let world = Arc::new(World::new(vec![success("fallback output")]));
        let service = service(world.clone());
        let request = StartCoordinationRequest {
            name: "recovery".to_string(),
            project_path: None,
            nodes: vec![CoordinationNodeInput {
                id: "implement".to_string(),
                primary_agent_id: "codex-cli".to_string(),
                fallback_agent_ids: vec!["claude-code".to_string()],
                instruction: "implement".to_string(),
                depends_on: Vec::new(),
            }],
        };
        let started = service.start(request).expect("start");
        let mut run = service.get(&started.run_id).expect("run");
        run.status = CoordinationRunStatus::Running;
        let node = run.node_mut("implement").expect("node");
        node.status = CoordinationNodeStatus::Running;
        node.attempts.push(CoordinationAttempt {
            attempt: 1,
            agent_id: "codex-cli".to_string(),
            candidate_role: CoordinationCandidateRole::Primary,
            status: CoordinationAttemptStatus::Running,
            failure_kind: None,
            error: None,
            started_at: "2026-07-23T00:00:00Z".to_string(),
            completed_at: None,
        });
        world.save(&run).expect("save orphan");

        assert_eq!(service.recover_startup().expect("recover"), vec![run.id]);
        service.execute(&started.run_id).expect("resume");

        let recovered = service.get(&started.run_id).expect("recovered run");
        let node = recovered.node("implement").expect("node");
        assert_eq!(node.status, CoordinationNodeStatus::Succeeded);
        assert_eq!(node.actual_agent_id.as_deref(), Some("claude-code"));
        assert_eq!(node.attempts.len(), 2);
        assert_eq!(
            node.attempts[0].failure_kind,
            Some(CoordinationFailureKind::Retryable)
        );
    }

    #[test]
    fn scheduler_failure_persists_terminal_state_without_exposing_error_detail() {
        let world = Arc::new(World::new(Vec::new()));
        let service = service(world.clone());
        let started = service.start(start_request()).expect("start");

        service
            .handle_scheduler_failure(
                &started.run_id,
                &AgentRuntimeApplicationError::PolicyDenied {
                    session_id: "secret-session".to_string(),
                    action: "secret-action".to_string(),
                },
            )
            .expect("settle scheduler failure");

        let run = service.get(&started.run_id).expect("run");
        assert_eq!(run.status, CoordinationRunStatus::Failed);
        assert_eq!(
            run.node("implement").map(|node| node.status),
            Some(CoordinationNodeStatus::Failed)
        );
        assert_eq!(
            run.node("review").map(|node| node.status),
            Some(CoordinationNodeStatus::Skipped)
        );
        let logs = world.logs.lock().expect("logs");
        let scheduler_log = logs
            .iter()
            .find(|log| log.category == "coordination.scheduler.failure")
            .expect("scheduler log");
        assert!(scheduler_log.message.contains("validation_or_policy"));
        assert!(!scheduler_log.message.contains("secret"));
    }

    #[test]
    fn active_cancellation_stops_attempt_without_starting_fallback() {
        let world = Arc::new(World::blocking(vec![success("late output")]));
        let service = service(world.clone());
        let started = service.start(start_request()).expect("start");
        let run_id = started.run_id.clone();
        let worker_service = service.clone();
        let worker = std::thread::spawn(move || worker_service.execute(&run_id));

        world.wait_for_execution_start();
        let cancelling = service.cancel(&started.run_id).expect("cancel");
        assert!(cancelling.cancel_requested);
        worker.join().expect("worker thread").expect("execute");

        let run = service.get(&started.run_id).expect("run");
        assert_eq!(run.status, CoordinationRunStatus::Cancelled);
        assert_eq!(
            run.node("implement").map(|node| node.status),
            Some(CoordinationNodeStatus::Cancelled)
        );
        assert_eq!(
            run.node("review").map(|node| node.status),
            Some(CoordinationNodeStatus::Skipped)
        );
        assert_eq!(world.executions.lock().expect("executions").len(), 1);
        assert_eq!(*world.cancel_calls.lock().expect("cancel calls"), 1);
    }

    #[test]
    fn combined_prerequisite_context_overflow_fails_before_dependent_execution() {
        let bounded_output =
            "x".repeat(crate::contexts::agent_runtime::domain::COORDINATION_OUTPUT_LIMIT_BYTES);
        let world = Arc::new(World::new(
            (0..5).map(|_| success(&bounded_output)).collect(),
        ));
        let service = service(world.clone());
        let dependency_ids = ["a", "b", "c", "d", "e"];
        let mut nodes = dependency_ids
            .iter()
            .map(|id| CoordinationNodeInput {
                id: (*id).to_string(),
                primary_agent_id: "codex-cli".to_string(),
                fallback_agent_ids: Vec::new(),
                instruction: format!("produce {id}"),
                depends_on: Vec::new(),
            })
            .collect::<Vec<_>>();
        nodes.push(CoordinationNodeInput {
            id: "target".to_string(),
            primary_agent_id: "gemini-cli".to_string(),
            fallback_agent_ids: vec!["claude-code".to_string()],
            instruction: "consume prerequisites".to_string(),
            depends_on: dependency_ids.iter().map(ToString::to_string).collect(),
        });
        let started = service
            .start(StartCoordinationRequest {
                name: "context overflow".to_string(),
                project_path: None,
                nodes,
            })
            .expect("start");
        service.execute(&started.run_id).expect("execute");

        let run = service.get(&started.run_id).expect("run");
        let target = run.node("target").expect("target");
        assert_eq!(run.status, CoordinationRunStatus::Failed);
        assert_eq!(target.status, CoordinationNodeStatus::Failed);
        assert!(target
            .error
            .as_deref()
            .is_some_and(|error| error.contains("Prerequisite context exceeds")));
        assert!(target.attempts.is_empty());
        assert_eq!(world.executions.lock().expect("executions").len(), 5);
    }

    fn agent(agent_id: &str) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        AgentDefinition::new(AgentDefinitionInput {
            id: agent_id.to_string(),
            display_name: agent_id.to_string(),
            provider: "fixture".to_string(),
            managed_sdk_dependency_id: None,
            launch: LaunchMetadata::new(
                "cli".to_string(),
                Some(agent_id.to_string()),
                None,
                Some(agent_id.to_string()),
            )?,
            supported_interaction_modes: vec![InteractionMode::Cli],
            availability: AvailabilityAssessment::new(AgentAvailability::Available, None),
            capability_tags: Vec::new(),
        })
        .map_err(Into::into)
    }

    fn coordination_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
        AgentRuntimeApplicationError::Coordination(error.to_string())
    }

    fn success(content: &str) -> CoordinationExecutionResult {
        CoordinationExecutionResult::Succeeded(CoordinationExecutionOutput {
            content: content.to_string(),
            byte_count: content.len(),
            truncated: false,
        })
    }

    #[allow(dead_code)]
    fn _type_guard(_: BTreeMap<String, String>) {}
}
