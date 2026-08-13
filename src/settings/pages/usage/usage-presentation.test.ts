import { describe, expect, it } from "vitest";
import type { UsageMeasure, UsageQualityTotals } from "../../../types/token-usage";
import { dimensionRows, reportedCoverage, tokenTotal, usageRangeQuery } from "./usage-presentation";

function measure(total: number | null, calls: number): UsageMeasure {
  return { unit: "tokens", dimensions: { input: 10, output: 5, cachedInput: 3, cacheWriteInput: 2, reasoningOutput: 4, providerTotal: total }, headlineTotal: total, callCount: calls, observationCount: calls };
}

describe("Token usage presentation semantics", () => {
  it("uses authoritative totals and never invents a total for unknown overlap", () => {
    const known: UsageQualityTotals = { reported: measure(15, 1), reportedDerived: measure(5, 1), estimated: { ...measure(0, 0), unit: "characters" } };
    expect(tokenTotal(known)).toBe(20);
    expect(tokenTotal({ ...known, reported: measure(null, 1) })).toBeNull();
  });

  it("preserves cache and reasoning as non-additive dimensions", () => {
    expect(dimensionRows(measure(15, 1).dimensions)).toEqual([
      ["input", 10], ["output", 5], ["cachedInput", 3], ["cacheWriteInput", 2],
      ["reasoningOutput", 4], ["providerTotal", 15],
    ]);
  });

  it("computes call coverage without mixing estimated characters into Tokens", () => {
    const totals: UsageQualityTotals = { reported: measure(15, 2), reportedDerived: measure(5, 1), estimated: { ...measure(100, 1), unit: "characters" } };
    expect(reportedCoverage(totals)).toEqual({ reported: 2, total: 4, percent: 50 });
  });

  it("builds local-calendar range boundaries", () => {
    const now = new Date(2026, 7, 13, 14, 30);
    const query = usageRangeQuery("last7Days", now);
    const start = new Date(query.rangeStart ?? "");
    const end = new Date(query.rangeEnd ?? "");
    expect([start.getFullYear(), start.getMonth(), start.getDate(), start.getHours()]).toEqual([2026, 7, 7, 0]);
    expect([end.getFullYear(), end.getMonth(), end.getDate(), end.getHours()]).toEqual([2026, 7, 14, 0]);
  });
});
