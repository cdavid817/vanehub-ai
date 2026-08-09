export type PlanStatus = "draft" | "approved" | "archived";
export type PlanRunStatus = "queued" | "preparing" | "running" | "pause_requested" | "paused" | "cancel_requested" | "awaiting_acceptance" | "recovery_required" | "completed" | "failed" | "cancelled";
export type SubTaskRunStatus = "pending" | "ready" | "dispatching" | "running" | "verifying" | "succeeded" | "failed" | "cancelled" | "interrupted" | "blocked" | "skipped";

export interface PlanResourceLimits {
  tokenBudget: number | null;
  toolCallLimit: number | null;
  timeoutSeconds: number | null;
}

export interface PlanVerificationCommand {
  id: string;
  program: string;
  args: string[];
  workingDirectory: string | null;
  timeoutSeconds: number;
  required: boolean;
}

export interface PlanSubTask {
  id: string;
  title: string;
  description: string;
  acceptanceCriteria: string[];
  ordinal: number;
  assignedRole: string;
  limits: PlanResourceLimits;
  validationCommands: PlanVerificationCommand[];
}

export interface PlanDependency {
  predecessorId: string;
  successorId: string;
}

export interface PlanDraft {
  id: string;
  versionId: string;
  version: number;
  goal: string;
  projectPath: string;
  baseRef: string;
  plannerProfileId: string | null;
  subtasks: PlanSubTask[];
  dependencies: PlanDependency[];
}

export interface GeneratePlanDraftInput {
  planId: string | null;
  version: number;
  goal: string;
  projectPath: string;
  baseRef: string;
  availableTools: string[];
}

export interface PlanValidationIssue {
  code: string;
  message: string;
  subtaskId: string | null;
}

export interface PlanAttemptEvidence {
  id: string;
  commandId: string;
  status: "passed" | "failed" | "timed_out" | "execution_error" | "cancelled";
  exitCode: number | null;
  durationMs: number | null;
  outputSummary: string | null;
  createdAt: string;
}

export interface PlanSubTaskAttempt {
  id: string;
  sequence: number;
  status: SubTaskRunStatus;
  sessionId: string | null;
  profileId: string | null;
  executionRunId: string | null;
  operationId: string | null;
  tokenUsage: number;
  toolCallCount: number;
  errorClass: string | null;
  startedAt: string;
  completedAt: string | null;
}

export interface PlanSubTaskRun {
  id: string;
  subtaskId: string;
  title: string;
  status: SubTaskRunStatus;
  topologicalRank: number;
  ordinal: number;
  resultSummary: string | null;
  changedFiles: string[];
  verificationSummary: string | null;
  attempts: PlanSubTaskAttempt[];
}

export interface PlanRunSummary {
  id: string;
  planId: string;
  status: PlanRunStatus;
  completedTasks: number;
  totalTasks: number;
  simulated: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface PlanRunDetail extends PlanRunSummary {
  projectPath: string;
  baseRef: string;
  baseOid: string | null;
  worktreePath: string | null;
  worktreeName: string | null;
  worktreeBranch: string | null;
  tasks: PlanSubTaskRun[];
  availableControls: PlanControlKind[];
}

export type PlanControlKind = "pause" | "resume" | "cancel" | "retry" | "recover" | "accept";

export interface PlanControlResult {
  requestId: string;
  run: PlanRunDetail;
}

export interface ApprovePlanResult { runId: string; }
export interface PreparedPlanRun {
  runId: string;
  status: "running";
  projectPath: string;
  baseOid: string;
  worktreePath: string;
  worktreeName: string;
  worktreeBranch: string;
}

export interface ExecutedPlanAttempt {
  attemptId: string;
  sessionId: string;
  status: "succeeded" | "failed" | "cancelled";
  contextTruncated: boolean;
}
