use super::application::{
    aggregate_evaluation, aggregate_verification, compare_aggregates, verify_diff_rules,
    EvaluationAggregate, EvaluationRepositoryPort, EvaluationVerifierPort,
};
use super::domain::{
    parse_evaluation_manifest, safe_dispatch_diagnostic, EvaluationAgentSnapshot, EvaluationArena,
    EvaluationAttempt, EvaluationCheck, EvaluationExport, EvaluationManifest, EvaluationOutcome,
    EVALUATION_RANKING_VERSION, EVALUATION_SCHEMA_VERSION,
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

/// Stable id for the diagnostic recorded when an attempt's Agent could not be dispatched.
const DISPATCH_CHECK_ID: &str = "agent-dispatch";

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
                if let Err(error) = background.execute(&arena_id) {
                    // Whatever `execute` gave up on -- a fixture that would not prepare, a run
                    // transition that was refused -- it abandoned every attempt it had not reached
                    // yet at `queued`. Queued is not a verdict: the client polls a non-terminal
                    // arena forever, so the arena has to be closed out here or it never settles.
                    background.abandon(&arena_id);
                    let _ = background
                        .operations
                        .fail(&operation_id, safe_benchmark_error(&error));
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

    /// Closes out an arena whose run aborted, so attempts it never reached carry a verdict.
    fn abandon(&self, arena_id: &str) {
        let Ok(mut arena) = self.get(arena_id) else {
            return;
        };
        for index in 0..arena.attempts.len() {
            if arena.attempts[index].outcome.is_terminal() {
                continue;
            }
            arena.attempts[index].outcome = EvaluationOutcome::BenchmarkError;
            let _ = self.repository.save_terminal(
                &arena,
                &arena.attempts[index],
                &chrono::Utc::now().to_rfc3339(),
            );
            let _ = self
                .workspaces
                .cleanup_evaluation_fixture(&self.run_root, &arena.attempts[index].id);
        }
    }

    pub(crate) fn execute(&self, arena_id: &str) -> Result<EvaluationArena, String> {
        let mut arena = self
            .get(arena_id)
            .map_err(|_| "evaluation arena loading failed".to_string())?;
        let manifest = self
            .manifest(&arena.task_id, arena.task_version)
            .map_err(|_| "evaluation manifest loading failed".to_string())?;
        for index in 0..arena.attempts.len() {
            let attempt = arena.attempts[index].clone();
            self.runs
                .transition(&attempt.canonical_run_id, RunTrigger::Prepare, None)
                .map_err(|_| "evaluation run preparation failed".to_string())?;
            self.runs
                .transition(&attempt.canonical_run_id, RunTrigger::Start, None)
                .map_err(|_| "evaluation run start failed".to_string())?;
            let source = self.fixture_root.join(&manifest.fixture);
            let prepared = self
                .workspaces
                .prepare_evaluation_fixture(&source, &self.run_root, &attempt.id)
                .map_err(|_| "evaluation workspace preparation failed".to_string())?;
            let dispatch = self.agent.dispatch(&EvaluationDispatchRequest {
                task_id: manifest.id.clone(),
                attempt_id: attempt.id.clone(),
                agent_id: attempt.agent.agent_id.clone(),
                prompt: manifest.prompt.clone(),
                workspace: prepared.workspace_path.clone(),
                timeout_seconds: u64::from(manifest.timeout_seconds),
                canonical_run_id: attempt.canonical_run_id.clone(),
            });
            let mut checks = manifest
                .acceptance
                .verifier_profiles
                .iter()
                // Static files and diff rules are expanded into their real bounded checks below;
                // persisting the verifier adapter's placeholder as well would duplicate the
                // `diff-rules` check id and reject an otherwise successful terminal write.
                .filter(|profile| !matches!(profile.as_str(), "static-files" | "diff-rules"))
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
            let changed_paths = self
                .workspaces
                .changed_evaluation_paths(
                    &source,
                    PathBuf::from(&prepared.workspace_path).as_path(),
                )
                .map_err(|_| "evaluation diff collection failed".to_string())?;
            checks.push(Ok(verify_diff_rules(&changed_paths)));
            // Kept before the aggregate consumes it: `aggregate_evaluation` maps a dispatch `Err`
            // to `agent_failed` and drops the reason, which left the user with an empty panel and
            // nothing anywhere on screen saying the Agent had, say, no configured model.
            let dispatch_error = dispatch.as_ref().err().cloned();
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
            if let Some(error) = dispatch_error {
                // Evidence, not a verdict: the outcome above is already `agent_failed`, and
                // `failed_checks` deliberately ignores checks on a non-completion outcome so
                // recording a reason cannot rank this attempt below one that recorded nothing.
                arena.attempts[index].checks.push(EvaluationCheck {
                    check_id: DISPATCH_CHECK_ID.into(),
                    passed: false,
                    summary: safe_dispatch_diagnostic(&error),
                });
            }
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
                .map_err(|_| "evaluation run finalization failed".to_string())?;
            self.repository
                .save_terminal(
                    &arena,
                    &arena.attempts[index],
                    &chrono::Utc::now().to_rfc3339(),
                )
                .map_err(|_| "evaluation result persistence failed".to_string())?;
            if arena.attempts[index].outcome != EvaluationOutcome::Succeeded {
                let _ = self
                    .workspaces
                    .cleanup_evaluation_fixture(&self.run_root, &attempt.id);
            }
        }
        let arena = ranked(arena);
        self.operations
            .complete(&arena.operation_id, serde_json::to_value(&arena).ok())
            .map_err(|_| "evaluation operation completion failed".to_string())?;
        Ok(arena)
    }

    pub(crate) fn get(&self, arena_id: &str) -> Result<EvaluationArena, String> {
        self.repository
            .get(arena_id)?
            .map(ranked)
            .ok_or_else(|| "evaluation arena not found".into())
    }
    pub(crate) fn list(&self, offset: usize, limit: usize) -> Result<Vec<EvaluationArena>, String> {
        Ok(self
            .repository
            .list(offset, limit)?
            .into_iter()
            .map(ranked)
            .collect())
    }
    pub(crate) fn cancel(&self, arena_id: &str) -> Result<EvaluationArena, String> {
        let mut arena = self.get(arena_id)?;
        for index in 0..arena.attempts.len() {
            let attempt = arena.attempts[index].clone();
            if !attempt.outcome.is_terminal() {
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

fn safe_benchmark_error(error: &str) -> String {
    const SAFE_STAGE_ERRORS: [&str; 9] = [
        "evaluation arena loading failed",
        "evaluation manifest loading failed",
        "evaluation run preparation failed",
        "evaluation run start failed",
        "evaluation workspace preparation failed",
        "evaluation diff collection failed",
        "evaluation run finalization failed",
        "evaluation result persistence failed",
        "evaluation operation completion failed",
    ];
    if SAFE_STAGE_ERRORS.contains(&error) {
        error.to_string()
    } else {
        "evaluation benchmark failed safely".to_string()
    }
}

/// Orders an arena's attempts best-first under `EVALUATION_RANKING_VERSION`.
///
/// Applied on every read rather than persisted: the repository stores attempts individually and
/// returns them ordered by their random UUID, so a ranking computed once at the end of a run
/// reached nobody -- the arena existed to rank Agents against each other and the client was handed
/// them in arrival order. Ranking is a pure function of the outcome, checks and metrics already
/// stored, so deriving it on read costs nothing and cannot drift from what was saved. The id is
/// the last tiebreak so two indistinguishable attempts still come back in a stable order.
fn ranked(mut arena: EvaluationArena) -> EvaluationArena {
    arena.attempts.sort_by(|left, right| {
        compare_aggregates(&attempt_aggregate(left), &attempt_aggregate(right))
            .then_with(|| left.id.cmp(&right.id))
    });
    arena
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::execution_observability::domain::{
        EvaluationCheck, SAFE_DISPATCH_REASONS,
    };

    fn attempt(id: &str, outcome: EvaluationOutcome, failed_checks: usize) -> EvaluationAttempt {
        EvaluationAttempt {
            id: id.into(),
            arena_id: "arena".into(),
            canonical_run_id: format!("run-{id}"),
            task_id: "task".into(),
            task_version: 1,
            agent: EvaluationAgentSnapshot {
                agent_id: id.into(),
                provider_id: "provider".into(),
                model_id: None,
                interaction_mode: "api".into(),
                configuration_fingerprint: "fingerprint".into(),
            },
            outcome,
            checks: (0..failed_checks)
                .map(|index| EvaluationCheck {
                    check_id: format!("check-{index}"),
                    passed: false,
                    summary: String::new(),
                })
                .collect(),
            judge: None,
            metrics: Vec::new(),
            context_evidence_manifest_id: None,
            artifact_ids: Vec::new(),
        }
    }

    fn arena(attempts: Vec<EvaluationAttempt>) -> EvaluationArena {
        EvaluationArena {
            id: "arena".into(),
            operation_id: "operation".into(),
            task_id: "task".into(),
            task_version: 1,
            ranking_version: EVALUATION_RANKING_VERSION.into(),
            attempts,
        }
    }

    /// The repository hands attempts back ordered by their random id, so ranking has to be applied
    /// on the way out or the arena's whole reason to exist -- comparing Agents -- never reaches the
    /// client.
    #[test]
    fn ranking_beats_repository_id_order() {
        let ordered = ranked(arena(vec![
            attempt("attempt-a", EvaluationOutcome::TaskFailed, 2),
            attempt("attempt-b", EvaluationOutcome::TaskFailed, 1),
            attempt("attempt-c", EvaluationOutcome::Succeeded, 0),
            // Recorded no checks at all: it must sort last rather than winning on an empty count.
            attempt("attempt-d", EvaluationOutcome::AgentFailed, 0),
            attempt("attempt-e", EvaluationOutcome::TimedOut, 0),
        ]));
        assert_eq!(
            ordered
                .attempts
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "attempt-c",
                "attempt-b",
                "attempt-a",
                "attempt-d",
                "attempt-e"
            ],
        );
    }

    /// The diagnostic recorded for a failed dispatch travels the same redaction rule as every
    /// other evaluation field, so a reason quoting a path or a secret collapses to the safe
    /// sentence instead of being persisted, exported, and rendered.
    #[test]
    fn dispatch_diagnostics_are_redacted_before_they_are_recorded() {
        for leaking in [
            "database failed at /home/user/private.db",
            "authentication rejected token sk-live-4f9c2a77b1e34d0e",
            // Near-misses: the rule is equality, so a reason that merely *contains* a safe one
            // does not inherit its safety.
            "evaluation Agent is not installed and available at /home/user/.local/bin/agy",
            "evaluation supports OnePiece",
        ] {
            let summary = safe_dispatch_diagnostic(leaking);
            assert!(!summary.contains("/home"), "{summary}");
            assert!(!summary.contains("sk-live"), "{summary}");
            assert_eq!(
                summary,
                "evaluation operation failed; inspect unified logs for redacted diagnostics",
            );
        }
        // The reasons the dispatch gate itself writes survive verbatim -- that is what makes
        // recording a diagnostic worth doing rather than printing "it failed" twice.
        for reason in SAFE_DISPATCH_REASONS {
            assert_eq!(safe_dispatch_diagnostic(reason), reason);
        }
    }

    #[test]
    fn benchmark_failures_expose_only_fixed_stage_diagnostics() {
        assert_eq!(
            safe_benchmark_error("evaluation result persistence failed"),
            "evaluation result persistence failed"
        );
        assert_eq!(
            safe_benchmark_error("database failed at /home/user/private.db"),
            "evaluation benchmark failed safely"
        );
    }

    #[test]
    fn indistinguishable_attempts_keep_a_stable_order() {
        let ordered = ranked(arena(vec![
            attempt("attempt-z", EvaluationOutcome::Succeeded, 0),
            attempt("attempt-a", EvaluationOutcome::Succeeded, 0),
        ]));
        assert_eq!(ordered.attempts[0].id, "attempt-a");
    }

    /// Cancelling must not rewrite a verdict an attempt already earned: a timed-out attempt
    /// reported as cancelled loses the one fact that explains why it failed.
    #[test]
    fn every_settled_outcome_is_terminal() {
        for outcome in [
            EvaluationOutcome::Succeeded,
            EvaluationOutcome::TaskFailed,
            EvaluationOutcome::AgentFailed,
            EvaluationOutcome::TimedOut,
            EvaluationOutcome::Stuck,
            EvaluationOutcome::Cancelled,
            EvaluationOutcome::BenchmarkError,
        ] {
            assert!(outcome.is_terminal(), "{outcome:?} should be terminal");
        }
        assert!(!EvaluationOutcome::Queued.is_terminal());
        assert!(!EvaluationOutcome::Running.is_terminal());
    }
}
