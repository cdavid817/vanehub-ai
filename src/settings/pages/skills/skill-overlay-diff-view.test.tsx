// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import "../../../i18n";
import type { SkillOverlayDetail } from "../../../types/skill-overlay";
import { SkillOverlayDiffView } from "./skill-overlay-diff-view";

const bounded = (content: string, truncated = false) => ({ content, totalCharacters: content.length, truncated });
const detail: SkillOverlayDetail = {
  summary: {
    canonicalSkillId: "developer", baseLayer: "system", status: "healthy", needsReconcile: false,
    pinned: false, baseInstructionHash: "base", basePackageHash: "package", effectiveHash: "project",
    lastHealthyScope: "project", scopes: [], scopesTruncated: false,
  },
  baseInstructions: bounded("Base instructions"),
  effectiveInstructions: bounded("Project instructions"),
  diff: {
    baseHash: "base", effectiveHash: "project", addedCharacters: 7, removedCharacters: 4,
    hunks: [{ label: "effective-instructions", before: bounded("Base instructions"), after: bounded("Project instructions") }],
    hunksTruncated: false,
  },
  scopeDiffs: [{
    scope: "user", revision: 2, inputHash: "base", outputHash: "user",
    diff: {
      baseHash: "base", effectiveHash: "user", addedCharacters: 4, removedCharacters: 4,
      hunks: [{ label: "overlay-scope:user", before: bounded("Base instructions"), after: bounded("User instructions", true) }],
      hunksTruncated: false,
    },
  }, {
    scope: "project", revision: 1, inputHash: "user", outputHash: "project",
    diff: { baseHash: "user", effectiveHash: "project", addedCharacters: 0, removedCharacters: 0, hunks: [], hunksTruncated: false },
  }],
  scopeDiffsTruncated: false,
  mutations: [], mutationsTruncated: false, resources: [], resourcesTruncated: false,
  conflicts: [], conflictsTruncated: false,
};

describe("SkillOverlayDiffView", () => {
  it("switches from the final comparison to a labeled scope comparison", async () => {
    const user = userEvent.setup();
    render(<SkillOverlayDiffView detail={detail} />);

    expect(screen.getByText((_, element) => element?.tagName === "P" && element.textContent?.includes("基础包（系统层）") === true)).toBeTruthy();
    expect(screen.getByText("Project instructions")).toBeTruthy();
    const userScope = screen.getByRole("button", { name: "用户 Overlay" });
    await user.click(userScope);

    expect(userScope.getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByText((_, element) => element?.tagName === "P" && element.textContent?.includes("前序作用域输出") === true)).toBeTruthy();
    expect(screen.getByText("User instructions")).toBeTruthy();
    expect(screen.getByText("此侧内容已按安全展示上限截断。")).toBeTruthy();
  });

  it("states explicitly when a scope makes no effective instruction change", async () => {
    const user = userEvent.setup();
    render(<SkillOverlayDiffView detail={detail} />);
    await user.click(screen.getByRole("button", { name: "项目 Overlay" }));
    expect(screen.getByText("此对比未改变有效指令。")).toBeTruthy();
  });
});
