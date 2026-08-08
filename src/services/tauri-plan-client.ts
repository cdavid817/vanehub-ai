import { invoke } from "@tauri-apps/api/core";
import type { PlanService } from "./plan-service";
import type { ApprovePlanResult, ExecutedPlanAttempt, PlanAttemptEvidence, PlanControlResult, PlanDraft, PlanRunDetail, PreparedPlanRun } from "../types/plan";

export const tauriPlanClient: PlanService = {
  generatePlanDraft(input) { return invoke<PlanDraft>("generate_plan_draft", { input }); },
  validatePlanDraft(input) { return invoke<void>("validate_plan_draft", { input }); },
  savePlanDraft(input) { return invoke<PlanDraft>("save_plan_draft", { input }); },
  getPlanDraft(planId) { return invoke<PlanDraft | null>("get_plan_draft", { planId }); },
  listPlanVersions(planId) { return invoke<PlanDraft[]>("list_plan_versions", { planId }); },
  deletePlanDraft(planId) { return invoke<void>("delete_plan_draft", { planId }); },
  approvePlan(planId) { return invoke<ApprovePlanResult>("approve_plan", { planId }); },
  startPlanRun(runId) { return invoke<PreparedPlanRun>("start_plan_run", { runId }); },
  executeNextAttempt(runId) { return invoke<ExecutedPlanAttempt | null>("execute_next_plan_attempt", { runId }); },
  listPlanRuns(cursor) { return invoke("list_plan_runs", { cursor: cursor ?? null }); },
  getPlanRun(runId) { return invoke<PlanRunDetail>("get_plan_run_detail", { runId }); },
  getPlanAttemptEvidence(attemptId) { return invoke<PlanAttemptEvidence[]>("get_plan_attempt_evidence", { attemptId }); },
  requestPlanControl(runId, kind) { return invoke<PlanControlResult>("request_plan_control", { runId, kind }); },
  retryPlanSubtask(runId, subtaskRunId) { return invoke<PlanControlResult>("retry_plan_subtask", { runId, subtaskRunId }); },
  recoverPlanRun(runId) { return invoke<PlanControlResult>("recover_plan_run", { runId }); },
  acceptPlanRun(runId) { return invoke<PlanControlResult>("accept_plan_run", { runId }); },
};
