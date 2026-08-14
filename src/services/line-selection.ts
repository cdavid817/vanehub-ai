import type { MentionLineRange } from "./composer-mention";

/**
 * A selection in progress. `extent` is null after the first click, which is what makes
 * "click one line and confirm" yield a one-line range.
 */
export interface PendingLineSelection {
  anchor: number;
  extent: number | null;
}

/**
 * First click anchors, second completes, a third starts over. Extending an existing
 * selection would need a modifier and a rule for which end moves; restarting is the
 * behavior with no ambiguity.
 */
export function advanceLineSelection(
  current: PendingLineSelection | null,
  line: number,
): PendingLineSelection {
  if (current === null || current.extent !== null) return { anchor: line, extent: null };
  return { anchor: current.anchor, extent: line };
}

/** Normalises the two ends so clicking bottom-up and top-down give the same range. */
export function selectionToRange(selection: PendingLineSelection | null): MentionLineRange {
  if (selection === null) return {};
  const other = selection.extent ?? selection.anchor;
  return {
    startLine: Math.min(selection.anchor, other),
    endLine: Math.max(selection.anchor, other),
  };
}

export function isLineSelected(selection: PendingLineSelection | null, line: number): boolean {
  const { startLine, endLine } = selectionToRange(selection);
  return startLine !== undefined && endLine !== undefined && line >= startLine && line <= endLine;
}
