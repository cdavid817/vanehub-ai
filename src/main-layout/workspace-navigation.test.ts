import { describe, expect, it } from "vitest";
import { resolveMissionControlNavigation } from "./workspace-navigation";

describe("resolveMissionControlNavigation", () => {
  it("routes loops and goals to their Automations tabs", () => {
    expect(resolveMissionControlNavigation({ kind: "loop", id: "loop-1" })).toEqual({
      kind: "workspace", location: { destination: "automations", view: "loops" },
    });
    expect(resolveMissionControlNavigation({ kind: "goal", id: "goal-1" })).toEqual({
      kind: "workspace", location: { destination: "automations", view: "goals" },
    });
  });

  it("routes evaluations to the settings page", () => {
    expect(resolveMissionControlNavigation({ kind: "evaluation", id: "arena-1" })).toEqual({ kind: "settings", pageId: "evaluation" });
  });

  it("opens the session for session, approval, and review targets, asking for the Changes tab on review", () => {
    expect(resolveMissionControlNavigation({ kind: "session", id: "s-1" })).toEqual({
      kind: "workspace", location: { destination: "sessions", sessionId: "s-1", creatingSession: false },
    });
    expect(resolveMissionControlNavigation({ kind: "approval", id: "a-1", sessionId: "s-2" })).toEqual({
      kind: "workspace", location: { destination: "sessions", sessionId: "s-2", creatingSession: false },
    });
    expect(resolveMissionControlNavigation({ kind: "review", id: "r-1", sessionId: "s-3" })).toEqual({
      kind: "workspace", location: { destination: "sessions", sessionId: "s-3", creatingSession: false }, sessionTab: "changes",
    });
  });
});
