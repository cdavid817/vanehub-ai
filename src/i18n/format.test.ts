import { describe, expect, it } from "vitest";
import { formatAppDateTime, formatAppNumber } from "./format";

describe("active-locale formatting", () => {
  it("formats dates and numbers with the requested application language", () => {
    const date = "2026-07-17T08:30:00.000Z";

    expect(formatAppDateTime(date, "ja", { dateStyle: "long", timeZone: "UTC" })).toContain("2026年");
    expect(formatAppDateTime(date, "ko", { dateStyle: "long", timeZone: "UTC" })).toContain("2026년");
    expect(formatAppNumber(12_345.6, "en", { maximumFractionDigits: 1 })).toBe("12,345.6");
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
