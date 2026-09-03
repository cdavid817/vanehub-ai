// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { ConversationOverflowMenu } from "./conversation-overflow-menu";

/**
 * `conversation-overflow-menu.test.tsx` renders with `renderToStaticMarkup` (no jsdom, no real
 * events), which is the right tool for its own static-markup assertions but cannot exercise a real
 * open-then-navigate flow -- this file is jsdom-based specifically to cover that, matching 20.7's
 * fix (this popup previously had Escape-to-close and nothing else; Up/Down/Home/End did nothing).
 */
describe("ConversationOverflowMenu keyboard navigation", () => {
  beforeAll(async () => activateAppLanguage("en"));

  function renderMenu() {
    return render(
      <ConversationOverflowMenu
        infoPanelExpanded={false}
        onToggleInfoPanel={vi.fn()}
        onToggleSessionList={vi.fn()}
        onToggleWorkspaceTabs={vi.fn()}
        sessionListExpanded
        workspaceTabsExpanded
      />,
    );
  }

  it("focuses the first item when opened", () => {
    renderMenu();
    fireEvent.click(screen.getByTestId("conversation-overflow-trigger"));
    expect(document.activeElement).toBe(screen.getByTestId("toggle-session-list"));
  });

  it("moves focus to the next item on ArrowDown, wrapping past the last", () => {
    renderMenu();
    fireEvent.click(screen.getByTestId("conversation-overflow-trigger"));
    const first = screen.getByTestId("toggle-session-list");

    fireEvent.keyDown(first, { key: "ArrowDown" });
    expect(document.activeElement).toBe(screen.getByTestId("toggle-info-panel"));

    fireEvent.keyDown(document.activeElement as HTMLElement, { key: "ArrowDown" });
    expect(document.activeElement).toBe(screen.getByTestId("toggle-workspace-tabs"));

    fireEvent.keyDown(document.activeElement as HTMLElement, { key: "ArrowDown" });
    expect(document.activeElement).toBe(first);
  });

  it("resets to the first item on a second open, even after navigating away from it", () => {
    renderMenu();
    fireEvent.click(screen.getByTestId("conversation-overflow-trigger"));
    fireEvent.keyDown(screen.getByTestId("toggle-session-list"), { key: "End" });
    expect(document.activeElement).toBe(screen.getByTestId("toggle-workspace-tabs"));

    // Close (Escape) and reopen -- this component persists across opens instead of remounting, so
    // the roving index needs its own reset rather than a fresh `useState` initializer doing it.
    fireEvent.keyDown(screen.getByRole("menu"), { key: "Escape" });
    fireEvent.click(screen.getByTestId("conversation-overflow-trigger"));

    expect(document.activeElement).toBe(screen.getByTestId("toggle-session-list"));
  });
});
