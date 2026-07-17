import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { i18n } from "../../i18n";
import type { UsageStatistics } from "../../types/chat";
import { UsageStatisticsPage } from "./usage-statistics-page";
import { UsageAgentBreakdown } from "./usage/usage-agent-breakdown";
import { UsageAccountingNote } from "./usage/usage-accounting-note";
import { UsageControls } from "./usage/usage-controls";
import { preserveUsageData, usagePollingInterval } from "./usage/usage-query";
import { UsageLoadError } from "./usage/usage-status";
import { UsageSummary } from "./usage/usage-summary";

const statistics: UsageStatistics = {
  range: "last30Days",
  reported: {
    inputTokens: 100,
    outputTokens: 50,
    cacheReadTokens: 20,
    cacheCreationTokens: 10,
    totalTokens: 180,
  },
  estimated: { inputCharacters: 1_000, outputCharacters: 500, totalCharacters: 1_500 },
  coverage: { reportedResponses: 1, estimatedResponses: 1, totalResponses: 2, reportedPercent: 50 },
  countedSessions: 2,
  daily: [],
  byAgent: [],
  generatedAt: "2026-07-17T04:00:00.000Z",
};

describe("UsageStatisticsPage", () => {
  it("renders localized controls and loading state", () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const html = renderToString(
      <QueryClientProvider client={queryClient}>
        <UsageStatisticsPage />
      </QueryClientProvider>,
    );
    expect(html).toContain("使用统计");
    expect(html).toContain("近 30 天");
    expect(html).toContain("加载中");
  });

  it("renders reported, estimated, and coverage values without mixing units", () => {
    const html = renderToString(<UsageSummary language="zh-CN" loading={false} stats={statistics} />);
    expect(html).toContain("真实总 Token");
    expect(html).toContain("180");
    expect(html).toContain("估算总字符");
    expect(html).toContain("1,500");
    expect(html).toContain("50%（1 / 2）");
  });

  it("renders reported-only and estimated-only accounting states", () => {
    const reportedOnly = {
      ...statistics,
      estimated: { inputCharacters: 0, outputCharacters: 0, totalCharacters: 0 },
      coverage: { reportedResponses: 1, estimatedResponses: 0, totalResponses: 1, reportedPercent: 100 },
    };
    const estimatedOnly = {
      ...statistics,
      reported: { inputTokens: 0, outputTokens: 0, cacheReadTokens: 0, cacheCreationTokens: 0, totalTokens: 0 },
      coverage: { reportedResponses: 0, estimatedResponses: 1, totalResponses: 1, reportedPercent: 0 },
    };
    const reportedHtml = renderToString(<UsageSummary language="zh-CN" loading={false} stats={reportedOnly} />);
    const estimatedHtml = renderToString(<UsageSummary language="zh-CN" loading={false} stats={estimatedOnly} />);
    expect(reportedHtml).toContain("100%（1 / 1）");
    expect(estimatedHtml).toContain("1,500");
    expect(estimatedHtml).toContain("0%（0 / 1）");
  });

  it("renders zero values for empty statistics", () => {
    const empty = {
      ...statistics,
      reported: { ...statistics.reported, inputTokens: 0, outputTokens: 0, cacheReadTokens: 0, cacheCreationTokens: 0, totalTokens: 0 },
      estimated: { inputCharacters: 0, outputCharacters: 0, totalCharacters: 0 },
      coverage: { reportedResponses: 0, estimatedResponses: 0, totalResponses: 0, reportedPercent: 0 },
    };
    const html = renderToString(<UsageSummary language="en" loading={false} stats={empty} />);
    expect(html).toContain("真实总 Token");
    expect(html).toContain("0%（0 / 0）");
  });

  it("keeps the stable Agent id visible", () => {
    const html = renderToString(
      <UsageAgentBreakdown
        agents={[{ agentId: "codex-cli", reported: statistics.reported, estimated: statistics.estimated, responseCount: 2 }]}
        language="en"
      />,
    );
    expect(html).toContain("codex-cli");
    expect(html).toContain("2 条响应");
  });

  it("marks the selected range and exposes refresh state", () => {
    const html = renderToString(
      <UsageControls
        isFetching
        onRangeChange={vi.fn()}
        onRefresh={vi.fn()}
        range="today"
      />,
    );
    expect(html).toContain('aria-pressed="true"');
    expect(html).toContain("刷新中");
  });

  it("renders localized errors and preserves prior data during polling refresh", () => {
    const html = renderToString(<UsageLoadError error={new Error("offline")} />);
    expect(html).toContain("使用统计加载失败：offline");
    expect(preserveUsageData(statistics)).toBe(statistics);
    expect(preserveUsageData(undefined)).toBeUndefined();
    expect(usagePollingInterval).toBe(30_000);
  });

  it("states every accounting boundary in both locales", () => {
    const zhHtml = renderToString(<UsageAccountingNote language="zh-CN" />);
    expect(zhHtml).toContain("外部 CLI 历史");
    expect(zhHtml).toContain("供应商账单对账");
    expect(zhHtml).toContain("费用估算");
    expect(zhHtml).toContain("请求详情日志");
    expect(zhHtml).toContain("Provider/模型过滤");

    const enText = i18n.t("usage.accounting.limitations", { lng: "en" });
    expect(enText).toContain("external CLI history");
    expect(enText).toContain("provider billing reconciliation");
    expect(enText).toContain("cost estimates");
    expect(enText).toContain("request-detail logs");
    expect(enText).toContain("Provider/model filtering");
  });
});
