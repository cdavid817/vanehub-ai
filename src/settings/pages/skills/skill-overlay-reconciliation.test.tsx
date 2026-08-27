// @vitest-environment jsdom

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { agentService } from "../../../services/runtime-agent-client";
import type { SkillOverlayDetail, SkillOverlayMutationKind } from "../../../types/skill-overlay";
import type { SkillOverlayReconciliationPreview } from "../../../types/skill-overlay-reconciliation";
import { SkillOverlayReconciliationDialog } from "./skill-overlay-reconciliation";

const target = { skillId: "developer", scope: "user" } as const;
const witnesses = {
  expectedOverlayRevision: 3,
  expectedBaseInstructionHash: "current-base-hash",
  expectedBasePackageHash: "current-package-hash",
  expectedPayloadHash: null,
  expectedPinned: false,
};

afterEach(() => vi.restoreAllMocks());

describe("SkillOverlayReconciliationDialog", () => {
  it("requires an edited patch, a complete final preview, and explicit confirmation", async () => {
    const user = userEvent.setup();
    const detail = overlayDetail("patch");
    const initial = reconciliationPreview(false);
    const final = reconciliationPreview(true);
    const previewSpy = vi.spyOn(agentService, "previewSkillOverlayReconciliation")
      .mockResolvedValueOnce(initial)
      .mockResolvedValueOnce(final);
    const reconcileSpy = vi.spyOn(agentService, "reconcileSkillOverlay").mockResolvedValue({ summary: detail.summary, committedRevision: 4, diff: final.finalDiff });
    renderDialog(detail);

    expect(await screen.findByText("见证基础")).toBeTruthy();
    expect(screen.getByText("当前基础")).toBeTruthy();
    expect(screen.getByText("拟议有效结果")).toBeTruthy();
    expect(screen.getByText(/见证基础正文不可用/)).toBeTruthy();
    await user.click(screen.getByRole("radio", { name: "编辑精确补丁" }));
    const previewButton = screen.getByRole("button", { name: "预览最终有效结果" });
    expect((previewButton as HTMLButtonElement).disabled).toBe(true);
    await user.type(screen.getByRole("textbox", { name: /当前基础中的精确文本/ }), "Current base");
    await user.type(screen.getByRole("textbox", { name: "替换文本" }), "Reconciled base");
    await user.click(previewButton);

    await waitFor(() => expect(previewSpy).toHaveBeenLastCalledWith({
      target,
      witnesses,
      choices: [{ conflictId: "conflict-1", resolution: "editPatch", oldString: "Current base", newString: "Reconciled base", replaceAll: false }],
    }));
    expect(await screen.findByText("完整预览已就绪")).toBeTruthy();
    const commit = screen.getByRole("button", { name: "确认协调" });
    expect((commit as HTMLButtonElement).disabled).toBe(true);
    await user.click(screen.getByRole("checkbox", { name: /我已审查完整的最终有效 Diff/ }));
    await user.click(commit);

    await waitFor(() => expect(reconcileSpy).toHaveBeenCalledWith({
      target,
      witnesses: final.witnesses,
      choices: [{ conflictId: "conflict-1", resolution: "editPatch", oldString: "Current base", newString: "Reconciled base", replaceAll: false }],
    }));
  }, 30_000);

  it("explains that ignoring disables a non-patch mutation but preserves audit history", async () => {
    const user = userEvent.setup();
    const detail = overlayDetail("supportingFile");
    const final = reconciliationPreview(true);
    const previewSpy = vi.spyOn(agentService, "previewSkillOverlayReconciliation")
      .mockResolvedValueOnce(reconciliationPreview(false))
      .mockResolvedValueOnce(final);
    renderDialog(detail);
    await screen.findByText("见证基础");

    expect(screen.queryByRole("radio", { name: "编辑精确补丁" })).toBeNull();
    await user.click(screen.getByRole("radio", { name: "忽略并禁用变更" }));
    expect(screen.getByText(/仍保留在追加式历史中/)).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "预览最终有效结果" }));
    await waitFor(() => expect(previewSpy).toHaveBeenLastCalledWith(expect.objectContaining({
      choices: [{ conflictId: "conflict-1", resolution: "ignore" }],
    })));
  });

  it("retains conflict edits when a stale commit requires a reload", async () => {
    const user = userEvent.setup();
    vi.spyOn(agentService, "previewSkillOverlayReconciliation")
      .mockResolvedValueOnce(reconciliationPreview(false))
      .mockResolvedValueOnce(reconciliationPreview(true));
    vi.spyOn(agentService, "reconcileSkillOverlay").mockRejectedValue({
      kind: "stale", code: "stale-witnesses", message: "Base changed during reconciliation.",
      expectedRevision: 3, currentRevision: 4, maximum: null, actual: null,
      baseChanged: true, payloadChanged: false, pinChanged: false,
    });
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    renderDialog(overlayDetail("patch"), onRefresh);
    await screen.findByText("见证基础");
    await user.click(screen.getByRole("radio", { name: "编辑精确补丁" }));
    const oldString = screen.getByRole("textbox", { name: /当前基础中的精确文本/ });
    await user.type(oldString, "Current base");
    await user.click(screen.getByRole("button", { name: "预览最终有效结果" }));
    await user.click(await screen.findByRole("checkbox", { name: /我已审查完整的最终有效 Diff/ }));
    await user.click(screen.getByRole("button", { name: "确认协调" }));

    expect((await screen.findByRole("alert")).textContent).toContain("Base changed");
    expect((oldString as HTMLTextAreaElement).value).toBe("Current base");
    await user.click(screen.getByRole("button", { name: "重新加载当前状态" }));
    expect(onRefresh).toHaveBeenCalledOnce();
    expect((oldString as HTMLTextAreaElement).value).toBe("Current base");
  });
});

function renderDialog(detail: SkillOverlayDetail, onRefresh = vi.fn()) {
  return render(<SkillOverlayReconciliationDialog detail={detail} onClose={vi.fn()} onCommitted={vi.fn()} onRefresh={onRefresh} returnFocus={null} target={target} />);
}

function overlayDetail(kind: SkillOverlayMutationKind): SkillOverlayDetail {
  return {
    summary: {
      canonicalSkillId: "developer", baseLayer: "system", status: "needsReconciliation", needsReconcile: true,
      pinned: false, baseInstructionHash: "current-base-hash", basePackageHash: "current-package-hash", effectiveHash: "fallback-hash",
      lastHealthyScope: null, scopes: [{ scope: "user", revision: 3, trust: "trusted", status: "needsReconciliation", activeMutationCount: 1, conflictCount: 1, baseHashChanged: true, needsReconcile: true }], scopesTruncated: false,
    },
    baseInstructions: bounded("Current base"), effectiveInstructions: bounded("Current base"), diff: emptyDiff(), scopeDiffs: [], scopeDiffsTruncated: false,
    mutations: [{ id: "mutation-1", kind, scope: "user", state: "active", createdAt: "now", updatedAt: "now" }], mutationsTruncated: false,
    resources: [], resourcesTruncated: false,
    conflicts: [{ id: "conflict-1", mutationId: "mutation-1", safeReason: "exact-target-missing", state: "active", resolutionRevision: null }], conflictsTruncated: false,
  };
}

function reconciliationPreview(canCommit: boolean): SkillOverlayReconciliationPreview {
  return {
    witnesses,
    witnessedBase: { baseIdentity: "system:developer", baseLayer: "system", instructionHash: "old-base-hash", packageHash: "old-package-hash", instructions: null },
    currentBase: { baseIdentity: "system:developer", baseLayer: "system", instructionHash: "current-base-hash", packageHash: "current-package-hash", instructions: bounded("Current base") },
    proposedEffective: { effectiveHash: canCommit ? "reconciled-hash" : "fallback-hash", instructions: bounded(canCommit ? "Reconciled base" : "Current base"), resources: [], resourcesTruncated: false },
    conflictChoices: [{ conflict: { id: "conflict-1", mutationId: "mutation-1", safeReason: "exact-target-missing", state: "active", resolutionRevision: null }, selectedResolution: canCommit ? "editPatch" : null }],
    conflictsTruncated: false,
    finalDiff: canCommit ? { baseHash: "current-base-hash", effectiveHash: "reconciled-hash", addedCharacters: 10, removedCharacters: 7, hunks: [{ label: "instructions", before: bounded("Current base"), after: bounded("Reconciled base") }], hunksTruncated: false } : emptyDiff(),
    finalDiffComplete: true,
    canCommit,
  };
}

function bounded(content: string) { return { content, totalCharacters: content.length, truncated: false }; }
function emptyDiff() { return { baseHash: "current-base-hash", effectiveHash: "fallback-hash", addedCharacters: 0, removedCharacters: 0, hunks: [], hunksTruncated: false }; }
