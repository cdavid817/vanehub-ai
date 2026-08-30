// @vitest-environment jsdom

import { afterEach, describe, expect, it } from "vitest";
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
    const state = { status: "failed", agentId: "claude-code", projectId: "proj-1", runner: "ssh" as const, sort: "newest" as const };
    writeMissionControlViewState(state);
    expect(readMissionControlViewState()).toEqual(state);
  });

  it("discards a malformed stored value rather than throwing", () => {
    sessionStorage.setItem("vanehub.mission-control.view.v1", "{not json");
    expect(readMissionControlViewState()).toBeNull();

    sessionStorage.setItem("vanehub.mission-control.view.v1", JSON.stringify({ status: "x", agentId: "y", projectId: "z", runner: "bogus", sort: "newest" }));
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
