export type CuratorCandidateState =
  | "pending" | "awaiting_draft" | "ready_for_review" | "deferred"
  | "rejected" | "applying" | "applied" | "apply_failed" | "superseded";
export type CuratorRoute = "advance" | "needs_human_review";
export type CuratorRisk = "low" | "medium" | "high";
export type CuratorConfidence = "low" | "medium" | "high";
export type CuratorDraftKind = "learn_block" | "exact_patch";
export type CuratorStalenessReason =
  | "assessment_changed" | "target_changed" | "evidence_purged" | "draft_changed"
  | "base_changed" | "overlay_changed" | "pin_changed" | "trust_changed"
  | "conflict_changed" | "policy_changed" | "preview_expired";

export interface CuratorError {
  code:
    | "not_found" | "invalid_input" | "unsafe_content" | "not_approvable"
    | "stale_conflict" | "preview_expired" | "pinned" | "application_failed"
    | "storage_unavailable";
  message: string;
  current?: CuratorSafeState;
  field?: string;
  reasonCode?: string;
}

export type CuratorResult<T> =
  | { ok: true; value: T }
  | { ok: false; error: CuratorError };

export interface CuratorQueueQuery {
  workspaceId: string;
  skillId?: string;
  states?: CuratorCandidateState[];
  routes?: CuratorRoute[];
  risks?: CuratorRisk[];
  draftReady?: boolean;
  stale?: boolean;
  notificationPending?: boolean;
  updatedBeforeMs?: number;
  limit?: number;
  cursor?: string;
}

export interface CuratorCandidateSummary {
  candidateId: string;
  targetSkillId: string;
  state: CuratorCandidateState;
  route: CuratorRoute;
  risk: CuratorRisk;
  draftReady: boolean;
  staleness: CuratorStalenessReason[];
  revision: number;
  updatedAtMs: number;
}

export interface CuratorQueuePage {
  items: CuratorCandidateSummary[];
  nextCursor?: string;
  totalCount: number;
  complete: boolean;
}

export interface CuratorQualityCheck {
  code: string;
  result: "pass" | "fail" | "review" | "not_applicable";
  reasonCode: string;
}

export interface CuratorEvidenceSource {
  evidenceId: string;
  evidenceRevision: string;
  lineageHash: string;
}

export type CuratorDraftMutation =
  | { kind: "learned_guidance"; guidance: string }
  | { kind: "exact_patch"; oldString: string; newString: string; replaceAll: boolean };

export interface CuratorDraftRevision {
  draftId: string;
  revision: number;
  kind: CuratorDraftKind;
  mutation: CuratorDraftMutation;
  rationale: string;
  expectedEffectiveChange: string;
  bodyHash: string;
  createdAtMs: number;
}

export interface CuratorCandidateDetail extends CuratorCandidateSummary {
  workspaceId: string;
  seedId: string;
  assessmentAttemptId: string;
  assessmentRevision: string;
  targetRevision: string;
  overlayScope: string;
  confidence: CuratorConfidence;
  evidenceSources: CuratorEvidenceSource[];
  qualityChecks: CuratorQualityCheck[];
  witnessHash: string;
  policyWitnessHash: string;
  drafts: CuratorDraftRevision[];
  currentPreview?: CuratorPreview;
  application?: CuratorApplicationResult;
  createdAtMs: number;
}

export interface CuratorDiffText {
  content: string;
  totalCharacters: number;
  truncated: boolean;
}

export interface CuratorDiffHunk {
  label: string;
  before: CuratorDiffText;
  after: CuratorDiffText;
}

export interface CuratorDiffProjection {
  fromHash: string;
  toHash: string;
  addedCharacters: number;
  removedCharacters: number;
  hunks: CuratorDiffHunk[];
  nextCursor?: string;
  complete: boolean;
}

export interface CuratorPreview {
  previewId: string;
  candidateId: string;
  candidateRevision: number;
  draftRevision: number;
  assessmentId: string;
  witnessHash: string;
  effectiveDiffHash: string;
  diffs: {
    baseToCurrent: CuratorDiffProjection;
    currentToProposed: CuratorDiffProjection;
    baseToProposed: CuratorDiffProjection;
  };
  validation: {
    scanPassed: boolean;
    canCommit: boolean;
    pinned: boolean;
    trusted: boolean;
    conflictCount: number;
    conflictsComplete: boolean;
    safeRuleIds: string[];
    rulesComplete: boolean;
  };
  issuedAtMs: number;
  expiresAtMs: number;
  invalidatedAtMs?: number;
}

export interface CuratorAuditEvent {
  sequence: number;
  eventKind: string;
  actorClass: "local_interactive_user" | "system" | "web_mock_interactive_user";
  occurredAtMs: number;
  priorState?: CuratorCandidateState;
  nextState: CuratorCandidateState;
  objectRevision: number;
  reasonCode?: string;
  eventHash: string;
}

export interface CuratorAuditPage {
  items: CuratorAuditEvent[];
  nextCursor?: string;
  complete: boolean;
}

export interface CuratorPolicy {
  schemaVersion: 1;
  workspaceId: string;
  enqueueRoutes: CuratorRoute[];
  requireRejectionReason: true;
  requireDeferReason: true;
  maximumDeferDays: number;
  openRetentionDays: number;
  terminalRetentionDays: number;
  notificationsEnabled: boolean;
  digestEnabled: boolean;
  draftDisplayLimitBytes: number;
  diffDisplayLimitBytes: number;
  revision: number;
}

export interface CuratorSafeState {
  candidateId: string;
  revision: number;
  state: CuratorCandidateState;
  witnessHash: string;
  policyWitnessHash: string;
  currentPreviewId?: string;
}

export interface CuratorActionReceipt extends CuratorSafeState {
  actionId: string;
  duplicate: boolean;
}

export interface CuratorApplicationResult extends CuratorSafeState {
  applicationId: string;
  status: "intent_recorded" | "applying" | "applied" | "failed" | "reconciled";
  overlayRevision?: string;
  overlayHistoryId?: string;
  failureCode?: string;
}

interface VersionedCuratorAction {
  candidateId: string;
  expectedCandidateRevision: number;
  idempotencyKey: string;
}

export interface SaveCuratorDraftInput extends VersionedCuratorAction {
  schemaVersion: 1;
  targetSkillId?: string;
  targetRevision?: string;
  overlayScope?: string;
  mutation: CuratorDraftMutation;
  rationale: string;
  expectedEffectiveChange: string;
}

export interface PreviewCuratorCandidateInput extends VersionedCuratorAction {
  expectedDraftRevision: number;
  expectedAssessmentId: string;
}

export interface ApproveCuratorCandidateInput extends VersionedCuratorAction {
  confirmedPreviewHash: string;
  confirmedEffectiveDiffHash: string;
}

export interface RejectCuratorCandidateInput extends VersionedCuratorAction {
  reason: "incorrect_target" | "unsupported_lesson" | "duplicate" | "too_risky" | "not_useful" | "other";
  note?: string;
}

export interface DeferCuratorCandidateInput extends VersionedCuratorAction {
  reason: "need_more_evidence" | "need_expert_review" | "waiting_for_change" | "lower_priority" | "other";
  note?: string;
  reviewAfterMs?: number;
}

export interface ResumeCuratorCandidateInput extends VersionedCuratorAction {
  expectedCandidateHash: string;
  expectedPolicyHash: string;
  expectedDraftRevision?: number;
  expectedAssessmentId?: string;
}

export interface UpdateCuratorPolicyInput {
  workspaceId: string;
  expectedRevision: number;
  policy: Omit<CuratorPolicy, "schemaVersion" | "workspaceId" | "revision">;
}

export type CuratorVersionedAction = VersionedCuratorAction;

export type CuratorNotificationEventKind =
  | "pending_review" | "deferral_date" | "supersession"
  | "rejection" | "apply_success" | "apply_failure" | "probation_regression";

export type CuratorNotificationNavigationTarget =
  | { kind: "candidate_review"; candidateId: string }
  | { kind: "overlay_history"; candidateId: string; skillId: string; overlayHistoryId: string };

export interface CuratorNotificationEvent {
  schemaVersion: 1;
  eventKind: CuratorNotificationEventKind;
  candidateId: string;
  candidateRevision: number;
  workspaceId: string;
  skillId: string;
  overlayScope: string;
  state: CuratorCandidateState;
  risk: CuratorRisk;
  route: CuratorRoute;
  navigationTarget: CuratorNotificationNavigationTarget;
}
