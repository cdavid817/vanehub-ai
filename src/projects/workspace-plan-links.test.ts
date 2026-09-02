import { describe, expect, it } from "vitest";
import type { Goal } from "../contracts/goal";
import type { WorkItem } from "../types/work-board";
import { selectRelatedGoals, selectRelatedWorkItems } from "./workspace-plan-links";

function workItem(overrides: Partial<WorkItem> = {}): WorkItem {
  return {
    archived: false, createdAt: "2026-08-01T00:00:00.000Z", description: "", dueAt: null,
    id: "item-1", priority: "none", projectPath: null, rank: 0, sources: [], stage: "inbox",
    title: "Item", updatedAt: "2026-08-01T00:00:00.000Z", ...overrides,
  };
}

function goal(overrides: Partial<Goal> = {}): Goal {
  return {
    acceptanceNotes: "", counted: 0, createdAt: "2026-08-01T00:00:00.000Z", derivedStatus: "active",
    description: "", id: "goal-1", links: [], projectPath: null, status: "active", terminal: 0,
    title: "Goal", unresolvable: 0, updatedAt: "2026-08-01T00:00:00.000Z", ...overrides,
  };
}

describe("selectRelatedWorkItems", () => {
  it("keeps only items whose projectPath exactly matches the workspace id", () => {
    const matching = workItem({ id: "match", projectPath: "D:\\repo\\app" });
    const other = workItem({ id: "other", projectPath: "D:\\repo\\other" });
    const none = workItem({ id: "none", projectPath: null });
    expect(selectRelatedWorkItems([matching, other, none], "D:\\repo\\app")).toEqual([matching]);
  });

  it("matches an ssh workspace id the same way, since projectPath is free text with no kind restriction", () => {
    const matching = workItem({ id: "match", projectPath: "ssh://vane@dev.example.com/work/app" });
    const local = workItem({ id: "local", projectPath: "D:\\repo\\app" });
    expect(selectRelatedWorkItems([matching, local], "ssh://vane@dev.example.com/work/app")).toEqual([matching]);
  });

  it("returns nothing when no item is related", () => {
    expect(selectRelatedWorkItems([workItem({ projectPath: null })], "D:\\repo\\app")).toEqual([]);
  });
});

describe("selectRelatedGoals", () => {
  it("keeps only goals whose projectPath exactly matches the workspace id", () => {
    const matching = goal({ id: "match", projectPath: "D:\\repo\\app" });
    const other = goal({ id: "other", projectPath: null });
    expect(selectRelatedGoals([matching, other], "D:\\repo\\app")).toEqual([matching]);
  });

  it("returns nothing when no goal is related", () => {
    expect(selectRelatedGoals([goal({ projectPath: "D:\\repo\\other" })], "D:\\repo\\app")).toEqual([]);
  });
});
