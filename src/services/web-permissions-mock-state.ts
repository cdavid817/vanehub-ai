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

/**
 * Mirrors the desktop `defaultPolicyTemplate` setting for agents with no explicit assignment
 * (`webPrincipalTemplates` has no entry for them) — shared here rather than read directly from
 * `web-settings-client.ts`'s storage so `web-permissions-client.ts` doesn't need to know that
 * module's storage key, matching this file's existing role as the neutral hub between mocks.
 */
let webDefaultPolicyTemplate: PolicyTemplateName = "standard";

export function getWebDefaultPolicyTemplate(): PolicyTemplateName {
  return webDefaultPolicyTemplate;
}

export function setWebDefaultPolicyTemplate(template: PolicyTemplateName): void {
  webDefaultPolicyTemplate = template;
}
