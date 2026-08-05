import type { PendingApprovalEntry, PolicyTemplateName, RiskLevel } from "../types/permissions";

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

const webPendingApprovalSubscribers = new Set<(event: PendingApprovalEntry) => void>();

/**
 * The mock mirror of the real backend's `permission:request` Tauri event
 * (`event_adapter.rs`'s `TauriPendingApprovalEventAdapter`) — `web-agent-client.ts`'s simulated
 * tool-call flow calls this instead of writing to `webPendingApprovals` directly, so
 * `web-permissions-client.ts`'s `subscribePendingApprovalEvents` mock has something to fire.
 */
export function createWebPendingApproval(callId: string, entry: MockPendingApproval): void {
  webPendingApprovals.set(callId, entry);
  const event: PendingApprovalEntry = {
    id: callId,
    agentId: entry.agentId,
    sessionId: entry.sessionId,
    callId,
    action: entry.action,
    resource: entry.resource,
    riskLevel: entry.riskLevel,
    createdAt: entry.createdAt,
  };
  webPendingApprovalSubscribers.forEach((handler) => handler(event));
}

export function subscribeWebPendingApprovals(handler: (event: PendingApprovalEntry) => void): () => void {
  webPendingApprovalSubscribers.add(handler);
  return () => webPendingApprovalSubscribers.delete(handler);
}

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
