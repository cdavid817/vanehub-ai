import type { PolicyTemplateName, RiskLevel } from "../types/permissions";

export interface MockPendingApproval {
  sessionId: string;
  messageId: string;
  toolName: string;
  input?: unknown;
  output?: unknown;
  agentId: string;
  action: string;
  resource: string;
  riskLevel: RiskLevel;
  createdAt: string;
}

/**
 * Shared, neutral mock state: imported one-way by both `web-agent-client.ts` (whose simulated
 * tool-call flow raises/checks these) and `web-permissions-client.ts` (which exposes them through
 * `PermissionsService`). Depending on neither avoids a circular import between the two.
 */
export const webPendingApprovals = new Map<string, MockPendingApproval>();
export const webPrincipalTemplates = new Map<string, PolicyTemplateName>();

export function isAgentAutoApproved(agentId: string): boolean {
  const template = webPrincipalTemplates.get(agentId) ?? "standard";
  return template === "trusted" || template === "yolo";
}
