import type { ApprovalScope, PendingApprovalEntry, PolicyTemplateName, PrincipalEntry } from "../types/permissions";
import type { PermissionsService } from "./permissions";
import { resolveWebMockToolApproval } from "./web-agent-client";
import {
  getWebDefaultPolicyTemplate,
  subscribeWebPendingApprovals,
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

  async resolvePendingApproval(requestId: string, approved: boolean, scope: ApprovalScope) {
    const pending = webPendingApprovals.get(requestId);
    if (!pending) return false;
    // Scope-based remembered grants are not simulated in Web/mock mode (a UI-development aid,
    // not a security boundary) — every resolution behaves like `Once` here, regardless of the
    // requested scope.
    void scope;
    return resolveWebMockToolApproval(pending.sessionId, requestId, approved);
  },

  async applyPolicyTemplate(agentId: string, template: PolicyTemplateName): Promise<PrincipalEntry> {
    webPrincipalTemplates.set(agentId, template);
    return {
      agentId,
      template,
      requiresConfirmationToAssign: requiresConfirmationToAssign(template),
    };
  },

  async getAgentPolicyPrincipal(agentId: string): Promise<PrincipalEntry> {
    const template = webPrincipalTemplates.get(agentId) ?? getWebDefaultPolicyTemplate();
    return {
      agentId,
      template,
      requiresConfirmationToAssign: requiresConfirmationToAssign(template),
    };
  },

  async subscribePendingApprovalEvents(handler: (event: PendingApprovalEntry) => void) {
    return subscribeWebPendingApprovals(handler);
  },
};
