import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ApprovalScope, PendingApprovalEntry, PolicyTemplateName, PrincipalEntry } from "../types/permissions";
import type { PermissionsService } from "./permissions";

export const tauriPermissionsClient: PermissionsService = {
  listPendingApprovals() {
    return invoke<PendingApprovalEntry[]>("list_pending_approvals");
  },

  resolvePendingApproval(requestId: string, approved: boolean, scope: ApprovalScope) {
    return invoke<boolean>("resolve_pending_approval", { input: { requestId, approved, scope } });
  },

  applyPolicyTemplate(agentId: string, template: PolicyTemplateName) {
    return invoke<PrincipalEntry>("apply_policy_template", { input: { agentId, template } });
  },

  getAgentPolicyPrincipal(agentId: string) {
    return invoke<PrincipalEntry>("get_agent_policy_principal", { input: { agentId } });
  },

  async subscribePendingApprovalEvents(handler: (event: PendingApprovalEntry) => void) {
    return listen<PendingApprovalEntry>("permission:request", (event) => handler(event.payload));
  },
};
