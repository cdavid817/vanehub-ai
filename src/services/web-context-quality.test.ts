// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  ContextQualityServiceError,
  getWebContextQualitySummary,
  listWebContextQualityHistory,
} from "./web-context-quality";
import { webSettingsClient } from "./web-settings-client";

describe("web context quality contract", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-08-14T12:00:00.000Z"));
    window.localStorage.clear();
  });

  afterEach(() => vi.useRealTimers());

  it("returns stable cursor pages without source content", () => {
    const first = listWebContextQualityHistory({ rangeDays: 7, limit: 1 });
    const second = listWebContextQualityHistory({
      rangeDays: 7,
      limit: 1,
      cursor: first.nextCursor,
    });

    expect(first.items).toHaveLength(1);
    expect(second.items).toHaveLength(1);
    expect(second.items[0]?.attemptId).not.toBe(first.items[0]?.attemptId);
    expect(JSON.stringify([...first.items, ...second.items])).not.toContain("prompt");
  });

  it("keeps character-only coverage separate from token savings", () => {
    const summary = getWebContextQualitySummary({ rangeDays: 30 });

    expect(summary.evaluated).toBe(4);
    expect(summary.qualityCoverage.measuredWithTokens).toBe(3);
    expect(summary.qualityCoverage.charactersOnly).toBe(1);
    expect(summary.qualityCoverage.tokenCoverageBasisPoints).toBe(7_500);
    expect(summary.savedTokens).toBeGreaterThan(0);
  });

  it("reports bounded typed failures", () => {
    expect(() => listWebContextQualityHistory({ rangeDays: 7, cursor: "unknown" }))
      .toThrowError(ContextQualityServiceError);
    try {
      listWebContextQualityHistory({ rangeDays: 7, cursor: "unknown" });
    } catch (error) {
      expect(error).toMatchObject({ code: "invalid-cursor" });
    }
  });

  it("prunes records outside the configured Web retention window", async () => {
    await webSettingsClient.saveSetting({ key: "contextQualityRetentionDays", value: 7 });

    expect(getWebContextQualitySummary({ rangeDays: 30 }).evaluated).toBe(2);
  });
});
