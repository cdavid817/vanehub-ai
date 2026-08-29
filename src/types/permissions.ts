export const policyTemplateNames = ["readonly", "standard", "trusted", "yolo"] as const;
export type PolicyTemplateName = (typeof policyTemplateNames)[number];

export type ApprovalScope = "once" | "session" | "project" | "global";

export type RiskLevel = "L0" | "L1" | "L2" | "L3";

export interface SkillApprovalProvenance {
  parentAgentId: string;
  skillId: string;
  toolId: string;
  effectiveRevision: string;
  sourceScope: string;
  requestedCapability: string;
  delegatedOperation: string;
  redactedInputSummary: string;
  immutableWitness: string;
}

export interface PendingApprovalEntry {
  id: string;
  agentId: string;
  sessionId: string;
  callId: string;
  action: string;
  resource: string;
  riskLevel: RiskLevel;
  skill?: SkillApprovalProvenance;
  createdAt: string;
}

/**
 * What resolving one approval did.
 *
 * Six outcomes rather than a boolean, because the user is shown something different for each and
 * only one of them means the tool actually ran. `delivery_failed` in particular is not a failure of
 * the decision — the decision is durable — it is a failure to tell anyone about it, and telling a
 * user "that didn't work, try again" would invite a second decision for a request that has one.
 */
export const approvalResolutionOutcomes = [
  "delivered",
  "stale",
  "delivery_failed",
  "resolving",
  "already_resolved",
  "not_found",
] as const;

export type KnownApprovalResolutionOutcome = (typeof approvalResolutionOutcomes)[number];

/**
 * `unknown` covers a native build newer than this frontend. It is a member rather than a fallback
 * to `delivered` or `delivery_failed` on purpose: both of those assert something about whether the
 * tool ran, and an outcome this build cannot name is exactly the case where that is not known.
 */
export type ApprovalResolutionOutcome = KnownApprovalResolutionOutcome | "unknown";

export function normalizeApprovalResolutionOutcome(value: string): ApprovalResolutionOutcome {
  return (approvalResolutionOutcomes as readonly string[]).includes(value)
    ? (value as KnownApprovalResolutionOutcome)
    : "unknown";
}

/** Whether this outcome means the request still has controls a user could act on. */
export function approvalIsUnresolved(outcome: ApprovalResolutionOutcome): boolean {
  return outcome === "resolving" || outcome === "unknown";
}

export interface PrincipalEntry {
  agentId: string;
  template: PolicyTemplateName;
  requiresConfirmationToAssign: boolean;
  /** Whether this principal has ever been explicitly assigned a template, vs. `template` being
   * a synthesized effective default. Used for the `claude-code` principal specifically, to
   * decide whether to show the first-use hook-installation confirmation. */
  hasExplicitAssignment: boolean;
}

/** The stable CLI principal id `permissions-approval`'s Agent Policies list also shows
 * alongside custom API agents and OnePiece — not a registered `AgentRegistryEntry`. */
export const CLAUDE_CODE_AGENT_ID = "claude-code";

/** The four managed CLI principal ids the Agent Policies page lists independently of
 * `agentService.listAgents()` — mirrors `MANAGED_CLI_AGENT_IDS` in `cli_parameters.rs`. Only
 * `claude-code` gets the extra hook-install confirmation; the other three project their
 * template straight into launch flags (`add-cli-agent-permission-launch-flags`). */
export const MANAGED_CLI_AGENT_IDS = ["claude-code", "codex-cli", "opencode", "antigravity-cli", "gemini-cli"] as const;
