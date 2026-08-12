export const policyTemplateNames = ["readonly", "standard", "trusted", "yolo"] as const;
export type PolicyTemplateName = (typeof policyTemplateNames)[number];

export type ApprovalScope = "once" | "session" | "project" | "global";

export type RiskLevel = "L0" | "L1" | "L2" | "L3";

export interface PendingApprovalEntry {
  id: string;
  agentId: string;
  sessionId: string;
  callId: string;
  action: string;
  resource: string;
  riskLevel: RiskLevel;
  createdAt: string;
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
