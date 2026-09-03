// @vitest-environment jsdom

import { useState } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import "../i18n";
import { loopDefinitionFixture, loopRunFixture } from "../test/loop-fixtures";
import { DestinationLayoutBody } from "../ui/destination-layout/DestinationLayoutBody";
import { useLoopNavigationRegion } from "./loop-center-regions";

/**
 * 17.16: keyboard/focus coverage for Loop Center's own navigation region specifically -- not a
 * duplicate of the shared mechanism's own tests. `useFocusTrap`'s Tab-wrap/Escape/focus-return
 * behaviour is already exhaustively covered generically by `Sheet.test.tsx` (a synthetic
 * two-button dialog), and `DestinationLayoutBody`'s own `returnFocus` plumbing is covered
 * generically by `DestinationLayoutBody.test.tsx` (a fabricated `region()` fixture) -- Loop
 * Center's own bespoke pre-17.3 focus trap (`useDrawerFocus`) is confirmed gone (Piece A of 17.3,
 * cacf8435), replaced entirely by these two shared primitives, so re-proving either mechanism in
 * isolation here would be redundant, not a real gap.
 *
 * What neither shared test exercises is Loop Center's own real `useLoopNavigationRegion` output --
 * built from real `LoopDefinition`/`LoopRun` fixtures, rendering the real `LoopNavigation` content
 * (a create button, a conditional edit button, a conditional close button, and one row per
 * definition/run -- denser and structurally different from `Sheet.test.tsx`'s own two-button
 * stand-in) -- ever reaching a non-inline (Sheet) tier and actually trapping/returning focus
 * correctly once it does. `loop-center-responsive.test.tsx` cannot cover this: it renders via
 * `renderToStaticMarkup` (no live DOM, no focus) and, per its own comment, "always renders at its
 * initial wide tier" since jsdom's `ResizeObserver` never fires -- this repo's own established
 * convention (`DestinationLayout.test.tsx`'s comment, `DestinationLayoutBody.test.tsx`'s whole
 * file) is to cover non-wide tiers against `DestinationLayoutBody` directly with a forced `tier`
 * prop instead, which is what this file does, feeding it Loop Center's own real region builder
 * rather than another fabricated fixture.
 */
function NavigationHarness() {
  const [open, setOpen] = useState(false);
  const definition = loopDefinitionFixture();
  const navigation = useLoopNavigationRegion({
    definitions: [definition],
    loading: false,
    onCreateDefinition: () => undefined,
    onDefinitionChange: () => undefined,
    onEditDefinition: () => undefined,
    onOpenChange: setOpen,
    onRunChange: () => undefined,
    onWidthChange: () => undefined,
    open,
    runs: [loopRunFixture("running"), loopRunFixture("succeeded", { id: "run-2" })],
    selectedDefinitionId: definition.id,
    selectedRunId: null,
    tier: "narrow",
    width: 280,
  });
  return (
    <>
      {/* Stands in for `loop-center.tsx`'s own `IconButton` trigger -- not reproduced verbatim
          here since its own rendering (icon, aria-controls/aria-expanded wiring) is not what this
          test targets; only that activating some external control flips `open`, exactly like that
          button's own `onClick={() => setNavigationOpen(true)}` does. */}
      <button onClick={() => setOpen(true)} type="button">open navigation</button>
      <DestinationLayoutBody containerWidth={375} main={<main>Work surface</main>} navigation={navigation} tier="narrow" />
    </>
  );
}

describe("Loop Center navigation region keyboard behavior", () => {
  it("moves focus into the real navigation content once opened, traps Tab across it, and returns focus to the trigger on Escape", () => {
    render(<NavigationHarness />);
    expect(screen.queryByRole("dialog")).toBeNull();

    const trigger = screen.getByRole("button", { name: "open navigation" });
    trigger.focus();
    fireEvent.click(trigger);

    const dialog = screen.getByRole("dialog", { name: "循环工程" });
    // Real Loop Center content reached the sheet, not a placeholder: the definition fixture's own
    // name and the create-definition action are both real, translated `LoopNavigation` output.
    expect(screen.getByText(loopDefinitionFixture().name)).toBeTruthy();
    expect(screen.getByLabelText("新建循环定义")).toBeTruthy();
    // useFocusTrap moves focus into the dialog root itself (no `[data-dialog-autofocus]` target
    // exists in LoopNavigation) the instant it mounts, rather than leaving it on the trigger
    // outside the trapped region.
    expect(dialog.contains(document.activeElement)).toBe(true);

    // Tab-wrap against Loop Center's own real, denser content (create + edit + close + 1
    // definition row + 2 run rows = 6 focusable controls here), not a synthetic 2-button dialog.
    const focusable = Array.from(dialog.querySelectorAll<HTMLElement>("button:not([disabled])"));
    expect(focusable.length).toBeGreaterThan(2);
    const [first] = focusable;
    const last = focusable[focusable.length - 1];

    last.focus();
    fireEvent.keyDown(document, { key: "Tab" });
    expect(document.activeElement).toBe(first);

    first.focus();
    fireEvent.keyDown(document, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(last);

    // Escape closes it and returns focus to whatever was focused right before it opened
    // (`useFocusTrap`'s own fallback -- `useLoopNavigationRegion` never sets `returnFocus`,
    // confirmed by reading loop-center-regions.tsx, so this fallback is what production actually
    // relies on too, not a test-only accident).
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });

  it("keeps the navigation pane inline with no dialog at the wide tier, where it is never a Sheet", () => {
    function WideHarness() {
      const definition = loopDefinitionFixture();
      const navigation = useLoopNavigationRegion({
        definitions: [definition],
        loading: false,
        onCreateDefinition: () => undefined,
        onDefinitionChange: () => undefined,
        onEditDefinition: () => undefined,
        onOpenChange: () => undefined,
        onRunChange: () => undefined,
        onWidthChange: () => undefined,
        open: false,
        runs: [],
        selectedDefinitionId: definition.id,
        selectedRunId: null,
        tier: "wide",
        width: 280,
      });
      return <DestinationLayoutBody containerWidth={1600} main={<main>Work surface</main>} navigation={navigation} tier="wide" />;
    }
    render(<WideHarness />);
    // `navigationInline` covers wide/standard (useLoopNavigationRegion's own comment): `open`
    // false from the caller is overridden to always-visible inline content, not a closed Sheet.
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByText(loopDefinitionFixture().name)).toBeTruthy();
  });
});
