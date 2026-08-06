import { describe, expect, it } from "vitest";
import { sessionTabDefinitions, type SessionTabId } from "./session-tab-bar";
import { tabScope, showsSeatSwitcher } from "./tab-scope";

describe("tabScope", () => {
  // What one Agent ran belongs to that Agent; what the project looks like does not.
  it("scopes the per-Agent views to a seat", () => {
    expect(tabScope("terminal")).toBe("seat");
    expect(tabScope("shell")).toBe("seat");
    expect(tabScope("logs")).toBe("seat");
  });

  it("keeps project-level views session-scoped", () => {
    expect(tabScope("changes")).toBe("session");
    expect(tabScope("documents")).toBe("session");
    expect(tabScope("files")).toBe("session");
    expect(tabScope("report")).toBe("session");
    expect(tabScope("chat")).toBe("session");
  });

  // The trace shows the whole round including handoffs, so splitting it by seat would destroy it.
  it("keeps the execution trace session-scoped", () => {
    expect(tabScope("traces")).toBe("session");
  });

  // A tab added later must be classified deliberately rather than defaulting to something.
  it("classifies every registered tab", () => {
    for (const definition of sessionTabDefinitions) {
      expect(["seat", "session"]).toContain(tabScope(definition.id as SessionTabId));
    }
  });
});

describe("showsSeatSwitcher", () => {
  it("shows the switcher only for seat-scoped tabs in a multi-seat session", () => {
    expect(showsSeatSwitcher("terminal", 3)).toBe(true);
    expect(showsSeatSwitcher("changes", 3)).toBe(false);
  });

  // A single-seat session must look exactly as it does today.
  it("hides the switcher when there is only one seat", () => {
    expect(showsSeatSwitcher("terminal", 1)).toBe(false);
  });
});
