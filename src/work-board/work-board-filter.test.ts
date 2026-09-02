import { describe, expect, it } from "vitest";
import type { WorkItem } from "../types/work-board";
import { filterWorkItems, matchesDueBucket } from "./work-board-filter";

const item = (overrides: Partial<WorkItem>): WorkItem => ({ id: "work", title: "Release", description: "Review build", stage: "review", priority: "high", rank: 1_000, projectPath: "D:/app", dueAt: null, archived: false, createdAt: "2026-01-01", updatedAt: "2026-01-01", sources: [], ...overrides });

describe("work board filters", () => {
  it("combines text, source, project, stage, priority, and archive filters", () => {
    const multiSource = item({ sources: [
      { sourceKind: "session", sourceId: "s", relation: "execution", title: "Session", status: "idle", available: true, projectPath: "D:/app", updatedAt: null },
      { sourceKind: "scheduled_task", sourceId: "task", relation: "automation", title: "Build", status: "idle", available: true, projectPath: "D:/app", updatedAt: null },
    ] });
    expect(filterWorkItems([multiSource], { query: "build", sourceKinds: ["scheduled_task"], projectPaths: ["D:/app"], stages: ["review"], priorities: ["high"] })).toEqual([multiSource]);
    expect(filterWorkItems([item({ sources: [multiSource.sources[0]] })], { sourceKinds: ["scheduled_task"] })).toEqual([]);
    expect(filterWorkItems([item({ archived: true })], { archived: true })).toHaveLength(1);
  });
});

describe("matchesDueBucket", () => {
  const today = new Date(2026, 5, 15);

  it("treats \"all\" as unconditionally matching, including items with no due date", () => {
    expect(matchesDueBucket(item({ dueAt: null }), "all", today)).toBe(true);
    expect(matchesDueBucket(item({ dueAt: "2026-06-01" }), "all", today)).toBe(true);
  });

  it("matches noDueDate only for items with no dueAt", () => {
    expect(matchesDueBucket(item({ dueAt: null }), "noDueDate", today)).toBe(true);
    expect(matchesDueBucket(item({ dueAt: "2026-06-15" }), "noDueDate", today)).toBe(false);
  });

  it("matches overdue for any due date strictly before today's local calendar day", () => {
    expect(matchesDueBucket(item({ dueAt: "2026-06-14" }), "overdue", today)).toBe(true);
    expect(matchesDueBucket(item({ dueAt: "2026-06-15" }), "overdue", today)).toBe(false);
    expect(matchesDueBucket(item({ dueAt: null }), "overdue", today)).toBe(false);
  });

  it("matches dueSoon from today through the 7-day window, inclusive on both ends", () => {
    expect(matchesDueBucket(item({ dueAt: "2026-06-15" }), "dueSoon", today)).toBe(true);
    expect(matchesDueBucket(item({ dueAt: "2026-06-22" }), "dueSoon", today)).toBe(true);
    expect(matchesDueBucket(item({ dueAt: "2026-06-23" }), "dueSoon", today)).toBe(false);
    expect(matchesDueBucket(item({ dueAt: "2026-06-14" }), "dueSoon", today)).toBe(false);
  });

  it("lets filterWorkItems narrow by due bucket the same way it narrows by every other dimension", () => {
    const overdue = item({ id: "overdue-item", dueAt: "2026-06-01" });
    const dueSoon = item({ id: "due-soon-item", dueAt: "2026-06-16" });
    const items = [overdue, dueSoon, item({ id: "no-due", dueAt: null })];
    expect(filterWorkItems(items, { due: "overdue", dueReferenceDate: today })).toEqual([overdue]);
    expect(filterWorkItems(items, { due: "dueSoon", dueReferenceDate: today })).toEqual([dueSoon]);
    expect(filterWorkItems(items, { due: "all", dueReferenceDate: today })).toHaveLength(3);
  });
});
