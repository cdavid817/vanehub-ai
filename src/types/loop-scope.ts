import type { LoopRun } from "./loop";

/** The user's requested enforcement. New definitions default to `preventive-required`. */
export type LoopRequestedMode = "preventive-required" | "artifact-audited";
/** `legacy-unverified` marks a definition saved before scope enforcement; it cannot start. */
export type LoopScopeState = "verified" | "legacy-unverified";
export type LoopCoverage =
  | "complete-enforcement"
  | "mediated-tools-only"
  | "artifact-validation-only"
  | "unsupported"
  | "unknown";
export type LoopVerificationKind = "process" | "native-check";
export type LoopBindingStatus = "bound" | "missing" | "legacy";
export type LoopControlAction = "start" | "resume" | "continue";

export const LOOP_SCOPE_SCHEMA_VERSION = 1;
export const LOOP_NATIVE_CHECK_PATCH_WHITESPACE = "patch-whitespace";

export interface LoopSurfaceAssessment {
  surface: string;
  coverage: LoopCoverage;
  detail: string;
  blocking: boolean;
}

export interface LoopExecutionAssessment {
  requestedMode: LoopRequestedMode;
  surfaces: LoopSurfaceAssessment[];
  blockers: string[];
  limitations: string[];
  satisfiesRequestedMode: boolean;
  acknowledgementRequired: boolean;
  witnessDigest: string;
  assessedAt: string;
  simulated: boolean;
}

export interface LoopRunScope {
  requestedMode: LoopRequestedMode | null;
  scopeDigest: string | null;
  bindingStatus: LoopBindingStatus;
  assessment: LoopExecutionAssessment | null;
  sealedEvidenceId: string | null;
  acceptanceOperationId: string | null;
  baselineDigest: string | null;
  allowedPaths: string[];
  protectedPaths: string[];
}

/** Concurrency and audit preconditions every control request carries. */
export interface LoopControlEnvelope {
  expectedRevision?: number | null;
  idempotencyKey?: string | null;
  auditAcknowledgementId?: string | null;
}

export interface PrepareLoopAdmissionInput {
  action: LoopControlAction;
  definitionId?: string | null;
  runId?: string | null;
  expectedRevision: number;
  clientContext?: string | null;
}

export interface LoopAdmission {
  action: LoopControlAction;
  targetId: string;
  expectedRevision: number;
  requestedMode: LoopRequestedMode;
  scopeDigest: string;
  allowedPaths: string[];
  protectedPaths: string[];
  assessment: LoopExecutionAssessment;
  acknowledgementRequired: boolean;
  challengeId: string | null;
  expiresAt: string | null;
  simulated: boolean;
}

export interface LoopAuditAcknowledgement {
  acknowledgementId: string;
  action: LoopControlAction;
  targetId: string;
  expectedRevision: number;
  expiresAt: string;
}

export interface RequestLoopAcceptanceInput {
  runId: string;
  expectedRevision?: number | null;
  expectedScopeDigest?: string | null;
  expectedEvidenceId?: string | null;
  idempotencyKey?: string | null;
}

export interface LoopAcceptanceResult {
  run: LoopRun;
  operationId: string;
}
