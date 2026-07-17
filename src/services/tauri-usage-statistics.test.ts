import { describe, expect, it } from "vitest";
import type { UsageStatistics } from "../types/chat";
import { normalizeTauriUsageStatistics } from "./tauri-usage-statistics";

const modern: UsageStatistics = {
  range: "last30Days",
  reported: { inputTokens: 10, outputTokens: 5, cacheReadTokens: 2, cacheCreationTokens: 1, totalTokens: 18 },
  estimated: { inputCharacters: 0, outputCharacters: 0, totalCharacters: 0 },
  coverage: { reportedResponses: 1, estimatedResponses: 0, totalResponses: 1, reportedPercent: 100 },
  countedSessions: 1,
  daily: [],
  byAgent: [],
  generatedAt: "2026-07-18T00:00:00.000Z",
};

describe("normalizeTauriUsageStatistics", () => {
  it("preserves the current desktop response contract", () => {
    expect(normalizeTauriUsageStatistics(modern, "last30Days")).toBe(modern);
  });

  it("converts the legacy flat desktop response without leaving missing coverage", () => {
    const result = normalizeTauriUsageStatistics({
      range: "all",
      totalTokens: 42,
      inputTokens: 15,
      outputTokens: 27,
      countedMessages: 2,
      countedSessions: 1,
      generatedAt: "2026-07-18T00:00:00.000Z",
    }, "all");

    expect(result.reported).toMatchObject({ inputTokens: 15, outputTokens: 27, totalTokens: 42 });
    expect(result.coverage).toEqual({
      reportedResponses: 2,
      estimatedResponses: 0,
      totalResponses: 2,
      reportedPercent: 100,
    });
    expect(result.daily).toEqual([]);
    expect(result.byAgent).toEqual([]);
  });

  it("rejects malformed desktop responses so the query error UI can handle them", () => {
    expect(() => normalizeTauriUsageStatistics(undefined, "today")).toThrow(
      "invalid usage-statistics response",
    );
  });
});
