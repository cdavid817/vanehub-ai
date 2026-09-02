// @vitest-environment jsdom

import { afterEach, describe, expect, it } from "vitest";
import type { AgentRunState } from "../types/agent-run";
import {
  readMissionControlScrollTop,
  readMissionControlViewState,
  writeMissionControlScrollTop,
  writeMissionControlViewState,
} from "./mission-control-view-state";

afterEach(() => sessionStorage.clear());

describe("mission control view-state persistence", () => {
  it("returns null when nothing has been written yet", () => {
    expect(readMissionControlViewState()).toBeNull();
  });

  it("round-trips a written view state", () => {
    const state = { states: ["failed"] as AgentRunState[], agentId: "claude-code", projectId: "proj-1", runner: "ssh" as const, sort: "newest" as const };
    writeMissionControlViewState(state);
    expect(readMissionControlViewState()).toEqual(state);
  });

  it("round-trips the two-state 'blocked' combination a metric card can write", () => {
    const state = { states: ["blocked", "stuck"] as AgentRunState[], agentId: "", projectId: "", runner: "" as const, sort: "attention" as const };
    writeMissionControlViewState(state);
    expect(readMissionControlViewState()).toEqual(state);
  });

  it("discards a malformed stored value rather than throwing", () => {
    sessionStorage.setItem("vanehub.mission-control.view.v2", "{not json");
    expect(readMissionControlViewState()).toBeNull();

    sessionStorage.setItem("vanehub.mission-control.view.v2", JSON.stringify({ states: ["failed"], agentId: "y", projectId: "z", runner: "bogus", sort: "newest" }));
    expect(readMissionControlViewState()).toBeNull();

    sessionStorage.setItem("vanehub.mission-control.view.v2", JSON.stringify({ states: ["not-a-real-state"], agentId: "y", projectId: "z", runner: "ssh", sort: "newest" }));
    expect(readMissionControlViewState()).toBeNull();
  });

  it("discards a pre-16.4 single-`status` blob left over from an older build rather than misreading it", () => {
    sessionStorage.setItem("vanehub.mission-control.view.v1", JSON.stringify({ status: "failed", agentId: "y", projectId: "z", runner: "ssh", sort: "newest" }));
    // Read under the new `.v2` key: an old build's `.v1` blob is simply invisible, not migrated.
    expect(readMissionControlViewState()).toBeNull();
  });

  it("defaults scroll position to 0 when nothing has been written yet", () => {
    expect(readMissionControlScrollTop()).toBe(0);
  });

  it("round-trips a written scroll position", () => {
    writeMissionControlScrollTop(240);
    expect(readMissionControlScrollTop()).toBe(240);
  });

  it("discards a negative or non-numeric stored scroll position", () => {
    sessionStorage.setItem("vanehub.mission-control.scroll.v1", "-5");
    expect(readMissionControlScrollTop()).toBe(0);

    sessionStorage.setItem("vanehub.mission-control.scroll.v1", "not-a-number");
    expect(readMissionControlScrollTop()).toBe(0);
  });
});
