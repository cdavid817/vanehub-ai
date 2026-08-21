import { describe, expect, it } from "vitest";
import type { WorkItem } from "../types/work-board";
import { filterWorkItems } from "./work-board-filter";

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
