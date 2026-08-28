import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

/**
 * Every sentence the console shows comes from a resource file.
 *
 * The guard beside this one lists strings that were once hard-coded and must not come back. That
 * catches a regression and nothing else: it can only fail for copy somebody already wrote down,
 * which means the next hard-coded sentence — the one nobody has thought of — passes.
 *
 * So this reads the JSX instead and asks the opposite question: is there prose here that did not
 * come from `t()`. Scoped to the two directories this change built, because widening it to the
 * whole repository would surface a backlog that has nothing to do with the console and would be
 * answered by an allowlist, which is the thing 14.8 says not to do.
 */

/**
 * A JSX text node holding prose.
 *
 * Two or more words, so an arrow, a separator, a count, or a single symbol does not register.
 * Anything inside braces is an expression and is not matched at all, which is where `t()` lives.
 */
const JSX_PROSE = />\s*([A-Za-z][A-Za-z'’]*(?:\s+[A-Za-z][A-Za-z'’]*)+[.?!]?)\s*</g;

/**
 * Names that are the same in every language.
 *
 * Named individually rather than matched by a pattern. A pattern would grow to cover whatever
 * failed next, and the difference between "this is a product name" and "nobody translated this
 * yet" is exactly the judgement a pattern cannot make.
 */
const PROPER_NOUNS = new Set(["Claude Code", "Codex CLI", "VaneHub AI"]);

function consoleSources(): { name: string; source: string }[] {
  const root = dirname(dirname(fileURLToPath(import.meta.url)));
  return (["session-workspace", "main-layout"] as const).flatMap((directory) =>
    readdirSync(join(root, directory))
      .filter((name) => /\.tsx$/.test(name) && !name.includes(".test."))
      .map((name) => ({
        name: `${directory}/${name}`,
        // Comments are stripped first. An explanation of why something reads a certain way is not
        // copy, and a guard that flagged its own rationale would be answered by deleting the
        // rationale — which this change has watched happen twice.
        source: readFileSync(join(root, directory, name), "utf8")
          .replace(/\/\*[\s\S]*?\*\//g, "")
          .replace(/^\s*\/\/.*$/gm, ""),
      })),
  );
}

describe("console copy", () => {
  it("recognises prose and ignores everything that is not", () => {
    // Without this a broken pattern leaves a guard that passes by matching nothing, and reports
    // every surface as translated.
    const matches = (markup: string) => [...markup.matchAll(JSX_PROSE)].map((match) => match[1]);

    expect(matches("<p>Nothing was recorded.</p>")).toEqual(["Nothing was recorded."]);
    expect(matches("<span>{t('a.b')}</span>")).toEqual([]);
    expect(matches("<span>→</span>")).toEqual([]);
    expect(matches("<span>Logs</span>")).toEqual([]);
  });

  it("scans the surfaces this change built", () => {
    const names = consoleSources().map((entry) => entry.name);

    // A guard whose file set silently emptied would pass forever.
    expect(names).toContain("session-workspace/review-center.tsx");
    expect(names).toContain("main-layout/session-evidence-summary.tsx");
    expect(names.length).toBeGreaterThan(40);
  });

  it("leaves no sentence outside the resource files", () => {
    const offenders = consoleSources().flatMap(({ name, source }) =>
      [...source.matchAll(JSX_PROSE)]
        .map((match) => match[1])
        .filter((text) => !PROPER_NOUNS.has(text))
        .map((text) => `${name}: ${text}`),
    );

    expect(offenders).toEqual([]);
  });

  it("keeps the proper-noun list to names that are actually there", () => {
    // An entry nobody renders is an exemption for a string that no longer exists, and it would
    // keep covering whatever took its place.
    const rendered = new Set(
      consoleSources().flatMap(({ source }) =>
        [...source.matchAll(JSX_PROSE)].map((match) => match[1]),
      ),
    );

    expect([...PROPER_NOUNS].filter((noun) => !rendered.has(noun))).toEqual([]);
  });
});
