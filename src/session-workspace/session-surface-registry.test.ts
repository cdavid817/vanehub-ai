import { describe, expect, it } from "vitest";
import {
  isPrimarySurface,
  isRuntimeSurface,
  lookupSessionSurfaceDefinition,
  SESSION_PRIMARY_SURFACE_IDS,
  SESSION_RUNTIME_SURFACE_IDS,
  SESSION_SURFACE_REGISTRY,
  sessionSurfaceDefinition,
  showsSessionSeatSwitcher,
} from "./session-surface-registry";

describe("session surface registry", () => {
  it("declares exactly four primary and four runtime surfaces", () => {
    expect(SESSION_PRIMARY_SURFACE_IDS).toEqual(["work", "changes", "files", "report"]);
    expect(SESSION_RUNTIME_SURFACE_IDS).toEqual(["terminal-history", "shell", "logs", "traces"]);
    expect(Object.keys(SESSION_SURFACE_REGISTRY).sort()).toEqual(
      [...SESSION_PRIMARY_SURFACE_IDS, ...SESSION_RUNTIME_SURFACE_IDS].sort(),
    );
    for (const id of [...SESSION_PRIMARY_SURFACE_IDS, ...SESSION_RUNTIME_SURFACE_IDS]) {
      expect(sessionSurfaceDefinition(id).id).toBe(id);
    }
  });

  it("refuses to answer for a surface nobody registered", () => {
    // A permissive default is the dangerous answer: an unregistered panel would inherit
    // "session-scoped, no live work, cached", keep its subscription running while hidden, and
    // never show a seat switcher it needed.
    expect(lookupSessionSurfaceDefinition("onepiece-scratch")).toBeNull();
    expect(lookupSessionSurfaceDefinition("logs")).toBe(SESSION_SURFACE_REGISTRY.logs);
    expect(lookupSessionSurfaceDefinition("toString")).toBeNull();
  });

  it("routes exactly the runtime four to the Runtime Panel", () => {
    for (const id of SESSION_RUNTIME_SURFACE_IDS) {
      expect(sessionSurfaceDefinition(id).region).toBe("runtime");
      expect(isRuntimeSurface(id)).toBe(true);
      expect(isPrimarySurface(id)).toBe(false);
    }
    for (const id of SESSION_PRIMARY_SURFACE_IDS) {
      expect(sessionSurfaceDefinition(id).region).toBe("primary");
      expect(isPrimarySurface(id)).toBe(true);
      expect(isRuntimeSurface(id)).toBe(false);
    }
  });

  it("marks a shell as needing one concrete seat and terminal history as accepting all", () => {
    expect(sessionSurfaceDefinition("shell").scope).toBe("seat-required");
    expect(sessionSurfaceDefinition("terminal-history").scope).toBe("seat-optional");
    expect(sessionSurfaceDefinition("logs").scope).toBe("seat-optional");
    expect(sessionSurfaceDefinition("traces").scope).toBe("session");
    expect(sessionSurfaceDefinition("report").scope).toBe("session");
  });

  it("keeps a live attachment mounted only where ending it would end the user's work", () => {
    expect(sessionSurfaceDefinition("shell").retention).toBe("keep-mounted-while-active-run");
    expect(sessionSurfaceDefinition("work").retention).toBe("keep-mounted-while-active-run");
    for (const id of ["changes", "files", "terminal-history", "logs", "traces", "report"] as const) {
      expect(sessionSurfaceDefinition(id).retention, id).toBe("cache");
    }
    // Nothing is thrown away on a tab switch, which is what makes a hidden panel's form survive.
    // Widened to strings on purpose: `satisfies` keeps the literal types, so comparing against
    // "unmount" directly is a type error rather than an assertion.
    const declared = new Set(
      Object.values(SESSION_SURFACE_REGISTRY).map((entry) => String(entry.retention)),
    );
    expect(declared.has("unmount")).toBe(false);
  });

  it("only lets background work outlive a hidden surface where a live terminal owns it", () => {
    expect(sessionSurfaceDefinition("work").liveUpdates).toBe("background-terminal");
    expect(sessionSurfaceDefinition("shell").liveUpdates).toBe("background-terminal");
    expect(sessionSurfaceDefinition("changes").liveUpdates).toBe("none");
    expect(sessionSurfaceDefinition("files").liveUpdates).toBe("none");
    expect(sessionSurfaceDefinition("report").liveUpdates).toBe("none");
  });

  it("shows the seat switcher only for a seat-scoped surface with more than one seat", () => {
    expect(showsSessionSeatSwitcher("logs", 2)).toBe(true);
    // One seat means one option, so the control would be a statement with no alternative.
    expect(showsSessionSeatSwitcher("logs", 1)).toBe(false);
    expect(showsSessionSeatSwitcher("report", 3)).toBe(false);
  });
});
