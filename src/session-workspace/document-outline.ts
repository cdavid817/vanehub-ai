/**
 * The headings in a document, and where each one is.
 *
 * Derived from the content that was already loaded rather than from a second read. The content is
 * bounded, so the outline is bounded by construction — no separate limit to keep in step with the
 * preview's, and no way for the two to disagree about what the document contains.
 *
 * Every entry carries both a line and an anchor because the two modes need different things: Source
 * scrolls to a line number, Preview scrolls to a rendered heading. One outline serving both is what
 * keeps switching modes from losing the reader's place.
 */
export interface OutlineEntry {
  /** 1 for `#`, 2 for `##`, and so on. */
  depth: number;
  text: string;
  /** 1-based, so Source mode can go straight to it. */
  line: number;
  /** The id the rendered heading carries, so Preview mode can scroll to the same place. */
  anchor: string;
}

/** How many headings one outline holds. A long document has more than a reader can use as a list. */
export const MAX_OUTLINE_ENTRIES = 200;

const ATX_HEADING = /^(#{1,6})\s+(.*?)\s*#*\s*$/;
const FENCE = /^\s*(?:```|~~~)/;

/**
 * Headings, in document order.
 *
 * Fenced code is skipped. A shell script in a Markdown block is full of `# comment` lines, and an
 * outline that listed them would be a table of contents made mostly of other people's comments.
 *
 * Only ATX headings (`# Title`). Setext underlines exist and are rare, and recognising them would
 * mean looking ahead a line on every line to catch a form almost nobody writes — the cost lands on
 * every document to serve a few.
 */
export function documentOutline(content: string): OutlineEntry[] {
  const entries: OutlineEntry[] = [];
  const used = new Map<string, number>();
  let inFence = false;

  content.split("\n").forEach((line, index) => {
    if (FENCE.test(line)) {
      inFence = !inFence;
      return;
    }
    if (inFence || entries.length >= MAX_OUTLINE_ENTRIES) return;
    const match = ATX_HEADING.exec(line);
    if (!match) return;
    const text = match[2]?.trim() ?? "";
    if (!text) return;
    entries.push({
      depth: match[1]?.length ?? 1,
      text,
      line: index + 1,
      anchor: uniqueAnchor(text, used),
    });
  });

  return entries;
}

/**
 * A stable id for a heading, unique within the document.
 *
 * Duplicates are real — "Overview" appears under three sections in half the documents anybody
 * writes — and two headings sharing an anchor would make the outline scroll to the first one every
 * time, which reads as the second entry being broken.
 */
function uniqueAnchor(text: string, used: Map<string, number>): string {
  const base =
    text
      .toLowerCase()
      .replace(/[^\p{L}\p{N}\s-]/gu, "")
      .trim()
      .replace(/\s+/g, "-") || "section";
  const seen = used.get(base) ?? 0;
  used.set(base, seen + 1);
  return seen === 0 ? base : `${base}-${seen}`;
}

/**
 * The anchor a rendered heading should carry.
 *
 * Matched by position rather than by re-deriving from the text: the renderer sees the heading's
 * children after Markdown has been parsed, which is not the same string the outline read. Counting
 * occurrences keeps the two in step without either having to reproduce the other's parsing.
 */
export function anchorAt(outline: readonly OutlineEntry[], index: number): string | undefined {
  return outline[index]?.anchor;
}
