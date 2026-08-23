import { readFileSync, readdirSync } from "node:fs";
import { describe, expect, it } from "vitest";
import en from "../../../i18n/locales/en.json";
import ja from "../../../i18n/locales/ja.json";
import ko from "../../../i18n/locales/ko.json";
import zhCN from "../../../i18n/locales/zh-CN.json";
import zhTW from "../../../i18n/locales/zh-TW.json";
import {
  CLI_BULK_SKIP_REASONS,
  CLI_MUTATION_OUTCOMES,
  CLI_OPERATION_PHASES,
  CLI_OPERATION_TERMINATIONS,
  CLI_VERIFICATION_WARNINGS,
} from "../../../types/cli-environment";

/**
 * Every string this page can put on screen has to exist in every locale.
 *
 * The two halves are different failures. The static half catches a `t("cli.thing")` whose key was
 * never added. The dynamic half catches the one that only shows up on a real machine: a template
 * key built from a backend enum value, where the page renders the raw key -- `cli.skip.plan-stale`
 * -- to whoever hits that state, and no amount of clicking through a healthy install finds it.
 *
 * The value lists below are the Rust `as_str` outputs. Rust has its own tests pinning them, so a
 * rename fails one side or the other rather than silently producing a key nobody translated.
 */

const MODULE_DIR = "src/settings/pages/cli-management";

const RESOURCES = { "zh-CN": zhCN, en, "zh-TW": zhTW, ja, ko } as const;

/** Enum vocabularies rendered as `${prefix}.${value}` by this module. */
const DYNAMIC_FAMILIES: Array<[prefix: string, values: readonly string[]]> = [
  ["cli.action", ["install", "upgrade", "downgrade", "reinstall", "uninstall", "repair"]],
  ["cli.authentication", ["authenticated", "required", "expired", "unknown", "not-applicable"]],
  ["cli.compatibility", ["supported", "unsupported-version", "unsupported-platform", "unknown"]],
  ["cli.confidence", ["unknown", "inferred", "verified"]],
  ["cli.conflict", [
    "duplicate-launcher-alias",
    "path-shadowing",
    "broken-path-precedence",
    "multiple-installation-sources",
    "version-divergence",
    "ambiguous-source-ownership",
    "environment-path-divergence",
    "architecture-mismatch",
    "stale-launcher-target",
  ]],
  ["cli.discovery", ["not-scanned", "not-found", "found-one", "found-multiple"]],
  ["cli.executable", [
    "not-applicable",
    "healthy",
    "broken",
    "timeout",
    "permission-denied",
    "unsupported-architecture",
    "unknown",
  ]],
  ["cli.freshness", ["never", "fresh", "stale", "refreshing"]],
  ["cli.guidance", ["homebrew", "bun", "volta", "desktop", "system", "manual", "unknown"]],
  ["cli.management", ["managed", "detect-only"]],
  ["cli.operationStatus", ["queued", "running", "succeeded", "failed", "cancelled"]],
  ["cli.origin", ["path", "known-location"]],
  ["cli.outcome", CLI_MUTATION_OUTCOMES],
  ["cli.outcome.guidance", ["applied-unverified", "changed-but-failed", "no-change-failed"]],
  ["cli.overallState", [
    "broken",
    "conflict",
    "needs-auth",
    "update-available",
    "ready",
    "missing",
    "unknown",
  ]],
  ["cli.phase", CLI_OPERATION_PHASES],
  ["cli.planWarning", [
    "target-is-latest-only",
    "installer-integrity-unverified",
    "exact-version-not-confirmed",
    "active-installation-shadowed",
    "downgrade-may-lose-state",
  ]],
  ["cli.precondition", [
    "source-executable-available",
    "network-reachable",
    "elevated-privileges",
  ]],
  ["cli.readiness", [
    "ready",
    "needs-auth",
    "missing-dependency",
    "misconfigured",
    "broken",
    "unknown",
  ]],
  ["cli.severity", ["info", "warning", "error"]],
  ["cli.skip", CLI_BULK_SKIP_REASONS],
  // Registry source ids plus every source kind, because a kind with no registry distribution is
  // summarized under its own id.
  ["cli.source", [
    "npm",
    "winget",
    "vendor",
    "vendor-installer",
    "homebrew",
    "bun",
    "volta",
    "desktop",
    "system",
    "manual",
    "unknown",
  ]],
  ["cli.termination", CLI_OPERATION_TERMINATIONS],
  ["cli.update", [
    "not-applicable",
    "up-to-date",
    "available",
    "ahead",
    "catalog-unavailable",
    "unknown",
  ]],
  ["cli.verificationWarning", CLI_VERIFICATION_WARNINGS],
];

function moduleSources(): string[] {
  return readdirSync(MODULE_DIR)
    .filter((name) => /\.tsx?$/.test(name) && !name.endsWith(".test.ts") && !name.endsWith(".test.tsx"))
    .map((name) => readFileSync(`${MODULE_DIR}/${name}`, "utf8"));
}

/** Keys written out in full, as opposed to assembled from a value at runtime. */
function staticKeys(): string[] {
  const keys = new Set<string>();
  for (const source of moduleSources()) {
    for (const match of source.matchAll(/"(cli\.[A-Za-z0-9._-]+)"/g)) keys.add(match[1]);
  }
  // Prefixes of dynamic families are written as bare strings too; they are never looked up alone.
  const prefixes = new Set(DYNAMIC_FAMILIES.map(([prefix]) => prefix));
  return [...keys].filter((key) => !prefixes.has(key));
}

describe("CLI management page localization", () => {
  it("writes out at least the keys this module is known to reference", () => {
    // A tripwire on the scanner itself: if it silently stops matching, the assertions below pass
    // by finding nothing.
    expect(staticKeys().length).toBeGreaterThan(60);
  });

  it.each(Object.keys(RESOURCES))("resolves every static key in %s", (locale) => {
    const resource: Record<string, string> = RESOURCES[locale as keyof typeof RESOURCES];
    const missing = staticKeys().filter(
      (key) => !(key in resource) && !(`${key}_other` in resource),
    );
    expect(missing).toEqual([]);
  });

  it.each(Object.keys(RESOURCES))("resolves every enum-derived key in %s", (locale) => {
    const resource: Record<string, string> = RESOURCES[locale as keyof typeof RESOURCES];
    const missing: string[] = [];
    for (const [prefix, values] of DYNAMIC_FAMILIES) {
      for (const value of values) {
        if (!(`${prefix}.${value}` in resource)) missing.push(`${prefix}.${value}`);
      }
    }
    expect(missing).toEqual([]);
  });

  it("keeps no visible copy hard-coded in the module", () => {
    // JSX text nodes made of letters. Anything user-visible has to come through `t`.
    const offenders: string[] = [];
    for (const name of readdirSync(MODULE_DIR).filter((file) => file.endsWith(".tsx"))) {
      const source = readFileSync(`${MODULE_DIR}/${name}`, "utf8");
      for (const match of source.matchAll(/>\s*([A-Za-z][A-Za-z ]{3,})\s*</g)) {
        offenders.push(`${name}: ${match[1]}`);
      }
    }
    expect(offenders).toEqual([]);
  });
});
