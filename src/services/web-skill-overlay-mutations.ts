import type {
  SkillOverlayFileInput,
  SkillOverlayGuidanceInput,
  SkillOverlayImportInput,
  SkillOverlayImportReview,
  SkillOverlayMutationOutcome,
  SkillOverlayMutationStateInput,
  SkillOverlayPatchInput,
  SkillOverlayPromotionInput,
} from "../types/skill-overlay";
import {
  appendOverlayHistory,
  applyOverlayMutations,
  assertOverlayMutable,
  assertOverlayWitnesses,
  newMutation,
  overlayDiff,
  overlayError,
  overlayKey,
  overlayScan,
  resourceSummaries,
  setMutationState,
  type WebOverlayMutation,
} from "./web-skill-overlay-support";
import {
  currentOverlayState,
  emptyOverlayState,
  overlaySummary,
  refreshOverlayDocumentHash,
  resolveOverlayBase,
  type WebOverlayContext,
} from "./web-skill-overlay-query";

type LocalMutationInput = SkillOverlayPatchInput | SkillOverlayGuidanceInput | SkillOverlayFileInput;

function scanText(content: string): void {
  if (/ignore previous instructions|<script|begin (?:rsa )?private key/i.test(content)) {
    throw overlayError("validation", "scan-hard-deny", "Overlay content failed deterministic scanning.");
  }
}

function validateFile(input: SkillOverlayFileInput): void {
  const normalized = input.logicalPath.replaceAll("\\", "/");
  const allowedRoot = /^(references|templates|assets)\/[a-zA-Z0-9][a-zA-Z0-9._/-]*$/.test(normalized);
  const unsafePath = normalized.includes("..") || normalized.split("/").some((part) => part.startsWith("."));
  if (!allowedRoot || unsafePath || /\.(py|sh|bat|cmd|ps1|exe|com|dll|msi|wasm)$/i.test(normalized)) {
    throw overlayError("validation", "unsafe-resource-path", "Overlay resource path is not allowed.");
  }
  if (input.content.length > 1_048_576) {
    throw overlayError("limit", "supporting-file-bytes", "Overlay supporting file exceeds its limit.", {
      maximum: 1_048_576,
      actual: input.content.length,
    });
  }
  if (input.mediaType.startsWith("text/")) scanText(new TextDecoder().decode(new Uint8Array(input.content)));
}

function mutationFor(input: LocalMutationInput, sequence: number): WebOverlayMutation {
  if ("oldString" in input) {
    scanText(`${input.oldString}\n${input.newString}`);
    return newMutation("patch", input.target.scope, sequence, {
      oldString: input.oldString,
      newString: input.newString,
      replaceAll: input.replaceAll,
    });
  }
  if ("guidance" in input) {
    scanText(input.guidance);
    return newMutation("learnedGuidance", input.target.scope, sequence, { guidance: input.guidance });
  }
  validateFile(input);
  return newMutation("supportingFile", input.target.scope, sequence, {
    logicalPath: input.logicalPath,
    mediaType: input.mediaType,
    content: [...input.content],
  });
}

function actionFor(mutation: WebOverlayMutation): "patch" | "learn" | "file" {
  return mutation.kind === "patch" ? "patch" : mutation.kind === "learnedGuidance" ? "learn" : "file";
}

function commitLocalMutation(
  context: WebOverlayContext,
  input: LocalMutationInput,
  replaceFile: boolean,
): SkillOverlayMutationOutcome {
  const base = resolveOverlayBase(context, input.target);
  const current = currentOverlayState(context, input.target);
  assertOverlayMutable(base);
  assertOverlayWitnesses(current, base, input.witnesses);
  const mutation = mutationFor(input, (current?.mutations.length ?? 0) + 1);
  const before = applyOverlayMutations(base.instructions, current).instructions;
  const next = current ? structuredClone(current) : emptyOverlayState(input.target, base);
  const priorRevision = current?.revision ?? null;
  const priorDocumentHash = current?.documentHash ?? null;
  next.revision = (current?.revision ?? 0) + 1;
  next.trust = "trusted";
  if (replaceFile && mutation.logicalPath) {
    const existing = next.mutations.find((value) =>
      value.kind === "supportingFile" && value.logicalPath === mutation.logicalPath && value.state === "active",
    );
    if (existing) setMutationState(existing, "disabled");
  }
  next.mutations.push(mutation);
  next.conflicts = applyOverlayMutations(base.instructions, next).conflicts;
  refreshOverlayDocumentHash(next);
  appendOverlayHistory(next, actionFor(mutation), priorRevision, priorDocumentHash);
  context.states.set(overlayKey(input.target), next);
  const after = applyOverlayMutations(base.instructions, next).instructions;
  return { summary: overlaySummary(context, input.target), committedRevision: next.revision, diff: overlayDiff(before, after) };
}

export function createOverlayPatch(
  context: WebOverlayContext,
  input: SkillOverlayPatchInput,
): SkillOverlayMutationOutcome {
  return commitLocalMutation(context, input, false);
}

export function createOverlayGuidance(
  context: WebOverlayContext,
  input: SkillOverlayGuidanceInput,
): SkillOverlayMutationOutcome {
  return commitLocalMutation(context, input, false);
}

export function addOverlayFile(
  context: WebOverlayContext,
  input: SkillOverlayFileInput,
): SkillOverlayMutationOutcome {
  return commitLocalMutation(context, input, false);
}

export function replaceOverlayFile(
  context: WebOverlayContext,
  input: SkillOverlayFileInput,
): SkillOverlayMutationOutcome {
  return commitLocalMutation(context, input, true);
}

export function importOverlay(
  context: WebOverlayContext,
  input: SkillOverlayImportInput,
): SkillOverlayImportReview {
  const base = resolveOverlayBase(context, input.target);
  const current = currentOverlayState(context, input.target);
  assertOverlayMutable(base);
  assertOverlayWitnesses(current, base, input.witnesses);
  if (input.archive.length > 8 * 1_048_576) {
    throw overlayError("limit", "import-package-bytes", "Overlay import exceeds its limit.", {
      maximum: 8 * 1_048_576,
      actual: input.archive.length,
    });
  }
  const priorRevision = current?.revision ?? null;
  const priorDocumentHash = current?.documentHash ?? null;
  const next = current ? structuredClone(current) : emptyOverlayState(input.target, base);
  next.revision = (current?.revision ?? 0) + 1;
  next.trust = "untrusted";
  next.sourceSummary = input.sourceName;
  next.baseInstructionHash = base.instructionHash;
  next.basePackageHash = base.packageHash;
  next.mutations = [newMutation("learnedGuidance", input.target.scope, 1, {
    guidance: `Reviewed import: ${input.sourceName}`,
  })];
  next.conflicts = [];
  refreshOverlayDocumentHash(next);
  appendOverlayHistory(next, "import", priorRevision, priorDocumentHash);
  context.states.set(overlayKey(input.target), next);
  const reviewed = structuredClone(next);
  reviewed.trust = "trusted";
  const proposed = applyOverlayMutations(base.instructions, reviewed).instructions;
  return {
    sourceSummary: input.sourceName,
    revision: next.revision,
    documentHash: next.documentHash,
    scan: overlayScan(),
    diff: overlayDiff(base.instructions, proposed),
    mutations: next.mutations,
    mutationsTruncated: false,
    resources: resourceSummaries(next),
    resourcesTruncated: false,
    conflicts: next.conflicts,
    conflictsTruncated: false,
  };
}

export function promoteOverlay(
  context: WebOverlayContext,
  input: SkillOverlayPromotionInput,
): SkillOverlayMutationOutcome {
  const base = resolveOverlayBase(context, input.target);
  const state = currentOverlayState(context, input.target);
  assertOverlayMutable(base);
  assertOverlayWitnesses(state, base, input.witnesses);
  if (!state || state.trust !== "untrusted") {
    throw overlayError("trust", "trust-required", "No untrusted Overlay revision is available for promotion.");
  }
  const reviewChanged = input.reviewedRevision !== state.revision || input.reviewedDocumentHash !== state.documentHash;
  if (reviewChanged || !input.reviewedScan.passed || input.reviewedScan.scannerVersion !== overlayScan().scannerVersion) {
    throw overlayError("stale", "promotion-witness-mismatch", "The imported Overlay review is stale.", {
      expectedRevision: input.reviewedRevision,
      currentRevision: state.revision,
      baseChanged: input.reviewedDocumentHash !== state.documentHash,
      payloadChanged: !input.reviewedScan.passed,
    });
  }
  const before = base.instructions;
  const priorRevision = state.revision;
  const priorDocumentHash = state.documentHash;
  state.revision += 1;
  state.trust = "trusted";
  refreshOverlayDocumentHash(state);
  appendOverlayHistory(state, "promote", priorRevision, priorDocumentHash);
  const after = applyOverlayMutations(base.instructions, state).instructions;
  return { summary: overlaySummary(context, input.target), committedRevision: state.revision, diff: overlayDiff(before, after) };
}

export function changeOverlayMutationState(
  context: WebOverlayContext,
  input: SkillOverlayMutationStateInput,
  nextState: "disabled" | "reverted",
): SkillOverlayMutationOutcome {
  const base = resolveOverlayBase(context, input.target);
  const state = currentOverlayState(context, input.target);
  assertOverlayMutable(base);
  assertOverlayWitnesses(state, base, input.witnesses);
  if (!state) throw overlayError("notFound", "overlay-not-found", "Skill Overlay not found.");
  const mutation = state.mutations.find((value) => value.id === input.mutationId && value.kind === input.mutationKind);
  if (!mutation) throw overlayError("notFound", "mutation-not-found", "Skill Overlay mutation not found.");
  const before = applyOverlayMutations(base.instructions, state).instructions;
  const priorRevision = state.revision;
  const priorDocumentHash = state.documentHash;
  state.revision += 1;
  setMutationState(mutation, nextState);
  state.conflicts = state.conflicts.map((conflict) =>
    conflict.mutationId === mutation.id && conflict.state === "active"
      ? { ...conflict, state: "ignored", resolutionRevision: state.revision }
      : conflict,
  );
  refreshOverlayDocumentHash(state);
  appendOverlayHistory(state, nextState === "disabled" ? "disable" : "revert", priorRevision, priorDocumentHash);
  const after = applyOverlayMutations(base.instructions, state).instructions;
  return { summary: overlaySummary(context, input.target), committedRevision: state.revision, diff: overlayDiff(before, after) };
}
