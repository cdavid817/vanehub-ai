import { readFileSync, readdirSync } from "node:fs";
import { describe, expect, it } from "vitest";
import en from "../i18n/locales/en.json";
import zhCN from "../i18n/locales/zh-CN.json";
import { ucdThemes } from "../theme/theme-registry";
import type {
  LoopEvidenceKind,
  LoopEvidenceStatus,
  LoopRunPhase,
  LoopRunStatus,
  LoopTerminalReason,
  LoopVerifierRecommendation,
} from "../types/loop";

const componentFiles = readdirSync("src/loop-center")
  .filter((file) => file.endsWith(".tsx") && !file.endsWith(".test.tsx"))
  .map((file) => `src/loop-center/${file}`);

const componentSource = componentFiles
  .map((file) => readFileSync(file, "utf8"))
  .join("\n");

function exactValues<Union>() {
  return <Values extends readonly Union[]>(
    values: Exclude<Union, Values[number]> extends never ? Values : never,
  ) => values;
}

describe("Loop Center localization and themes", () => {
  it("keeps every static Loop translation key synchronized", () => {
    const keys = [...componentSource.matchAll(/["'](loops\.[A-Za-z0-9.-]+)["']/g)]
      .map((match) => match[1]);

    for (const key of new Set(keys)) {
      expect(en, `missing en key: ${key}`).toHaveProperty(key);
      expect(zhCN, `missing zh-CN key: ${key}`).toHaveProperty(key);
    }
  });

  it("covers every dynamic Loop model translation in both locales", () => {
    const dynamicKeys = [
      ...exactValues<LoopRunStatus>()(["queued", "running", "paused", "awaiting-acceptance", "succeeded", "failed", "cancelled"] as const)
        .map((value) => `loops.status.${value}`),
      ...exactValues<LoopRunPhase>()(["preparing", "acting", "verifying", "deciding", "finalizing"] as const)
        .map((value) => `loops.phase.${value}`),
      ...exactValues<LoopVerifierRecommendation>()(["pass", "revise", "blocked"] as const)
        .map((value) => `loops.recommendation.${value}`),
      ...exactValues<LoopTerminalReason>()([
        "goal-met", "max-iterations", "time-budget", "phase-timeout", "runtime-errors", "no-progress",
        "verification-failed", "verifier-blocked", "runtime-error", "recovery-required", "user-rejected", "user-stopped",
      ] as const).map((value) => `loops.reason.${value}`),
      ...exactValues<LoopEvidenceKind>()(["worktree", "worker", "verification", "verifier", "decision", "recovery"] as const)
        .map((value) => `loops.evidence.kind.${value}`),
      ...exactValues<LoopEvidenceStatus>()(["pending", "passed", "failed", "blocked", "cancelled"] as const)
        .map((value) => `loops.evidence.status.${value}`),
    ];

    for (const key of dynamicKeys) {
      expect(Object.prototype.hasOwnProperty.call(en, key), `missing en key: ${key}`).toBe(true);
      expect(Object.prototype.hasOwnProperty.call(zhCN, key), `missing zh-CN key: ${key}`).toBe(true);
      expect(en[key as keyof typeof en]).not.toBe(key);
      expect(zhCN[key as keyof typeof zhCN]).not.toBe(key);
    }
  });

  it("keeps frontend-owned copy behind localization resources", () => {
    expect(componentSource).not.toMatch(/<[a-z][^>]*>\s*[A-Za-z][A-Za-z0-9 ,.'!?-]*\s*<\/[a-z]/);
    expect(componentSource).not.toMatch(/(?:aria-label|placeholder|title)=["'][^"']+["']/);
  });

  it("uses shared semantic tokens without style-specific branches or fixed palettes", () => {
    expect(componentSource).not.toMatch(/#[0-9a-f]{3,8}\b|\brgba?\(/i);
    expect(componentSource).not.toMatch(/(?:bg|border|text)-(?:blue|cyan|gray|green|orange|purple|red|slate|yellow)-\d{2,3}/);
    expect(componentSource).not.toMatch(/theme\s*===\s*["'](?:futuristic|minimal)["']/);
    expect(componentSource).not.toMatch(/rounded-(?:xl|2xl|3xl)/);
    expect(componentSource).toContain("ucd-panel");
    expect(componentSource).toContain("ucd-list-row");
    expect(componentSource).toContain("text-success");
    expect(componentSource).toContain("text-warning");
    expect(componentSource).toContain("text-destructive");
  });

  it("defines every Loop semantic token in both registered themes", () => {
    const css = readFileSync("src/styles.css", "utf8");
    const tokens = [
      "--background",
      "--foreground",
      "--muted",
      "--muted-foreground",
      "--border",
      "--ring",
      "--primary",
      "--destructive",
      "--panel",
      "--panel-muted",
      "--panel-border",
      "--panel-hover",
      "--panel-glass",
      "--success",
      "--warning",
    ];

    for (const theme of ucdThemes) {
      const block = css.match(new RegExp(`:root\\[data-theme="${theme.id}"\\] \\{([\\s\\S]*?)\\n\\}`))?.[1] ?? "";
      for (const token of tokens) expect(block, `${theme.id} missing ${token}`).toContain(token);
    }
  });
});
