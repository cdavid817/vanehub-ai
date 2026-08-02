import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from ".";
import en from "./locales/en.json";
import ja from "./locales/ja.json";
import ko from "./locales/ko.json";
import zhCN from "./locales/zh-CN.json";
import zhTW from "./locales/zh-TW.json";
import { appLanguages, supportedLocales, type AppLanguage, type TranslationResource } from "./supported-locales";

const resources: Record<AppLanguage, TranslationResource> = {
  "zh-CN": zhCN,
  en,
  "zh-TW": zhTW,
  ja,
  ko,
};
const canonicalResource: TranslationResource = en;

function findDuplicateKeys(filePath: string): string[] {
  const raw = readFileSync(filePath, "utf8");
  const keys = [...raw.matchAll(/"([a-zA-Z0-9_.-]+)":\s*"/g)].map((match) => match[1]);
  const seen = new Set<string>();
  const duplicates = new Set<string>();
  for (const key of keys) {
    if (seen.has(key)) duplicates.add(key);
    seen.add(key);
  }
  return [...duplicates];
}

function interpolationVariables(value: string): string[] {
  return [...value.matchAll(/\{\{\s*([^,}\s]+)(?:\s*,[^}]*)?}}/g)]
    .map((match) => match[1])
    .sort();
}

describe("i18n resources", () => {
  it("keeps the locale registry and bundled resources aligned", () => {
    expect(appLanguages).toEqual(["zh-CN", "en", "zh-TW", "ja", "ko"]);
    expect(supportedLocales.map(({ id }) => id)).toEqual(Object.keys(resources));
  });

  it.each(appLanguages)("keeps %s keys and interpolation variables aligned with English", (language) => {
    const resource = resources[language];
    expect(Object.keys(resource).sort()).toEqual(Object.keys(canonicalResource).sort());
    for (const key of Object.keys(canonicalResource)) {
      expect(interpolationVariables(resource[key]), `${language}:${key}`).toEqual(interpolationVariables(canonicalResource[key]));
    }
  });

  it.each(appLanguages)("has no duplicate keys in the raw %s locale source", (language) => {
    expect(findDuplicateKeys(`src/i18n/locales/${language}.json`)).toEqual([]);
  });

  it("uses complete i18next v4 plural pairs for count-sensitive messages", () => {
    const pluralKeys = Object.keys(canonicalResource).filter((key) => /_(?:one|other)$/.test(key));
    const pluralBases = new Set(pluralKeys.map((key) => key.replace(/_(?:one|other)$/, "")));
    expect(pluralBases.size).toBeGreaterThan(0);
    for (const base of pluralBases) {
      expect(canonicalResource).toHaveProperty(`${base}_one`);
      expect(canonicalResource).toHaveProperty(`${base}_other`);
      expect(canonicalResource).not.toHaveProperty(base);
      for (const language of appLanguages) {
        expect(resources[language][`${base}_one`], `${language}:${base}_one`).toContain("{{count, number}}");
        expect(resources[language][`${base}_other`], `${language}:${base}_other`).toContain("{{count, number}}");
      }
    }
  });

  it.each([
    ["zh-CN", "Agent 管理"],
    ["en", "Agent Management"],
    ["zh-TW", "Agent 管理"],
    ["ja", "Agent 管理"],
    ["ko", "Agent 관리"],
  ] as const)("activates representative %s translations without fallback copy", async (language, title) => {
    await activateAppLanguage(language);
    expect(i18n.t("agents.title")).toBe(title);
    expect(i18n.resolvedLanguage).toBe(language);
  });

  it("selects English plural forms and formats numeric counts", async () => {
    await activateAppLanguage("en");
    expect(i18n.t("cli.installationsCount", { count: 1 })).toBe("1 installation");
    expect(i18n.t("cli.installationsCount", { count: 1200 })).toBe("1,200 installations");
  });

  it("formats representative locale-sensitive dates, numbers, and percentages", () => {
    const date = new Date("2026-08-02T00:00:00.000Z");
    expect(new Intl.DateTimeFormat("ja", { dateStyle: "long", timeZone: "UTC" }).format(date)).toContain("2026年");
    expect(new Intl.DateTimeFormat("ko", { dateStyle: "long", timeZone: "UTC" }).format(date)).toContain("2026년");
    expect(new Intl.NumberFormat("zh-TW").format(12_345)).toContain("12,345");
    expect(new Intl.NumberFormat("ko", { style: "percent" }).format(0.25)).toContain("25%");
  });
});
