import type { SkillLayer } from "./skill";

export type SkillOverlayScope = "system" | "user" | "project";
export type SkillOverlayTrust = "trusted" | "untrusted";
export type SkillOverlayStatus =
  | "none"
  | "healthy"
  | "untrusted"
  | "needsReconciliation"
  | "blocked"
  | "integrityFailure";
export type SkillOverlayScopeStatus =
  | "applied"
  | "untrusted"
  | "needsReconciliation"
  | "blockedByEarlierScope"
  | "integrityFailure";
export type SkillOverlayMutationKind = "patch" | "learnedGuidance" | "supportingFile";
export type SkillOverlayMutationState = "active" | "disabled" | "reverted";
export type SkillOverlayConflictState = "active" | "resolved" | "ignored";
export type SkillOverlayActor = "user" | "system";
export type SkillOverlayHistoryAction =
  | "create"
  | "patch"
  | "learn"
  | "file"
  | "import"
  | "promote"
  | "disable"
  | "revert"
  | "reconcile"
  | "conflict";

export interface SkillOverlayTargetInput {
  skillId: string;
  scope: SkillOverlayScope;
  workspacePath?: string | null;
}

export interface SkillOverlayWitnesses {
  expectedOverlayRevision: number | null;
  expectedBaseInstructionHash: string;
  expectedBasePackageHash: string;
  expectedPayloadHash: string | null;
  expectedPinned: boolean;
}

export interface SkillOverlayBoundedText {
  content: string;
  totalCharacters: number;
  truncated: boolean;
}

export interface SkillOverlayScopeSummary {
  scope: SkillOverlayScope;
  revision: number;
  trust: SkillOverlayTrust;
  status: SkillOverlayScopeStatus;
  activeMutationCount: number;
  conflictCount: number;
  baseHashChanged: boolean;
  needsReconcile: boolean;
}

export interface SkillOverlaySummary {
  canonicalSkillId: string;
  baseLayer: SkillLayer;
  status: SkillOverlayStatus;
  needsReconcile: boolean;
  pinned: boolean;
  baseInstructionHash: string;
  basePackageHash: string;
  effectiveHash: string;
  lastHealthyScope: SkillOverlayScope | null;
  scopes: SkillOverlayScopeSummary[];
  scopesTruncated: boolean;
}

export interface SkillOverlayDiffHunk {
  label: string;
  before: SkillOverlayBoundedText;
  after: SkillOverlayBoundedText;
}

export interface SkillOverlayDiff {
  baseHash: string;
  effectiveHash: string;
  addedCharacters: number;
  removedCharacters: number;
  hunks: SkillOverlayDiffHunk[];
  hunksTruncated: boolean;
}

export interface SkillOverlayScopeDiff {
  scope: SkillOverlayScope;
  revision: number;
  inputHash: string;
  outputHash: string;
  diff: SkillOverlayDiff;
}

export interface SkillOverlayMutationSummary {
  id: string;
  kind: SkillOverlayMutationKind;
  scope: SkillOverlayScope;
  state: SkillOverlayMutationState;
  createdAt: string;
  updatedAt: string;
}

export interface SkillOverlayConflictSummary {
  id: string;
  mutationId: string;
  safeReason: string;
  state: SkillOverlayConflictState;
  resolutionRevision: number | null;
}

export interface SkillOverlayResourceShadow {
  scope: SkillOverlayScope | null;
  baseLayer: SkillLayer | null;
  contentHash: string;
}

export interface SkillOverlayResourceSummary {
  mutationId: string;
  logicalPath: string;
  mediaType: string;
  sizeBytes: number;
  contentHash: string;
  effectiveScope: SkillOverlayScope;
  state: SkillOverlayMutationState;
  shadowed: SkillOverlayResourceShadow[];
  shadowedTruncated: boolean;
}

export interface SkillOverlayDetail {
  summary: SkillOverlaySummary;
  baseInstructions: SkillOverlayBoundedText;
  effectiveInstructions: SkillOverlayBoundedText;
  diff: SkillOverlayDiff;
  scopeDiffs: SkillOverlayScopeDiff[];
  scopeDiffsTruncated: boolean;
  mutations: SkillOverlayMutationSummary[];
  mutationsTruncated: boolean;
  resources: SkillOverlayResourceSummary[];
  resourcesTruncated: boolean;
  conflicts: SkillOverlayConflictSummary[];
  conflictsTruncated: boolean;
}

export interface SkillOverlayScanResult {
  scannerVersion: string;
  passed: boolean;
  safeRuleIds: string[];
  ruleIdsTruncated: boolean;
}

export type SkillOverlayMutationInput =
  | { kind: "exactPatch"; oldString: string; newString: string; replaceAll: boolean }
  | { kind: "learnedGuidance"; guidance: string }
  | { kind: "supportingFile"; logicalPath: string; mediaType: string; content: number[] }
  | { kind: "disable"; mutationId: string }
  | { kind: "revert"; mutationId: string };

export interface SkillOverlayPreviewInput {
  target: SkillOverlayTargetInput;
  witnesses: SkillOverlayWitnesses;
  mutation: SkillOverlayMutationInput;
}

export interface SkillOverlayPreview {
  witnesses: SkillOverlayWitnesses;
  tentativeRevision: number;
  scan: SkillOverlayScanResult;
  diff: SkillOverlayDiff;
  conflicts: SkillOverlayConflictSummary[];
  conflictsTruncated: boolean;
  canCommit: boolean;
}

export interface SkillOverlayPatchInput {
  target: SkillOverlayTargetInput;
  witnesses: SkillOverlayWitnesses;
  oldString: string;
  newString: string;
  replaceAll: boolean;
}

export interface SkillOverlayGuidanceInput {
  target: SkillOverlayTargetInput;
  witnesses: SkillOverlayWitnesses;
  guidance: string;
}

export interface SkillOverlayFileInput {
  target: SkillOverlayTargetInput;
  witnesses: SkillOverlayWitnesses;
  logicalPath: string;
  mediaType: string;
  content: number[];
}

export interface SkillOverlayMutationStateInput {
  target: SkillOverlayTargetInput;
  witnesses: SkillOverlayWitnesses;
  mutationId: string;
  mutationKind: SkillOverlayMutationKind;
}

export interface SkillOverlayMutationOutcome {
  summary: SkillOverlaySummary;
  committedRevision: number;
  diff: SkillOverlayDiff;
}

export interface SkillOverlayImportInput {
  target: SkillOverlayTargetInput;
  witnesses: SkillOverlayWitnesses;
  sourceName: string;
  archive: number[];
}

export interface SkillOverlayImportReview {
  sourceSummary: string;
  revision: number;
  documentHash: string;
  scan: SkillOverlayScanResult;
  diff: SkillOverlayDiff;
  mutations: SkillOverlayMutationSummary[];
  mutationsTruncated: boolean;
  resources: SkillOverlayResourceSummary[];
  resourcesTruncated: boolean;
  conflicts: SkillOverlayConflictSummary[];
  conflictsTruncated: boolean;
}

export interface SkillOverlayPromotionInput {
  target: SkillOverlayTargetInput;
  witnesses: SkillOverlayWitnesses;
  reviewedRevision: number;
  reviewedDocumentHash: string;
  reviewedScan: SkillOverlayScanResult;
}

export interface SkillOverlayHistoryInput {
  target: SkillOverlayTargetInput;
  cursor?: string | null;
  limit: number;
}

export interface SkillOverlayHistoryEntry {
  eventId: string;
  canonicalSkillId: string;
  scope: SkillOverlayScope;
  priorRevision: number | null;
  nextRevision: number;
  actor: SkillOverlayActor;
  action: SkillOverlayHistoryAction;
  timestamp: string;
  priorDocumentHash: string | null;
  nextDocumentHash: string;
  scannerVersion: string;
  safeOutcome: string;
  priorEventHash: string | null;
  eventHash: string;
}

export type SkillOverlayHistoryIntegrity = "verified" | `failed:${string}`;

export interface SkillOverlayHistoryPage {
  entries: SkillOverlayHistoryEntry[];
  nextCursor: string | null;
  integrity: SkillOverlayHistoryIntegrity;
}
