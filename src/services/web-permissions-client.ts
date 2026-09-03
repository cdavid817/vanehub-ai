import type {
  ApprovalResolutionOutcome,
  ApprovalScope,
  PendingApprovalEntry,
  PolicyTemplateName,
  PrincipalEntry,
} from "../types/permissions";
import type { PermissionsService } from "./permissions";
import { resolveWebMockToolApproval } from "./web-agent-client";
import {
  getWebDefaultPolicyTemplate,
  nextWebResolutionId,
  subscribeWebPendingApprovals,
  webApprovalClaims,
  webApprovalDeliveryFaults,
  webApprovalResolutions,
  webPendingApprovals,
  webPrincipalTemplates,
} from "./web-permissions-mock-state";

function requiresConfirmationToAssign(template: PolicyTemplateName): boolean {
  return template === "trusted" || template === "yolo";
}

export const webPermissionsClient: PermissionsService = {
  async listPendingApprovals() {
    return Array.from(webPendingApprovals.entries()).map(([id, pending]) => ({
      id,
      agentId: pending.agentId,
      sessionId: pending.sessionId,
      callId: id,
      action: pending.action,
      resource: pending.resource,
      riskLevel: pending.riskLevel,
      createdAt: pending.createdAt,
    }));
  },

  /**
   * Simulates the native `claim -> commit -> deliver -> acknowledge -> activate` sequence.
   *
   * Nothing here executes anything, and no state is a security boundary. What it does reproduce is
   * the *shape* the UI is written against: one request gets one immutable resolution, a second
   * resolve reports that one instead of making another, a claimed request is `resolving`, and a
   * remembered grant stays inactive until delivery is acknowledged. A mock that answered "done" to
   * every call would let a duplicate-click or stale-state bug pass every Web-mode test and surface
   * only on the desktop client.
   */
  async resolvePendingApproval(
    requestId: string,
    approved: boolean,
    scope: ApprovalScope,
  ): Promise<ApprovalResolutionOutcome> {
    const existing = webApprovalResolutions.get(requestId);
    if (existing) {
      return existing.state === "stale" ? "stale" : "already_resolved";
    }
    if (webApprovalClaims.has(requestId)) return "resolving";

    const pending = webPendingApprovals.get(requestId);
    if (!pending) return "not_found";

    const resolutionId = nextWebResolutionId();
    webApprovalClaims.set(requestId, resolutionId);
    try {
      const fault = webApprovalDeliveryFaults.get(requestId);
      const effect = approved ? "allow" : "deny";

      if (fault === "stale") {
        // Discovered before anything is delivered, exactly as the reservation does natively: the
        // decision is recorded as evidence and no grant intent is written.
        webApprovalResolutions.set(requestId, {
          resolutionId,
          effect,
          scope,
          state: "stale",
          grant: null,
        });
        webPendingApprovals.delete(requestId);
        return "stale";
      }

      // Committed. The grant intent exists from here but is not active yet.
      const grant = scope === "once" ? null : { active: false };
      webApprovalResolutions.set(requestId, {
        resolutionId,
        effect,
        scope,
        state: "committed",
        grant,
      });

      if (fault === "delivery_failed") {
        const committed = webApprovalResolutions.get(requestId);
        if (committed) committed.state = "delivery_failed";
        // The decision stays durable and the pending entry stays claimed-and-committed, so no
        // second decision is offered for it.
        webPendingApprovals.delete(requestId);
        return "delivery_failed";
      }

      const delivered = resolveWebMockToolApproval(pending.sessionId, requestId, approved);
      const committed = webApprovalResolutions.get(requestId);
      if (!committed) return "unknown";
      if (!delivered) {
        committed.state = "delivery_failed";
        return "delivery_failed";
      }
      committed.state = "delivered";
      // Acknowledged, so the remembered grant becomes visible — and only now.
      if (committed.grant) committed.grant.active = true;
      return "delivered";
    } finally {
      webApprovalClaims.delete(requestId);
    }
  },

  async applyPolicyTemplate(agentId: string, template: PolicyTemplateName): Promise<PrincipalEntry> {
    webPrincipalTemplates.set(agentId, template);
    return {
      agentId,
      template,
      requiresConfirmationToAssign: requiresConfirmationToAssign(template),
      hasExplicitAssignment: true,
    };
  },

  async getAgentPolicyPrincipal(agentId: string): Promise<PrincipalEntry> {
    const hasExplicitAssignment = webPrincipalTemplates.has(agentId);
    const template = webPrincipalTemplates.get(agentId) ?? getWebDefaultPolicyTemplate();
    return {
      agentId,
      template,
      requiresConfirmationToAssign: requiresConfirmationToAssign(template),
      hasExplicitAssignment,
    };
  },

  async subscribePendingApprovalEvents(handler: (event: PendingApprovalEntry) => void) {
    return subscribeWebPendingApprovals(handler);
  },
};
