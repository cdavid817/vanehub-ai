import { describe, expect, it } from "vitest";
import type { AgentRunState } from "../types/agent-run";
import type { MissionControlCounts } from "../types/mission-control";
import {
  clearMissionControlFilters, defaultMissionControlFilterState, isMissionControlFilterActive,
  MISSION_CONTROL_COUNT_STATES, MISSION_CONTROL_STATUS_OPTIONS, sameStateSet, toMissionControlQuery,
  type MissionControlFilterState,
} from "./mission-control-query";

// 16.4's own "confirm the mapping is exact, don't guess": every `MissionControlCounts` field name
// must have a matching mapping, and the values must be the exact `AgentRunState`(s) both backends'
// own count projection derives that field from (verified by reading mission_control.rs and
// web-mission-control-client.ts directly -- see mission-control-query.ts's own doc comment).
const expectedCountStates: Record<keyof MissionControlCounts, AgentRunState[]> = {
  running: ["running"],
  waitingApproval: ["waiting_approval"],
  waitingUser: ["waiting_user"],
  retrying: ["retrying"],
  blocked: ["blocked", "stuck"],
  failed: ["failed"],
  completedRecently: ["completed"],
};

describe("MISSION_CONTROL_COUNT_STATES", () => {
  it("maps every count field to exactly the AgentRunState value(s) both backends derive it from", () => {
    expect(MISSION_CONTROL_COUNT_STATES).toEqual(expectedCountStates);
  });

  it("is the one count whose own mapping is a union of two states, not a single one", () => {
    const singleStateCounts = (Object.keys(MISSION_CONTROL_COUNT_STATES) as (keyof MissionControlCounts)[])
      .filter((key) => key !== "blocked");
    for (const key of singleStateCounts) expect(MISSION_CONTROL_COUNT_STATES[key]).toHaveLength(1);
    expect(MISSION_CONTROL_COUNT_STATES.blocked).toHaveLength(2);
  });

  it("keeps the plain status dropdown single-state only -- 'blocked' is reachable solely via the count card", () => {
    expect(MISSION_CONTROL_STATUS_OPTIONS).not.toContain("blocked");
    expect(MISSION_CONTROL_STATUS_OPTIONS).toContain("stuck");
  });
});

describe("sameStateSet", () => {
  it("is true for the same states in a different order", () => {
    expect(sameStateSet(["blocked", "stuck"], ["stuck", "blocked"])).toBe(true);
  });

  it("is false when lengths differ, even if one is a subset of the other", () => {
    expect(sameStateSet(["stuck"], ["blocked", "stuck"])).toBe(false);
  });

  it("is false for two disjoint single-state arrays", () => {
    expect(sameStateSet(["failed"], ["completed"])).toBe(false);
  });

  it("is true for two empty arrays", () => {
    expect(sameStateSet([], [])).toBe(true);
  });
});

describe("isMissionControlFilterActive", () => {
  it("is false for the default (no) filters", () => {
    expect(isMissionControlFilterActive(defaultMissionControlFilterState)).toBe(false);
  });

  it("ignores sort -- a display-order change is not a narrowing filter", () => {
    expect(isMissionControlFilterActive({ ...defaultMissionControlFilterState, sort: "newest" })).toBe(false);
  });

  it.each<[string, Partial<MissionControlFilterState>]>([
    ["agentId", { agentId: "claude-code" }],
    ["projectId", { projectId: "proj-1" }],
    ["states", { states: ["failed"] }],
    ["runner", { runner: "ssh" }],
  ])("is true once %s is set", (_name, patch) => {
    expect(isMissionControlFilterActive({ ...defaultMissionControlFilterState, ...patch })).toBe(true);
  });
});

describe("clearMissionControlFilters", () => {
  it("resets every narrowing dimension but preserves sort", () => {
    const active: MissionControlFilterState = { agentId: "claude-code", projectId: "proj-1", states: ["blocked", "stuck"], runner: "ssh", sort: "newest" };
    expect(clearMissionControlFilters(active)).toEqual({ ...defaultMissionControlFilterState, sort: "newest" });
  });
});

describe("toMissionControlQuery", () => {
  it("projects empty strings to undefined and an empty states array to undefined", () => {
    expect(toMissionControlQuery(defaultMissionControlFilterState, null)).toEqual({
      agentId: undefined, cursor: null, limit: 20, projectId: undefined, runner: undefined, sort: "attention", states: undefined,
    });
  });

  it("passes every set value straight through, including a two-state array and a non-null cursor", () => {
    const filter: MissionControlFilterState = { agentId: "claude-code", projectId: "proj-1", states: ["blocked", "stuck"], runner: "ssh", sort: "newest" };
    expect(toMissionControlQuery(filter, "20")).toEqual({
      agentId: "claude-code", cursor: "20", limit: 20, projectId: "proj-1", runner: "ssh", sort: "newest", states: ["blocked", "stuck"],
    });
  });
});
