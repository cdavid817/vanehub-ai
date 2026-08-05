import { invoke } from "@tauri-apps/api/core";
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
};
