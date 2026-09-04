import { afterEach, describe, expect, it, vi } from "vitest";
import { goalService } from "../services/runtime-goal-client";
import type { Goal } from "../contracts/goal";
import { goalSearchProvider } from "./goal-search-provider";

afterEach(() => vi.restoreAllMocks());

function goal(overrides: Partial<Goal> = {}): Goal {
  return {
    id: "goal-1",
    title: "Ship the auth redesign",
    description: "SECRET_DESCRIPTION should never leak",
    acceptanceNotes: "SECRET_ACCEPTANCE_NOTES should never leak",
    status: "active",
    derivedStatus: "active",
    projectPath: null,
    createdAt: "2026-08-14T00:00:00.000Z",
    updatedAt: "2026-08-14T01:00:00.000Z",
    counted: 2,
    terminal: 1,
    unresolvable: 0,
    links: [{ targetKind: "work_item", targetId: "SECRET_TARGET_ID should never leak", progress: "active" }],
    ...overrides,
  };
}

function searchRequest(overrides: Partial<{ query: string; limit: number }> = {}) {
  return { query: "auth", scopes: ["goal" as const], limit: 20, signal: new AbortController().signal, ...overrides };
}

describe("goalSearchProvider", () => {
  it("supports only the goal scope", () => {
    expect(goalSearchProvider.supports("goal")).toBe(true);
    expect(goalSearchProvider.supports("work-item")).toBe(false);
    expect(goalSearchProvider.supports("evaluation")).toBe(false);
  });

  it("maps title, route, and updatedAt", async () => {
    vi.spyOn(goalService, "listGoals").mockResolvedValue([goal()]);
    const page = await goalSearchProvider.search(searchRequest());
    expect(page.nextCursor).toBeNull();
    expect(page.items).toEqual([{
      key: "goal-1",
      kind: "goal",
      title: "Ship the auth redesign",
      status: "active",
      route: { destination: "plan", section: "goals", goalId: "goal-1" },
      updatedAt: "2026-08-14T01:00:00.000Z",
    }]);
  });

  it.each([
    ["draft" as const, "neutral"],
    ["active" as const, "active"],
    ["awaiting_acceptance" as const, "attention"],
    ["achieved" as const, "success"],
    ["abandoned" as const, "neutral"],
  ] as const)("maps derivedStatus %s to status %s", async (derivedStatus, status) => {
    vi.spyOn(goalService, "listGoals").mockResolvedValue([goal({ derivedStatus })]);
    const page = await goalSearchProvider.search(searchRequest());
    expect(page.items[0].status).toBe(status);
  });

  it("filters by a case-insensitive substring of the title", async () => {
    vi.spyOn(goalService, "listGoals").mockResolvedValue([
      goal({ id: "goal-1", title: "Ship the auth redesign" }),
      goal({ id: "goal-2", title: "Refactor search" }),
    ]);
    const page = await goalSearchProvider.search(searchRequest({ query: "AUTH" }));
    expect(page.items.map((item) => item.key)).toEqual(["goal-1"]);
  });

  it("respects the requested limit", async () => {
    vi.spyOn(goalService, "listGoals").mockResolvedValue([
      goal({ id: "goal-1" }),
      goal({ id: "goal-2" }),
      goal({ id: "goal-3" }),
    ]);
    const page = await goalSearchProvider.search(searchRequest({ query: "", limit: 2 }));
    expect(page.items).toHaveLength(2);
  });

  it("returns an empty page when nothing matches", async () => {
    vi.spyOn(goalService, "listGoals").mockResolvedValue([goal({ title: "Refactor search" })]);
    const page = await goalSearchProvider.search(searchRequest({ query: "auth" }));
    expect(page.items).toEqual([]);
  });

  it("never surfaces description, acceptanceNotes, or link target ids in the result", async () => {
    vi.spyOn(goalService, "listGoals").mockResolvedValue([goal()]);
    const page = await goalSearchProvider.search(searchRequest());
    const serialized = JSON.stringify(page);
    expect(serialized).not.toContain("SECRET_DESCRIPTION");
    expect(serialized).not.toContain("SECRET_ACCEPTANCE_NOTES");
    expect(serialized).not.toContain("SECRET_TARGET_ID");
  });
});
