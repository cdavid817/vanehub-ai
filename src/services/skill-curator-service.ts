import type * as Curator from "../types/skill-curator";

export interface SkillCuratorService {
  querySkillCuratorQueue(input: Curator.CuratorQueueQuery): Promise<Curator.CuratorResult<Curator.CuratorQueuePage>>;
  getSkillCuratorCandidate(id: string): Promise<Curator.CuratorResult<Curator.CuratorCandidateDetail>>;
  querySkillCuratorAudit(id: string, cursor?: string): Promise<Curator.CuratorResult<Curator.CuratorAuditPage>>;
  getSkillCuratorPolicy(workspace: string): Promise<Curator.CuratorResult<Curator.CuratorPolicy>>;
  updateSkillCuratorPolicy(input: Curator.UpdateCuratorPolicyInput): Promise<Curator.CuratorResult<Curator.CuratorPolicy>>;
  saveSkillCuratorDraft(input: Curator.SaveCuratorDraftInput): Promise<Curator.CuratorResult<Curator.CuratorActionReceipt>>;
  previewSkillCuratorCandidate(input: Curator.PreviewCuratorCandidateInput): Promise<Curator.CuratorResult<Curator.CuratorPreview>>;
  approveSkillCuratorCandidate(input: Curator.ApproveCuratorCandidateInput): Promise<Curator.CuratorResult<Curator.CuratorApplicationResult>>;
  rejectSkillCuratorCandidate(input: Curator.RejectCuratorCandidateInput): Promise<Curator.CuratorResult<Curator.CuratorActionReceipt>>;
  deferSkillCuratorCandidate(input: Curator.DeferCuratorCandidateInput): Promise<Curator.CuratorResult<Curator.CuratorActionReceipt>>;
  resumeSkillCuratorCandidate(input: Curator.ResumeCuratorCandidateInput): Promise<Curator.CuratorResult<Curator.CuratorActionReceipt>>;
  retrySkillCuratorApplication(input: Curator.CuratorVersionedAction): Promise<Curator.CuratorResult<Curator.CuratorActionReceipt>>;
  subscribeSkillCuratorNotifications(handler: (event: Curator.CuratorNotificationEvent) => void): Promise<() => void>;
}
