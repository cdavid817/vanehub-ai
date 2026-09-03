import { describe, expect, it } from "vitest";
import type { ScheduledTask } from "../types/agent";
import {
  defaultScheduledTaskFilterState, filterScheduledTasks, isScheduledTaskFilterActive, matchesNextRunRange,
  type ScheduledTaskFilterState,
} from "./scheduled-task-query";

function task(overrides: Partial<ScheduledTask> = {}): ScheduledTask {
  return {
    id: "t-1",
    name: "Weekly report",
    content: "Summarize the week's progress",
    agentId: "onepiece",
    frequency: { kind: "daily", timeOfDay: "09:00" },
    enabled: true,
    nextRunAt: "2026-06-15T09:00:00.000Z",
    latestStatus: "never-run",
    latestRunAt: null,
    latestRunSessionId: null,
    latestError: null,
    createdAt: "2026-06-01T00:00:00.000Z",
    updatedAt: "2026-06-01T00:00:00.000Z",
    version: 1,
    ...overrides,
  };
}

const now = new Date("2026-06-15T09:00:00.000Z");

describe("matchesNextRunRange", () => {
  it("treats \"all\" as unconditionally matching, regardless of nextRunAt", () => {
    expect(matchesNextRunRange(task({ nextRunAt: "2020-01-01T00:00:00.000Z" }), "all", now)).toBe(true);
    expect(matchesNextRunRange(task({ nextRunAt: "2099-01-01T00:00:00.000Z" }), "all", now)).toBe(true);
  });

  it("matches overdue only for a nextRunAt strictly before the reference instant", () => {
    expect(matchesNextRunRange(task({ nextRunAt: "2026-06-15T08:59:59.000Z" }), "overdue", now)).toBe(true);
    expect(matchesNextRunRange(task({ nextRunAt: "2026-06-15T09:00:00.000Z" }), "overdue", now)).toBe(false);
    expect(matchesNextRunRange(task({ nextRunAt: "2026-06-15T09:00:01.000Z" }), "overdue", now)).toBe(false);
  });

  it("matches next24h inclusively from now through +24 hours", () => {
    expect(matchesNextRunRange(task({ nextRunAt: "2026-06-15T09:00:00.000Z" }), "next24h", now)).toBe(true);
    expect(matchesNextRunRange(task({ nextRunAt: "2026-06-16T09:00:00.000Z" }), "next24h", now)).toBe(true);
    expect(matchesNextRunRange(task({ nextRunAt: "2026-06-16T09:00:01.000Z" }), "next24h", now)).toBe(false);
    expect(matchesNextRunRange(task({ nextRunAt: "2026-06-15T08:59:59.000Z" }), "next24h", now)).toBe(false);
  });

  it("matches next7d inclusively from now through +7 days", () => {
    expect(matchesNextRunRange(task({ nextRunAt: "2026-06-20T09:00:00.000Z" }), "next7d", now)).toBe(true);
    expect(matchesNextRunRange(task({ nextRunAt: "2026-06-22T09:00:00.000Z" }), "next7d", now)).toBe(true);
    expect(matchesNextRunRange(task({ nextRunAt: "2026-06-22T09:00:01.000Z" }), "next7d", now)).toBe(false);
  });

  it("matches later only strictly beyond the 7-day window", () => {
    expect(matchesNextRunRange(task({ nextRunAt: "2026-06-22T09:00:01.000Z" }), "later", now)).toBe(true);
    expect(matchesNextRunRange(task({ nextRunAt: "2026-06-22T09:00:00.000Z" }), "later", now)).toBe(false);
    expect(matchesNextRunRange(task({ nextRunAt: "2026-06-15T08:00:00.000Z" }), "later", now)).toBe(false);
  });
});

describe("isScheduledTaskFilterActive", () => {
  it("is false for the default state and true for whitespace-only search", () => {
    expect(isScheduledTaskFilterActive(defaultScheduledTaskFilterState)).toBe(false);
    expect(isScheduledTaskFilterActive({ ...defaultScheduledTaskFilterState, search: "   " })).toBe(false);
  });

  it("is true when any single narrowing dimension moves off its default", () => {
    const cases: Partial<ScheduledTaskFilterState>[] = [
      { search: "report" },
      { agentId: "onepiece" },
      { frequencyKind: "weekly" },
      { enabled: "true" },
      { status: "failed" },
      { nextRunRange: "overdue" },
    ];
    for (const patch of cases) {
      expect(isScheduledTaskFilterActive({ ...defaultScheduledTaskFilterState, ...patch })).toBe(true);
    }
  });
});

describe("filterScheduledTasks", () => {
  it("matches search case-insensitively against name or content, scoped to the already-fetched list", () => {
    const tasks = [task({ id: "a", name: "Weekly report" }), task({ id: "b", name: "Nightly digest", content: "Send the REPORT summary" }), task({ id: "c", name: "Unrelated" })];
    const result = filterScheduledTasks(tasks, { ...defaultScheduledTaskFilterState, search: "report" });
    expect(result.map((item) => item.id)).toEqual(["a", "b"]);
  });

  it("filters by exact agentId", () => {
    const tasks = [task({ id: "a", agentId: "onepiece" }), task({ id: "b", agentId: "claude-code" })];
    expect(filterScheduledTasks(tasks, { ...defaultScheduledTaskFilterState, agentId: "claude-code" }).map((item) => item.id)).toEqual(["b"]);
  });

  it("filters by frequency kind", () => {
    const tasks = [task({ id: "a", frequency: { kind: "daily", timeOfDay: "09:00" } }), task({ id: "b", frequency: { kind: "weekly", weekday: 1, timeOfDay: "09:00" } })];
    expect(filterScheduledTasks(tasks, { ...defaultScheduledTaskFilterState, frequencyKind: "weekly" }).map((item) => item.id)).toEqual(["b"]);
  });

  it("filters by enabled state", () => {
    const tasks = [task({ id: "a", enabled: true }), task({ id: "b", enabled: false })];
    expect(filterScheduledTasks(tasks, { ...defaultScheduledTaskFilterState, enabled: "false" }).map((item) => item.id)).toEqual(["b"]);
    expect(filterScheduledTasks(tasks, { ...defaultScheduledTaskFilterState, enabled: "true" }).map((item) => item.id)).toEqual(["a"]);
  });

  it("filters by latest status", () => {
    const tasks = [task({ id: "a", latestStatus: "succeeded" }), task({ id: "b", latestStatus: "failed" })];
    expect(filterScheduledTasks(tasks, { ...defaultScheduledTaskFilterState, status: "failed" }).map((item) => item.id)).toEqual(["b"]);
  });

  it("filters by next-run range using the same referenceDate parameter as matchesNextRunRange", () => {
    const tasks = [task({ id: "a", nextRunAt: "2026-06-15T08:00:00.000Z" }), task({ id: "b", nextRunAt: "2026-06-20T00:00:00.000Z" })];
    expect(filterScheduledTasks(tasks, { ...defaultScheduledTaskFilterState, nextRunRange: "overdue" }, now).map((item) => item.id)).toEqual(["a"]);
  });

  it("combines every dimension with AND semantics", () => {
    const tasks = [
      task({ id: "match", name: "Weekly report", agentId: "onepiece", enabled: true, latestStatus: "succeeded" }),
      task({ id: "wrong-agent", name: "Weekly report", agentId: "claude-code", enabled: true, latestStatus: "succeeded" }),
      task({ id: "wrong-enabled", name: "Weekly report", agentId: "onepiece", enabled: false, latestStatus: "succeeded" }),
    ];
    const filter: ScheduledTaskFilterState = { ...defaultScheduledTaskFilterState, search: "weekly", agentId: "onepiece", enabled: "true", status: "succeeded" };
    expect(filterScheduledTasks(tasks, filter).map((item) => item.id)).toEqual(["match"]);
  });

  it("returns every task unchanged under the default (no-op) filter", () => {
    const tasks = [task({ id: "a" }), task({ id: "b" })];
    expect(filterScheduledTasks(tasks, defaultScheduledTaskFilterState)).toEqual(tasks);
  });
});
