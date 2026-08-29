import type { ApprovalScope, PendingApprovalEntry, PolicyTemplateName, RiskLevel } from "../types/permissions";

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

/**
 * The mock's stand-in for the native resolution ledger.
 *
 * Web/mock does not execute anything, but it has to reproduce the *shape* of resolving: one request
 * gets one immutable resolution, a second resolve reports that one rather than making another, and
 * a remembered grant is not visible until its delivery is acknowledged. Those are the rules the UI
 * is written against, so a mock that answered "true, done" to everything would let a duplicate-click
 * or stale-state bug pass every Web-mode test and appear only on the desktop client.
 */
export interface MockApprovalResolution {
  resolutionId: string;
  effect: "allow" | "deny";
  scope: ApprovalScope;
  state: "committed" | "delivered" | "delivery_failed" | "stale";
  /** Simulated remembered grant, inactive until the delivery is acknowledged. */
  grant: { active: boolean } | null;
}

export const webApprovalResolutions = new Map<string, MockApprovalResolution>();

/** Request ids currently claimed by an in-flight resolve, keyed to their resolution id. */
export const webApprovalClaims = new Map<string, string>();

let webResolutionSequence = 0;

export function nextWebResolutionId(): string {
  webResolutionSequence += 1;
  return `web-resolution-${webResolutionSequence}`;
}

/**
 * Makes the next resolve of `requestId` behave as though its waiter had already ended, or as though
 * delivery failed after the decision was durable.
 *
 * Injected rather than raced for: both outcomes are ones a user can meet and neither can be
 * produced on demand by clicking, so without this the UI states for them would have no test.
 */
export const webApprovalDeliveryFaults = new Map<string, "stale" | "delivery_failed">();

export function resetWebApprovalResolutionsForTest(): void {
  webApprovalResolutions.clear();
  webApprovalClaims.clear();
  webApprovalDeliveryFaults.clear();
  webResolutionSequence = 0;
}

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
