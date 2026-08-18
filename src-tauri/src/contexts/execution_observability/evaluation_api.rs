use super::application::{
    aggregate_evaluation, aggregate_verification, compare_aggregates, verify_diff_rules,
    EvaluationAggregate, EvaluationRepositoryPort, EvaluationVerifierPort,
};
use super::domain::{
    parse_evaluation_manifest, EvaluationAgentSnapshot, EvaluationArena, EvaluationAttempt,
    EvaluationExport, EvaluationManifest, EvaluationOutcome, EVALUATION_RANKING_VERSION,
    EVALUATION_SCHEMA_VERSION,
};
use super::infrastructure::{
    verify_static_acceptance, EvaluationDispatchRequest, NativeEvaluationAgentAdapter,
    NativeEvaluationVerifierAdapter,
};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::operations::api::{
    AgentRunsApi, CreateAgentRun, OperationKind, OperationsApi, RunOwner, RunRecoveryPolicy,
    RunTrigger,
};
use crate::contexts::workspaces::api::WorkspaceApi;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartEvaluationRequest {
    pub(crate) task_id: String,
    pub(crate) task_version: u32,
    pub(crate) agent_ids: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct EvaluationApi {
    repository: Arc<dyn EvaluationRepositoryPort>,
    operations: OperationsApi,
    runs: AgentRunsApi,
    runtime: AgentRuntimeApi,
    agent: NativeEvaluationAgentAdapter,
    verifier: NativeEvaluationVerifierAdapter,
    workspaces: WorkspaceApi,
    fixture_root: PathBuf,
    run_root: PathBuf,
}

impl EvaluationApi {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        repository: Arc<dyn EvaluationRepositoryPort>,
        operations: OperationsApi,
        runs: AgentRunsApi,
        runtime: AgentRuntimeApi,
        agent: NativeEvaluationAgentAdapter,
        verifier: NativeEvaluationVerifierAdapter,
        workspaces: WorkspaceApi,
        fixture_root: PathBuf,
        run_root: PathBuf,
    ) -> Self {
        Self {
            repository,
            operations,
            runs,
            runtime,
            agent,
            verifier,
            workspaces,
            fixture_root,
            run_root,
        }
    }

    pub(crate) fn list_tasks(&self) -> Result<Vec<EvaluationManifest>, String> {
        built_in_manifests()
            .into_iter()
            .map(|value| parse_evaluation_manifest(value).map_err(|error| format!("{error:?}")))
            .collect()
    }

    pub(crate) fn start(&self, request: StartEvaluationRequest) -> Result<EvaluationArena, String> {
        let manifest = self.manifest(&request.task_id, request.task_version)?;
        if request.agent_ids.is_empty()
            || request.agent_ids.len() > super::application::MAX_ARENA_ATTEMPTS
        {
            return Err("evaluation requires between one and eight Agents".into());
        }
        let arena_id = format!("eval-{}", uuid::Uuid::new_v4());
        let operation = self
            .operations
            .start(
                OperationKind::Agent,
                Some(arena_id.clone()),
                Some(format!("Evaluating {}", manifest.id)),
            )
            .map_err(display)?;
        let mut arena = EvaluationArena {
            id: arena_id.clone(),
            operation_id: operation.id,
            task_id: manifest.id.clone(),
            task_version: manifest.version,
            ranking_version: EVALUATION_RANKING_VERSION.into(),
            attempts: Vec::new(),
        };
        for agent_id in request.agent_ids {
            let view = self.runtime.get_agent(&agent_id).map_err(display)?;
            let attempt_id = format!("attempt-{}", uuid::Uuid::new_v4());
            let canonical = self
                .runs
                .create(CreateAgentRun {
                    id: None,
                    owner: RunOwner {
                        owner_type: "evaluation_attempt".into(),
                        owner_id: attempt_id.clone(),
                    },
                    links: Vec::new(),
                    parent_run_id: None,
                    recovery_policy: RunRecoveryPolicy::NotRecoverable,
                    runner: None,
                    max_retries: 0,
                    witness: format!("evaluation:{attempt_id}:created"),
                })
                .map_err(display)?;
            let mode = if agent_id == "onepiece" { "api" } else { "cli" };
            let attempt = EvaluationAttempt {
                id: attempt_id,
                arena_id: arena_id.clone(),
                canonical_run_id: canonical.id,
                task_id: manifest.id.clone(),
                task_version: manifest.version,
                agent: EvaluationAgentSnapshot {
                    agent_id: view.id,
                    provider_id: view.provider,
                    model_id: None,
                    interaction_mode: mode.into(),
                    configuration_fingerprint: "runtime-snapshot-v1".into(),
                },
                outcome: EvaluationOutcome::Queued,
                checks: Vec::new(),
                judge: None,
                metrics: Vec::new(),
                context_evidence_manifest_id: None,
                artifact_ids: Vec::new(),
            };
            arena.attempts.push(attempt.clone());
            self.repository
                .save_terminal(&arena, &attempt, &chrono::Utc::now().to_rfc3339())?;
        }
        Ok(arena)
    }

    pub(crate) fn start_async(
        &self,
        request: StartEvaluationRequest,
    ) -> Result<EvaluationArena, String> {
        let arena = self.start(request)?;
        let arena_id = arena.id.clone();
        let operation_id = arena.operation_id.clone();
        let background = self.clone();
        let worker = std::thread::Builder::new()
            .name("evaluation-arena".into())
            .spawn(move || {
                if background.execute(&arena_id).is_err() {
                    let _ = background
                        .operations
                        .fail(&operation_id, "evaluation benchmark failed safely".into());
                }
            });
        if worker.is_err() {
            let _ = self.operations.fail(
                &arena.operation_id,
                "evaluation worker is unavailable".into(),
            );
            return Err("evaluation worker is unavailable".into());
        }
        Ok(arena)
    }

    pub(crate) fn execute(&self, arena_id: &str) -> Result<EvaluationArena, String> {
        let mut arena = self.get(arena_id)?;
        let manifest = self.manifest(&arena.task_id, arena.task_version)?;
        for index in 0..arena.attempts.len() {
            let attempt = arena.attempts[index].clone();
            self.runs
                .transition(&attempt.canonical_run_id, RunTrigger::Prepare, None)
                .map_err(display)?;
            self.runs
                .transition(&attempt.canonical_run_id, RunTrigger::Start, None)
                .map_err(display)?;
            let source = self.fixture_root.join(&manifest.fixture);
            let prepared =
                self.workspaces
                    .prepare_evaluation_fixture(&source, &self.run_root, &attempt.id)?;
            let dispatch = self.agent.dispatch(&EvaluationDispatchRequest {
                task_id: manifest.id.clone(),
                attempt_id: attempt.id.clone(),
                agent_id: attempt.agent.agent_id.clone(),
                provider_id: Some(attempt.agent.provider_id.clone()),
                model_id: attempt.agent.model_id.clone(),
                prompt: manifest.prompt.clone(),
                workspace: prepared.workspace_path.clone(),
                timeout_seconds: u64::from(manifest.timeout_seconds),
                canonical_run_id: attempt.canonical_run_id.clone(),
            });
            let mut checks = manifest
                .acceptance
                .verifier_profiles
                .iter()
                .map(|profile| self.verifier.verify(profile, &prepared.workspace_path))
                .collect::<Vec<_>>();
            checks.extend(
                verify_static_acceptance(
                    PathBuf::from(&prepared.workspace_path).as_path(),
                    &manifest.acceptance,
                )
                .unwrap_or_else(|error| {
                    vec![super::domain::EvaluationCheck {
                        check_id: "static-files".into(),
                        passed: false,
                        summary: error,
                    }]
                })
                .into_iter()
                .map(Ok),
            );
            let changed_paths = self.workspaces.changed_evaluation_paths(
                &source,
                PathBuf::from(&prepared.workspace_path).as_path(),
            )?;
            checks.push(Ok(verify_diff_rules(&changed_paths)));
            let aggregate =
                aggregate_evaluation(dispatch.map(|result| result.evidence), checks, None);
            let verification = aggregate_verification(aggregate.checks.clone(), None, None);
            arena.attempts[index].outcome = if matches!(
                aggregate.outcome,
                EvaluationOutcome::Succeeded | EvaluationOutcome::TaskFailed
            ) {
                verification.outcome
            } else {
                aggregate.outcome.clone()
            };
            arena.attempts[index].checks = aggregate.checks;
            arena.attempts[index].judge = verification.judge;
            arena.attempts[index].metrics = aggregate.metrics;
            arena.attempts[index].artifact_ids = changed_paths;
            let trigger = if arena.attempts[index].outcome == EvaluationOutcome::Succeeded {
                RunTrigger::Complete
            } else {
                RunTrigger::Fail
            };
            self.runs
                .transition(&attempt.canonical_run_id, trigger, None)
                .map_err(display)?;
            self.repository.save_terminal(
                &arena,
                &arena.attempts[index],
                &chrono::Utc::now().to_rfc3339(),
            )?;
            if arena.attempts[index].outcome != EvaluationOutcome::Succeeded {
                let _ = self
                    .workspaces
                    .cleanup_evaluation_fixture(&self.run_root, &attempt.id);
            }
        }
        arena.attempts.sort_by(|left, right| {
            compare_aggregates(&attempt_aggregate(left), &attempt_aggregate(right))
        });
        self.operations
            .complete(&arena.operation_id, serde_json::to_value(&arena).ok())
            .map_err(display)?;
        Ok(arena)
    }

    pub(crate) fn get(&self, arena_id: &str) -> Result<EvaluationArena, String> {
        self.repository
            .get(arena_id)?
            .ok_or_else(|| "evaluation arena not found".into())
    }
    pub(crate) fn list(&self, offset: usize, limit: usize) -> Result<Vec<EvaluationArena>, String> {
        self.repository.list(offset, limit)
    }
    pub(crate) fn cancel(&self, arena_id: &str) -> Result<EvaluationArena, String> {
        let mut arena = self.get(arena_id)?;
        for index in 0..arena.attempts.len() {
            let attempt = arena.attempts[index].clone();
            if !matches!(
                attempt.outcome,
                EvaluationOutcome::Succeeded
                    | EvaluationOutcome::TaskFailed
                    | EvaluationOutcome::AgentFailed
            ) {
                let run = self.runs.get(&attempt.canonical_run_id).map_err(display)?;
                let _ = self.runs.cancel(&run.id, run.version);
                arena.attempts[index].outcome = EvaluationOutcome::Cancelled;
                self.repository.save_terminal(
                    &arena,
                    &arena.attempts[index],
                    &chrono::Utc::now().to_rfc3339(),
                )?;
                let _ = self
                    .workspaces
                    .cleanup_evaluation_fixture(&self.run_root, &attempt.id);
            }
        }
        let _ = self.operations.cancel(&arena.operation_id);
        Ok(arena)
    }
    pub(crate) fn export(&self, arena_id: &str) -> Result<EvaluationExport, String> {
        Ok(EvaluationExport {
            schema_version: EVALUATION_SCHEMA_VERSION,
            arena: self.get(arena_id)?,
        })
    }
    fn manifest(&self, id: &str, version: u32) -> Result<EvaluationManifest, String> {
        self.list_tasks()?
            .into_iter()
            .find(|item| item.id == id && item.version == version)
            .ok_or_else(|| "evaluation task version not found".into())
    }
}

fn attempt_aggregate(attempt: &EvaluationAttempt) -> EvaluationAggregate {
    EvaluationAggregate {
        outcome: attempt.outcome.clone(),
        checks: attempt.checks.clone(),
        metrics: attempt.metrics.clone(),
        flaky: false,
    }
}

fn built_in_manifests() -> [&'static str; 3] {
    [
        include_str!("../../../evaluation-fixtures/fix-null-auth-token/manifest.yaml"),
        include_str!("../../../evaluation-fixtures/add-parser-test/manifest.yaml"),
        include_str!("../../../evaluation-fixtures/refactor-search/manifest.yaml"),
    ]
}
fn display(error: impl std::fmt::Display) -> String {
    error.to_string()
}
