import { describe, expect, it } from "vitest";
import type { SessionSeat } from "../types/agent";
import { SESSION_PRIMARY_SURFACE_IDS, SESSION_RUNTIME_SURFACE_IDS } from "./session-surface-registry";
import { effectiveSeatId, tabScope, showsSeatSwitcher } from "./tab-scope";

describe("tabScope", () => {
  // What one Agent ran belongs to that Agent; what the project looks like does not.
  it("scopes the per-Agent views to a seat", () => {
    expect(tabScope("terminal-history")).toBe("seat");
    expect(tabScope("shell")).toBe("seat");
    expect(tabScope("logs")).toBe("seat");
  });

  it("keeps project-level views session-scoped", () => {
    expect(tabScope("changes")).toBe("session");
    expect(tabScope("files")).toBe("session");
    expect(tabScope("report")).toBe("session");
    expect(tabScope("work")).toBe("session");
  });

  // The trace shows the whole round including handoffs, so splitting it by seat would destroy it.
  it("keeps the execution trace session-scoped", () => {
    expect(tabScope("traces")).toBe("session");
  });

  // A surface added later must be classified deliberately rather than defaulting to something.
  it("classifies every registered surface", () => {
    for (const id of [...SESSION_PRIMARY_SURFACE_IDS, ...SESSION_RUNTIME_SURFACE_IDS]) {
      expect(["seat", "session"]).toContain(tabScope(id));
    }
  });
});

describe("showsSeatSwitcher", () => {
  it("shows the switcher only for seat-scoped tabs in a multi-seat session", () => {
    expect(showsSeatSwitcher("terminal-history", 3)).toBe(true);
    expect(showsSeatSwitcher("changes", 3)).toBe(false);
  });

  // A single-seat session must look exactly as it does today.
  it("hides the switcher when there is only one seat", () => {
    expect(showsSeatSwitcher("terminal-history", 1)).toBe(false);
  });
});

describe("effectiveSeatId", () => {
  const seats: SessionSeat[] = [
    { seatId: "seat-planner", agentId: "claude-code", roleId: "planner" },
    { seatId: "seat-builder", agentId: "codex-cli", roleId: "builder" },
  ];

  // The switcher used to be rendered next to these tabs without reaching their queries, so
  // choosing a seat changed the highlighted button and nothing else.
  it("resolves the selected seat for a seat-scoped tab", () => {
    expect(effectiveSeatId("logs", seats, 1)).toBe("seat-builder");
    expect(effectiveSeatId("terminal-history", seats, 0)).toBe("seat-planner");
    expect(effectiveSeatId("shell", seats, 1)).toBe("seat-builder");
  });

  it("returns no seat for a session-scoped tab even while a seat is selected", () => {
    expect(effectiveSeatId("changes", seats, 1)).toBeNull();
    expect(effectiveSeatId("traces", seats, 1)).toBeNull();
    expect(effectiveSeatId("report", seats, 1)).toBeNull();
  });

  // One seat is the whole session; scoping the query would only invite an empty result.
  it("returns no seat for a single-seat session", () => {
    expect(effectiveSeatId("logs", [seats[0]], 0)).toBeNull();
  });

  // A seat persisted before stable seat ids existed cannot be used as a filter, and guessing an
  // id would silently scope the query to the wrong participant.
  it("returns no seat when the selected seat has no stable id", () => {
    expect(effectiveSeatId("logs", [{ agentId: "claude-code", roleId: null }, seats[1]], 0)).toBeNull();
  });

  it("returns no seat when the selection is out of range", () => {
    expect(effectiveSeatId("logs", seats, 7)).toBeNull();
  });
});
