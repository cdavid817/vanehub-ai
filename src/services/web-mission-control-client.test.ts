import { afterEach, describe, expect, it } from "vitest";
import {
  resetWebMissionControlRunsForTest,
  seedWebMissionControlRunsForTest,
  webAgentClient,
} from "./web-agent-client";

afterEach(resetWebMissionControlRunsForTest);

describe("web Mission Control adapter", () => {
  for (const runCount of [100, 1_000] as const) {
    it(`keeps the ${runCount}-Run fixture page-bounded and detail-lazy`, async () => {
      seedWebMissionControlRunsForTest(runCount);

      const first = await webAgentClient.getMissionControlOverview({ limit: 50, sort: "attention" });
      const repeated = await webAgentClient.getMissionControlOverview({ limit: 50, sort: "attention" });
      const total = Object.values(first.counts).reduce((sum, count) => sum + count, 0);

      expect(total).toBe(runCount);
      expect(repeated).toEqual(first);
      expect(first.attention.items.length).toBeLessThanOrEqual(50);
      expect(first.active.items.length).toBeLessThanOrEqual(50);
      expect(first.recent.items.length).toBeLessThanOrEqual(50);
      expect(first.active.nextCursor).not.toBeNull();
      expect(first.active.items[0]).not.toHaveProperty("facets");

      const detail = await webAgentClient.getMissionControlRun(first.active.items[0].runId);
      expect(detail.facets).toHaveLength(10);
    });
  }

  it("provides deterministic bounded attention, filters, detail availability, and safe missing metrics", async () => {
    const first = await webAgentClient.getMissionControlOverview({ limit: 2, sort: "attention" });
    const second = await webAgentClient.getMissionControlOverview({ limit: 2, sort: "attention" });
    expect(second).toEqual(first); expect(first.attention.items).toHaveLength(2); expect(first.attention.nextCursor).toBe("2");
    expect(first.attention.items[0].tokens).toBeNull(); expect(first.attention.items[0].cost).toBeNull();
    const failed = await webAgentClient.getMissionControlOverview({ states: ["failed"] });
    expect(failed.recent.items.every((run) => run.state === "failed")).toBe(true);
    const detail = await webAgentClient.getMissionControlRun(first.attention.items[0].runId);
    expect(detail.facets.find((facet) => facet.facet === "overview")?.state).toBe("available");
    expect(detail.facets.some((facet) => facet.state === "unavailable")).toBe(true);
  });

  it("normalizes legacy missing Runner metadata and filters explicit Runner metadata", async () => {
    const legacy = await webAgentClient.getMissionControlOverview({ runner: "local" });
    expect(legacy.active.items.every((run) => run.runner?.kind === "local")).toBe(true);
    const overview = await webAgentClient.getMissionControlOverview();
    expect(overview.active.items.some((run) => run.runner === null)).toBe(true);
    expect(overview.active.items.some((run) => run.runner?.kind === "ssh")).toBe(true);
  });

  it("rejects invalid cursors and stale or unsupported mutations", async () => {
    await expect(webAgentClient.getMissionControlOverview({ cursor: "../secret" })).rejects.toThrow("invalid mission control cursor");
    const overview = await webAgentClient.getMissionControlOverview({ states: ["completed"] });
    const run = overview.recent.items[0];
    await expect(webAgentClient.performMissionControlAction({ runId: run.runId, version: run.version - 1, action: "verify" })).rejects.toThrow("conflict");
  });
});
