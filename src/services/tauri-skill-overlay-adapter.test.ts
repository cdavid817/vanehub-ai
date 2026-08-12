import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  SkillOverlayErrorKind,
  SkillOverlayReconciliationInput,
  SkillOverlayServiceError,
} from "../types/skill-overlay-reconciliation";
import type {
  SkillOverlayFileInput,
  SkillOverlayGuidanceInput,
  SkillOverlayHistoryInput,
  SkillOverlayImportInput,
  SkillOverlayMutationStateInput,
  SkillOverlayPatchInput,
  SkillOverlayPreviewInput,
  SkillOverlayPromotionInput,
  SkillOverlayTargetInput,
  SkillOverlayWitnesses,
} from "../types/skill-overlay";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

import { withServiceErrorNormalization } from "./service-error";
import { tauriAgentClient } from "./tauri-agent-client";

const target: SkillOverlayTargetInput = {
  skillId: "developer",
  scope: "project",
  workspacePath: "D:/code/project",
};

const witnesses: SkillOverlayWitnesses = {
  expectedOverlayRevision: 4,
  expectedBaseInstructionHash: "instruction-v2",
  expectedBasePackageHash: "package-v2",
  expectedPayloadHash: "payload-v1",
  expectedPinned: false,
};

const patchInput: SkillOverlayPatchInput = {
  target,
  witnesses,
  oldString: "old guidance",
  newString: "new guidance",
  replaceAll: false,
};
const guidanceInput: SkillOverlayGuidanceInput = { target, witnesses, guidance: "Prefer narrow changes." };
const fileInput: SkillOverlayFileInput = {
  target,
  witnesses,
  logicalPath: "references/team.md",
  mediaType: "text/markdown",
  content: [35, 32, 84, 101, 97, 109],
};
const importInput: SkillOverlayImportInput = {
  target,
  witnesses,
  sourceName: "team-overlay.zip",
  archive: [80, 75, 3, 4],
};
const promotionInput: SkillOverlayPromotionInput = {
  target,
  witnesses,
  reviewedRevision: 4,
  reviewedDocumentHash: "document-v4",
  reviewedScan: {
    scannerVersion: "overlay-scan-v1",
    passed: true,
    safeRuleIds: [],
    ruleIdsTruncated: false,
  },
};
const mutationStateInput: SkillOverlayMutationStateInput = {
  target,
  witnesses,
  mutationId: "mutation-4",
  mutationKind: "patch",
};
const historyInput: SkillOverlayHistoryInput = { target, cursor: "segment-2:8", limit: 25 };
const previewInput: SkillOverlayPreviewInput = {
  target,
  witnesses,
  mutation: { kind: "exactPatch", oldString: "old", newString: "new", replaceAll: true },
};
const reconciliationInput: SkillOverlayReconciliationInput = {
  target,
  witnesses,
  choices: [
    {
      conflictId: "conflict-1",
      resolution: "editPatch",
      oldString: "current text",
      newString: "approved text",
      replaceAll: false,
    },
    { conflictId: "conflict-2", resolution: "ignore" },
  ],
};

const operations = [
  { command: "get_skill_overlay_summary", input: target, call: () => tauriAgentClient.getSkillOverlaySummary(target) },
  { command: "get_skill_overlay_detail", input: target, call: () => tauriAgentClient.getSkillOverlayDetail(target) },
  { command: "preview_skill_overlay", input: previewInput, call: () => tauriAgentClient.previewSkillOverlay(previewInput) },
  { command: "get_skill_overlay_history", input: historyInput, call: () => tauriAgentClient.getSkillOverlayHistory(historyInput) },
  { command: "create_skill_overlay_patch", input: patchInput, call: () => tauriAgentClient.createSkillOverlayPatch(patchInput) },
  { command: "create_skill_overlay_guidance", input: guidanceInput, call: () => tauriAgentClient.createSkillOverlayGuidance(guidanceInput) },
  { command: "add_skill_overlay_file", input: fileInput, call: () => tauriAgentClient.addSkillOverlayFile(fileInput) },
  { command: "replace_skill_overlay_file", input: fileInput, call: () => tauriAgentClient.replaceSkillOverlayFile(fileInput) },
  { command: "import_skill_overlay", input: importInput, call: () => tauriAgentClient.importSkillOverlay(importInput) },
  { command: "promote_skill_overlay", input: promotionInput, call: () => tauriAgentClient.promoteSkillOverlay(promotionInput) },
  { command: "disable_skill_overlay_mutation", input: mutationStateInput, call: () => tauriAgentClient.disableSkillOverlayMutation(mutationStateInput) },
  { command: "revert_skill_overlay_mutation", input: mutationStateInput, call: () => tauriAgentClient.revertSkillOverlayMutation(mutationStateInput) },
  {
    command: "preview_skill_overlay_reconciliation",
    input: reconciliationInput,
    call: () => tauriAgentClient.previewSkillOverlayReconciliation(reconciliationInput),
  },
  { command: "reconcile_skill_overlay", input: reconciliationInput, call: () => tauriAgentClient.reconcileSkillOverlay(reconciliationInput) },
] as const;

function overlayError(
  kind: SkillOverlayErrorKind,
  values: Partial<SkillOverlayServiceError> = {},
): SkillOverlayServiceError {
  return {
    kind,
    code: `${kind}-failure`,
    message: `Overlay ${kind} failure`,
    expectedRevision: null,
    currentRevision: null,
    maximum: null,
    actual: null,
    baseChanged: null,
    payloadChanged: null,
    pinChanged: null,
    ...values,
  };
}

describe("Tauri Skill Overlay adapter", () => {
  beforeEach(() => invokeMock.mockReset());

  it("maps every operation to its native command with one input payload", async () => {
    invokeMock.mockResolvedValue({});

    for (const operation of operations) await operation.call();

    expect(invokeMock.mock.calls).toEqual(
      operations.map(({ command, input }) => [command, { input }]),
    );
  });

  it.each(operations)("preserves a structured stale error from $command", async ({ call }) => {
    const error = overlayError("stale", {
      code: "stale-witnesses",
      expectedRevision: 4,
      currentRevision: 5,
      baseChanged: true,
      payloadChanged: false,
      pinChanged: true,
    });
    invokeMock.mockRejectedValueOnce(error);

    await expect(call()).rejects.toBe(error);
  });

  it.each<SkillOverlayErrorKind>([
    "validation",
    "notFound",
    "conflict",
    "stale",
    "pinned",
    "limit",
    "trust",
    "import",
    "integrity",
    "infrastructure",
  ])("preserves the %s structured error kind through runtime normalization", async (kind) => {
    const error = overlayError(kind);
    const normalizedClient = withServiceErrorNormalization(tauriAgentClient);
    invokeMock.mockRejectedValueOnce(error);

    await expect(normalizedClient.getSkillOverlaySummary(target)).rejects.toBe(error);
  });

  it("maps an unstructured native rejection to a safe infrastructure error", async () => {
    invokeMock.mockRejectedValueOnce(new Error("native bridge unavailable"));

    await expect(tauriAgentClient.getSkillOverlaySummary(target)).rejects.toEqual({
      kind: "infrastructure",
      code: "native-overlay-failure",
      message: "native bridge unavailable",
      expectedRevision: null,
      currentRevision: null,
      maximum: null,
      actual: null,
      baseChanged: null,
      payloadChanged: null,
      pinChanged: null,
    });
  });
});
