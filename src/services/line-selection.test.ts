import { describe, expect, it } from "vitest";
import { advanceLineSelection, isLineSelected, selectionToRange } from "./line-selection";

describe("line selection", () => {
  it("anchors on the first click and yields a one-line range", () => {
    const first = advanceLineSelection(null, 42);
    expect(first).toEqual({ anchor: 42, extent: null });
    expect(selectionToRange(first)).toEqual({ startLine: 42, endLine: 42 });
  });

  it("completes on the second click", () => {
    const selection = advanceLineSelection(advanceLineSelection(null, 10), 50);
    expect(selectionToRange(selection)).toEqual({ startLine: 10, endLine: 50 });
  });

  it("gives the same range regardless of click order", () => {
    const downward = advanceLineSelection(advanceLineSelection(null, 10), 50);
    const upward = advanceLineSelection(advanceLineSelection(null, 50), 10);
    expect(selectionToRange(downward)).toEqual(selectionToRange(upward));
  });

  it("restarts on the third click rather than extending", () => {
    const completed = advanceLineSelection(advanceLineSelection(null, 10), 50);
    const third = advanceLineSelection(completed, 80);
    expect(third).toEqual({ anchor: 80, extent: null });
    expect(selectionToRange(third)).toEqual({ startLine: 80, endLine: 80 });
  });

  it("reports no range when nothing is selected", () => {
    expect(selectionToRange(null)).toEqual({});
    expect(isLineSelected(null, 1)).toBe(false);
  });

  it("reports which lines fall inside the selection", () => {
    const selection = advanceLineSelection(advanceLineSelection(null, 50), 10);
    expect([9, 10, 30, 50, 51].map((line) => isLineSelected(selection, line))).toEqual([
      false,
      true,
      true,
      true,
      false,
    ]);
  });
});
