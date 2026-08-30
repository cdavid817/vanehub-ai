import { afterEach, describe, expect, it, vi } from "vitest";
import { agentService } from "../services/runtime-agent-client";
import type { MissionControlOverview, MissionControlRunSummary } from "../types/mission-control";
import { runSearchProvider } from "./run-search-provider";

afterEach(() => vi.restoreAllMocks());

function run(overrides: Partial<MissionControlRunSummary> = {}): MissionControlRunSummary {
  return {
    runId: "run-1",
    version: 1,
    ownerType: "session",
    ownerId: "session-1",
    agentId: "claude-code",
    title: "Fix null auth token",
    state: "running",
    createdAt: "2026-08-14T00:00:00.000Z",
    updatedAt: "2026-08-14T01:00:00.000Z",
    endedAt: null,
    projectId: null,
    workspace: null,
    phase: null,
    attention: null,
    reasonCode: null,
    verification: "unavailable",
    tokens: null,
    cost: null,
    actions: ["open"],
    navigation: null,
    runner: null,
    ...overrides,
  };
}

function overview(sections: Partial<{ attention: MissionControlRunSummary[]; active: MissionControlRunSummary[]; recent: MissionControlRunSummary[] }>): MissionControlOverview {
  return {
    counts: { running: 0, waitingApproval: 0, waitingUser: 0, retrying: 0, blocked: 0, failed: 0, completedRecently: 0 },
    attention: { items: sections.attention ?? [], nextCursor: null },
    active: { items: sections.active ?? [], nextCursor: null },
    recent: { items: sections.recent ?? [], nextCursor: null },
  };
}

function searchRequest(overrides: Partial<{ query: string; limit: number }> = {}) {
  return { query: "auth", scopes: ["run" as const], limit: 20, signal: new AbortController().signal, ...overrides };
}

describe("runSearchProvider", () => {
  it("supports only the run scope", () => {
    expect(runSearchProvider.supports("run")).toBe(true);
    expect(runSearchProvider.supports("session")).toBe(false);
    expect(runSearchProvider.supports("project")).toBe(false);
  });

  it("maps title, route, and updatedAt", async () => {
    vi.spyOn(agentService, "getMissionControlOverview").mockResolvedValue(overview({ active: [run()] }));
    const page = await runSearchProvider.search(searchRequest());
    expect(page.nextCursor).toBeNull();
    expect(page.items).toEqual([{
      key: "run-1",
      kind: "run",
      title: "Fix null auth token",
      status: "active",
      route: { destination: "runs", section: "attention", runId: "run-1" },
      updatedAt: "2026-08-14T01:00:00.000Z",
    }]);
  });

  it.each([
    [{ state: "created" as const }, "neutral"],
    [{ state: "preparing" as const }, "active"],
    [{ state: "running" as const }, "active"],
    [{ state: "verifying" as const }, "active"],
    [{ state: "retrying" as const }, "active"],
    [{ state: "completed" as const }, "success"],
    [{ state: "cancelled" as const }, "neutral"],
    [{ attention: "approval" as const }, "attention"],
    [{ attention: "stuck" as const }, "attention"],
    [{ attention: "failed" as const }, "error"],
  ] as const)("maps %o to status %s", async (overridesToApply, status) => {
    vi.spyOn(agentService, "getMissionControlOverview").mockResolvedValue(overview({ active: [run(overridesToApply)] }));
    const page = await runSearchProvider.search(searchRequest());
    expect(page.items[0].status).toBe(status);
  });

  it("filters by a case-insensitive substring of the title", async () => {
    vi.spyOn(agentService, "getMissionControlOverview").mockResolvedValue(overview({
      active: [run({ runId: "run-1", title: "Fix null auth token" }), run({ runId: "run-2", title: "Refactor search" })],
    }));
    const page = await runSearchProvider.search(searchRequest({ query: "AUTH" }));
    expect(page.items.map((item) => item.key)).toEqual(["run-1"]);
  });

  it("dedupes a run that appears in more than one section", async () => {
    const duplicated = run({ runId: "run-1" });
    vi.spyOn(agentService, "getMissionControlOverview").mockResolvedValue(overview({
      attention: [duplicated],
      active: [duplicated],
    }));
    const page = await runSearchProvider.search(searchRequest());
    expect(page.items).toHaveLength(1);
  });

  it("respects the requested limit across the combined, deduped pool", async () => {
    vi.spyOn(agentService, "getMissionControlOverview").mockResolvedValue(overview({
      active: [run({ runId: "run-1" }), run({ runId: "run-2" }), run({ runId: "run-3" })],
    }));
    const page = await runSearchProvider.search(searchRequest({ limit: 2 }));
    expect(page.items).toHaveLength(2);
  });

  it("returns an empty page when nothing matches", async () => {
    vi.spyOn(agentService, "getMissionControlOverview").mockResolvedValue(overview({ active: [run({ title: "Refactor search" })] }));
    const page = await runSearchProvider.search(searchRequest({ query: "auth" }));
    expect(page.items).toEqual([]);
  });

  it("never surfaces reasonCode text anywhere in the result", async () => {
    vi.spyOn(agentService, "getMissionControlOverview").mockResolvedValue(overview({
      active: [run({ reasonCode: "SECRET_REASON_CODE should never leak" })],
    }));
    const page = await runSearchProvider.search(searchRequest());
    expect(JSON.stringify(page)).not.toContain("SECRET_REASON_CODE");
  });
});
