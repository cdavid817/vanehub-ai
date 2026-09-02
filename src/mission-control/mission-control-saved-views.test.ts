// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from "vitest";
import { defaultMissionControlFilterState, type MissionControlFilterState } from "./mission-control-query";
import {
  applyMissionControlSavedView, captureMissionControlSavedView, MISSION_CONTROL_SAVED_VIEW_NAME_MAX_LENGTH,
  readMissionControlSavedViews, writeMissionControlSavedViews, type MissionControlSavedView,
} from "./mission-control-saved-views";

const STORAGE_KEY = "vanehub.mission-control.saved-views.v1";

const urgentFilter: MissionControlFilterState = {
  ...defaultMissionControlFilterState,
  agentId: "claude-code",
  projectId: "proj-1",
  states: ["blocked", "stuck"],
  runner: "ssh",
  sort: "newest",
};

beforeEach(() => {
  localStorage.clear();
});

describe("captureMissionControlSavedView", () => {
  it("captures every filter dimension, including the two-state 'blocked' combination", () => {
    const view = captureMissionControlSavedView(urgentFilter, "Only failed on SSH", "view-1");
    expect(view).toEqual({
      id: "view-1", name: "Only failed on SSH", agentId: "claude-code", projectId: "proj-1",
      states: ["blocked", "stuck"], runner: "ssh", sort: "newest",
    });
  });

  it("trims and length-caps the name so it cannot become a dumping ground for unrestricted content", () => {
    const long = "x".repeat(MISSION_CONTROL_SAVED_VIEW_NAME_MAX_LENGTH + 50);
    const view = captureMissionControlSavedView(defaultMissionControlFilterState, `  ${long}  `, "view-2");
    expect(view.name).toHaveLength(MISSION_CONTROL_SAVED_VIEW_NAME_MAX_LENGTH);
  });
});

describe("applyMissionControlSavedView", () => {
  it("restores every stored dimension exactly", () => {
    const view = captureMissionControlSavedView(urgentFilter, "Only failed on SSH", "view-1");
    expect(applyMissionControlSavedView(view)).toEqual(urgentFilter);
  });
});

describe("read/writeMissionControlSavedViews round-trip", () => {
  it("returns an empty list before anything has been saved", () => {
    expect(readMissionControlSavedViews()).toEqual([]);
  });

  it("round-trips a saved view through write then read", () => {
    const view = captureMissionControlSavedView(urgentFilter, "Only failed on SSH", "view-1");
    writeMissionControlSavedViews([view]);
    expect(readMissionControlSavedViews()).toEqual([view]);
  });

  it("survives a whole-payload version bump gracefully by discarding the unreadable format", () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ version: 2, views: [captureMissionControlSavedView(urgentFilter, "Future format", "view-1")] }));
    expect(readMissionControlSavedViews()).toEqual([]);
  });

  it("drops one malformed entry within an otherwise-current payload without losing the others", () => {
    const good = captureMissionControlSavedView(urgentFilter, "Good view", "view-good");
    const corrupt = { ...good, id: "view-bad", runner: "not-a-real-runner" };
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ version: 1, views: [good, corrupt] }));
    expect(readMissionControlSavedViews()).toEqual([good]);
  });

  it("drops an entry whose states array contains a value no UI path can ever produce", () => {
    const good = captureMissionControlSavedView(urgentFilter, "Good view", "view-good");
    const corrupt: MissionControlSavedView = { ...good, id: "view-bad", states: ["not-a-real-state" as MissionControlSavedView["states"][number]] };
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ version: 1, views: [good, corrupt] }));
    expect(readMissionControlSavedViews()).toEqual([good]);
  });

  it("fails closed to an empty list rather than throwing on unparseable JSON", () => {
    localStorage.setItem(STORAGE_KEY, "not json");
    expect(readMissionControlSavedViews()).toEqual([]);
  });

  it("deletes are just a write of the filtered list -- confirmed by round-tripping one out", () => {
    const keep = captureMissionControlSavedView(urgentFilter, "Keep me", "view-keep");
    const drop: MissionControlSavedView = { ...keep, id: "view-drop", name: "Drop me" };
    writeMissionControlSavedViews([keep, drop]);
    writeMissionControlSavedViews(readMissionControlSavedViews().filter((view) => view.id !== "view-drop"));
    expect(readMissionControlSavedViews()).toEqual([keep]);
  });
});
