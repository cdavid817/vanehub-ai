import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { readFileSync } from "node:fs";
import { renderToString } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../../i18n";
import type { TokenUsageSummary, UsageMeasure, UsageQualityTotals } from "../../types/token-usage";
import { UsageStatisticsPage } from "./usage-statistics-page";
import { UsageAccountingNote } from "./usage/usage-accounting-note";
import { UsageBreakdowns } from "./usage/usage-breakdowns";
import { UsageControls } from "./usage/usage-controls";
import { preserveUsageData, usagePollingInterval, usageRefetchInterval } from "./usage/usage-query";
import { UsageLoadError } from "./usage/usage-status";
import { UsageSummary } from "./usage/usage-summary";

function measure(unit: "tokens" | "characters", headlineTotal: number, callCount: number): UsageMeasure {
  return { unit, dimensions: { input: 100, output: 50, cachedInput: 20, cacheWriteInput: 10, reasoningOutput: 5, providerTotal: headlineTotal }, headlineTotal, callCount, observationCount: callCount };
}

function quality(reported = 180, derived = 20, estimated = 1_500): UsageQualityTotals {
  return { reported: measure("tokens", reported, reported > 0 ? 1 : 0), reportedDerived: measure("tokens", derived, derived > 0 ? 1 : 0), estimated: measure("characters", estimated, estimated > 0 ? 1 : 0) };
}

const statistics: TokenUsageSummary = {
  schemaVersion: 1,
  totals: quality(),
  userResponse: quality(160, 20, 1_500),
  internal: quality(20, 0, 0),
  counts: { calls: 3, generations: 2, sessions: 2 },
  daily: [],
  breakdowns: [{ dimension: "agent", entries: [{ key: "codex-cli", totals: quality(), counts: { calls: 3, generations: 2, sessions: 2 } }] }],
  generatedAt: "2026-07-17T04:00:00.000Z",
};

describe("UsageStatisticsPage", () => {
  it("renders localized responsive controls and loading state", () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const html = renderToString(<QueryClientProvider client={queryClient}><UsageStatisticsPage /></QueryClientProvider>);
    expect(html).toContain("使用统计");
    expect(html).toContain("近 30 天");
    expect(html).toContain("加载中");
    expect(html).toContain("sm:grid-cols-2");
  });

  it("renders reported, derived, estimated, internal, and coverage values without mixing units", () => {
    const html = renderToString(<UsageSummary language="zh-CN" loading={false} stats={statistics} />);
    expect(html).toContain("真实总 Token");
    expect(html).toContain("推导 Token");
    expect(html).toContain("估算总字符");
    expect(html).toContain("内部用途 Token");
    expect(html).toContain("33.3%（1 / 3）");
  });

  it("keeps stable ids and separate units in bounded breakdowns", () => {
    const html = renderToString(<UsageBreakdowns breakdowns={statistics.breakdowns} language="zh-CN" />);
    expect(html).toContain("codex-cli");
    expect(html).toContain("Token");
    expect(html).toContain("字符");
    expect(html).toContain("3 次调用");
  });

  it("localizes unknown dimensions without theme-specific rendering branches", () => {
    const unknown = [{
      dimension: "provider" as const,
      entries: [{ key: "unknown", totals: quality(), counts: { calls: 1, generations: 1, sessions: 1 } }],
    }];
    const html = renderToString(<UsageBreakdowns breakdowns={unknown} language="zh-CN" />);
    const source = ["./usage-statistics-page.tsx", "./usage/usage-breakdowns.tsx", "./usage/usage-summary.tsx"]
      .map((path) => readFileSync(new URL(path, import.meta.url), "utf8"))
      .join("\n");

    expect(html).toContain("未知");
    expect(source).toContain("ucd-panel");
    expect(source).not.toMatch(/theme\s*===\s*["'](?:minimal|futuristic)/);
  });

  it("marks range and filters as accessible controls", () => {
    const html = renderToString(<UsageControls filters={{ quality: "reported" }} isFetching onFiltersChange={vi.fn()} onRangeChange={vi.fn()} onRefresh={vi.fn()} options={{ agents: ["codex-cli"], providers: ["openai"], models: ["gpt-5"] }} range="today" />);
    expect(html).toContain('aria-pressed="true"');
    expect(html).toContain("刷新中");
    expect(html).toContain("重置筛选");
    expect(html).toContain("focus-visible:ring-2");
  });

  it("renders errors and preserves prior ledger data during polling refresh", () => {
    const html = renderToString(<UsageLoadError error={new Error("offline")} />);
    expect(html).toContain("使用统计加载失败：offline");
    expect(preserveUsageData(statistics)).toBe(statistics);
    expect(preserveUsageData(undefined)).toBeUndefined();
    expect(usageRefetchInterval(true)).toBe(usagePollingInterval);
    expect(usageRefetchInterval(false)).toBe(false);
  });

  it("states quality and non-billing boundaries in both locales", async () => {
    const zhHtml = renderToString(<UsageAccountingNote language="zh-CN" />);
    expect(zhHtml).toContain("推导 Token");
    expect(zhHtml).toContain("供应商账单对账");
    expect(zhHtml).toContain("不支持的数据源");
    await activateAppLanguage("en");
    expect(i18n.t("usage.accounting.limitations")).toContain("not provider billing reconciliation");
    expect(i18n.t("usage.accounting.limitations")).toContain("unsupported sources");
    await activateAppLanguage("zh-CN");
  });
});
