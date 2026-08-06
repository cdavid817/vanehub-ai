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
