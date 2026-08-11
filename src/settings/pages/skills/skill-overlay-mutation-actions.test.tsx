// @vitest-environment jsdom

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import type { SkillOverlayDetail } from "../../../types/skill-overlay";
import { SkillOverlayMutationActions } from "./skill-overlay-mutation-actions";

describe("SkillOverlayMutationActions", () => {
  it("uses a stacked touch layout that wraps into compact actions on wider screens", () => {
    renderActions();
    const patch = screen.getByRole("button", { name: "添加精确补丁" });
    expect(patch.parentElement?.className).toContain("flex-col");
    expect(patch.parentElement?.className).toContain("sm:flex-row");
    for (const button of screen.getAllByRole("button")) {
      expect(button.className).toContain("min-h-11");
      expect(button.className).toContain("sm:min-h-9");
    }
  });

  it("traps keyboard focus, closes on Escape, and restores focus to its trigger", async () => {
    const user = userEvent.setup();
    renderActions();
    const trigger = screen.getByRole("button", { name: "添加精确补丁" });
    await user.click(trigger);

    const dialog = screen.getByRole("dialog", { name: "添加 Overlay 精确补丁" });
    const oldString = screen.getByRole("textbox", { name: /要查找的完整文本/ });
    const cancel = screen.getByRole("button", { name: "取消" });
    expect(document.activeElement).toBe(oldString);
    await user.tab({ shift: true });
    expect(document.activeElement).toBe(cancel);
    await user.tab();
    expect(document.activeElement).toBe(oldString);

    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "添加 Overlay 精确补丁" })).toBeNull());
    expect(document.activeElement).toBe(trigger);
    expect(dialog.isConnected).toBe(false);
  });
});

function renderActions() {
  return render(<SkillOverlayMutationActions
    detail={detail}
    onCommitted={vi.fn()}
    onRefresh={vi.fn()}
    target={{ skillId: "developer", scope: "user" }}
  />);
}

const detail: SkillOverlayDetail = {
  summary: {
    canonicalSkillId: "developer", baseLayer: "system", status: "none", needsReconcile: false,
    pinned: false, baseInstructionHash: "base-hash", basePackageHash: "package-hash", effectiveHash: "base-hash",
    lastHealthyScope: null, scopes: [], scopesTruncated: false,
  },
  baseInstructions: { content: "Base instructions", totalCharacters: 17, truncated: false },
  effectiveInstructions: { content: "Base instructions", totalCharacters: 17, truncated: false },
  diff: { baseHash: "base-hash", effectiveHash: "base-hash", addedCharacters: 0, removedCharacters: 0, hunks: [], hunksTruncated: false },
  scopeDiffs: [], scopeDiffsTruncated: false, mutations: [], mutationsTruncated: false,
  resources: [], resourcesTruncated: false, conflicts: [], conflictsTruncated: false,
};
