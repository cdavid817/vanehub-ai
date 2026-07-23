export type CoordinationRunStatus = "queued" | "running" | "succeeded" | "failed" | "cancelled";
export type CoordinationNodeStatus = "blocked" | "queued" | "running" | "succeeded" | "failed" | "skipped" | "cancelled";
export type CoordinationAttemptStatus = "running" | "succeeded" | "failed" | "cancelled";
export type CoordinationFailureKind = "retryable" | "non-retryable" | "cancelled";
export type CoordinationCandidateRole = "primary" | "fallback";

export interface CoordinationNodeInput {
  id: string;
  primaryAgentId: string;
  fallbackAgentIds: string[];
  instruction: string;
  dependsOn: string[];
}

export interface StartCoordinationInput {
  name: string;
  projectPath?: string | null;
  nodes: CoordinationNodeInput[];
}

export interface CoordinationOutput {
  sourceNodeId: string;
  agentId: string;
  attempt: number;
  content: string;
  byteCount: number;
  truncated: boolean;
}

export interface CoordinationAttempt {
  attempt: number;
  agentId: string;
  candidateRole: CoordinationCandidateRole;
  status: CoordinationAttemptStatus;
  failureKind: CoordinationFailureKind | null;
  error: string | null;
  startedAt: string;
  completedAt: string | null;
}

export interface CoordinationNodeRun {
  id: string;
  primaryAgentId: string;
  fallbackAgentIds: string[];
  instruction: string;
  dependsOn: string[];
  status: CoordinationNodeStatus;
  actualAgentId: string | null;
  output: CoordinationOutput | null;
  attempts: CoordinationAttempt[];
  error: string | null;
  startedAt: string | null;
  completedAt: string | null;
}

export interface CoordinationRun {
  id: string;
  operationId: string;
  name: string;
  projectPath: string | null;
  status: CoordinationRunStatus;
  nodes: CoordinationNodeRun[];
  simulated: boolean;
  cancelRequested: boolean;
  createdAt: string;
  startedAt: string | null;
  updatedAt: string;
  completedAt: string | null;
}

export interface StartCoordinationResult {
  runId: string;
  operationId: string;
}
