// A mention token is the trailing `@...` run at the caret. Shared by the composer (which
// rewrites it on selection) and the layout model (which turns it into a search query), so
// both agree on exactly what the user is typing.
const MENTION_TOKEN = /(?:^|\s)@([^\s@]*)$/;

// Only digits or digits-dash-digits count as a range, so a path that happens to contain a
// colon stays a path. Session-relative paths never carry a drive letter, so `C:` cannot
// reach here in the first place.
const RANGE_SUFFIX = /^(\d+)(?:-(\d+))?$/;

export interface MentionLineRange {
  startLine?: number;
  endLine?: number;
}

export interface ParsedComposerMention extends MentionLineRange {
  path: string;
}

export function composerMentionToken(value: string): string | null {
  return value.match(MENTION_TOKEN)?.[1] ?? null;
}

/**
 * Splits `src/utils.rs:10-50` into its path and its range. Bounds are carried through as
 * written — the domain, not the composer, decides whether a range is valid, so there is one
 * source of truth for that rule.
 */
export function parseComposerMention(token: string): ParsedComposerMention {
  const separator = token.lastIndexOf(":");
  if (separator <= 0) return { path: token };
  const match = RANGE_SUFFIX.exec(token.slice(separator + 1));
  if (!match) return { path: token };
  const startLine = Number(match[1]);
  const endLine = match[2] === undefined ? startLine : Number(match[2]);
  return { path: token.slice(0, separator), startLine, endLine };
}

/** The search query for the mention at the caret: the path portion, never the range. */
export function composerMentionQuery(value: string): string | null {
  const token = composerMentionToken(value);
  return token === null ? null : parseComposerMention(token).path.toLowerCase();
}

/** Renders a range back into the `:start-end` form the composer accepts. */
export function formatMentionRange(range: MentionLineRange): string {
  if (range.startLine === undefined) return "";
  return range.endLine === range.startLine
    ? `:${range.startLine}`
    : `:${range.startLine}-${range.endLine}`;
}

/** Compact chip label for a range: `10-50`, or `42` when it is a single line. */
export function describeLineRange(range: MentionLineRange): string | null {
  if (range.startLine === undefined) return null;
  return range.endLine === range.startLine ? `${range.startLine}` : `${range.startLine}-${range.endLine}`;
}

/** Distinguishes two references to one file, which a path alone no longer can. */
export function fileReferenceId(path: string, range: MentionLineRange): string {
  return `${path}${formatMentionRange(range)}`;
}

/** Replaces the trailing mention token, preserving whichever whitespace character preceded it. */
export function replaceComposerMention(value: string, insertion: string): string {
  return value.replace(MENTION_TOKEN, (token) => {
    const leading = token.startsWith(" ") || token.startsWith("\n") ? token[0] : "";
    return `${leading}@${insertion} `;
  });
}
