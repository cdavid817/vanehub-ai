/** @vitest-environment jsdom */
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

/**
 * Everything the console can be clicked on, it can also be reached.
 *
 * The failure this catches has one shape: a `<div onClick={...}>`. It looks identical to a button
 * on screen, it works for every reviewer who tried it with a mouse, and it does not exist at all
 * for anybody driving the keyboard. Nothing in review flags it, because reviewing a diff is
 * reading, not tabbing.
 *
 * A native element or an explicit `tabIndex` is the answer. The exception is a surface that takes
 * the keyboard wholesale — a virtualized list where the rows nobody scrolled to are not in the DOM
 * to be tabbed through — and those declare it with `aria-activedescendant`, which is checked for
 * rather than assumed.
 */

/** A click handler on an element that is not focusable by default. */
const CLICKABLE_NON_INTERACTIVE = /<(?:div|span|li|section|article|td|tr)\b[^>]*\sonClick=/;

/** What makes a non-interactive element reachable anyway. */
const REACHABLE = /\btabIndex=|\baria-activedescendant=|\brole="option"|\brole="listitem"/;

/**
 * A full-bleed scrim, which is not a control pretending to be one.
 *
 * `fixed inset-0` with a click handler is how a menu closes when the reader clicks away. There is
 * nothing to reach: it has no label, no purpose of its own, and a keyboard reader never wants to
 * "activate the area behind the menu". What they want is to close the menu, and the rule for that
 * is the one below — Escape — rather than a tab stop on a transparent rectangle.
 */
const SCRIM = /<div[^>]*className="[^"]*\bfixed inset-0\b[^"]*"[^>]*\sonClick=/;

/** A layer that swallows the scrim's click so the menu itself does not dismiss. */
const STOP_PROPAGATION = /onClick=\{\(event\) => event\.stopPropagation\(\)\}/;

/** Closing without a pointer. */
const ESCAPE_CLOSES = /event\.key === "Escape"/;

function consoleModules(): { name: string; source: string }[] {
  const root = dirname(dirname(fileURLToPath(import.meta.url)));
  return (["session-workspace", "main-layout"] as const).flatMap((directory) =>
    readdirSync(join(root, directory))
      .filter((name) => /\.tsx$/.test(name) && !name.includes(".test."))
      .map((name) => ({
        name: `${directory}/${name}`,
        source: readFileSync(join(root, directory, name), "utf8"),
      })),
  );
}

describe("keyboard reach across the console", () => {
  it("recognises the shape it is looking for", () => {
    // Without this, a broken pattern reports every surface as reachable.
    expect(CLICKABLE_NON_INTERACTIVE.test('<div className="row" onClick={pick}>')).toBe(true);
    expect(CLICKABLE_NON_INTERACTIVE.test('<button className="row" onClick={pick}>')).toBe(false);
    expect(REACHABLE.test('<div tabIndex={0} onClick={pick}>')).toBe(true);
    expect(REACHABLE.test('<div className="row" onClick={pick}>')).toBe(false);
  });

  it("leaves nothing clickable that a keyboard cannot get to", () => {
    const offenders = consoleModules()
      .filter(({ source }) => {
        if (!CLICKABLE_NON_INTERACTIVE.test(source) || REACHABLE.test(source)) return false;
        // A scrim and the layer that stops its click are the two shapes that are legitimately
        // pointer-only. Excluded by their shape rather than by filename, so the exemption cannot
        // quietly grow to cover the next div somebody makes clickable.
        return !(SCRIM.test(source) || STOP_PROPAGATION.test(source));
      })
      .map(({ name }) => name);

    expect(offenders).toEqual([]);
  });

  it("lets a keyboard close anything a click-away can close", () => {
    // The rule the scrim exemption owes back. A surface dismissed by clicking outside it must have
    // some other way out, or a reader who opened it with the keyboard is stuck inside it — which
    // is exactly what the session context menu did until this task.
    const offenders = consoleModules()
      .filter(({ source }) => SCRIM.test(source) && !ESCAPE_CLOSES.test(source))
      .map(({ name }) => name);

    expect(offenders).toEqual([]);
  });

  it("recognises a scrim and the layer that shields it", () => {
    expect(SCRIM.test('<div className="fixed inset-0 z-50" onClick={onDismiss}>')).toBe(true);
    expect(SCRIM.test('<div className="rounded p-2" onClick={pick}>')).toBe(false);
    expect(STOP_PROPAGATION.test("onClick={(event) => event.stopPropagation()}")).toBe(true);
    expect(ESCAPE_CLOSES.test('if (event.key === "Escape") onDismiss();')).toBe(true);
  });

  it("scans the surfaces the task names", () => {
    const names = consoleModules().map((entry) => entry.name);

    // Named so a rename fails here. A glob keeps passing by scanning what is left, and what was
    // renamed is what just changed.
    for (const surface of [
      "session-workspace/execution-record-row.tsx",
      "session-workspace/shell-strip.tsx",
      "session-workspace/logs-toolbar.tsx",
      "session-workspace/trace-span-row.tsx",
      "session-workspace/execution-record-detail-drawer.tsx",
      "session-workspace/quick-open-dialog.tsx",
      "session-workspace/document-sidebar.tsx",
      "session-workspace/review-center.tsx",
    ]) {
      expect(names, `${surface} is missing`).toContain(surface);
    }
  });

  it("gives back the focus a drawer took", () => {
    // Taking focus on open is half of it. Without the return, closing a drawer drops focus on the
    // document body, so the next Tab starts from the top of the page rather than from the row the
    // reader was reading.
    const root = dirname(dirname(fileURLToPath(import.meta.url)));
    const drawer = readFileSync(
      join(root, "session-workspace", "execution-record-detail-drawer.tsx"),
      "utf8",
    );

    expect(drawer).toMatch(/const returnTo = document\.activeElement/);
    expect(drawer).toMatch(/returnTo\.focus\(\)/);
  });
});
