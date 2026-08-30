import type { WorkbenchSearchResult } from "./command-center-types";

/**
 * 6.11's five named signals, each a bounded score added on top of the last — order matters more
 * than magnitude, so the gaps are large enough that no combination of the lower signals can outrank
 * a single higher one. Never reads `keywords`/prompt/response content (design.md Decision 4's
 * privacy rule) — only `title`, `updatedAt`, and `subtitle` (a project path/label, not free text)
 * ever factor into a score.
 */
const EXACT_TITLE = 1_000;
const PREFIX_TITLE = 500;
const CURRENT_PROJECT = 100;
const NEEDS_ATTENTION = 50;
/** Recency is a tiebreaker only — capped well under the smallest named signal above it. */
const MAX_RECENCY = 10;

function recencyScore(updatedAt: string | undefined, now: number): number {
  if (!updatedAt) return 0;
  const ageMs = now - Date.parse(updatedAt);
  if (!Number.isFinite(ageMs) || ageMs < 0) return 0;
  const ageDays = ageMs / (24 * 60 * 60 * 1000);
  // Halves roughly every 3 days; asymptotes to 0 rather than going negative for old results.
  return MAX_RECENCY / (1 + ageDays / 3);
}

function needsAttention(result: WorkbenchSearchResult): boolean {
  return result.status === "attention" || result.status === "error";
}

/**
 * "Substring" is not one of 6.11's five named scoring tiers, and earns no score boost of its own —
 * but it must still count as a *match* (included, just unboosted), or a query like "auth" would
 * drop "fix null auth token" entirely for not starting with it. Only "none" excludes a result.
 */
function titleMatch(title: string, normalizedQuery: string): "exact" | "prefix" | "substring" | "none" {
  const normalizedTitle = title.toLowerCase();
  if (normalizedTitle === normalizedQuery) return "exact";
  if (normalizedTitle.startsWith(normalizedQuery)) return "prefix";
  if (normalizedTitle.includes(normalizedQuery)) return "substring";
  return "none";
}

/**
 * "Current-project" (6.11) needs *some* notion of what the reader is currently looking at to be
 * meaningful — `currentProjectPath` is that one signal, optional because not every caller has it
 * (e.g. ranking with no active session yet). Matches on `subtitle`, the one field every provider
 * populates with a project-identifying string (a path, in practice — session/project providers
 * both set it that way) rather than free text.
 *
 * A non-empty query that doesn't even substring-match the title drops the result entirely, not
 * just ranks it last: a wholly unrelated result sorted by recency alone would read as "everything,
 * vaguely ordered" rather than "no matches" — the one behavior a query-less browse (empty `query`,
 * kept unfiltered) and a real search share the type signature for but not the semantics.
 */
export function rankSearchResults(
  results: WorkbenchSearchResult[],
  query: string,
  options: { currentProjectPath?: string | null; now?: number } = {},
): WorkbenchSearchResult[] {
  const now = options.now ?? Date.now();
  const normalizedQuery = query.trim().toLowerCase();
  return results
    .map((result) => {
      const match = normalizedQuery ? titleMatch(result.title, normalizedQuery) : "none";
      let score = 0;
      if (match === "exact") score += EXACT_TITLE;
      else if (match === "prefix") score += PREFIX_TITLE;
      if (options.currentProjectPath && result.subtitle === options.currentProjectPath) score += CURRENT_PROJECT;
      if (needsAttention(result)) score += NEEDS_ATTENTION;
      score += recencyScore(result.updatedAt, now);
      return { result, score, matches: !normalizedQuery || match !== "none" };
    })
    .filter((entry) => entry.matches)
    // `Array.prototype.sort` is stable (spec-guaranteed since ES2019) — a tie keeps each
    // provider's own relative order rather than reshuffling arbitrarily.
    .sort((a, b) => b.score - a.score)
    .map((entry) => entry.result);
}
