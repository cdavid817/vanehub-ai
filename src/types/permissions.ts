export type PolicyTemplateName = "readonly" | "standard" | "trusted" | "yolo";

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
}
