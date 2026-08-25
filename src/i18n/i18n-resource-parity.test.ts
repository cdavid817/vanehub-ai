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
const overlayGovernanceKeys = [
  "skills.overlay.diff.basePackage",
  "skills.overlay.activeScopes",
  "skills.overlay.import.quarantineTitle",
  "skills.overlay.pinnedReadOnlyTitle",
  "skills.overlay.mutation.scan",
  "skills.overlay.reconcile.previewStale",
  "skills.overlay.reconcile.title",
  "skills.overlay.scopeStatus.blockedByEarlierScope",
  "skills.overlay.history.integrityFailure",
] as const;

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

  it.each(appLanguages.filter((language) => language !== "en"))(
    "localizes representative Overlay governance states in %s without English fallback copy",
    (language) => {
      for (const key of overlayGovernanceKeys) {
        expect(resources[language][key], `${language}:${key}`).not.toBe(canonicalResource[key]);
      }
    },
  );

  it.each(appLanguages.filter((language) => language !== "en" && language !== "zh-CN"))(
    "localizes CLI management copy in %s instead of copying a reference locale",
    (language) => {
      // Copying English or Simplified Chinese into every locale passes the key-parity check above
      // and still ships an untranslated page. These are sentences rather than terms, so an honest
      // translation cannot come out byte-identical to either reference.
      const sentences = [
        "cli.outcome.guidance.changed-but-failed",
        "cli.conflict.path-shadowing",
        "cli.guidance.homebrew",
        "cli.plan.description",
        "cli.planWarning.downgrade-may-lose-state",
        "cli.error.plan-revision-mismatch",
      ];
      const simplified = resources["zh-CN"];
      for (const key of sentences) {
        expect(resources[language][key], `${language}:${key} copies English`).not.toBe(canonicalResource[key]);
        expect(resources[language][key], `${language}:${key} copies Simplified Chinese`).not.toBe(simplified[key]);
      }
    },
  );

  it.each(appLanguages)("keeps every CLI placeholder intact in %s", (language) => {
    // The parity check above compares variable names across locales; this one is about the CLI
    // surface specifically, where a dropped `{{count, number}}` renders a bare label next to a
    // number that never arrives.
    for (const key of Object.keys(canonicalResource).filter((name) => name.startsWith("cli."))) {
      const expected = interpolationVariables(canonicalResource[key]);
      expect(interpolationVariables(resources[language][key]), `${language}:${key}`).toEqual(expected);
      if (expected.includes("count")) {
        expect(resources[language][key], `${language}:${key}`).toContain("{{count, number}}");
      }
    }
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
    ["zh-CN", "Agent 配置"],
    ["en", "Agent Configurations"],
    ["zh-TW", "Agent 配置"],
    ["ja", "Agent Configurations"],
    ["ko", "Agent Configurations"],
  ] as const)("activates representative %s translations without fallback copy", async (language, title) => {
    await activateAppLanguage(language);
    expect(i18n.t("agentConfigurations.title")).toBe(title);
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
