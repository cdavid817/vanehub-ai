import AxeBuilder from "@axe-core/playwright";
import { expect, type Page } from "@playwright/test";
import type { Result } from "axe-core";

/**
 * 20.18: what this task's own wording ("fix serious/critical findings") treats as blocking --
 * `minor`/`moderate` findings are real too, but scoping the very first automated-axe pass to the
 * two impact levels the task itself names keeps this increment's bar unambiguous rather than
 * silently widening what "pass" means beyond what was asked for.
 */
const BLOCKING_IMPACTS = new Set(["serious", "critical"]);

function describeViolation(violation: Result): string {
  const targets = violation.nodes.map((node) => node.target.join(" ")).join(", ");
  return `[${violation.impact}] ${violation.id} -- ${violation.help} (${violation.nodes.length} node(s): ${targets})`;
}

/**
 * Runs a real axe-core scan against the page's current DOM and fails with a readable, per-finding
 * message if any `serious`/`critical` violation exists -- shared by every destination's own axe
 * test (task 20.18) instead of duplicated per spec file, since the scan-and-filter logic itself is
 * identical everywhere and worth one source of truth, unlike this directory's smaller per-file
 * helpers (`openLoops`-style) that are each genuinely specific to one destination's own UI.
 */
export async function expectNoSeriousAxeViolations(page: Page): Promise<void> {
  const results = await new AxeBuilder({ page }).analyze();
  const blocking = results.violations.filter((violation) => BLOCKING_IMPACTS.has(violation.impact ?? ""));
  expect(blocking.map(describeViolation), "serious/critical accessibility violations").toEqual([]);
}
