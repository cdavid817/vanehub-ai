import { QueryClient, QueryObserver } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";
import type { UsageStatistics } from "../../../types/chat";
import { preserveUsageData, usagePollingInterval } from "./usage-query";

const statistics: UsageStatistics = {
  range: "last30Days",
  reported: {
    inputTokens: 100,
    outputTokens: 40,
    cacheReadTokens: 10,
    cacheCreationTokens: 5,
    totalTokens: 155,
  },
  estimated: { inputCharacters: 200, outputCharacters: 80, totalCharacters: 280 },
  coverage: { reportedResponses: 1, estimatedResponses: 1, totalResponses: 2, reportedPercent: 50 },
  countedSessions: 2,
  daily: [],
  byAgent: [],
  generatedAt: "2026-07-17T04:00:00.000Z",
};

describe("usage query lifecycle", () => {
  it("preserves stale data across range changes and removes the observer on cleanup", () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const previousKey = ["usage-statistics", "last30Days"] as const;
    const nextKey = ["usage-statistics", "today"] as const;
    client.setQueryData(previousKey, statistics);
    const pending = new Promise<UsageStatistics>(() => undefined);
    const observer = new QueryObserver<UsageStatistics>(client, {
      queryKey: previousKey,
      queryFn: async () => statistics,
      placeholderData: preserveUsageData,
      refetchInterval: usagePollingInterval,
      staleTime: Number.POSITIVE_INFINITY,
    });
    const unsubscribe = observer.subscribe(() => undefined);

    observer.setOptions({
      queryKey: nextKey,
      queryFn: () => pending,
      placeholderData: preserveUsageData,
      refetchInterval: usagePollingInterval,
    });

    expect(observer.getCurrentResult().data).toBe(statistics);
    expect(observer.getCurrentResult().isPlaceholderData).toBe(true);
    expect(client.getQueryCache().find({ queryKey: nextKey })?.getObserversCount()).toBe(1);

    unsubscribe();
    expect(client.getQueryCache().find({ queryKey: nextKey })?.getObserversCount()).toBe(0);
    client.clear();
  });
});
