import type {
  SkillOverlayBoundedText,
  SkillOverlayConflictSummary,
  SkillOverlayDiff,
  SkillOverlayResourceSummary,
  SkillOverlayTargetInput,
  SkillOverlayWitnesses,
} from "./skill-overlay";
import type { SkillLayer } from "./skill";

export type SkillOverlayConflictResolutionInput =
  | { conflictId: string; resolution: "ignore" }
  | {
      conflictId: string;
      resolution: "editPatch";
      oldString: string;
      newString: string;
      replaceAll: boolean;
    };

export interface SkillOverlayReconciliationInput {
  target: SkillOverlayTargetInput;
  witnesses: SkillOverlayWitnesses;
  choices: SkillOverlayConflictResolutionInput[];
}

export interface SkillOverlayReconciliationBaseSnapshot {
  baseIdentity: string;
  baseLayer: SkillLayer;
  instructionHash: string;
  packageHash: string;
  instructions: SkillOverlayBoundedText | null;
}

export interface SkillOverlayReconciliationProposedResult {
  effectiveHash: string;
  instructions: SkillOverlayBoundedText;
  resources: SkillOverlayResourceSummary[];
  resourcesTruncated: boolean;
}

export interface SkillOverlayReconciliationConflictChoice {
  conflict: SkillOverlayConflictSummary;
  selectedResolution: "editPatch" | "ignore" | null;
}

export interface SkillOverlayReconciliationPreview {
  witnesses: SkillOverlayWitnesses;
  witnessedBase: SkillOverlayReconciliationBaseSnapshot;
  currentBase: SkillOverlayReconciliationBaseSnapshot;
  proposedEffective: SkillOverlayReconciliationProposedResult;
  conflictChoices: SkillOverlayReconciliationConflictChoice[];
  conflictsTruncated: boolean;
  finalDiff: SkillOverlayDiff;
  finalDiffComplete: boolean;
  canCommit: boolean;
}

export type SkillOverlayErrorKind =
  | "validation"
  | "notFound"
  | "conflict"
  | "stale"
  | "pinned"
  | "limit"
  | "trust"
  | "import"
  | "integrity"
  | "infrastructure";

export interface SkillOverlayServiceError {
  kind: SkillOverlayErrorKind;
  code: string;
  message: string;
  expectedRevision: number | null;
  currentRevision: number | null;
  maximum: number | null;
  actual: number | null;
  baseChanged: boolean | null;
  payloadChanged: boolean | null;
  pinChanged: boolean | null;
}
