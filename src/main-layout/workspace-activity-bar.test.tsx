// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { Inbox, MessagesSquare, Settings, Workflow } from "lucide-react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { WorkspaceActivityBar, type ActivityItem } from "./workspace-activity-bar";

afterEach(cleanup);

function items(overrides: Partial<Record<ActivityItem["id"], Partial<ActivityItem>>> = {}) {
  const primary: ActivityItem[] = [
    { id: "sessions", icon: MessagesSquare, label: "Collapse sessions", shortcut: "Mod+1", ariaControls: "workspace-session-sidebar", expanded: true, active: true, onSelect: vi.fn(), ...overrides.sessions },
    { id: "inbox", icon: Inbox, label: "Inbox", shortcut: "Mod+2", ariaControls: "inbox", onSelect: vi.fn(), ...overrides.inbox },
    { id: "automations", icon: Workflow, label: "Automations", shortcut: "Mod+3", ariaControls: "automations", onSelect: vi.fn(), ...overrides.automations },
  ];
  const utility: ActivityItem[] = [
    { id: "settings", icon: Settings, label: "Settings", shortcut: "Mod+4", testId: "desktop-smoke-settings", onSelect: vi.fn(), ...overrides.settings },
  ];
  return { primary, utility };
}

describe("WorkspaceActivityBar", () => {
  it("renders exactly the configured icon-only entries in two groups", () => {
    const { primary, utility } = items();
    render(<WorkspaceActivityBar items={primary} label="Workspace navigation" utilityItems={utility} />);

    const nav = screen.getByRole("navigation", { name: "Workspace navigation" });
    const buttons = nav.querySelectorAll("button");
    expect(buttons).toHaveLength(4);
    expect([...buttons].map((button) => button.getAttribute("aria-label"))).toEqual(["Collapse sessions", "Inbox", "Automations", "Settings"]);
    expect(nav.querySelector("[data-activity-group='primary']")?.querySelectorAll("button")).toHaveLength(3);
    expect(nav.querySelector("[data-activity-group='utility']")?.querySelectorAll("button")).toHaveLength(1);
    expect(nav.textContent).toBe("");
    expect(screen.getByTestId("desktop-smoke-settings").getAttribute("aria-label")).toBe("Settings");
    for (const retired of ["Loops", "Scheduled tasks", "Todo Board", "Goals", "Evaluations", "System activity", "Mission Control", "Help"]) {
      expect(screen.queryByRole("button", { name: retired })).toBeNull();
    }
  });

  it("carries the shortcut in the tooltip and aria-keyshortcuts, and the sidebar state on Sessions", () => {
    const { primary, utility } = items({ sessions: { expanded: false, label: "Expand sessions", active: false } });
    render(<WorkspaceActivityBar items={primary} label="nav" utilityItems={utility} />);

    const sessions = screen.getByRole("button", { name: "Expand sessions" });
    expect(sessions.getAttribute("aria-expanded")).toBe("false");
    expect(sessions.getAttribute("aria-controls")).toBe("workspace-session-sidebar");
    expect(sessions.title).toMatch(/^Expand sessions · (Ctrl\+1|⌘1)$/);
    expect(sessions.getAttribute("aria-keyshortcuts")).toMatch(/^(Control|Meta)\+1$/);
    expect(screen.getByRole("button", { name: "Inbox" }).getAttribute("aria-expanded")).toBeNull();
    expect(screen.getByRole("button", { name: "Automations" }).title).toMatch(/3$/);
    expect(screen.getByRole("button", { name: "Settings" }).title).toMatch(/4$/);
  });

  it("marks only the active entry and forwards selection", () => {
    const { primary, utility } = items({ sessions: { active: false }, inbox: { active: true } });
    render(<WorkspaceActivityBar items={primary} label="nav" utilityItems={utility} />);

    expect(screen.getByRole("button", { name: "Inbox" }).className).toContain("text-primary");
    expect(screen.getByRole("button", { name: "Collapse sessions" }).className).not.toContain("text-primary");
    fireEvent.click(screen.getByRole("button", { name: "Automations" }));
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(primary[2].onSelect).toHaveBeenCalledOnce();
    expect(utility[0].onSelect).toHaveBeenCalledOnce();
    expect(primary[0].onSelect).not.toHaveBeenCalled();
  });

  it("renders a badge only on an entry whose count is above zero", () => {
    const { primary, utility } = items({ inbox: { badge: 7 }, automations: { badge: 0 } });
    render(<WorkspaceActivityBar items={primary} label="nav" utilityItems={utility} />);

    expect(screen.getByTestId("activity-bar-badge-inbox").textContent).toBe("7");
    expect(screen.queryByTestId("activity-bar-badge-automations")).toBeNull();
    expect(screen.queryByTestId("activity-bar-badge-sessions")).toBeNull();
    expect(document.querySelectorAll("[data-testid^='activity-bar-badge-']")).toHaveLength(1);
  });

  it("caps a large badge", () => {
    const { primary, utility } = items({ inbox: { badge: 250 } });
    render(<WorkspaceActivityBar items={primary} label="nav" utilityItems={utility} />);
    expect(screen.getByTestId("activity-bar-badge-inbox").textContent).toBe("99+");
  });
});
