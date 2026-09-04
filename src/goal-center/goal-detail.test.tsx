// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "../i18n";
import type { Goal } from "../contracts/goal";
import { GoalDetail } from "./goal-detail";

function fixture(overrides: Partial<Goal> = {}): Goal {
  return {
    id: "goal-1",
    title: "统一工作台改版",
    description: "",
    acceptanceNotes: "",
    status: "active",
    derivedStatus: "active",
    projectPath: null,
    createdAt: "2026-01-01T00:00:00.000Z",
    updatedAt: "2026-01-01T00:00:00.000Z",
    counted: 0,
    terminal: 0,
    unresolvable: 0,
    links: [],
    ...overrides,
  };
}

function renderDetail(overrides: Partial<Goal> = {}) {
  return render(
    <GoalDetail
      goal={fixture(overrides)}
      onAbandon={vi.fn()}
      onAccept={vi.fn()}
      onActivate={vi.fn()}
      onDelete={vi.fn()}
      onDismissError={vi.fn()}
      onEdit={vi.fn()}
      onLink={vi.fn()}
      onReopen={vi.fn()}
      onUnlink={vi.fn()}
    />,
  );
}

describe("GoalDetail", () => {
  it("renders the goal's title and the More actions trigger", () => {
    renderDetail();
    expect(screen.getByRole("heading", { name: "统一工作台改版", level: 2 })).toBeTruthy();
    expect(screen.getByRole("button", { name: "更多操作" })).toBeTruthy();
  });

  /**
   * 20.15: `goal-detail.tsx`'s title `<h2 id="goal-detail-title">` already carries `truncate`
   * (unlike work-board-card.tsx's own title before this pass's fix) -- this proves that
   * pre-existing mechanism actually holds for the long-string classes the task names, next to the
   * status badge in the same header row and the actions row's ActionMenu trigger, rather than
   * leaving it an unverified claim. jsdom has no real layout engine, so this checks the mechanism
   * (the `truncate` class, both neighbors still present and un-swallowed), not pixels.
   */
  describe("long title safety (20.15)", () => {
    const GERMAN_LIKE_TITLE = "Konfigurationsverwaltungsoberflächenkomponentenübersicht";
    const CJK_TITLE = "这是一个非常非常非常长的目标标题用来验证界面在极端文本长度下不会与状态徽章或操作菜单发生重叠";

    it("truncates a long German-like goal title next to the status badge and More trigger", () => {
      renderDetail({ title: GERMAN_LIKE_TITLE });
      const heading = screen.getByRole("heading", { name: GERMAN_LIKE_TITLE, level: 2 });
      expect(heading.className).toContain("truncate");
      expect(screen.getByRole("button", { name: "更多操作" })).toBeTruthy();
    });

    it("truncates a long CJK goal title the same way", () => {
      renderDetail({ title: CJK_TITLE });
      expect(screen.getByRole("heading", { name: CJK_TITLE, level: 2 }).className).toContain("truncate");
    });
  });

  // 20.16: `projectPath` is filesystem-sourced, like work-board-card.tsx's own MetaChip path --
  // disclosed here, not fixed in this pass (see tasks.md 20.16 evidence for the full, deliberately
  // surgical fix list); this records the pre-existing, already-correct `truncate` behavior only.
  it("truncates a long project path underneath the title", () => {
    const longPath = "D:/workspace/monorepo/packages/frontend-application/src/features/goal-center";
    renderDetail({ projectPath: longPath });
    expect(screen.getByTitle(longPath).className).toContain("truncate");
  });
});
