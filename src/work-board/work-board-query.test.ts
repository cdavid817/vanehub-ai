import { describe, expect, it } from "vitest";
import type { WorkItem } from "../types/work-board";
import {
  defaultWorkBoardQuery, groupWorkItemsByStage, isWorkBoardFilterActive, sortWorkItems, toWorkBoardFilters,
} from "./work-board-query";

const item = (overrides: Partial<WorkItem>): WorkItem => ({
  id: "work", title: "Release", description: "", stage: "review", priority: "medium", rank: 1_000,
  projectPath: null, dueAt: null, archived: false, createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z",
  sources: [], ...overrides,
});

describe("isWorkBoardFilterActive", () => {
  it("is false for the default query", () => {
    expect(isWorkBoardFilterActive(defaultWorkBoardQuery)).toBe(false);
  });

  it("is true when any filtering dimension moves off its default", () => {
    expect(isWorkBoardFilterActive({ ...defaultWorkBoardQuery, text: "release" })).toBe(true);
    expect(isWorkBoardFilterActive({ ...defaultWorkBoardQuery, source: "session" })).toBe(true);
    expect(isWorkBoardFilterActive({ ...defaultWorkBoardQuery, due: "overdue" })).toBe(true);
  });

  it("ignores sort, grouping, and presentation -- those change display, not membership", () => {
    expect(isWorkBoardFilterActive({ ...defaultWorkBoardQuery, sort: "priority", grouping: "none", presentation: "list" })).toBe(false);
  });
});

describe("toWorkBoardFilters", () => {
  it("threads archived separately and turns single-selection query fields into filter arrays", () => {
    const filters = toWorkBoardFilters({ ...defaultWorkBoardQuery, source: "session", project: "D:/app" }, true);
    expect(filters).toEqual({ archived: true, query: "", sourceKinds: ["session"], stages: undefined, priorities: undefined, projectPaths: ["D:/app"], due: "all" });
  });
});

describe("sortWorkItems", () => {
  const low = item({ id: "low", rank: 2, priority: "low", dueAt: "2026-03-01", updatedAt: "2026-01-01T00:00:00Z", title: "Zeta" });
  const urgent = item({ id: "urgent", rank: 1, priority: "urgent", dueAt: "2026-01-15", updatedAt: "2026-03-01T00:00:00Z", title: "Alpha" });
  const noDue = item({ id: "no-due", rank: 3, priority: "medium", dueAt: null, updatedAt: "2026-02-01T00:00:00Z", title: "Mid" });

  it("orders by rank for the manual sort, the drag-and-drop order", () => {
    expect(sortWorkItems([low, urgent, noDue], "manual").map((entry) => entry.id)).toEqual(["urgent", "low", "no-due"]);
  });

  it("orders by soonest due date first, sinking items with no due date to the end", () => {
    expect(sortWorkItems([low, urgent, noDue], "dueAt").map((entry) => entry.id)).toEqual(["urgent", "low", "no-due"]);
  });

  it("orders by highest priority first", () => {
    expect(sortWorkItems([low, urgent, noDue], "priority").map((entry) => entry.id)).toEqual(["urgent", "no-due", "low"]);
  });

  it("orders by most recently updated first", () => {
    expect(sortWorkItems([low, urgent, noDue], "updatedAt").map((entry) => entry.id)).toEqual(["urgent", "no-due", "low"]);
  });

  it("orders alphabetically by title", () => {
    expect(sortWorkItems([low, urgent, noDue], "title").map((entry) => entry.id)).toEqual(["urgent", "no-due", "low"]);
  });

  it("never mutates the input array", () => {
    const input = [low, urgent];
    sortWorkItems(input, "title");
    expect(input).toEqual([low, urgent]);
  });
});

describe("groupWorkItemsByStage", () => {
  it("groups items under their own stage and omits stages with no items", () => {
    const reviewItem = item({ id: "r", stage: "review" });
    const doneItem = item({ id: "d", stage: "done" });
    const groups = groupWorkItemsByStage([reviewItem, doneItem]);
    expect(groups.map((group) => group.stage)).toEqual(["review", "done"]);
    expect(groups.find((group) => group.stage === "review")?.items).toEqual([reviewItem]);
  });

  it("returns no groups at all for an empty list", () => {
    expect(groupWorkItemsByStage([])).toEqual([]);
  });
});
