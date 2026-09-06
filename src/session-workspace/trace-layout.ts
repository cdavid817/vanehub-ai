/**
 * Where the waterfall's two columns begin and end.
 *
 * Separated from the time scale because they answer different questions. The scale maps
 * milliseconds to pixels; this decides how many pixels there are to map into. Keeping them in one
 * file hid that the axis was being sized from the whole viewport rather than from its own column.
 */

/**
 * Width of the span-label column, and the gap after it.
 *
 * A fixed width rather than `minmax(10rem,18rem)`, because the axis width has to be *derived* from
 * it and a range cannot be subtracted. The range was also the bug: a `minmax` whose maximum is a
 * length absorbs free space ahead of a `1fr` sibling, so in a narrow container the labels took the
 * whole row and the axis column resolved to zero — every bar was rendered, correctly sized, into a
 * column with no width.
 *
 * Shared with the row component so the header ticks and the bars below them cannot disagree about
 * where the axis starts.
 */
export const LABEL_COLUMN_PX = 208;
export const LABEL_COLUMN_GAP_PX = 8;

/**
 * The narrowest the label column may become.
 *
 * The replaced `minmax(10rem,18rem)` collapsed to its 10rem minimum when space ran short, so a
 * single fixed width would have made the labels *wider* on a phone than the range they replaced —
 * on a 390px screen 208px of names leaves the axis with less room than the labels, and the axis is
 * what the view is for.
 */
export const NARROW_LABEL_COLUMN_PX = 132;

/** How much of a narrow viewport the labels may take before the maximum applies. */
const LABEL_SHARE = 0.35;

/**
 * The label column width for a viewport of `viewportWidthPx`.
 *
 * Clamped rather than stepped at a breakpoint. A step makes the axis *narrower as the panel grows*:
 * crossing the threshold moved 76px from the axis to the labels, so closing the detail drawer —
 * which widens the waterfall — made every bar visibly jump narrower. Monotonic here means widening
 * the panel can only ever widen the axis.
 */
export function labelColumnFor(viewportWidthPx: number): number {
  return Math.round(
    Math.min(LABEL_COLUMN_PX, Math.max(NARROW_LABEL_COLUMN_PX, viewportWidthPx * LABEL_SHARE)),
  );
}

/**
 * Horizontal padding each row carries, which the axis does not get.
 *
 * The rows and the tick header spell this and the column gap as literal pixels (`px-[4px]`,
 * `gap-[8px]`) rather than Tailwind's rem-based `px-1`/`gap-2`. The app lets the reader change the
 * root font size, and a rem-based gap would then no longer be the 8px this arithmetic subtracts —
 * leaving a permanent sliver of horizontal scroll at one setting and an overshooting bar at
 * another, from a derivation whose whole purpose is that the two agree.
 */
const ROW_PADDING_PX = 8;

/**
 * The axis width available inside a viewport of `viewportWidthPx`.
 *
 * Subtracts the row padding as well as the label column: a full-run bar computed against the
 * unpadded width overshoots its own track by exactly that much, which reads as a run whose last
 * span continues past the end of the axis.
 */
export function axisWidthFor(viewportWidthPx: number): number {
  return Math.max(
    1,
    viewportWidthPx - labelColumnFor(viewportWidthPx) - LABEL_COLUMN_GAP_PX - ROW_PADDING_PX,
  );
}

/**
 * The scrollable content width that gives an axis of `contentWidthPx` exactly that much room.
 *
 * The inverse of `axisWidthFor`, and it has to stay that way. When the two disagree the padding is
 * subtracted twice — once from the axis and once from the row it sits in — and the widest bar
 * overshoots its track by that difference at every viewport too narrow to avoid scrolling.
 */
export function contentMinWidthFor(viewportWidthPx: number, contentWidthPx: number): number {
  return (
    labelColumnFor(viewportWidthPx) + LABEL_COLUMN_GAP_PX + ROW_PADDING_PX + contentWidthPx
  );
}
