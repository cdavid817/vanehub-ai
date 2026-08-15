import type { FileSearchMatch } from "../types/session-workspace";

// Mirrors the native scoring tiers so the Web/mock adapter returns the same ordering
// contract as the desktop runtime. Keep in step with session_search.rs.
const SCORE_EXACT = 100;
const SCORE_PREFIX = 80;
const SCORE_SUBSTRING = 60;
const SCORE_PATH = 40;

function scoreText(query: string, text: string): number | null {
  if (text === query) return SCORE_EXACT;
  if (text.startsWith(query)) return SCORE_PREFIX;
  if (text.includes(query)) return SCORE_SUBSTRING;
  return null;
}

function segmentsInOrder(query: string, relative: string): boolean {
  let cursor = 0;
  for (const segment of query.split("/").filter(Boolean)) {
    const offset = relative.indexOf(segment, cursor);
    if (offset === -1) return false;
    cursor = offset + segment.length;
  }
  return true;
}

export function normalizeFileSearchQuery(query: string): string {
  return query.trim().toLowerCase().replace(/\\/g, "/");
}

/** Scores a candidate against an already normalized query. `null` excludes it. */
export function scoreFileCandidate(query: string, name: string, path: string): number | null {
  if (!query) return 0;
  const loweredName = name.toLowerCase();
  const loweredPath = path.toLowerCase();
  if (query.includes("/")) {
    return scoreText(query, loweredPath) ?? (segmentsInOrder(query, loweredPath) ? SCORE_PATH : null);
  }
  return scoreText(query, loweredName) ?? (loweredPath.includes(query) ? SCORE_PATH : null);
}

function depth(path: string): number {
  return path.split("/").length - 1;
}

export function rankFileCandidates(query: string, candidates: FileSearchMatch[], limit: number): FileSearchMatch[] {
  const normalized = normalizeFileSearchQuery(query);
  const scored: { candidate: FileSearchMatch; score: number }[] = [];
  for (const candidate of candidates) {
    const score = scoreFileCandidate(normalized, candidate.name, candidate.path);
    if (score !== null) scored.push({ candidate, score });
  }
  // Byte-order path comparison rather than localeCompare, matching the native tie-break.
  scored.sort((left, right) => {
    if (right.score !== left.score) return right.score - left.score;
    const depthDelta = depth(left.candidate.path) - depth(right.candidate.path);
    if (depthDelta !== 0) return depthDelta;
    const leftPath = left.candidate.path.toLowerCase();
    const rightPath = right.candidate.path.toLowerCase();
    return leftPath < rightPath ? -1 : leftPath > rightPath ? 1 : 0;
  });
  return scored.slice(0, Math.max(1, limit)).map((entry) => entry.candidate);
}
