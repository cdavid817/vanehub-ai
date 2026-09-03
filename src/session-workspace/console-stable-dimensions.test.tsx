/** @vitest-environment jsdom */
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { render } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import { WorkspaceState } from "./workspace-state";

/**
 * A state change may repaint. It may not move anything.
 *
 * Hovering a row, a panel switching from loading to loaded, a badge turning red — none of these
 * are allowed to change how much space something takes, because the reader is usually pointing at
 * something when it happens. A list that grows a pixel under the cursor moves the row beneath the
 * pointer, and the click that follows lands on the wrong one.
 *
 * jsdom has no layout engine, so nothing here can measure a box. What it can do is read the
 * classes that decide the box, which is where the mistake actually lives: `hover:px-3` is a
 * padding change written down, and it is invisible in review precisely because it looks like
 * styling rather than like a layout decision.
 */

/** Utilities that occupy space. Anything a state toggles must not be one of these. */
const DIMENSION_UTILITY =
  "(?:-?[mp][trblxy]?-|(?:min-|max-)?[hw]-|gap(?:-[xy])?-|space-[xy]-|text-(?:xs|sm|base|lg|[0-9]?xl)\\b|leading-|tracking-|inset-|top-|right-|bottom-|left-|translate-|scale-|border(?:-[trblxy])?-[0-9])";

/**
 * State variants, as Tailwind spells them.
 *
 * Purely syntactic and therefore free of judgement calls: `hover:px-3` is a padding change that
 * happens on hover, and there is no reading of it that is not one.
 */
const STATE_VARIANT_DIMENSION = new RegExp(
  `\\b(?:hover|focus|focus-visible|focus-within|active|disabled|group-hover|group-focus|aria-selected|aria-expanded|data-\\[[^\\]]+\\]):${DIMENSION_UTILITY}`,
);

/** A class list a condition adds on its own — the JS spelling of the same mistake. */
const CONDITIONAL_DIMENSION = new RegExp(`&& "[^"]*\\b${DIMENSION_UTILITY}`);

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

/** The classes from one class list that decide how much space it takes. */
function sizeOf(classNames: string): string {
  const dimension = new RegExp(`^${DIMENSION_UTILITY}`);
  return classNames
    .split(/\s+/)
    .filter((token) => dimension.test(token))
    .sort()
    .join(" ");
}

beforeAll(async () => {
  await activateAppLanguage("en");
});

describe("state changes across the console", () => {
  it("uses patterns that match the mistake and spare the ordinary case", () => {
    // Without this a typo leaves a check that passes by matching nothing, which is worse than no
    // check: it reports the rule as held.
    expect(STATE_VARIANT_DIMENSION.test('className="hover:px-3"')).toBe(true);
    expect(STATE_VARIANT_DIMENSION.test('className="hover:bg-muted"')).toBe(false);
    expect(CONDITIONAL_DIMENSION.test('cn("row", selected && "py-2")')).toBe(true);
    expect(CONDITIONAL_DIMENSION.test('cn("row", selected && "bg-muted text-primary")')).toBe(false);
  });

  it("never changes a dimension on hover, focus, or any other variant", () => {
    const offenders = consoleModules()
      .filter(({ source }) => STATE_VARIANT_DIMENSION.test(source))
      .map(({ name }) => name);

    expect(offenders).toEqual([]);
  });

  it("never lets a condition add spacing of its own", () => {
    const offenders = consoleModules()
      .filter(({ source }) => CONDITIONAL_DIMENSION.test(source))
      .map(({ name }) => name);

    // A structural class is fine — indentation by depth is not a state. What this catches is a
    // class added *because* something is selected, loading, or failing.
    expect(offenders).toEqual([]);
  });

  it("gives loading, empty, unavailable, and error the same box", () => {
    const boxes = (["loading", "empty", "unavailable", "error"] as const).map((kind) => {
      const { container, unmount } = render(<WorkspaceState kind={kind} />);
      const box = (container.firstElementChild as HTMLElement).className;
      const icon = container.querySelector("svg")?.getAttribute("class") ?? "";
      unmount();
      // Only the classes that decide the box. Loading draws a different glyph and spins it, and
      // both of those are repaints -- comparing the whole class list would fail for the two things
      // this rule permits and say nothing about the one it forbids.
      return { box: sizeOf(box), icon: sizeOf(icon) };
    });

    // One container size and one icon size for all four. A panel that shrank when it finished
    // loading would move everything under it at the moment the reader started reading.
    expect(new Set(boxes.map((entry) => entry.box)).size).toBe(1);
    expect(new Set(boxes.map((entry) => entry.icon)).size).toBe(1);
  });
});

describe("the tab badges", () => {
  it("keeps one size for every tone and for the unknown marker", async () => {
    const { SessionTabBar } = await import("./session-tab-bar");
    const sizes = new Set<string>();

    for (const badge of [
      { atLeast: false, count: 3, kind: "count" as const, tone: "neutral" as const },
      { atLeast: false, count: 3, kind: "count" as const, tone: "danger" as const },
      { kind: "unknown" as const, reason: "partial" as const },
    ]) {
      const { container, unmount } = renderWithAppProviders(
        <SessionTabBar
          activeTab="logs"
          badges={{ logs: badge }}
          onActivate={() => {}}
          onOpenSettings={() => {}}
          session={null}
        />,
      );
      const marker = container.querySelector("[data-badge]");
      sizes.add(sizeOf(marker?.className ?? ""));
      unmount();
    }

    // A red badge and a grey one are the same badge in two colours. A danger tone that also grew
    // would shift the tabs beside it at the moment something went wrong -- which is the moment a
    // reader is least able to afford the row moving under their pointer.
    expect(sizes.size).toBe(1);
  });
});
