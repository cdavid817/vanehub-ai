import { describe, expect, it } from "vitest";
import { webAgentClient } from "./web-agent-client";

describe("web Mission Control adapter", () => {
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

  it("rejects invalid cursors and stale or unsupported mutations", async () => {
    await expect(webAgentClient.getMissionControlOverview({ cursor: "../secret" })).rejects.toThrow("invalid mission control cursor");
    const overview = await webAgentClient.getMissionControlOverview({ states: ["completed"] });
    const run = overview.recent.items[0];
    await expect(webAgentClient.performMissionControlAction({ runId: run.runId, version: run.version - 1, action: "verify" })).rejects.toThrow("conflict");
  });
});
