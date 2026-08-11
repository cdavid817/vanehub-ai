import type {
  SkillOverlayDetail,
  SkillOverlayHistoryInput,
  SkillOverlayHistoryPage,
  SkillOverlayPreview,
  SkillOverlayPreviewInput,
  SkillOverlaySummary,
  SkillOverlayTargetInput,
} from "../types/skill-overlay";
import {
  applyOverlayMutations,
  assertOverlayMutable,
  assertOverlayWitnesses,
  boundedText,
  conflictFor,
  newMutation,
  overlayDiff,
  overlayKey,
  overlayScan,
  resourceSummaries,
  webOverlayHash,
  type WebOverlayBase,
  type WebOverlayMutation,
  type WebOverlayState,
} from "./web-skill-overlay-support";

export interface WebOverlayContext {
  states: Map<string, WebOverlayState>;
  pinned: Map<string, boolean>;
  resolveBase: (target: SkillOverlayTargetInput) => WebOverlayBase;
}

export function resolveOverlayBase(context: WebOverlayContext, target: SkillOverlayTargetInput): WebOverlayBase {
  const base = context.resolveBase(target);
  return { ...base, pinned: context.pinned.get(overlayKey(target)) ?? base.pinned };
}

export function currentOverlayState(
  context: WebOverlayContext,
  target: SkillOverlayTargetInput,
): WebOverlayState | undefined {
  return context.states.get(overlayKey(target));
}

export function effectiveOverlay(
  context: WebOverlayContext,
  target: SkillOverlayTargetInput,
): { base: WebOverlayBase; state: WebOverlayState | undefined; instructions: string } {
  const base = resolveOverlayBase(context, target);
  const state = currentOverlayState(context, target);
  const replay = applyOverlayMutations(base.instructions, state);
  const drifted = state != null
    && (state.baseInstructionHash !== base.instructionHash || state.basePackageHash !== base.packageHash);
  return { base, state, instructions: drifted ? base.instructions : replay.instructions };
}

export function overlaySummary(context: WebOverlayContext, target: SkillOverlayTargetInput): SkillOverlaySummary {
  const { base, state, instructions } = effectiveOverlay(context, target);
  if (!state) {
    return {
      canonicalSkillId: base.skillId,
      baseLayer: base.layer,
      status: "none",
      needsReconcile: false,
      pinned: base.pinned,
      baseInstructionHash: base.instructionHash,
      basePackageHash: base.packageHash,
      effectiveHash: webOverlayHash(instructions),
      lastHealthyScope: null,
      scopes: [],
      scopesTruncated: false,
    };
  }
  const replay = applyOverlayMutations(base.instructions, state);
  const drifted = state.baseInstructionHash !== base.instructionHash || state.basePackageHash !== base.packageHash;
  const conflicted = replay.conflicts.some((conflict) => conflict.state === "active") || drifted;
  const status = state.trust === "untrusted" ? "untrusted" : conflicted ? "needsReconciliation" : "healthy";
  return {
    canonicalSkillId: base.skillId,
    baseLayer: base.layer,
    status,
    needsReconcile: conflicted,
    pinned: base.pinned,
    baseInstructionHash: base.instructionHash,
    basePackageHash: base.packageHash,
    effectiveHash: webOverlayHash(instructions),
    lastHealthyScope: status === "healthy" ? state.target.scope : null,
    scopes: [{
      scope: state.target.scope,
      revision: state.revision,
      trust: state.trust,
      status: state.trust === "untrusted" ? "untrusted" : conflicted ? "needsReconciliation" : "applied",
      activeMutationCount: state.mutations.filter((mutation) => mutation.state === "active").length,
      conflictCount: replay.conflicts.filter((conflict) => conflict.state === "active").length,
      baseHashChanged: drifted,
      needsReconcile: conflicted,
    }],
    scopesTruncated: false,
  };
}

export function overlayDetail(context: WebOverlayContext, target: SkillOverlayTargetInput): SkillOverlayDetail {
  const { base, state, instructions } = effectiveOverlay(context, target);
  const conflicts = state ? applyOverlayMutations(base.instructions, state).conflicts : [];
  const scopeDiff = overlayDiff(base.instructions, instructions);
  return {
    summary: overlaySummary(context, target),
    baseInstructions: boundedText(base.instructions),
    effectiveInstructions: boundedText(instructions),
    diff: scopeDiff,
    scopeDiffs: state ? [{
      scope: state.target.scope,
      revision: state.revision,
      inputHash: scopeDiff.baseHash,
      outputHash: scopeDiff.effectiveHash,
      diff: scopeDiff,
    }] : [],
    scopeDiffsTruncated: false,
    mutations: state?.mutations ?? [],
    mutationsTruncated: false,
    resources: resourceSummaries(state),
    resourcesTruncated: false,
    conflicts,
    conflictsTruncated: false,
  };
}

function previewMutation(input: SkillOverlayPreviewInput, sequence: number): WebOverlayMutation {
  const { mutation } = input;
  switch (mutation.kind) {
    case "exactPatch":
      return newMutation("patch", input.target.scope, sequence, {
        oldString: mutation.oldString,
        newString: mutation.newString,
        replaceAll: mutation.replaceAll,
      });
    case "learnedGuidance":
      return newMutation("learnedGuidance", input.target.scope, sequence, { guidance: mutation.guidance });
    case "supportingFile":
      return newMutation("supportingFile", input.target.scope, sequence, mutation);
    case "disable":
    case "revert":
      return newMutation("patch", input.target.scope, sequence, { id: mutation.mutationId });
  }
}

export function previewOverlay(context: WebOverlayContext, input: SkillOverlayPreviewInput): SkillOverlayPreview {
  const base = resolveOverlayBase(context, input.target);
  const state = currentOverlayState(context, input.target);
  assertOverlayMutable(base);
  assertOverlayWitnesses(state, base, input.witnesses);
  const mutation = previewMutation(input, (state?.mutations.length ?? 0) + 1);
  const tentative: WebOverlayState = state
    ? structuredClone(state)
    : emptyOverlayState(input.target, base);
  tentative.revision = (state?.revision ?? 0) + 1;
  const mutationInput = input.mutation;
  if (mutationInput.kind === "disable" || mutationInput.kind === "revert") {
    const existing = tentative.mutations.find((value) => value.id === mutationInput.mutationId);
    if (existing) existing.state = mutationInput.kind === "disable" ? "disabled" : "reverted";
  } else {
    tentative.mutations.push(mutation);
  }
  const replay = applyOverlayMutations(base.instructions, tentative);
  tentative.conflicts = replay.conflicts;
  return {
    witnesses: input.witnesses,
    tentativeRevision: tentative.revision,
    scan: overlayScan(),
    diff: overlayDiff(base.instructions, replay.instructions),
    conflicts: replay.conflicts,
    conflictsTruncated: false,
    canCommit: replay.conflicts.length === 0,
  };
}

export function overlayHistory(context: WebOverlayContext, input: SkillOverlayHistoryInput): SkillOverlayHistoryPage {
  const entries = currentOverlayState(context, input.target)?.history ?? [];
  const start = Number.parseInt(input.cursor ?? "0", 10);
  const offset = Number.isFinite(start) && start >= 0 ? start : 0;
  const page = entries.slice(offset, offset + input.limit);
  const nextOffset = offset + page.length;
  const state = currentOverlayState(context, input.target);
  return {
    entries: page,
    nextCursor: nextOffset < entries.length ? String(nextOffset) : null,
    integrity: state?.historyIntegrity === false ? "failed:web-history-link" : "verified",
  };
}

export function emptyOverlayState(target: SkillOverlayTargetInput, base: WebOverlayBase): WebOverlayState {
  return {
    target: { ...target, workspacePath: target.scope === "project" ? target.workspacePath ?? null : null },
    revision: 0,
    trust: "trusted",
    baseInstructionHash: base.instructionHash,
    basePackageHash: base.packageHash,
    documentHash: webOverlayHash(`${target.skillId}:0`),
    sourceSummary: null,
    mutations: [],
    conflicts: [],
    history: [],
    historyIntegrity: true,
  };
}

export function refreshOverlayDocumentHash(state: WebOverlayState): void {
  state.documentHash = webOverlayHash(JSON.stringify({
    target: state.target,
    revision: state.revision,
    trust: state.trust,
    baseInstructionHash: state.baseInstructionHash,
    basePackageHash: state.basePackageHash,
    sourceSummary: state.sourceSummary,
    mutations: state.mutations,
    conflicts: state.conflicts,
  }));
}

export function previewConflictForMutation(mutation: WebOverlayMutation, revision: number) {
  return conflictFor(mutation.id, revision);
}
