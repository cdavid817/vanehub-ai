export type LoopRunStatus =
  | "queued"
  | "running"
  | "paused"
  | "awaiting-acceptance"
  | "succeeded"
  | "failed"
  | "cancelled";

export type LoopRunPhase = "preparing" | "acting" | "verifying" | "deciding" | "finalizing";

export type LoopTerminalReason =
  | "goal-met"
  | "max-iterations"
  | "time-budget"
  | "phase-timeout"
  | "runtime-errors"
  | "no-progress"
  | "verification-failed"
  | "verifier-blocked"
  | "runtime-error"
  | "recovery-required"
  | "user-rejected"
  | "user-stopped";

export type LoopRole = "worker" | "verifier";
export type LoopVerifierRecommendation = "pass" | "revise" | "blocked";
export type LoopEvidenceKind = "worktree" | "worker" | "verification" | "verifier" | "decision" | "recovery";
export type LoopEvidenceStatus = "pending" | "passed" | "failed" | "blocked" | "cancelled";

export interface LoopVerificationCommand {
  id: string;
  program: string;
  args: string[];
  workingDirectory: string | null;
  timeoutSeconds: number;
  required: boolean;
}

export interface LoopLimits {
  maxIterations: number;
  stepTimeoutSeconds: number;
  totalTimeoutSeconds: number;
  maxConsecutiveRuntimeErrors: number;
  maxConsecutiveNoProgress: number;
}

export interface LoopDefinition {
  id: string;
  name: string;
  enabled: boolean;
  projectPath: string;
  baseBranch: string;
  goal: string;
  acceptanceCriteria: string[];
  allowedPaths: string[];
  protectedPaths: string[];
  workerAgentId: string;
  verifierAgentId: string;
  verificationCommands: LoopVerificationCommand[];
  limits: LoopLimits;
  version: number;
  createdAt: string;
  updatedAt: string;
}

export interface SaveLoopDefinitionInput {
  name: string;
  enabled: boolean;
  projectPath: string;
  baseBranch: string;
  goal: string;
  acceptanceCriteria: string[];
  allowedPaths: string[];
  protectedPaths: string[];
  workerAgentId: string;
  verifierAgentId: string;
  verificationCommands: LoopVerificationCommand[];
  limits: LoopLimits;
  expectedVersion?: number | null;
}

export interface LoopEvidence {
  id: string;
  runId: string;
  iterationId: string | null;
  kind: LoopEvidenceKind;
  status: LoopEvidenceStatus;
  summary: string;
  operationId: string | null;
  commandId: string | null;
  exitCode: number | null;
  durationMs: number | null;
  details: Record<string, unknown> | null;
  createdAt: string;
}

export interface LoopIteration {
  id: string;
  runId: string;
  sequence: number;
  status: LoopRunStatus;
  workerSessionId: string | null;
  verifierSessionId: string | null;
  workerSummary: string | null;
  verifierRecommendation: LoopVerifierRecommendation | null;
  verifierFindings: string[];
  decisionReason: string | null;
  diffFingerprint: string | null;
  checkFailureFingerprint: string | null;
  userFeedback: string | null;
  evidence: LoopEvidence[];
  startedAt: string;
  completedAt: string | null;
}

export interface LoopRun {
  id: string;
  definitionId: string;
  definitionSnapshot: LoopDefinition;
  status: LoopRunStatus;
  phase: LoopRunPhase;
  terminalReason: LoopTerminalReason | null;
  currentIteration: number;
  consecutiveRuntimeErrors: number;
  consecutiveNoProgress: number;
  pauseRequested: boolean;
  projectPath: string;
  worktreePath: string | null;
  worktreeName: string | null;
  worktreeBranch: string | null;
  activeOperationId: string | null;
  iterations: LoopIteration[];
  simulated: boolean;
  createdAt: string;
  startedAt: string | null;
  updatedAt: string;
  completedAt: string | null;
}

export interface StartLoopResult {
  run: LoopRun;
  operationId: string;
}

export interface LoopProjectChoice {
  path: string;
  displayName: string;
  available: boolean;
  simulated: boolean;
}

export type LoopBranchKind = "local" | "remote";

export interface LoopBranchChoice {
  name: string;
  kind: LoopBranchKind;
  available: boolean;
  simulated: boolean;
}

export type LoopReadinessCheckCode =
  | "definition-enabled"
  | "project-available"
  | "branch-available"
  | "worker-eligible"
  | "verifier-eligible"
  | "verification-valid"
  | "path-scope-valid"
  | "no-active-run";

export type LoopReadinessCategory = "definition" | "workspace" | "agent" | "verification" | "runtime";
export type LoopReadinessCheckStatus = "passed" | "blocked";
export type LoopReadinessRemediationTarget =
  | "definition"
  | "project"
  | "branch"
  | "worker"
  | "verifier"
  | "verification"
  | "runs";

export interface LoopReadinessCheck {
  code: LoopReadinessCheckCode;
  category: LoopReadinessCategory;
  status: LoopReadinessCheckStatus;
  blocking: boolean;
  detail: string | null;
  remediationTarget: LoopReadinessRemediationTarget | null;
}

export interface LoopReadinessReport {
  definitionId: string;
  ready: boolean;
  simulated: boolean;
  checks: LoopReadinessCheck[];
  checkedAt: string;
}

export type LoopEventKind = "run-updated" | "iteration-updated" | "evidence-added";

export interface LoopEvent {
  kind: LoopEventKind;
  run: LoopRun;
}

export interface ContinueLoopInput {
  runId: string;
  feedback: string;
}
