import type { SkillOverlayTargetInput } from "../types/skill-overlay";
import {
  addOverlayFile,
  changeOverlayMutationState,
  createOverlayGuidance,
  createOverlayPatch,
  importOverlay,
  promoteOverlay,
  replaceOverlayFile,
} from "./web-skill-overlay-mutations";
import {
  currentOverlayState,
  overlayDetail,
  overlayHistory,
  overlaySummary,
  previewOverlay,
  type WebOverlayContext,
} from "./web-skill-overlay-query";
import { previewOverlayReconciliation, reconcileOverlay } from "./web-skill-overlay-reconciliation";
import { overlayKey, type WebOverlayBase } from "./web-skill-overlay-support";

export function createWebSkillOverlayRuntime(
  resolveBase: (target: SkillOverlayTargetInput) => WebOverlayBase,
) {
  const context: WebOverlayContext = { states: new Map(), pinned: new Map(), resolveBase };
  return {
    getSummary: (input: SkillOverlayTargetInput) => overlaySummary(context, input),
    getDetail: (input: SkillOverlayTargetInput) => overlayDetail(context, input),
    preview: (input: Parameters<typeof previewOverlay>[1]) => previewOverlay(context, input),
    getHistory: (input: Parameters<typeof overlayHistory>[1]) => overlayHistory(context, input),
    createPatch: (input: Parameters<typeof createOverlayPatch>[1]) => createOverlayPatch(context, input),
    createGuidance: (input: Parameters<typeof createOverlayGuidance>[1]) => createOverlayGuidance(context, input),
    addFile: (input: Parameters<typeof addOverlayFile>[1]) => addOverlayFile(context, input),
    replaceFile: (input: Parameters<typeof replaceOverlayFile>[1]) => replaceOverlayFile(context, input),
    importOverlay: (input: Parameters<typeof importOverlay>[1]) => importOverlay(context, input),
    promote: (input: Parameters<typeof promoteOverlay>[1]) => promoteOverlay(context, input),
    disable: (input: Parameters<typeof changeOverlayMutationState>[1]) =>
      changeOverlayMutationState(context, input, "disabled"),
    revert: (input: Parameters<typeof changeOverlayMutationState>[1]) =>
      changeOverlayMutationState(context, input, "reverted"),
    previewReconciliation: (input: Parameters<typeof previewOverlayReconciliation>[1]) =>
      previewOverlayReconciliation(context, input),
    reconcile: (input: Parameters<typeof reconcileOverlay>[1]) => reconcileOverlay(context, input),
    setPinned(target: SkillOverlayTargetInput, pinned: boolean) {
      context.pinned.set(overlayKey(target), pinned);
    },
    corruptHistory(target: SkillOverlayTargetInput) {
      const state = currentOverlayState(context, target);
      if (state) state.historyIntegrity = false;
    },
    reset() {
      context.states.clear();
      context.pinned.clear();
    },
  };
}
