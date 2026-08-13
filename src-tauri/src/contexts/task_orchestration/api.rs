//! Published in-process task-orchestration facade.

pub(crate) use super::application::GeneratePlanDraftRequest;
pub(crate) use super::application::PlanApplicationError;
use super::application::{build_attempt_context, AttemptContextRequest, PlanApplicationService};
pub(crate) use super::application::{
    PlanAttemptEvidenceView, PlanRunDetailView, PlanRunPageView, PlanRunSummaryView,
};
use super::application::{PlanDiagnostic, PlanDiagnosticLevel, PlanDiagnosticsPort};
pub(crate) use super::domain::PlanDraft;
use super::infrastructure::{
    AttemptTerminalUpdate, NativePlanDriverRegistry, NativeRecoveryEvidenceGateway,
    OnePieceAttemptExecutor, OnePieceAttemptVerifier, OnePiecePlanGenerator, PlanRunWorktree,
    SqlitePlanRepository,
};
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::operations::api::OperationsApi;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::workspaces::api::WorkspaceApi;
use crate::platform::database::NativeDatabase;
use serde::Serialize;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct TaskOrchestrationApi {
    plans: PlanApplicationService,
    repository: SqlitePlanRepository,
    workspaces: WorkspaceApi,
    executor: OnePieceAttemptExecutor,
    verifier: OnePieceAttemptVerifier,
    diagnostics: Arc<dyn PlanDiagnosticsPort>,
    active_drivers: NativePlanDriverRegistry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparedPlanRun {
    pub(crate) run_id: String,
    pub(crate) status: String,
    pub(crate) project_path: String,
    pub(crate) base_oid: String,
    pub(crate) worktree_path: String,
    pub(crate) worktree_name: String,
    pub(crate) worktree_branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecutedPlanAttempt {
    pub(crate) attempt_id: String,
    pub(crate) session_id: String,
    pub(crate) status: String,
    pub(crate) context_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlanControlResult {
    pub(crate) request_id: String,
    pub(crate) run: PlanRunDetailView,
}

impl TaskOrchestrationApi {
    pub(crate) fn native(
        database: NativeDatabase,
        sessions: SessionsApi,
        agents: AgentRuntimeApi,
        workspaces: WorkspaceApi,
        operations: OperationsApi,
        diagnostics: Arc<dyn PlanDiagnosticsPort>,
    ) -> Result<Self, PlanApplicationError> {
        let repository = SqlitePlanRepository::new(database);
        repository.recover_ambiguous_inflight(
            &NativeRecoveryEvidenceGateway::new(sessions.clone(), operations),
            &chrono::Utc::now().to_rfc3339(),
        )?;
        let executor = OnePieceAttemptExecutor::new(sessions.clone(), agents.clone());
        let verifier = OnePieceAttemptVerifier::new(agents.clone());
        let api = Self {
            plans: PlanApplicationService::new(
                Arc::new(repository.clone()),
                Arc::new(OnePiecePlanGenerator::new(agents, sessions.clone())),
            ),
            repository,
            workspaces,
            executor,
            verifier,
            diagnostics,
            active_drivers: NativePlanDriverRegistry::default(),
        };
        for run_id in api.repository.runnable_driver_run_ids()? {
            api.activate_driver(&run_id)?;
        }
        Ok(api)
    }

    pub(crate) fn save_plan_draft(
        &self,
        draft: &PlanDraft,
    ) -> Result<PlanDraft, PlanApplicationError> {
        self.plans.save_draft(draft)
    }

    pub(crate) fn generate_plan_draft(
        &self,
        request: &GeneratePlanDraftRequest,
    ) -> Result<PlanDraft, PlanApplicationError> {
        match self.plans.generate_draft(request) {
            Ok(draft) => {
                self.record_lifecycle(
                    PlanDiagnosticLevel::Info,
                    "plan.discovery.completed",
                    None,
                    Some("draft"),
                );
                self.record_lifecycle(
                    PlanDiagnosticLevel::Info,
                    "plan.draft.generated",
                    None,
                    Some("draft"),
                );
                Ok(draft)
            }
            Err(error) => {
                self.record_lifecycle(
                    PlanDiagnosticLevel::Warn,
                    "plan.draft.generation_failed",
                    None,
                    None,
                );
                Err(error)
            }
        }
    }

    pub(crate) fn find_plan_draft(
        &self,
        plan_id: &str,
    ) -> Result<Option<PlanDraft>, PlanApplicationError> {
        self.plans.find_draft(plan_id)
    }

    pub(crate) fn validate_plan_draft(
        &self,
        draft: &PlanDraft,
    ) -> Result<(), PlanApplicationError> {
        self.plans.validate_draft(draft)
    }

    pub(crate) fn list_plan_versions(
        &self,
        plan_id: &str,
    ) -> Result<Vec<PlanDraft>, PlanApplicationError> {
        self.plans.list_versions(plan_id)
    }

    pub(crate) fn delete_plan_draft(&self, plan_id: &str) -> Result<(), PlanApplicationError> {
        self.plans.delete_draft(plan_id)
    }

    pub(crate) fn approve_plan(
        &self,
        plan_id: &str,
        originating_session_id: Option<&str>,
        now: &str,
    ) -> Result<String, PlanApplicationError> {
        let run_id =
            self.repository
                .approve_latest_for_session(plan_id, originating_session_id, now)?;
        self.record_lifecycle(
            PlanDiagnosticLevel::Info,
            "plan.run.approved",
            Some(&run_id),
            Some("queued"),
        );
        Ok(run_id)
    }

    pub(crate) fn find_plan_run_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<PlanRunSummaryView>, PlanApplicationError> {
        self.repository.find_run_for_originating_session(session_id)
    }

    pub(crate) fn prepare_plan_run(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<PreparedPlanRun, PlanApplicationError> {
        let preparation = self.repository.begin_preparation(run_id, now)?;
        let worktree = self
            .workspaces
            .create_guarded_plan_worktree(
                &preparation.project_path,
                &format!("plan-{}", preparation.id),
                &preparation.base_ref,
            )
            .map_err(|error| PlanApplicationError::Storage(error.to_string()))?;
        self.repository.attach_worktree_and_start(
            run_id,
            &PlanRunWorktree {
                project_path: &worktree.project_path,
                base_oid: &worktree.base_oid,
                path: &worktree.path,
                name: &worktree.name,
                branch: &worktree.branch,
            },
            now,
        )?;
        self.record_lifecycle(
            PlanDiagnosticLevel::Info,
            "plan.run.started",
            Some(run_id),
            Some("running"),
        );
        self.activate_driver(run_id)?;
        Ok(PreparedPlanRun {
            run_id: run_id.to_string(),
            status: "running".to_string(),
            project_path: worktree.project_path,
            base_oid: worktree.base_oid,
            worktree_path: worktree.path,
            worktree_name: worktree.name,
            worktree_branch: worktree.branch,
        })
    }

    pub(crate) fn pause_plan_run(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<PlanControlResult, PlanApplicationError> {
        self.repository.request_pause(run_id, now)?;
        let result = self.control_result(run_id)?;
        self.record_lifecycle(
            PlanDiagnosticLevel::Info,
            "plan.run.pause_requested",
            Some(run_id),
            Some("pause_requested"),
        );
        Ok(result)
    }

    pub(crate) fn request_plan_control(
        &self,
        run_id: &str,
        kind: &str,
        now: &str,
    ) -> Result<PlanControlResult, PlanApplicationError> {
        match kind {
            "pause" => self.pause_plan_run(run_id, now),
            "resume" => self.resume_plan_run(run_id, now),
            "cancel" => self.cancel_plan_run(run_id, now),
            "retry" => {
                self.repository.retry_final_verification(run_id, now)?;
                self.activate_driver(run_id)?;
                self.record_lifecycle(
                    PlanDiagnosticLevel::Info,
                    "plan.finalization.retry_requested",
                    Some(run_id),
                    Some("running"),
                );
                self.control_result(run_id)
            }
            _ => Err(PlanApplicationError::Validation(
                "unsupported Plan control kind".to_string(),
            )),
        }
    }

    pub(crate) fn resume_plan_run(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<PlanControlResult, PlanApplicationError> {
        self.repository.resume_run(run_id, now)?;
        self.repository.project_after_attempt(run_id, now)?;
        self.activate_driver(run_id)?;
        let result = self.control_result(run_id)?;
        self.record_lifecycle(
            PlanDiagnosticLevel::Info,
            "plan.run.resumed",
            Some(run_id),
            Some("running"),
        );
        Ok(result)
    }

    pub(crate) fn cancel_plan_run(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<PlanControlResult, PlanApplicationError> {
        let result = self.repository.request_cancel(run_id, now)?;
        self.verifier.cancel(run_id);
        if let Some(session_id) = result.active_session_id {
            let _ = self.executor.stop_session(&session_id);
        }
        let result = self.control_result(run_id)?;
        self.record_lifecycle(
            PlanDiagnosticLevel::Warn,
            "plan.run.cancel_requested",
            Some(run_id),
            Some("cancel_requested"),
        );
        Ok(result)
    }

    pub(crate) fn retry_plan_subtask(
        &self,
        run_id: &str,
        subtask_run_id: &str,
        now: &str,
    ) -> Result<PlanControlResult, PlanApplicationError> {
        self.repository.retry_subtask(run_id, subtask_run_id, now)?;
        self.activate_driver(run_id)?;
        self.record_lifecycle(
            PlanDiagnosticLevel::Info,
            "plan.repair.retry_requested",
            Some(run_id),
            Some("running"),
        );
        self.control_result(run_id)
    }

    pub(crate) fn accept_plan_run(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<PlanControlResult, PlanApplicationError> {
        self.repository.accept_run(run_id, now)?;
        self.control_result(run_id)
    }

    pub(crate) fn recover_plan_run(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<PlanControlResult, PlanApplicationError> {
        self.repository.recover_run(run_id, now)?;
        let result = self.control_result(run_id)?;
        self.record_lifecycle(
            PlanDiagnosticLevel::Warn,
            "plan.run.recovered",
            Some(run_id),
            Some("paused"),
        );
        Ok(result)
    }

    pub(crate) fn list_plan_runs(
        &self,
        cursor: Option<&str>,
    ) -> Result<PlanRunPageView, PlanApplicationError> {
        self.repository.list_run_summaries(cursor)
    }

    pub(crate) fn get_plan_run(
        &self,
        run_id: &str,
    ) -> Result<PlanRunDetailView, PlanApplicationError> {
        self.repository.get_run_detail(run_id)
    }

    pub(crate) fn get_plan_attempt_evidence(
        &self,
        attempt_id: &str,
    ) -> Result<Vec<PlanAttemptEvidenceView>, PlanApplicationError> {
        self.repository.get_attempt_evidence(attempt_id)
    }

    fn control_result(&self, run_id: &str) -> Result<PlanControlResult, PlanApplicationError> {
        Ok(PlanControlResult {
            request_id: self.repository.latest_control_id(run_id)?,
            run: self.repository.get_run_detail(run_id)?,
        })
    }

    pub(crate) fn execute_next_attempt(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<Option<ExecutedPlanAttempt>, PlanApplicationError> {
        let claim = self.repository.schedule_next(run_id, now)?;
        let Some(subtask_run_id) = claim.subtask_run_id else {
            return Ok(None);
        };
        self.record_lifecycle(
            PlanDiagnosticLevel::Debug,
            "plan.driver.claimed",
            Some(run_id),
            Some("dispatching"),
        );
        let dispatch = self.repository.load_attempt_dispatch(&subtask_run_id)?;
        let context = build_attempt_context(&AttemptContextRequest {
            plan_run_id: dispatch.plan_run_id.clone(),
            subtask_run_id: dispatch.subtask_run_id.clone(),
            task: dispatch.task.clone(),
            direct_predecessor_ids: dispatch.direct_predecessor_ids.clone(),
            predecessor_sources: dispatch.predecessor_sources.clone(),
            repair: dispatch.repair.clone(),
            character_budget: dispatch
                .task
                .limits
                .token_budget
                .unwrap_or(8_000)
                .saturating_mul(4) as usize,
        })?;
        let attempt = self.repository.create_attempt(&subtask_run_id, now)?;
        let session = match self.executor.create_session(&dispatch) {
            Ok(session) => session,
            Err(error) => {
                let error_class = if error.to_string().contains("Profile") {
                    "missing_credentials"
                } else {
                    "session_creation_failed"
                };
                self.repository.fail_attempt_dispatch(
                    &subtask_run_id,
                    &attempt.id,
                    error_class,
                    now,
                )?;
                return Err(error);
            }
        };
        if let Err(error) = self.repository.start_attempt(
            &subtask_run_id,
            &attempt.id,
            &session.session_id,
            &session.profile_id,
            None,
            now,
        ) {
            self.repository.fail_attempt_dispatch(
                &subtask_run_id,
                &attempt.id,
                "attempt_start_conflict",
                now,
            )?;
            return Err(error);
        }
        let output = match self
            .executor
            .execute(&dispatch, &attempt.id, &session, context.prompt)
        {
            Ok(output) => output,
            Err(_error) => super::infrastructure::AttemptExecutionOutput {
                succeeded: false,
                result_summary: None,
                error_class: Some("execution_error".to_string()),
                operation_id: None,
                execution_run_id: None,
            },
        };
        self.repository.correlate_attempt_execution(
            &attempt.id,
            output.operation_id.as_deref(),
            output.execution_run_id.as_deref(),
        )?;
        self.diagnostics.record(PlanDiagnostic {
            level: PlanDiagnosticLevel::Debug,
            event: "plan.attempt.correlated",
            plan_run_id: Some(run_id.to_string()),
            subtask_run_id: Some(subtask_run_id.clone()),
            attempt_id: Some(attempt.id.clone()),
            session_id: Some(session.session_id.clone()),
            operation_id: output.operation_id.clone(),
            execution_run_id: output.execution_run_id.clone(),
            state: Some("running"),
            error_class: None,
        });
        let (token_usage, tool_call_count) =
            self.executor.usage(&session.session_id).unwrap_or((0, 0));
        let changed_files = self
            .workspaces
            .get_session_git_status(&session.session_id)
            .map(|status| status.items.into_iter().map(|item| item.path).collect())
            .unwrap_or_default();
        let generation_succeeded = output.succeeded;
        let terminal_now = chrono::Utc::now().to_rfc3339();
        let terminal_update = AttemptTerminalUpdate {
            result_summary: output.result_summary,
            changed_files,
            token_usage,
            tool_call_count,
            error_class: output.error_class,
        };
        if self.repository.run_status(run_id)?.as_str() == "cancel_requested" {
            self.repository.cancel_attempt_generation(
                &subtask_run_id,
                &attempt.id,
                &terminal_update,
                &terminal_now,
            )?;
            self.repository
                .settle_control_boundary(run_id, &terminal_now)?;
            return Ok(Some(ExecutedPlanAttempt {
                attempt_id: attempt.id,
                session_id: session.session_id,
                status: "cancelled".to_string(),
                context_truncated: context.truncated,
            }));
        }
        self.repository.finish_attempt_generation(
            &subtask_run_id,
            &attempt.id,
            &terminal_update,
            generation_succeeded,
            &terminal_now,
        )?;
        if self.repository.run_status(run_id)?.as_str() == "cancel_requested" {
            self.repository.cancel_attempt_generation(
                &subtask_run_id,
                &attempt.id,
                &terminal_update,
                &terminal_now,
            )?;
            self.repository
                .settle_control_boundary(run_id, &chrono::Utc::now().to_rfc3339())?;
            return Ok(Some(ExecutedPlanAttempt {
                attempt_id: attempt.id,
                session_id: session.session_id,
                status: "cancelled".to_string(),
                context_truncated: context.truncated,
            }));
        }
        let status = if generation_succeeded {
            let verification_dispatch =
                self.repository.load_attempt_verification(&subtask_run_id)?;
            let verification = self.verifier.verify(&verification_dispatch)?;
            if self.repository.run_status(run_id)?.as_str() == "cancel_requested" {
                let cancelled_at = chrono::Utc::now().to_rfc3339();
                self.repository.cancel_attempt_verification(
                    &verification_dispatch,
                    &verification.evidence,
                    &verification.summary,
                    &cancelled_at,
                )?;
                self.repository
                    .settle_control_boundary(run_id, &cancelled_at)?;
                return Ok(Some(ExecutedPlanAttempt {
                    attempt_id: attempt.id,
                    session_id: session.session_id,
                    status: "cancelled".to_string(),
                    context_truncated: context.truncated,
                }));
            }
            self.repository.finish_attempt_verification(
                &verification_dispatch,
                &verification.evidence,
                verification.passed,
                &verification.summary,
                &chrono::Utc::now().to_rfc3339(),
            )?;
            if verification.passed {
                "succeeded"
            } else {
                "failed"
            }
        } else {
            "failed"
        };
        let settled = self
            .repository
            .settle_control_boundary(run_id, &chrono::Utc::now().to_rfc3339())?;
        if settled.as_str() == "running" {
            self.repository
                .project_after_attempt(run_id, &chrono::Utc::now().to_rfc3339())?;
        }
        let status = if settled.as_str() == "cancelled" {
            "cancelled"
        } else {
            status
        };
        self.diagnostics.record(PlanDiagnostic {
            level: if status == "failed" {
                PlanDiagnosticLevel::Error
            } else {
                PlanDiagnosticLevel::Info
            },
            event: if status == "failed" {
                "plan.attempt.failed"
            } else {
                "plan.attempt.finished"
            },
            plan_run_id: Some(run_id.to_string()),
            subtask_run_id: Some(subtask_run_id),
            attempt_id: Some(attempt.id.clone()),
            session_id: Some(session.session_id.clone()),
            operation_id: output.operation_id,
            execution_run_id: output.execution_run_id,
            state: Some(status),
            error_class: (status == "failed").then_some("attempt_failed"),
        });
        Ok(Some(ExecutedPlanAttempt {
            attempt_id: attempt.id,
            session_id: session.session_id,
            status: status.to_string(),
            context_truncated: context.truncated,
        }))
    }

    pub(crate) fn activate_driver(&self, run_id: &str) -> Result<bool, PlanApplicationError> {
        if !self.active_drivers.activate(run_id)? {
            return Ok(false);
        }
        let now = chrono::Utc::now().to_rfc3339();
        if let Err(error) = self.repository.set_driver_intent(run_id, "run", &now) {
            self.active_drivers.deactivate(run_id);
            return Err(error);
        }

        let api = self.clone();
        let owned_run_id = run_id.to_string();
        std::thread::Builder::new()
            .name(format!("onepiece-plan-{run_id}"))
            .spawn(move || api.drive_plan_run(owned_run_id))
            .map_err(|error| {
                self.active_drivers.deactivate(run_id);
                PlanApplicationError::Storage(format!("failed to start Plan driver: {error}"))
            })?;
        self.record_lifecycle(
            PlanDiagnosticLevel::Info,
            "plan.driver.activated",
            Some(run_id),
            Some("running"),
        );
        Ok(true)
    }

    fn drive_plan_run(&self, run_id: String) {
        loop {
            if !matches!(
                self.repository.run_status(&run_id),
                Ok(super::domain::PlanRunStatus::Running)
            ) {
                break;
            }
            match self.execute_next_attempt(&run_id, &chrono::Utc::now().to_rfc3339()) {
                Ok(Some(attempt)) if attempt.status == "failed" => {
                    let retried = self
                        .repository
                        .auto_retry_failed_attempt(
                            &run_id,
                            &attempt.attempt_id,
                            &chrono::Utc::now().to_rfc3339(),
                        )
                        .unwrap_or(false);
                    self.record_lifecycle(
                        if retried {
                            PlanDiagnosticLevel::Info
                        } else {
                            PlanDiagnosticLevel::Warn
                        },
                        if retried {
                            "plan.repair.dispatched"
                        } else {
                            "plan.repair.action_required"
                        },
                        Some(&run_id),
                        Some(if retried {
                            "running"
                        } else {
                            "action_required"
                        }),
                    );
                    if !retried
                        && !matches!(
                            self.repository.run_status(&run_id),
                            Ok(super::domain::PlanRunStatus::Running)
                        )
                    {
                        break;
                    }
                }
                Ok(Some(_)) => {
                    if matches!(
                        self.repository.run_status(&run_id),
                        Ok(super::domain::PlanRunStatus::FinalVerifying)
                    ) {
                        let _ = self.execute_final_verification(&run_id);
                        break;
                    }
                }
                Ok(None) => {
                    if matches!(
                        self.repository.run_status(&run_id),
                        Ok(super::domain::PlanRunStatus::FinalVerifying)
                    ) {
                        let _ = self.execute_final_verification(&run_id);
                    }
                    break;
                }
                Err(_) => {
                    let _ = self
                        .repository
                        .project_after_attempt(&run_id, &chrono::Utc::now().to_rfc3339());
                    self.record_lifecycle(
                        PlanDiagnosticLevel::Error,
                        "plan.driver.stopped_on_error",
                        Some(&run_id),
                        Some("stopped"),
                    );
                    break;
                }
            }
        }
        let intent = match self.repository.run_status(&run_id) {
            Ok(super::domain::PlanRunStatus::Paused) => "pause",
            Ok(super::domain::PlanRunStatus::CancelRequested) => "cancel",
            _ => "stopped",
        };
        let _ =
            self.repository
                .set_driver_intent(&run_id, intent, &chrono::Utc::now().to_rfc3339());
        self.active_drivers.deactivate(&run_id);
    }

    fn execute_final_verification(&self, run_id: &str) -> Result<(), PlanApplicationError> {
        loop {
            self.record_lifecycle(
                PlanDiagnosticLevel::Info,
                "plan.finalization.started",
                Some(run_id),
                Some("final_verifying"),
            );
            let dispatch = self
                .repository
                .load_final_verification(run_id, &chrono::Utc::now().to_rfc3339())?;
            let result = match self.verifier.verify(&dispatch) {
                Ok(result) => result,
                Err(_error)
                    if matches!(
                        self.repository.run_status(run_id),
                        Ok(super::domain::PlanRunStatus::CancelRequested)
                    ) =>
                {
                    self.repository
                        .settle_control_boundary(run_id, &chrono::Utc::now().to_rfc3339())?;
                    return Ok(());
                }
                Err(error) => return Err(error),
            };
            if matches!(
                self.repository.run_status(run_id),
                Ok(super::domain::PlanRunStatus::CancelRequested)
            ) {
                self.repository
                    .settle_control_boundary(run_id, &chrono::Utc::now().to_rfc3339())?;
                return Ok(());
            }
            self.repository.finish_final_verification(
                &dispatch,
                &result.evidence,
                result.passed,
                &chrono::Utc::now().to_rfc3339(),
            )?;
            if result.passed {
                self.record_lifecycle(
                    PlanDiagnosticLevel::Info,
                    "plan.finalization.passed",
                    Some(run_id),
                    Some("awaiting_acceptance"),
                );
                return Ok(());
            }
            if !self.execute_final_repair(run_id)? {
                self.record_lifecycle(
                    PlanDiagnosticLevel::Warn,
                    "plan.finalization.action_required",
                    Some(run_id),
                    Some("action_required"),
                );
                return Ok(());
            }
        }
    }

    fn execute_final_repair(&self, run_id: &str) -> Result<bool, PlanApplicationError> {
        let now = chrono::Utc::now().to_rfc3339();
        let Some(dispatch) = self.repository.claim_final_repair(run_id, &now)? else {
            return Ok(false);
        };
        let context = build_attempt_context(&AttemptContextRequest {
            plan_run_id: dispatch.attempt.plan_run_id.clone(),
            subtask_run_id: dispatch.attempt.subtask_run_id.clone(),
            task: dispatch.attempt.task.clone(),
            direct_predecessor_ids: Vec::new(),
            predecessor_sources: Vec::new(),
            repair: dispatch.attempt.repair.clone(),
            character_budget: 32_000,
        })?;
        let session = match self.executor.create_session(&dispatch.attempt) {
            Ok(session) => session,
            Err(error) => {
                self.repository.fail_final_repair_dispatch(
                    &dispatch,
                    "session_creation_failed",
                    &chrono::Utc::now().to_rfc3339(),
                )?;
                return Err(error);
            }
        };
        self.repository
            .start_final_repair(&dispatch, &session.session_id, &session.profile_id)?;
        self.record_lifecycle(
            PlanDiagnosticLevel::Info,
            "plan.final_repair.dispatched",
            Some(run_id),
            Some("running"),
        );
        let output = self.executor.execute(
            &dispatch.attempt,
            &dispatch.repair_id,
            &session,
            context.prompt,
        )?;
        if matches!(
            self.repository.run_status(run_id),
            Ok(super::domain::PlanRunStatus::CancelRequested)
        ) {
            self.repository
                .settle_control_boundary(run_id, &chrono::Utc::now().to_rfc3339())?;
            return Ok(false);
        }
        let (token_usage, tool_call_count) =
            self.executor.usage(&session.session_id).unwrap_or((0, 0));
        let update = AttemptTerminalUpdate {
            result_summary: output.result_summary,
            changed_files: Vec::new(),
            token_usage,
            tool_call_count,
            error_class: output.error_class,
        };
        self.repository.finish_final_repair(
            &dispatch,
            &update,
            output.operation_id.as_deref(),
            output.execution_run_id.as_deref(),
            output.succeeded,
            &chrono::Utc::now().to_rfc3339(),
        )?;
        self.record_lifecycle(
            if output.succeeded {
                PlanDiagnosticLevel::Info
            } else {
                PlanDiagnosticLevel::Warn
            },
            if output.succeeded {
                "plan.final_repair.finished"
            } else {
                "plan.final_repair.failed"
            },
            Some(run_id),
            Some(if output.succeeded {
                "final_verifying"
            } else {
                "action_required"
            }),
        );
        Ok(output.succeeded)
    }

    fn record_lifecycle(
        &self,
        level: PlanDiagnosticLevel,
        event: &'static str,
        run_id: Option<&str>,
        state: Option<&'static str>,
    ) {
        self.diagnostics
            .record(PlanDiagnostic::lifecycle(level, event, run_id, state));
    }
}
