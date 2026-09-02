import { describe, expect, it } from "vitest";
import { formatAppDateTime, formatAppNumber, formatAppWeekdayNames } from "./format";

describe("active-locale formatting", () => {
  it("formats dates and numbers with the requested application language", () => {
    const date = "2026-07-17T08:30:00.000Z";

    expect(formatAppDateTime(date, "ja", { dateStyle: "long", timeZone: "UTC" })).toContain("2026年");
    expect(formatAppDateTime(date, "ko", { dateStyle: "long", timeZone: "UTC" })).toContain("2026년");
    expect(formatAppNumber(12_345.6, "en", { maximumFractionDigits: 1 })).toBe("12,345.6");
  });

  it("derives weekday names from Intl instead of a hand-maintained translated array", () => {
    // Index 0 must be Sunday to match Date#getDay()/getUTCDay(), which is the convention
    // ScheduledTaskFrequency's own `weekday: number` field already relies on.
    expect(formatAppWeekdayNames("en")).toEqual(["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]);
    expect(formatAppWeekdayNames("zh-CN")).toEqual(["周日", "周一", "周二", "周三", "周四", "周五", "周六"]);
    expect(formatAppWeekdayNames("ja")).toEqual(["日", "月", "火", "水", "木", "金", "土"]);
    expect(formatAppWeekdayNames("ko")).toEqual(["일", "월", "화", "수", "목", "금", "토"]);
    expect(formatAppWeekdayNames("en", { weekday: "long" })[1]).toBe("Monday");
  });

  it("does not leave frontend-owned formatting to the host operating-system locale", () => {
    const files = import.meta.glob(["../**/*.ts", "../**/*.tsx"], {
      eager: true,
      query: "?raw",
      import: "default",
    }) as Record<string, string>;

    for (const [path, source] of Object.entries(files)) {
      if (path.endsWith("format.test.ts")) continue;
      expect(source, path).not.toMatch(/\.toLocale(?:String|DateString|TimeString)\(\)/);
    }
  });
});
