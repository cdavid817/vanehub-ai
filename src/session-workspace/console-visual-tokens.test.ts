import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { LITERAL_COLOR, PALETTE_COLOR, TOKEN_RULE_SAMPLES } from "./visual-token-rules";

/**
 * Every console surface takes its colour from the theme.
 *
 * The whole console has two styles, and a literal colour is correct in exactly the one its author
 * had open. Nothing about it looks wrong to the person who wrote it, which is why this is a scan
 * and not a review checklist: the control that gets a hard-coded blue is always the one somebody
 * added in a hurry while looking at a screenshot.
 *
 * 12.15 established the same rule over Files and Documents while those were being built. This is
 * the same rule over all ten surfaces the change touches, reading the same patterns from the same
 * place so the two cannot drift apart.
 */

/** One named surface per line of the task, so a rename fails here rather than going unscanned. */
const CONSOLE_SURFACES: readonly { readonly directory: "main-layout" | "session-workspace"; readonly file: string }[] = [
  { directory: "session-workspace", file: "session-tab-bar.tsx" },
  { directory: "session-workspace", file: "execution-record-list.tsx" },
  { directory: "session-workspace", file: "execution-record-row.tsx" },
  { directory: "session-workspace", file: "shell-strip.tsx" },
  { directory: "session-workspace", file: "shell-tab.tsx" },
  { directory: "session-workspace", file: "logs-toolbar.tsx" },
  { directory: "session-workspace", file: "trace-waterfall.tsx" },
  { directory: "session-workspace", file: "trace-span-row.tsx" },
  { directory: "session-workspace", file: "report-tab.tsx" },
  { directory: "session-workspace", file: "report-section.tsx" },
  { directory: "session-workspace", file: "files-tab.tsx" },
  { directory: "session-workspace", file: "documents-tab.tsx" },
  { directory: "session-workspace", file: "review-center.tsx" },
  { directory: "main-layout", file: "session-overview.tsx" },
  { directory: "main-layout", file: "session-overview-sections.tsx" },
  { directory: "main-layout", file: "session-overview-runtime-workspace.tsx" },
  { directory: "main-layout", file: "session-evidence-summary.tsx" },
];

function root() {
  return dirname(dirname(fileURLToPath(import.meta.url)));
}

function read(entry: (typeof CONSOLE_SURFACES)[number]): string {
  return readFileSync(join(root(), entry.directory, entry.file), "utf8");
}

/**
 * The one module excluded, and why that is not a loosened guard.
 *
 * `visual-token-rules.ts` renders nothing. It holds the patterns and the strings that prove those
 * patterns work, and one of those strings is a hard-coded colour on purpose — without it a typo in
 * the pattern would leave a check that passes by matching nothing. Scanning it would mean deleting
 * the sample to satisfy the rule the sample exists to protect, which is a trap this change has
 * already walked into twice.
 */
const RULE_MODULE = "visual-token-rules.ts";

/** Every production module in the two directories the console lives in. */
function everyConsoleModule(): { name: string; source: string }[] {
  return (["session-workspace", "main-layout"] as const).flatMap((directory) =>
    readdirSync(join(root(), directory))
      .filter((name) => /\.tsx?$/.test(name) && !name.includes(".test.") && name !== RULE_MODULE)
      .map((name) => ({
        name: `${directory}/${name}`,
        source: readFileSync(join(root(), directory, name), "utf8"),
      })),
  );
}

describe("the console's visual tokens", () => {
  it("uses patterns that match a real violation and spare a real token", () => {
    expect(PALETTE_COLOR.test(TOKEN_RULE_SAMPLES.paletteMatches)).toBe(true);
    expect(PALETTE_COLOR.test(TOKEN_RULE_SAMPLES.paletteRejects)).toBe(false);
    expect(LITERAL_COLOR.test(TOKEN_RULE_SAMPLES.literalMatches)).toBe(true);
    expect(LITERAL_COLOR.test(TOKEN_RULE_SAMPLES.literalRejects)).toBe(false);
  });

  it("scans every surface the change styles", () => {
    // Named rather than globbed. A glob keeps passing after a rename by scanning whatever is left,
    // and the surface that was renamed is exactly the one that just changed.
    for (const entry of CONSOLE_SURFACES) {
      expect(() => read(entry), `${entry.directory}/${entry.file} is missing`).not.toThrow();
    }
  });

  it("takes every colour on those surfaces from the theme", () => {
    const offenders = CONSOLE_SURFACES.filter((entry) => {
      const source = read(entry);
      return PALETTE_COLOR.test(source) || LITERAL_COLOR.test(source);
    }).map((entry) => `${entry.directory}/${entry.file}`);

    expect(offenders).toEqual([]);
  });

  it("holds the same rule across both directories, not only the named files", () => {
    // The named list says which surfaces the task owns; this says nothing beside them slipped. A
    // helper module with a hard-coded colour reaches the screen exactly as surely as a panel does.
    const offenders = everyConsoleModule()
      .filter(({ source }) => PALETTE_COLOR.test(source) || LITERAL_COLOR.test(source))
      .map(({ name }) => name);

    expect(offenders).toEqual([]);
  });
});
