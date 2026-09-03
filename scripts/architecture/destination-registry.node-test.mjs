import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { workbenchDestinations } from "../../src/main-layout/workbench-route.ts";

// Task 21.4: "destination registry" architecture coverage. Unlike Settings (settingsPages, checked
// by src/settings/settings-pages-architecture.test.ts) there is no single array-of-objects registry
// for the 5 top-level workbench destinations — `main-layout.tsx` wires each one with its own
// `location.destination === "<id>" ? <XDestination /> : null` ternary, which gives no compiler-
// enforced exhaustiveness the way `DESTINATION_LIFECYCLE`'s `Record<WorkbenchDestination, ...>`
// (destination-lifecycle.ts) already gets from `tsc --noEmit` alone. This file is what actually
// catches a destination added to `workbenchDestinations` without a live render branch (or the
// reverse: a stale branch left behind for a destination id that no longer exists), reading the real
// source files rather than fixtures.

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const mainLayoutPath = path.join(projectRoot, "src/main-layout/main-layout.tsx");
const mainLayoutSource = fs.readFileSync(mainLayoutPath, "utf8");
const activityBarPath = path.join(projectRoot, "src/main-layout/workspace-activity-bar.tsx");
const activityBarSource = fs.readFileSync(activityBarPath, "utf8");

// Sessions is the one destination `main-layout.tsx` documents (see its own comments around
// "workbench-route-outlet") as deliberately *not* going through the conditional-render ternary:
// it keeps `DestinationLayout` permanently mounted and toggles a CSS `hidden` class instead,
// specifically so the composer's in-progress draft survives switching away and back
// (`DESTINATION_LIFECYCLE.sessions` is `keepAlive: "draft-only"`, destination-lifecycle.ts). Naming
// it here keeps the generic per-destination checks below from misreporting a real, intentional
// difference as an unregistered component.
const UNCONDITIONALLY_RENDERED_DESTINATIONS = new Set(["sessions"]);

// The real, current naming convention for the 4 conditionally-rendered destinations — confirmed by
// reading main-layout.tsx's own import block directly (plan-destination.tsx exports
// PlanDestination, projects-destination.tsx exports ProjectsDestination, etc.), not assumed from
// the pattern alone.
function componentNameFor(destinationId) {
  return `${destinationId[0].toUpperCase()}${destinationId.slice(1)}Destination`;
}

const registeredDestinations = workbenchDestinations.filter((id) => !UNCONDITIONALLY_RENDERED_DESTINATIONS.has(id));

test("workbenchDestinations has not silently grown past the 5 stable business domains design.md Decision 1 names", () => {
  assert.deepEqual(workbenchDestinations, ["sessions", "projects", "runs", "plan", "quality"]);
});

for (const id of registeredDestinations) {
  test(`"${id}" has its own <${componentNameFor(id)}> component file exporting a real component`, () => {
    const file = path.join(projectRoot, "src/main-layout", `${id}-destination.tsx`);
    assert.ok(fs.existsSync(file), `expected ${file} to exist for destination "${id}"`);
    const source = fs.readFileSync(file, "utf8");
    assert.match(
      source,
      new RegExp(`export function ${componentNameFor(id)}\\b`),
      `expected ${file} to export function ${componentNameFor(id)}`,
    );
  });

  test(`main-layout.tsx imports <${componentNameFor(id)}> from ./${id}-destination`, () => {
    const componentName = componentNameFor(id);
    assert.match(
      mainLayoutSource,
      new RegExp(`import\\s*\\{[^}]*\\b${componentName}\\b[^}]*\\}\\s*from\\s*["']\\./${id}-destination["']`),
      `expected main-layout.tsx to import { ${componentName} } from "./${id}-destination"`,
    );
  });

  test(`main-layout.tsx renders <${componentNameFor(id)}> gated by location.destination === "${id}"`, () => {
    const componentName = componentNameFor(id);
    // Bounded lookahead, not an unbounded one: this only needs to prove the render sits close to
    // its own gate, not that it is the literal next character — real JSX has attributes and
    // whitespace between the ternary and the opening tag.
    assert.match(
      mainLayoutSource,
      new RegExp(`location\\.destination === ["']${id}["'][\\s\\S]{0,200}<${componentName}\\b`),
      `expected main-layout.tsx to render <${componentName} /> gated by location.destination === "${id}"`,
    );
  });

  test(`workspace-activity-bar.tsx's label contract has a "${id}" entry for its nav button`, () => {
    assert.match(
      activityBarSource,
      new RegExp(`^\\s*${id}:\\s*string;`, "m"),
      `expected WorkspaceActivityBarLabels in ${activityBarPath} to declare a "${id}" field`,
    );
  });
}

test("sessions renders unconditionally via the documented CSS hidden/block toggle, not a conditional destination branch", () => {
  assert.match(mainLayoutSource, /destination === ["']sessions["']\s*\?\s*["']block["']\s*:\s*["']hidden["']/);
});

test("main-layout.tsx has no destination === \"<id>\" check for an id outside workbenchDestinations (no orphaned branch)", () => {
  const known = new Set(workbenchDestinations);
  const pattern = /(?:location\.)?destination === ["']([a-zA-Z0-9-]+)["']/g;
  const found = new Set();
  for (const match of mainLayoutSource.matchAll(pattern)) found.add(match[1]);
  // A destination-registry test that silently found nothing would pass for the wrong reason
  // (a rewritten file whose comparisons no longer match this regex at all) — assert real findings
  // exist before asserting they are all known, so a regex drift shows up as a failure, not a no-op.
  assert.ok(found.size > 0, "expected to find at least one destination === \"...\" check in main-layout.tsx");
  assert.deepEqual([...found].filter((id) => !known.has(id)).sort(), []);
});
