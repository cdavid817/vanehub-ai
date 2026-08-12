import type { SkillOverlayMutationOutcome } from "../types/skill-overlay";
import type {
  SkillOverlayReconciliationInput,
  SkillOverlayReconciliationPreview,
} from "../types/skill-overlay-reconciliation";
import {
  appendOverlayHistory,
  applyOverlayMutations,
  assertOverlayMutable,
  assertOverlayWitnesses,
  boundedText,
  overlayDiff,
  overlayError,
  overlayKey,
  resourceSummaries,
  setMutationState,
  type WebOverlayBase,
  type WebOverlayState,
} from "./web-skill-overlay-support";
import {
  currentOverlayState,
  overlaySummary,
  refreshOverlayDocumentHash,
  resolveOverlayBase,
  type WebOverlayContext,
} from "./web-skill-overlay-query";

function applyReconciliationChoices(
  state: WebOverlayState,
  input: SkillOverlayReconciliationInput,
): WebOverlayState {
  const proposed = structuredClone(state);
  for (const choice of input.choices) {
    const conflict = proposed.conflicts.find((value) => value.id === choice.conflictId);
    if (!conflict || conflict.state !== "active") continue;
    const mutation = proposed.mutations.find((value) => value.id === conflict.mutationId);
    if (!mutation) continue;
    if (choice.resolution === "ignore") {
      setMutationState(mutation, "disabled");
      conflict.state = "ignored";
    } else {
      mutation.oldString = choice.oldString;
      mutation.newString = choice.newString;
      mutation.replaceAll = choice.replaceAll;
      mutation.updatedAt = new Date().toISOString();
      conflict.state = "resolved";
    }
    conflict.resolutionRevision = proposed.revision + 1;
  }
  return proposed;
}

function baseSnapshot(
  base: WebOverlayBase,
  instructionHash: string,
  packageHash: string,
  includeInstructions: boolean,
) {
  return {
    baseIdentity: `${base.layer}:${base.skillId}`,
    baseLayer: base.layer,
    instructionHash,
    packageHash,
    instructions: includeInstructions ? boundedText(base.instructions) : null,
  };
}

export function previewOverlayReconciliation(
  context: WebOverlayContext,
  input: SkillOverlayReconciliationInput,
): SkillOverlayReconciliationPreview {
  const base = resolveOverlayBase(context, input.target);
  const state = currentOverlayState(context, input.target);
  assertOverlayMutable(base);
  assertOverlayWitnesses(state, base, input.witnesses);
  if (!state) throw overlayError("notFound", "overlay-not-found", "Skill Overlay not found.");
  const proposed = applyReconciliationChoices(state, input);
  const replay = applyOverlayMutations(base.instructions, proposed);
  const unresolved = proposed.conflicts.filter((conflict) => conflict.state === "active");
  const replayConflicts = replay.conflicts.filter((conflict) => conflict.state === "active");
  const choices = new Map(input.choices.map((choice) => [choice.conflictId, choice.resolution]));
  return {
    witnesses: input.witnesses,
    witnessedBase: baseSnapshot(
      base,
      state.baseInstructionHash,
      state.basePackageHash,
      state.baseInstructionHash === base.instructionHash,
    ),
    currentBase: baseSnapshot(base, base.instructionHash, base.packageHash, true),
    proposedEffective: {
      effectiveHash: overlayDiff(base.instructions, replay.instructions).effectiveHash,
      instructions: boundedText(replay.instructions),
      resources: resourceSummaries(proposed),
      resourcesTruncated: false,
    },
    conflictChoices: state.conflicts.map((conflict) => ({
      conflict,
      selectedResolution: choices.get(conflict.id) ?? null,
    })),
    conflictsTruncated: false,
    finalDiff: overlayDiff(base.instructions, replay.instructions),
    finalDiffComplete: true,
    canCommit: unresolved.length === 0 && replayConflicts.length === 0,
  };
}

export function reconcileOverlay(
  context: WebOverlayContext,
  input: SkillOverlayReconciliationInput,
): SkillOverlayMutationOutcome {
  const preview = previewOverlayReconciliation(context, input);
  if (!preview.canCommit) {
    throw overlayError("conflict", "needs-reconciliation", "Every active conflict requires a valid resolution.");
  }
  const base = resolveOverlayBase(context, input.target);
  const state = currentOverlayState(context, input.target);
  if (!state) throw overlayError("notFound", "overlay-not-found", "Skill Overlay not found.");
  const before = applyOverlayMutations(base.instructions, state).instructions;
  const priorRevision = state.revision;
  const priorDocumentHash = state.documentHash;
  const reconciled = applyReconciliationChoices(state, input);
  reconciled.revision += 1;
  reconciled.baseInstructionHash = base.instructionHash;
  reconciled.basePackageHash = base.packageHash;
  reconciled.conflicts = reconciled.conflicts.map((conflict) =>
    conflict.state === "active"
      ? conflict
      : { ...conflict, resolutionRevision: reconciled.revision },
  );
  refreshOverlayDocumentHash(reconciled);
  appendOverlayHistory(reconciled, "reconcile", priorRevision, priorDocumentHash);
  context.states.set(overlayKey(input.target), reconciled);
  const after = applyOverlayMutations(base.instructions, reconciled).instructions;
  return {
    summary: overlaySummary(context, input.target),
    committedRevision: reconciled.revision,
    diff: overlayDiff(before, after),
  };
}
