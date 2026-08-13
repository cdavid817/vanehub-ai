import type {
  ApprovePlanResult,
  GeneratePlanDraftInput,
  PlanControlKind,
  PlanControlResult,
  PlanDraft,
  PlanAttemptEvidence,
  PlanRunDetail,
  PlanRunSummary,
  PreparedPlanRun,
  ExecutedPlanAttempt,
} from "../types/plan";

export interface PlanService {
  generatePlanDraft(input: GeneratePlanDraftInput): Promise<PlanDraft>;
  validatePlanDraft(input: PlanDraft): Promise<void>;
  savePlanDraft(input: PlanDraft): Promise<PlanDraft>;
  getPlanDraft(planId: string): Promise<PlanDraft | null>;
  listPlanVersions(planId: string): Promise<PlanDraft[]>;
  deletePlanDraft(planId: string): Promise<void>;
  approvePlan(planId: string, originatingSessionId?: string): Promise<ApprovePlanResult>;
  startPlanRun(runId: string): Promise<PreparedPlanRun>;
  executeNextAttempt(runId: string): Promise<ExecutedPlanAttempt | null>;
  listPlanRuns(cursor?: string): Promise<{ items: PlanRunSummary[]; nextCursor: string | null }>;
  getPlanRun(runId: string): Promise<PlanRunDetail>;
  getPlanRunForSession(sessionId: string): Promise<PlanRunSummary | null>;
  getPlanAttemptEvidence(attemptId: string): Promise<PlanAttemptEvidence[]>;
  requestPlanControl(runId: string, kind: PlanControlKind): Promise<PlanControlResult>;
  retryPlanSubtask(runId: string, subtaskRunId: string): Promise<PlanControlResult>;
  recoverPlanRun(runId: string): Promise<PlanControlResult>;
  acceptPlanRun(runId: string): Promise<PlanControlResult>;
}
