import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  SkillOverlayHistoryInput,
  SkillOverlayImportInput,
  SkillOverlayPatchInput,
  SkillOverlayPreviewInput,
  SkillOverlayPromotionInput,
  SkillOverlayTargetInput,
  SkillOverlayWitnesses,
} from "../types/skill-overlay";
import type { SkillOverlayReconciliationInput } from "../types/skill-overlay-reconciliation";
import { createWebSkillOverlayRuntime } from "./web-skill-overlay-runtime";
import type { WebOverlayBase } from "./web-skill-overlay-support";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

import { tauriAgentClient } from "./tauri-agent-client";
import { webAgentClient } from "./web-agent-client";

type OverlayRuntime = ReturnType<typeof createWebSkillOverlayRuntime>;
let nativeRuntime: OverlayRuntime;

function payloadInput<T>(payload: unknown): T {
  if (typeof payload !== "object" || payload === null || !("input" in payload)) {
    throw new Error("Missing Tauri input payload");
  }
  return (payload as { input: T }).input;
}

async function routeNativeCommand(command: string, payload: unknown): Promise<unknown> {
  switch (command) {
    case "get_skill_overlay_summary":
      return nativeRuntime.getSummary(payloadInput<SkillOverlayTargetInput>(payload));
    case "get_skill_overlay_detail":
      return nativeRuntime.getDetail(payloadInput<SkillOverlayTargetInput>(payload));
    case "preview_skill_overlay":
      return nativeRuntime.preview(payloadInput<SkillOverlayPreviewInput>(payload));
    case "get_skill_overlay_history":
      return nativeRuntime.getHistory(payloadInput<SkillOverlayHistoryInput>(payload));
    case "create_skill_overlay_patch":
      return nativeRuntime.createPatch(payloadInput<SkillOverlayPatchInput>(payload));
    case "create_skill_overlay_guidance":
      return nativeRuntime.createGuidance(payloadInput<Parameters<OverlayRuntime["createGuidance"]>[0]>(payload));
    case "import_skill_overlay":
      return nativeRuntime.importOverlay(payloadInput<SkillOverlayImportInput>(payload));
    case "promote_skill_overlay":
      return nativeRuntime.promote(payloadInput<SkillOverlayPromotionInput>(payload));
    case "revert_skill_overlay_mutation":
      return nativeRuntime.revert(payloadInput<Parameters<OverlayRuntime["revert"]>[0]>(payload));
    case "preview_skill_overlay_reconciliation":
      return nativeRuntime.previewReconciliation(payloadInput<SkillOverlayReconciliationInput>(payload));
    case "reconcile_skill_overlay":
      return nativeRuntime.reconcile(payloadInput<SkillOverlayReconciliationInput>(payload));
    default:
      throw new Error(`Unexpected Overlay command: ${command}`);
  }
}

async function prepareScenario(target: SkillOverlayTargetInput): Promise<SkillOverlayWitnesses> {
  const webDetail = await webAgentClient.getSkillOverlayDetail(target);
  const base: WebOverlayBase = {
    skillId: target.skillId,
    layer: webDetail.summary.baseLayer,
    instructions: webDetail.baseInstructions.content,
    instructionHash: webDetail.summary.baseInstructionHash,
    packageHash: webDetail.summary.basePackageHash,
    pinned: webDetail.summary.pinned,
  };
  nativeRuntime = createWebSkillOverlayRuntime(() => base);
  return {
    expectedOverlayRevision: null,
    expectedBaseInstructionHash: base.instructionHash,
    expectedBasePackageHash: base.packageHash,
    expectedPayloadHash: null,
    expectedPinned: base.pinned,
  };
}

async function rejectedValue(operation: Promise<unknown>): Promise<unknown> {
  try {
    await operation;
    throw new Error("Expected operation to reject");
  } catch (error) {
    return error;
  }
}

describe("Skill Overlay adapter parity", () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(new Date("2026-08-11T12:00:00.000Z"));
    invokeMock.mockReset();
    invokeMock.mockImplementation(routeNativeCommand);
  });

  afterEach(() => vi.useRealTimers());

  it("keeps preview, commit, stale error, revert, detail, and history equivalent", async () => {
    const target = { skillId: "api-doc-generation", scope: "user" as const, workspacePath: null };
    const initialWitnesses = await prepareScenario(target);
    await expect(tauriAgentClient.getSkillOverlaySummary(target))
      .resolves.toEqual(await webAgentClient.getSkillOverlaySummary(target));

    const previewInput: SkillOverlayPreviewInput = {
      target,
      witnesses: initialWitnesses,
      mutation: { kind: "learnedGuidance", guidance: "Document error responses." },
    };
    await expect(tauriAgentClient.previewSkillOverlay(previewInput))
      .resolves.toEqual(await webAgentClient.previewSkillOverlay(previewInput));

    const mutationInput = { target, witnesses: initialWitnesses, guidance: "Document error responses." };
    const webCommitted = await webAgentClient.createSkillOverlayGuidance(mutationInput);
    await expect(tauriAgentClient.createSkillOverlayGuidance(mutationInput)).resolves.toEqual(webCommitted);

    const webStale = await rejectedValue(webAgentClient.createSkillOverlayGuidance(mutationInput));
    const nativeStale = await rejectedValue(tauriAgentClient.createSkillOverlayGuidance(mutationInput));
    expect(nativeStale).toEqual(webStale);
    expect(nativeStale).toMatchObject({ kind: "stale", code: "stale-witnesses", currentRevision: 1 });

    const webDetail = await webAgentClient.getSkillOverlayDetail(target);
    await expect(tauriAgentClient.getSkillOverlayDetail(target)).resolves.toEqual(webDetail);
    const revertInput = {
      target,
      witnesses: { ...initialWitnesses, expectedOverlayRevision: 1 },
      mutationId: webDetail.mutations[0].id,
      mutationKind: "learnedGuidance" as const,
    };
    const webReverted = await webAgentClient.revertSkillOverlayMutation(revertInput);
    await expect(tauriAgentClient.revertSkillOverlayMutation(revertInput)).resolves.toEqual(webReverted);

    const historyInput = { target, limit: 10 };
    await expect(tauriAgentClient.getSkillOverlayHistory(historyInput))
      .resolves.toEqual(await webAgentClient.getSkillOverlayHistory(historyInput));
  });

  it("keeps import quarantine and exact trust promotion equivalent", async () => {
    const target = { skillId: "readme-generation", scope: "user" as const, workspacePath: null };
    const initialWitnesses = await prepareScenario(target);
    const importInput: SkillOverlayImportInput = {
      target,
      witnesses: initialWitnesses,
      sourceName: "team-overlay.zip",
      archive: [80, 75, 3, 4],
    };
    const webReview = await webAgentClient.importSkillOverlay(importInput);
    const nativeReview = await tauriAgentClient.importSkillOverlay(importInput);
    expect(nativeReview).toEqual(webReview);
    await expect(tauriAgentClient.getSkillOverlaySummary(target))
      .resolves.toEqual(await webAgentClient.getSkillOverlaySummary(target));
    expect((await webAgentClient.getSkillOverlaySummary(target)).status).toBe("untrusted");

    const promotionInput: SkillOverlayPromotionInput = {
      target,
      witnesses: {
        ...initialWitnesses,
        expectedOverlayRevision: webReview.revision,
        expectedPayloadHash: webReview.documentHash,
      },
      reviewedRevision: webReview.revision,
      reviewedDocumentHash: webReview.documentHash,
      reviewedScan: webReview.scan,
    };
    const webPromoted = await webAgentClient.promoteSkillOverlay(promotionInput);
    await expect(tauriAgentClient.promoteSkillOverlay(promotionInput)).resolves.toEqual(webPromoted);
    expect(webPromoted).toMatchObject({ committedRevision: 2, summary: { status: "healthy" } });
  });

  it("keeps conflict creation and reconciliation transitions equivalent", async () => {
    const target = { skillId: "code-review", scope: "user" as const, workspacePath: null };
    const initialWitnesses = await prepareScenario(target);
    const patchInput: SkillOverlayPatchInput = {
      target,
      witnesses: initialWitnesses,
      oldString: "missing exact text",
      newString: "replacement",
      replaceAll: false,
    };
    const webConflicted = await webAgentClient.createSkillOverlayPatch(patchInput);
    await expect(tauriAgentClient.createSkillOverlayPatch(patchInput)).resolves.toEqual(webConflicted);
    expect(webConflicted.summary.status).toBe("needsReconciliation");

    const detail = await webAgentClient.getSkillOverlayDetail(target);
    await expect(tauriAgentClient.getSkillOverlayDetail(target)).resolves.toEqual(detail);
    const reconciliationInput: SkillOverlayReconciliationInput = {
      target,
      witnesses: { ...initialWitnesses, expectedOverlayRevision: 1 },
      choices: [{
        conflictId: detail.conflicts[0].id,
        resolution: "editPatch",
        oldString: detail.baseInstructions.content,
        newString: `${detail.baseInstructions.content}\nReconciled guidance.`,
        replaceAll: false,
      }],
    };
    const webPreview = await webAgentClient.previewSkillOverlayReconciliation(reconciliationInput);
    await expect(tauriAgentClient.previewSkillOverlayReconciliation(reconciliationInput)).resolves.toEqual(webPreview);
    expect(webPreview).toMatchObject({ canCommit: true, finalDiffComplete: true });

    const webReconciled = await webAgentClient.reconcileSkillOverlay(reconciliationInput);
    await expect(tauriAgentClient.reconcileSkillOverlay(reconciliationInput)).resolves.toEqual(webReconciled);
    expect(webReconciled).toMatchObject({ committedRevision: 2, summary: { status: "healthy" } });
  });
});
