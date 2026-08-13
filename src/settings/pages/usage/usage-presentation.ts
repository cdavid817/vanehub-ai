import type { UsageStatisticsRange } from "../../../types/chat";
import type {
  TokenDimensions,
  TokenUsageSummaryQuery,
  UsageMeasure,
  UsageQualityTotals,
} from "../../../types/token-usage";

export function usageRangeQuery(
  range: UsageStatisticsRange,
  now = new Date(),
): Pick<TokenUsageSummaryQuery, "rangeStart" | "rangeEnd"> {
  if (range === "all") return {};
  const end = new Date(now);
  end.setHours(0, 0, 0, 0);
  end.setDate(end.getDate() + 1);
  const start = new Date(end);
  start.setDate(start.getDate() - (range === "today" ? 1 : range === "last7Days" ? 7 : 30));
  return { rangeStart: start.toISOString(), rangeEnd: end.toISOString() };
}

export function headline(measure: UsageMeasure): number | null {
  return measure.headlineTotal;
}

export function tokenTotal(totals: UsageQualityTotals): number | null {
  const reported = headline(totals.reported);
  const derived = headline(totals.reportedDerived);
  return reported === null || derived === null ? null : reported + derived;
}

export function reportedCoverage(totals: UsageQualityTotals) {
  const reported = totals.reported.callCount;
  const total = reported + totals.reportedDerived.callCount + totals.estimated.callCount;
  return { reported, total, percent: total === 0 ? 0 : (reported / total) * 100 };
}

export function dimensionRows(dimensions: TokenDimensions) {
  return [
    ["input", dimensions.input],
    ["output", dimensions.output],
    ["cachedInput", dimensions.cachedInput],
    ["cacheWriteInput", dimensions.cacheWriteInput],
    ["reasoningOutput", dimensions.reasoningOutput],
    ["providerTotal", dimensions.providerTotal],
  ] as const;
}

export function breakdownKeys(
  breakdowns: import("../../../types/token-usage").UsageBreakdown[],
  dimension: import("../../../types/token-usage").UsageBreakdownDimension,
) {
  return breakdowns
    .find((breakdown) => breakdown.dimension === dimension)
    ?.entries.map((entry) => entry.key)
    .filter((key) => key !== "unknown") ?? [];
}
