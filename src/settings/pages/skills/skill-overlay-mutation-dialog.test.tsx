// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { agentService } from "../../../services/runtime-agent-client";
import type { SkillOverlayDetail, SkillOverlayPreview } from "../../../types/skill-overlay";
import { SkillOverlayMutationDialog } from "./skill-overlay-mutation-dialog";

const target = { skillId: "developer", scope: "user" } as const;
const detail: SkillOverlayDetail = {
  summary: {
    canonicalSkillId: "developer", baseLayer: "system", status: "healthy", needsReconcile: false,
    pinned: false, baseInstructionHash: "base-hash", basePackageHash: "package-hash", effectiveHash: "effective-hash",
    lastHealthyScope: "user", scopes: [{ scope: "user", revision: 3, trust: "trusted", status: "applied", activeMutationCount: 1, conflictCount: 0, baseHashChanged: false, needsReconcile: false }], scopesTruncated: false,
  },
  baseInstructions: bounded("Base text and Base text"),
  effectiveInstructions: bounded("Base text and Base text"),
  diff: emptyDiff(), scopeDiffs: [], scopeDiffsTruncated: false,
  mutations: [], mutationsTruncated: false, resources: [], resourcesTruncated: false,
  conflicts: [], conflictsTruncated: false,
};
const preview: SkillOverlayPreview = {
  witnesses: {
    expectedOverlayRevision: 3, expectedBaseInstructionHash: "base-hash", expectedBasePackageHash: "package-hash",
    expectedPayloadHash: null, expectedPinned: false,
  },
  tentativeRevision: 4,
  scan: { scannerVersion: "overlay-text-v1", passed: true, safeRuleIds: [], ruleIdsTruncated: false },
  diff: {
    baseHash: "base-hash", effectiveHash: "next-hash", addedCharacters: 7, removedCharacters: 4,
    hunks: [{ label: "instructions", before: bounded("Base text"), after: bounded("Changed text") }], hunksTruncated: false,
  },
  conflicts: [], conflictsTruncated: false, canCommit: true,
};

afterEach(() => vi.restoreAllMocks());

describe("SkillOverlayMutationDialog", () => {
  it("requires a current exact-patch preview before commit", async () => {
    const user = userEvent.setup();
    const previewSpy = vi.spyOn(agentService, "previewSkillOverlay").mockResolvedValue(preview);
    renderDialog("patch");

    const commit = screen.getByRole("button", { name: "提交 Overlay" });
    expect((commit as HTMLButtonElement).disabled).toBe(true);
    fireEvent.change(screen.getByRole("textbox", { name: /要查找的完整文本/ }), { target: { value: "Base text" } });
    fireEvent.change(screen.getByRole("textbox", { name: "替换文本" }), { target: { value: "Changed text" } });
    await user.click(screen.getByRole("checkbox", { name: "替换所有完全匹配项" }));
    await user.click(screen.getByRole("button", { name: "预览最终效果" }));

    await waitFor(() => expect((commit as HTMLButtonElement).disabled).toBe(false));
    expect(previewSpy).toHaveBeenCalledWith(expect.objectContaining({
      target,
      mutation: { kind: "exactPatch", oldString: "Base text", newString: "Changed text", replaceAll: true },
    }));
    expect(screen.getByText("2")).toBeTruthy();

    fireEvent.change(screen.getByRole("textbox", { name: "替换文本" }), { target: { value: "Changed text updated" } });
    expect((commit as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText(/修改任何字段都会使当前预览失效/)).toBeTruthy();
  });

  it("retains patch input and requires re-preview after a stale commit", async () => {
    const user = userEvent.setup();
    vi.spyOn(agentService, "previewSkillOverlay").mockResolvedValue(preview);
    vi.spyOn(agentService, "createSkillOverlayPatch").mockRejectedValue({
      kind: "stale", code: "overlay-stale", message: "Overlay changed. Reload and preview again.",
      expectedRevision: 3, currentRevision: 4, maximum: null, actual: null,
      baseChanged: false, payloadChanged: false, pinChanged: false,
    });
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    renderDialog("patch", onRefresh);
    const oldText = screen.getByRole("textbox", { name: /要查找的完整文本/ });
    fireEvent.change(oldText, { target: { value: "Base text" } });
    fireEvent.change(screen.getByRole("textbox", { name: "替换文本" }), { target: { value: "Changed text" } });
    await user.click(screen.getByRole("button", { name: "预览最终效果" }));
    await user.click(await screen.findByRole("button", { name: "提交 Overlay" }));

    expect((await screen.findByRole("alert")).textContent).toContain("Overlay changed");
    expect((oldText as HTMLTextAreaElement).value).toBe("Base text");
    expect((screen.getByRole("button", { name: "提交 Overlay" }) as HTMLButtonElement).disabled).toBe(true);
    await user.click(screen.getByRole("button", { name: "重新加载当前状态" }));
    expect(onRefresh).toHaveBeenCalledOnce();
    expect((oldText as HTMLTextAreaElement).value).toBe("Base text");
  });

  it("retains guidance after preview validation fails", async () => {
    const user = userEvent.setup();
    vi.spyOn(agentService, "previewSkillOverlay").mockRejectedValue({
      kind: "validation", code: "scan-hard-deny", message: "Guidance failed deterministic scanning.",
      expectedRevision: null, currentRevision: null, maximum: null, actual: null,
      baseChanged: null, payloadChanged: null, pinChanged: null,
    });
    renderDialog("guidance");
    const guidance = screen.getByRole("textbox", { name: /指导内容/ });
    fireEvent.change(guidance, { target: { value: "Keep this unsaved guidance." } });
    await user.click(screen.getByRole("button", { name: "预览最终效果" }));

    expect((await screen.findByRole("alert")).textContent).toContain("failed deterministic scanning");
    expect((guidance as HTMLTextAreaElement).value).toBe("Keep this unsaved guidance.");
    expect((screen.getByRole("button", { name: "提交 Overlay" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("commits learned guidance only with preview witnesses", async () => {
    const user = userEvent.setup();
    vi.spyOn(agentService, "previewSkillOverlay").mockResolvedValue(preview);
    const commitSpy = vi.spyOn(agentService, "createSkillOverlayGuidance").mockResolvedValue({
      summary: detail.summary, committedRevision: 4, diff: preview.diff,
    });
    const onCommitted = vi.fn();
    render(<SkillOverlayMutationDialog detail={detail} kind="guidance" onClose={vi.fn()} onCommitted={onCommitted} onRefresh={vi.fn()} returnFocus={null} target={target} />);
    fireEvent.change(screen.getByRole("textbox", { name: /指导内容/ }), { target: { value: "Prefer focused modules." } });
    await user.click(screen.getByRole("button", { name: "预览最终效果" }));
    await user.click(await screen.findByRole("button", { name: "提交 Overlay" }));

    await waitFor(() => expect(commitSpy).toHaveBeenCalledWith({
      target, witnesses: preview.witnesses, guidance: "Prefer focused modules.",
    }));
    expect(onCommitted).toHaveBeenCalledOnce();
  });

  it("keeps an already-open mutation dialog read-only after the Skill is pinned", () => {
    const previewSpy = vi.spyOn(agentService, "previewSkillOverlay");
    render(<SkillOverlayMutationDialog
      detail={{ ...detail, summary: { ...detail.summary, pinned: true } }}
      kind="patch"
      onClose={vi.fn()}
      onCommitted={vi.fn()}
      onRefresh={vi.fn()}
      returnFocus={null}
      target={target}
    />);

    expect((screen.getByRole("textbox", { name: /要查找的完整文本/ }) as HTMLTextAreaElement).disabled).toBe(true);
    expect((screen.getByRole("textbox", { name: "替换文本" }) as HTMLTextAreaElement).disabled).toBe(true);
    expect((screen.getByRole("checkbox", { name: "替换所有完全匹配项" }) as HTMLInputElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "预览最终效果" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "提交 Overlay" }) as HTMLButtonElement).disabled).toBe(true);
    expect(previewSpy).not.toHaveBeenCalled();
  });
});

function renderDialog(kind: "patch" | "guidance", onRefresh = vi.fn()) {
  return render(<SkillOverlayMutationDialog detail={detail} kind={kind} onClose={vi.fn()} onCommitted={vi.fn()} onRefresh={onRefresh} returnFocus={null} target={target} />);
}

function bounded(content: string) {
  return { content, totalCharacters: content.length, truncated: false };
}

function emptyDiff() {
  return { baseHash: "base-hash", effectiveHash: "effective-hash", addedCharacters: 0, removedCharacters: 0, hunks: [], hunksTruncated: false };
}
