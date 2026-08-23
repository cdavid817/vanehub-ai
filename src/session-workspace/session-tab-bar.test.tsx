// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { I18nextProvider } from "react-i18next";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import { SessionTabBar, type SessionTabId } from "./session-tab-bar";
import type { WorkspaceTabBadge } from "./workspace-evidence-badges";

function mount(badges: Partial<Record<SessionTabId, WorkspaceTabBadge>>) {
  return render(
    <I18nextProvider i18n={i18n}>
      <SessionTabBar
        activeTab="chat"
        badges={badges}
        onActivate={() => undefined}
        onOpenSettings={() => undefined}
        session={null}
      />
    </I18nextProvider>,
  );
}

function describedText(tab: SessionTabId): string {
  const button = screen.getByRole("tab", { name: tabLabel(tab) });
  const id = button.getAttribute("aria-describedby");
  return id === null ? "" : (document.getElementById(id)?.textContent ?? "");
}

function tabLabel(tab: SessionTabId): string {
  return i18n.t(`sessionTabs.tab.${tab}`);
}

describe("SessionTabBar badges", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("keeps a tab's accessible name free of its badge", () => {
    mount({
      changes: { kind: "count", count: 4, tone: "neutral", atLeast: false },
      shell: { kind: "unknown", reason: "indexing" },
    });

    // The name identifies the tab and nothing else. Folding a live count into it makes the name
    // change as work runs, and a name like "Changes, unviewed changed files: 4" then answers a
    // search for the Files tab — which is how a click lands on the wrong panel.
    for (const tab of ["changes", "documents", "files", "shell"] as const) {
      expect(screen.getByRole("tab", { name: tabLabel(tab), exact: true })).toBeTruthy();
    }
  });

  it("speaks the badge as the tab's description", () => {
    mount({
      changes: { kind: "count", count: 4, tone: "neutral", atLeast: false },
      logs: { kind: "count", count: 2, tone: "danger", atLeast: true },
      shell: { kind: "unknown", reason: "indexing" },
    });

    expect(describedText("changes")).toBe("Unviewed changed files: 4");
    expect(describedText("logs")).toBe("New errors: at least 2");
    // The placeholder says why it cannot count, rather than leaving a bare glyph unexplained.
    expect(describedText("shell")).toContain("the index is still building");
  });

  it("renders a floor and a placeholder differently on screen", () => {
    mount({
      logs: { kind: "count", count: 2, tone: "danger", atLeast: true },
      shell: { kind: "unknown", reason: "unavailable" },
      report: { kind: "none" },
    });

    const floor = document.querySelector('[data-badge="logs-count"]');
    const placeholder = document.querySelector('[data-badge="shell-unknown"]');
    expect(floor?.textContent).toBe("≥2");
    expect(placeholder?.textContent).toBe("·");
    // A known zero is the absence of a badge, not a rendered "0".
    expect(document.querySelector('[data-badge="report-count"]')).toBeNull();
    expect(document.querySelector('[data-badge="report-unknown"]')).toBeNull();
  });

  it("gives a tab with nothing to report no description at all", () => {
    mount({});
    for (const { id } of [{ id: "changes" as const }, { id: "shell" as const }]) {
      expect(screen.getByRole("tab", { name: tabLabel(id) }).getAttribute("aria-describedby")).toBeNull();
    }
  });
});
