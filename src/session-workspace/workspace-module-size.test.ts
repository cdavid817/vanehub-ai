import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

/**
 * Headroom, not the ceiling.
 *
 * ESLint already refuses a file over 300 lines, so a test asserting the same number would prove
 * nothing the commit hook has not already proved. What it cannot say is how close a file is: the
 * tab host reached 282 over four rounds of "just one more callback", and the round that would have
 * pushed it over is the one where somebody splits under pressure, badly.
 *
 * So this asserts a lower bound and reports the worst offender by name. Failing here is not "you
 * broke the rule" — it is "the next change to this file has nowhere to go", which is a thing worth
 * knowing before the change rather than during it.
 */

/** Where a file stops having room for the next honest addition. */
const HEADROOM_LIMIT = 280;

/** The rule ESLint enforces. Named so the gap between the two is visible rather than folklore. */
const HARD_LIMIT = 300;

describe("workspace module size", () => {
  it("leaves every production module room to grow", () => {
    const directory = dirname(fileURLToPath(import.meta.url));
    const measured = readdirSync(directory)
      .filter((name) => /\.tsx?$/.test(name) && !name.includes(".test."))
      .map((name) => ({
        name,
        lines: readFileSync(join(directory, name), "utf8").split("\n").length,
      }))
      .sort((left, right) => right.lines - left.lines);

    const crowded = measured.filter((entry) => entry.lines > HEADROOM_LIMIT);

    expect(HEADROOM_LIMIT).toBeLessThan(HARD_LIMIT);
    expect(
      crowded.map((entry) => `${entry.name}:${entry.lines}`),
      `split before adding to these; the hard limit is ${HARD_LIMIT}`,
    ).toEqual([]);
  });
});
