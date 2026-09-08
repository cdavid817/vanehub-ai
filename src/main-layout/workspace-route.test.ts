// @vitest-environment jsdom

import { afterEach, describe, expect, it } from "vitest";
import {
  automationsViews,
  defaultWorkspaceLocation,
  inboxViews,
  parseWorkspaceLocation,
  recallWorkspacePath,
  rememberWorkspaceLocation,
  resolveWorkspaceRoute,
  workspaceDestinations,
  workspacePath,
} from "./workspace-route";

afterEach(() => localStorage.clear());

describe("workspace route", () => {
  it("round-trips every destination on its default view", () => {
    for (const destination of workspaceDestinations) {
      const path = workspacePath({ destination });
      const parsed = parseWorkspaceLocation(path);
      expect(parsed.destination, path).toBe(destination);
      expect(workspacePath(parsed), path).toBe(path);
    }
  });

  it("round-trips every inbox and automations view", () => {
    for (const view of inboxViews) {
      const path = workspacePath({ destination: "inbox", view });
      expect(path).toBe(`/workspace/inbox/${view}`);
      expect(parseWorkspaceLocation(path)).toEqual({ destination: "inbox", view, sessionId: null, creatingSession: false });
    }
    for (const view of automationsViews) {
      const path = workspacePath({ destination: "automations", view });
      expect(path).toBe(`/workspace/automations/${view}`);
      expect(parseWorkspaceLocation(path)).toEqual({ destination: "automations", view, sessionId: null, creatingSession: false });
    }
  });

  it("falls back to the destination's default view for a missing, unknown, or foreign view", () => {
    expect(parseWorkspaceLocation("/workspace/inbox").view).toBe("attention");
    expect(parseWorkspaceLocation("/workspace/inbox/nope").view).toBe("attention");
    expect(parseWorkspaceLocation("/workspace/inbox/loops").view).toBe("attention");
    expect(parseWorkspaceLocation("/workspace/automations").view).toBe("loops");
    expect(parseWorkspaceLocation("/workspace/automations/attention").view).toBe("loops");
    // Formatting normalizes too, so a location spread from another destination cannot leak its view.
    expect(workspacePath({ destination: "automations", view: "attention" })).toBe("/workspace/automations/loops");
    expect(workspacePath({ destination: "inbox", view: "goals" })).toBe("/workspace/inbox/attention");
  });

  it("addresses a session and round-trips its id", () => {
    const path = workspacePath({ destination: "sessions", sessionId: "session-7" });
    expect(path).toBe("/workspace/sessions/session-7");
    expect(parseWorkspaceLocation(path)).toEqual({
      destination: "sessions",
      view: null,
      sessionId: "session-7",
      creatingSession: false,
    });
  });

  it("escapes and restores session ids that need encoding", () => {
    const sessionId = "session/with space";
    const path = workspacePath({ destination: "sessions", sessionId });
    expect(path).not.toContain(" ");
    expect(parseWorkspaceLocation(path).sessionId).toBe(sessionId);
  });

  it("treats the reserved creation segment as a request, not a session id", () => {
    const location = parseWorkspaceLocation("/workspace/sessions/new");
    expect(location.creatingSession).toBe(true);
    expect(location.sessionId).toBeNull();
    expect(workspacePath({ destination: "sessions", creatingSession: true })).toBe("/workspace/sessions/new");
  });

  it("falls back to sessions for an unknown destination rather than rendering nothing", () => {
    expect(parseWorkspaceLocation("/workspace/nope")).toEqual(defaultWorkspaceLocation);
    expect(parseWorkspaceLocation("/workspace/plans")).toEqual(defaultWorkspaceLocation);
    expect(parseWorkspaceLocation("/workspace")).toEqual(defaultWorkspaceLocation);
    expect(parseWorkspaceLocation("/settings")).toEqual(defaultWorkspaceLocation);
  });

  it.each([
    ["/workspace/mission-control", "inbox", "attention"],
    ["/workspace/system-activity", "inbox", "attention"],
    ["/workspace/work-board", "inbox", "board"],
    ["/workspace/loops", "automations", "loops"],
    ["/workspace/goals", "automations", "goals"],
  ] as const)("redirects the retired path %s to %s/%s", (path, destination, view) => {
    expect(resolveWorkspaceRoute(path)).toEqual({
      kind: "workspace",
      legacy: true,
      location: { destination, view, sessionId: null, creatingSession: false },
    });
    expect(resolveWorkspaceRoute(workspacePath({ destination, view }))).toMatchObject({ legacy: false });
  });

  it("redirects the retired evaluations path to the evaluation settings page over the session workspace", () => {
    expect(resolveWorkspaceRoute("/workspace/evaluations")).toEqual({
      kind: "settings",
      pageId: "evaluation",
      location: defaultWorkspaceLocation,
    });
    expect(parseWorkspaceLocation("/workspace/evaluations")).toEqual(defaultWorkspaceLocation);
  });

  it("applies the same redirects to a remembered location without rewriting storage", () => {
    localStorage.setItem("vanehub.workspace.location.v1", "/workspace/loops");
    expect(recallWorkspacePath()).toBe("/workspace/automations/loops");
    localStorage.setItem("vanehub.workspace.location.v1", "/workspace/work-board");
    expect(recallWorkspacePath()).toBe("/workspace/inbox/board");
    localStorage.setItem("vanehub.workspace.location.v1", "/workspace/mission-control");
    expect(recallWorkspacePath()).toBe("/workspace/inbox/attention");
    // The evaluation redirect opens settings, which only the workspace route can do, so the
    // remembered path is handed back verbatim for it to resolve.
    localStorage.setItem("vanehub.workspace.location.v1", "/workspace/evaluations");
    expect(recallWorkspacePath()).toBe("/workspace/evaluations");
    localStorage.setItem("vanehub.workspace.location.v1", "/workspace/nonsense");
    expect(recallWorkspacePath()).toBe("/workspace/sessions");
    expect(localStorage.getItem("vanehub.workspace.location.v1")).toBe("/workspace/nonsense");
  });

  it("remembers a sub-view and recalls it unchanged", () => {
    rememberWorkspaceLocation({ destination: "automations", view: "scheduled", sessionId: null, creatingSession: false });
    expect(recallWorkspacePath()).toBe("/workspace/automations/scheduled");
    rememberWorkspaceLocation({ destination: "sessions", view: null, sessionId: "s-1", creatingSession: true });
    expect(recallWorkspacePath()).toBe("/workspace/sessions/s-1");
  });
});
