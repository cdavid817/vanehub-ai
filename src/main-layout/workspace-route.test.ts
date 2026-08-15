import { describe, expect, it } from "vitest";
import {
  defaultWorkspaceLocation,
  parseWorkspaceLocation,
  workspaceDestinations,
  workspacePath,
} from "./workspace-route";

describe("workspace route", () => {
  it("round-trips every destination", () => {
    for (const destination of workspaceDestinations) {
      const path = workspacePath({ destination });
      expect(parseWorkspaceLocation(path).destination, path).toBe(destination);
    }
  });

  it("addresses a session and round-trips its id", () => {
    const path = workspacePath({ destination: "sessions", sessionId: "session-7" });
    expect(path).toBe("/workspace/sessions/session-7");
    expect(parseWorkspaceLocation(path)).toEqual({
      destination: "sessions",
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
    expect(parseWorkspaceLocation("/workspace")).toEqual(defaultWorkspaceLocation);
    expect(parseWorkspaceLocation("/settings")).toEqual(defaultWorkspaceLocation);
  });

  it("drops the session detail from non-session destinations", () => {
    expect(parseWorkspaceLocation("/workspace/loops")).toEqual({
      destination: "loops",
      sessionId: null,
      creatingSession: false,
    });
  });
});
