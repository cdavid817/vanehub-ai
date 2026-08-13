import { QueryClient, QueryObserver } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";
import type { TokenUsageSummary } from "../../../types/token-usage";
import { preserveUsageData, usagePollingInterval } from "./usage-query";

const statistics: TokenUsageSummary = {
  schemaVersion: 1,
  totals: { reported: { unit: "tokens", dimensions: { input: 1, output: 1, cachedInput: 0, cacheWriteInput: 0, reasoningOutput: 0, providerTotal: 2 }, headlineTotal: 2, callCount: 1, observationCount: 1 }, reportedDerived: { unit: "tokens", dimensions: { input: 0, output: 0, cachedInput: 0, cacheWriteInput: 0, reasoningOutput: 0, providerTotal: 0 }, headlineTotal: 0, callCount: 0, observationCount: 0 }, estimated: { unit: "characters", dimensions: { input: 0, output: 0, cachedInput: 0, cacheWriteInput: 0, reasoningOutput: 0, providerTotal: 0 }, headlineTotal: 0, callCount: 0, observationCount: 0 } },
  userResponse: { reported: { unit: "tokens", dimensions: { input: 1, output: 1, cachedInput: 0, cacheWriteInput: 0, reasoningOutput: 0, providerTotal: 2 }, headlineTotal: 2, callCount: 1, observationCount: 1 }, reportedDerived: { unit: "tokens", dimensions: { input: 0, output: 0, cachedInput: 0, cacheWriteInput: 0, reasoningOutput: 0, providerTotal: 0 }, headlineTotal: 0, callCount: 0, observationCount: 0 }, estimated: { unit: "characters", dimensions: { input: 0, output: 0, cachedInput: 0, cacheWriteInput: 0, reasoningOutput: 0, providerTotal: 0 }, headlineTotal: 0, callCount: 0, observationCount: 0 } },
  internal: { reported: { unit: "tokens", dimensions: { input: 0, output: 0, cachedInput: 0, cacheWriteInput: 0, reasoningOutput: 0, providerTotal: 0 }, headlineTotal: 0, callCount: 0, observationCount: 0 }, reportedDerived: { unit: "tokens", dimensions: { input: 0, output: 0, cachedInput: 0, cacheWriteInput: 0, reasoningOutput: 0, providerTotal: 0 }, headlineTotal: 0, callCount: 0, observationCount: 0 }, estimated: { unit: "characters", dimensions: { input: 0, output: 0, cachedInput: 0, cacheWriteInput: 0, reasoningOutput: 0, providerTotal: 0 }, headlineTotal: 0, callCount: 0, observationCount: 0 } },
  counts: { calls: 1, generations: 1, sessions: 1 }, daily: [], breakdowns: [], generatedAt: "2026-07-17T04:00:00.000Z",
};

describe("usage query lifecycle", () => {
  it("preserves stale data across filter changes and removes the observer on cleanup", () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const previousKey = ["token-usage-summary", "last30Days", {}] as const;
    const nextKey = ["token-usage-summary", "last30Days", { quality: "reported" }] as const;
    client.setQueryData(previousKey, statistics);
    const pending = new Promise<TokenUsageSummary>(() => undefined);
    const observer = new QueryObserver<TokenUsageSummary>(client, { queryKey: previousKey, queryFn: async () => statistics, placeholderData: preserveUsageData, refetchInterval: usagePollingInterval, staleTime: Number.POSITIVE_INFINITY });
    const unsubscribe = observer.subscribe(() => undefined);
    observer.setOptions({ queryKey: nextKey, queryFn: () => pending, placeholderData: preserveUsageData, refetchInterval: usagePollingInterval });
    expect(observer.getCurrentResult().data).toBe(statistics);
    expect(observer.getCurrentResult().isPlaceholderData).toBe(true);
    unsubscribe();
    expect(client.getQueryCache().find({ queryKey: nextKey })?.getObserversCount()).toBe(0);
    client.clear();
  });
});
