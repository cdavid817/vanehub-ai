import { describe, expect, it } from "vitest";
import parityFixture from "../../tests/fixtures/usage-statistics-parity.json";
import type { UsageStatisticsRange } from "../types/chat";
import { aggregateUsageRecords, localDateKey, type UsageRecord } from "./usage-statistics";

const now = new Date(2026, 6, 17, 12, 0, 0);

function record(overrides: Partial<UsageRecord> = {}): UsageRecord {
  return {
    messageId: "message-1",
    sessionId: "session-1",
    agentId: "codex-cli",
    accountingKind: "reported",
    inputCount: 100,
    outputCount: 40,
    cacheReadCount: 10,
    cacheCreationCount: 5,
    occurredAt: now.toISOString(),
    ...overrides,
  };
}

describe("aggregateUsageRecords", () => {
  it("keeps reported tokens and estimated characters in separate totals", () => {
    const result = aggregateUsageRecords([
      record(),
      record({
        messageId: "message-2",
        accountingKind: "estimated",
        inputCount: 1_000,
        outputCount: 400,
        cacheReadCount: 0,
        cacheCreationCount: 0,
      }),
    ], "all", now);

    expect(result.reported).toEqual({
      inputTokens: 100,
      outputTokens: 40,
      cacheReadTokens: 10,
      cacheCreationTokens: 5,
      totalTokens: 155,
    });
    expect(result.estimated).toEqual({
      inputCharacters: 1_000,
      outputCharacters: 400,
      totalCharacters: 1_400,
    });
    expect(result.coverage).toEqual({
      reportedResponses: 1,
      estimatedResponses: 1,
      totalResponses: 2,
      reportedPercent: 50,
    });
  });

  it("uses local calendar boundaries and ignores invalid timestamps", () => {
    const sixDaysAgo = new Date(2026, 6, 11, 0, 0, 0);
    const sevenDaysAgo = new Date(2026, 6, 10, 23, 59, 59);
    const result = aggregateUsageRecords([
      record({ messageId: "included", occurredAt: sixDaysAgo.toISOString() }),
      record({ messageId: "excluded", occurredAt: sevenDaysAgo.toISOString() }),
      record({ messageId: "invalid", occurredAt: "not-a-date" }),
    ], "last7Days", now);

    expect(result.coverage.totalResponses).toBe(1);
    expect(result.daily[0].date).toBe(localDateKey(sixDaysAgo));
  });

  it("groups by stable Agent id and counts distinct sessions", () => {
    const result = aggregateUsageRecords([
      record(),
      record({ messageId: "message-2", sessionId: "session-2", agentId: "claude-code" }),
      record({ messageId: "message-3", sessionId: "session-2", agentId: "claude-code" }),
    ], "all", now);

    expect(result.countedSessions).toBe(2);
    expect(result.byAgent.map((agent) => [agent.agentId, agent.responseCount])).toEqual([
      ["claude-code", 2],
      ["codex-cli", 1],
    ]);
  });

  it("returns a complete zero-value contract for empty ranges", () => {
    const result = aggregateUsageRecords([], "today", now);
    expect(result.reported.totalTokens).toBe(0);
    expect(result.estimated.totalCharacters).toBe(0);
    expect(result.coverage).toEqual({
      reportedResponses: 0,
      estimatedResponses: 0,
      totalResponses: 0,
      reportedPercent: 0,
    });
    expect(result.daily).toEqual([]);
    expect(result.byAgent).toEqual([]);
  });

  it("matches the desktop adapter contract for equivalent normalized fixtures", () => {
    const { generatedAt: _generatedAt, ...result } = aggregateUsageRecords(
      parityFixture.records as UsageRecord[],
      parityFixture.range as UsageStatisticsRange,
      new Date(parityFixture.now),
    );

    expect(result).toEqual(parityFixture.expected);
  });
});
