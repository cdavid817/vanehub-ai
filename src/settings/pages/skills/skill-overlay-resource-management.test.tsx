// @vitest-environment jsdom

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { agentService } from "../../../services/runtime-agent-client";
import type { SkillOverlayDetail, SkillOverlayPreview } from "../../../types/skill-overlay";
import { SkillOverlayResourceDialog } from "./skill-overlay-resource-dialog";
import { SkillOverlayResourceList } from "./skill-overlay-resource-list";

const target = { skillId: "developer", scope: "user" } as const;
const detail = overlayDetail(false);
const preview: SkillOverlayPreview = {
  witnesses: {
    expectedOverlayRevision: 3, expectedBaseInstructionHash: "base-hash", expectedBasePackageHash: "package-hash",
    expectedPayloadHash: null, expectedPinned: false,
  },
  tentativeRevision: 4,
  scan: { scannerVersion: "overlay-text-v1", passed: true, safeRuleIds: [], ruleIdsTruncated: false },
  diff: emptyDiff(), conflicts: [], conflictsTruncated: false, canCommit: true,
};

afterEach(() => vi.restoreAllMocks());

describe("Skill Overlay resource management", () => {
  it("previews file metadata before adding a supported resource", async () => {
    const user = userEvent.setup({ applyAccept: false });
    const previewSpy = vi.spyOn(agentService, "previewSkillOverlay").mockResolvedValue(preview);
    const addSpy = vi.spyOn(agentService, "addSkillOverlayFile").mockResolvedValue(outcome(detail));
    renderResourceDialog(detail);
    const file = new File(["# Guide"], "guide.md", { type: "text/markdown", lastModified: 10 });
    await user.upload(screen.getByLabelText(/本地文件/), file);

    expect((screen.getByRole("textbox", { name: /逻辑资源路径/ }) as HTMLInputElement).value).toBe("references/guide.md");
    expect(screen.getAllByText("text/markdown").length).toBeGreaterThan(0);
    expect(screen.getByText("7 B")).toBeTruthy();
    expect((screen.getByRole("button", { name: "添加资源" }) as HTMLButtonElement).disabled).toBe(true);

    await user.click(screen.getByRole("button", { name: "预览最终效果" }));
    await waitFor(() => expect((screen.getByRole("button", { name: "添加资源" }) as HTMLButtonElement).disabled).toBe(false));
    expect(previewSpy).toHaveBeenCalledWith(expect.objectContaining({
      target,
      mutation: expect.objectContaining({ kind: "supportingFile", logicalPath: "references/guide.md", mediaType: "text/markdown" }),
    }));
    await user.click(screen.getByRole("button", { name: "添加资源" }));
    await waitFor(() => expect(addSpy).toHaveBeenCalledWith(expect.objectContaining({
      target, witnesses: preview.witnesses, logicalPath: "references/guide.md", mediaType: "text/markdown",
    })));
  });

  it("rejects executable files before preview and clears the selection", async () => {
    const user = userEvent.setup({ applyAccept: false });
    const previewSpy = vi.spyOn(agentService, "previewSkillOverlay");
    renderResourceDialog(detail);
    const input = screen.getByLabelText(/本地文件/) as HTMLInputElement;
    await user.upload(input, new File(["print('unsafe')"], "tool.py", { type: "text/x-python" }));

    expect((await screen.findByRole("alert")).textContent).toContain("不允许添加脚本或可执行文件");
    expect(input.files?.length).toBe(0);
    expect((screen.getByRole("button", { name: "预览最终效果" }) as HTMLButtonElement).disabled).toBe(true);
    expect(previewSpy).not.toHaveBeenCalled();
  });

  it("uses the current payload witness when replacing an effective resource", async () => {
    const user = userEvent.setup();
    const existingDetail = overlayDetail(true);
    const replacementPreview = { ...preview, witnesses: { ...preview.witnesses, expectedPayloadHash: "resource-hash" } };
    const previewSpy = vi.spyOn(agentService, "previewSkillOverlay").mockResolvedValue(replacementPreview);
    const replaceSpy = vi.spyOn(agentService, "replaceSkillOverlayFile").mockResolvedValue(outcome(existingDetail));
    render(<SkillOverlayResourceDialog detail={existingDetail} initialPath="references/team.md" onClose={vi.fn()} onCommitted={vi.fn()} onRefresh={vi.fn()} returnFocus={null} target={target} />);
    await user.upload(screen.getByLabelText(/本地文件/), new File(["updated"], "team.md", { type: "text/markdown" }));
    await user.click(screen.getByRole("button", { name: "预览最终效果" }));

    await waitFor(() => expect(previewSpy).toHaveBeenCalledWith(expect.objectContaining({
      witnesses: expect.objectContaining({ expectedPayloadHash: "resource-hash" }),
    })));
    await waitFor(() => expect((screen.getByRole("button", { name: "替换" }) as HTMLButtonElement).disabled).toBe(false));
    await user.click(screen.getByRole("button", { name: "替换" }));
    await waitFor(() => expect(replaceSpy).toHaveBeenCalledWith(expect.objectContaining({ witnesses: replacementPreview.witnesses })));
  });

  it("confirms disable and revert as witnessed lifecycle mutations", async () => {
    const user = userEvent.setup();
    const existingDetail = overlayDetail(true);
    const disableSpy = vi.spyOn(agentService, "disableSkillOverlayMutation").mockResolvedValue(outcome(existingDetail));
    const revertSpy = vi.spyOn(agentService, "revertSkillOverlayMutation").mockResolvedValue(outcome(existingDetail));
    render(<SkillOverlayResourceList detail={existingDetail} onCommitted={vi.fn()} onRefresh={vi.fn()} target={target} />);

    await user.click(screen.getByRole("button", { name: "禁用" }));
    await user.click(screen.getByRole("dialog").querySelector("button:not([disabled]):last-child") as HTMLButtonElement);
    await waitFor(() => expect(disableSpy).toHaveBeenCalledWith(expect.objectContaining({
      target: { ...target, workspacePath: null }, mutationId: "file-1", mutationKind: "supportingFile",
    })));

    await user.click(screen.getByRole("button", { name: "回退" }));
    const revertDialog = screen.getByRole("dialog", { name: "回退支撑资源" });
    await user.click(Array.from(revertDialog.querySelectorAll("button")).at(-1) as HTMLButtonElement);
    await waitFor(() => expect(revertSpy).toHaveBeenCalledWith(expect.objectContaining({ mutationId: "file-1" })));
  });
});

function renderResourceDialog(value: SkillOverlayDetail) {
  return render(<SkillOverlayResourceDialog detail={value} onClose={vi.fn()} onCommitted={vi.fn()} onRefresh={vi.fn()} returnFocus={null} target={target} />);
}

function overlayDetail(withResource: boolean): SkillOverlayDetail {
  return {
    summary: {
      canonicalSkillId: "developer", baseLayer: "system", status: "healthy", needsReconcile: false,
      pinned: false, baseInstructionHash: "base-hash", basePackageHash: "package-hash", effectiveHash: "effective-hash",
      lastHealthyScope: "user", scopes: [{ scope: "user", revision: 3, trust: "trusted", status: "applied", activeMutationCount: withResource ? 1 : 0, conflictCount: 0, baseHashChanged: false, needsReconcile: false }], scopesTruncated: false,
    },
    baseInstructions: bounded("Base"), effectiveInstructions: bounded("Base"), diff: emptyDiff(), scopeDiffs: [], scopeDiffsTruncated: false,
    mutations: withResource ? [{ id: "file-1", kind: "supportingFile", scope: "user", state: "active", createdAt: "now", updatedAt: "now" }] : [], mutationsTruncated: false,
    resources: withResource ? [{ mutationId: "file-1", logicalPath: "references/team.md", mediaType: "text/markdown", sizeBytes: 8, contentHash: "resource-hash", effectiveScope: "user", state: "active", shadowed: [{ scope: null, baseLayer: "system", contentHash: "base-resource-hash" }], shadowedTruncated: false }] : [], resourcesTruncated: false,
    conflicts: [], conflictsTruncated: false,
  };
}

function outcome(value: SkillOverlayDetail) {
  return { summary: value.summary, committedRevision: 4, diff: emptyDiff() };
}

function bounded(content: string) {
  return { content, totalCharacters: content.length, truncated: false };
}

function emptyDiff() {
  return { baseHash: "base-hash", effectiveHash: "effective-hash", addedCharacters: 0, removedCharacters: 0, hunks: [], hunksTruncated: false };
}
