// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "../i18n";
import type { Session, SessionCategory } from "../types/agent";
import { SessionContextPanel } from "./session-context-panel";

const session = (overrides: Partial<Session> = {}): Session => ({
  id: "session-1",
  title: "发布会话",
  agentId: "claude",
  interactionMode: "cli",
  lifecycleState: "idle",
  archived: false,
  pinned: false,
  categoryId: null,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
  ...overrides,
} as Session);

const categories: SessionCategory[] = [
  { id: "cat-a", name: "分类A" } as SessionCategory,
  { id: "cat-b", name: "分类B" } as SessionCategory,
];

/**
 * 20.8: the "move to category" list inside this menu is Session category movement's real
 * non-drag alternative (reached from a session card's own row-actions button, not just
 * right-click) -- this covers it is genuinely keyboard-operable end to end, not just
 * click-reachable, which is all `tests/e2e/session-category-management.spec.ts` proves (and that
 * spec fails before reaching this menu at all, on an unrelated pre-existing "新建分类" dialog
 * timeout, so it cannot stand in for this either).
 */
describe("SessionContextPanel category assignment keyboard navigation", () => {
  function renderMenu(onAssignCategory = vi.fn()) {
    const target = session();
    render(
      <SessionContextPanel
        categories={categories}
        onArchive={vi.fn()}
        onAssignCategory={onAssignCategory}
        onChange={vi.fn()}
        onCreateCategory={vi.fn()}
        onDelete={vi.fn()}
        onDismiss={vi.fn()}
        onExport={vi.fn()}
        onPin={vi.fn()}
        onRecover={vi.fn()}
        onRename={vi.fn()}
        value={{ session: target, mode: "menu", draftTitle: target.title }}
      />,
    );
    return target;
  }

  it("lists every real category plus Uncategorized as menu items", () => {
    renderMenu();
    expect(screen.getByRole("menuitem", { name: "未分类" })).toBeTruthy();
    expect(screen.getByRole("menuitem", { name: "分类A" })).toBeTruthy();
    expect(screen.getByRole("menuitem", { name: "分类B" })).toBeTruthy();
  });

  it("moves focus between menu items on ArrowDown, reaching every category", () => {
    renderMenu();
    const rename = screen.getByRole("menuitem", { name: "重命名" });
    expect(document.activeElement).toBe(rename);

    // Walk forward until "分类A" is reached -- this menu's item count varies with the session's
    // own state (recover is conditional), so this does not hardcode a fixed number of presses.
    let steps = 0;
    while (document.activeElement !== screen.getByRole("menuitem", { name: "分类A" }) && steps < 20) {
      fireEvent.keyDown(document.activeElement as HTMLElement, { key: "ArrowDown" });
      steps += 1;
    }
    expect(document.activeElement).toBe(screen.getByRole("menuitem", { name: "分类A" }));

    fireEvent.keyDown(document.activeElement as HTMLElement, { key: "ArrowDown" });
    expect(document.activeElement).toBe(screen.getByRole("menuitem", { name: "分类B" }));
  });

  it("assigns the category under keyboard focus when activated", () => {
    const onAssignCategory = vi.fn();
    const target = renderMenu(onAssignCategory);
    fireEvent.click(screen.getByRole("menuitem", { name: "分类B" }));
    expect(onAssignCategory).toHaveBeenCalledWith(target, "cat-b");
  });

  it("jumps to the last menu item on End and back to the first on Home", () => {
    renderMenu();
    const rename = screen.getByRole("menuitem", { name: "重命名" });

    fireEvent.keyDown(rename, { key: "End" });
    expect(document.activeElement).toBe(screen.getByRole("menuitem", { name: "删除" }));

    fireEvent.keyDown(document.activeElement as HTMLElement, { key: "Home" });
    expect(document.activeElement).toBe(rename);
  });
});
