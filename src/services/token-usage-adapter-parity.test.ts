import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

import { tauriAgentClient } from "./tauri-agent-client";
import { webAgentClient } from "./web-agent-client";

describe("Token usage adapter parity", () => {
  beforeEach(() => invokeMock.mockReset());

  it("returns the same filtered summary contract from desktop and Web", async () => {
    const query = { agentId: "onepiece", quality: "reported" as const, breakdownLimit: 2 };
    const web = await webAgentClient.getTokenUsageSummary(query);
    invokeMock.mockResolvedValueOnce(web);

    await expect(tauriAgentClient.getTokenUsageSummary(query)).resolves.toEqual(web);
    expect(web.schemaVersion).toBe(1);
    expect(web.counts.calls).toBe(3);
    expect(web.breakdowns.every(({ entries }) => entries.length <= 2)).toBe(true);
    expect(web.breakdowns.find(({ dimension }) => dimension === "quality")?.entries)
      .toMatchObject([{ key: "reported" }]);
    expect(invokeMock).toHaveBeenCalledWith("get_token_usage_summary", { input: query });
  });

  it("keeps cursor pagination identical at the service boundary", async () => {
    const firstQuery = { limit: 2 };
    const firstWeb = await webAgentClient.getTokenUsageDetails(firstQuery);
    invokeMock.mockResolvedValueOnce(firstWeb);
    await expect(tauriAgentClient.getTokenUsageDetails(firstQuery)).resolves.toEqual(firstWeb);

    const secondQuery = { afterId: firstWeb.nextCursor ?? undefined, limit: 10 };
    const secondWeb = await webAgentClient.getTokenUsageDetails(secondQuery);
    invokeMock.mockResolvedValueOnce(secondWeb);
    await expect(tauriAgentClient.getTokenUsageDetails(secondQuery)).resolves.toEqual(secondWeb);
    expect(firstWeb.invocations).toHaveLength(2);
    expect(secondWeb.invocations).toHaveLength(4);
  });

  it("preserves empty ranges and propagates bounded query errors", async () => {
    const emptyQuery = { rangeStart: "2026-08-11T00:00:00.000Z" };
    const emptyWeb = await webAgentClient.getTokenUsageSummary(emptyQuery);
    invokeMock.mockResolvedValueOnce(emptyWeb);
    await expect(tauriAgentClient.getTokenUsageSummary(emptyQuery)).resolves.toEqual(emptyWeb);
    expect(emptyWeb.counts.calls).toBe(0);

    const error = new Error("session not found");
    invokeMock.mockRejectedValueOnce(error);
    await expect(tauriAgentClient.getTokenUsageSummary({ sessionId: "missing" }))
      .rejects.toThrow("session not found");
    await expect(webAgentClient.getTokenUsageSummary({ sessionId: "missing" }))
      .rejects.toThrow("Session not found");
  });
});
