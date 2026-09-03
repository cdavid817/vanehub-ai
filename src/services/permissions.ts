import type {
  ApprovalResolutionOutcome,
  ApprovalScope,
  PendingApprovalEntry,
  PolicyTemplateName,
  PrincipalEntry,
} from "../types/permissions";

export interface PermissionsService {
  listPendingApprovals(): Promise<PendingApprovalEntry[]>;
  /**
   * Returns what resolving did, not whether it "worked".
   *
   * This used to be a `boolean` that meant "a live waiter received the decision", which collapsed
   * four materially different outcomes into one: delivered, committed-but-undelivered, resolved by
   * somebody else, and no such request. A caller could not tell the one case where the tool ran
   * from the three where it did not.
   */
  resolvePendingApproval(
    requestId: string,
    approved: boolean,
    scope: ApprovalScope,
  ): Promise<ApprovalResolutionOutcome>;
  applyPolicyTemplate(agentId: string, template: PolicyTemplateName): Promise<PrincipalEntry>;
  getAgentPolicyPrincipal(agentId: string): Promise<PrincipalEntry>;
  /** Global, not scoped to any one session — fires for a new pending approval anywhere. */
  subscribePendingApprovalEvents(handler: (event: PendingApprovalEntry) => void): Promise<() => void>;
}
