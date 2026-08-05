import type { ApprovalScope, PendingApprovalEntry, PolicyTemplateName, PrincipalEntry } from "../types/permissions";

export interface PermissionsService {
  listPendingApprovals(): Promise<PendingApprovalEntry[]>;
  resolvePendingApproval(requestId: string, approved: boolean, scope: ApprovalScope): Promise<boolean>;
  applyPolicyTemplate(agentId: string, template: PolicyTemplateName): Promise<PrincipalEntry>;
  getAgentPolicyPrincipal(agentId: string): Promise<PrincipalEntry>;
}
