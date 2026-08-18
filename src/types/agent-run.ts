export type AgentRunState = "created" | "preparing" | "running" | "waiting_approval"
  | "waiting_user" | "paused" | "retrying" | "blocked" | "stuck" | "verifying"
  | "completed" | "failed" | "cancelled";
export interface AgentRunOwner { ownerType: string; ownerId: string }
export interface AgentRunLink { linkType: string; linkId: string }
export interface AgentRunRunner {
  kind: Extract<import("./agent-runner").AgentRunnerKind, "local" | "ssh">; targetId: string; targetRevision: number | null; label: string;
  hostLabel: string | null; recovery: "none" | "inspect_only" | "reattach";
  capabilityWitness: string; authorityWitness: string; recoveryReference: string | null;
}
export interface AgentRun {
  id: string; owner: AgentRunOwner; links: AgentRunLink[]; parentRunId: string | null; state: AgentRunState;
  recoveryPolicy: "not_recoverable" | "owner_reconciles"; retryCount: number;
  maxRetries: number; reasonCode: string | null; createdAt: string; updatedAt: string;
  version: number; lastWitness: string; runner?: AgentRunRunner | null;
}
export interface AgentRunEvent { sequence: number; state: AgentRunState; trigger: string;
  timestamp: string; reasonCode: string | null; witness: string }
export interface AgentRunPage { items: AgentRun[]; offset: number; limit: number }
export interface AgentRunFilter {
  ownerType?: string; ownerId?: string; parentRunId?: string; state?: AgentRunState;
}
