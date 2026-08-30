import type {
  WorkspacePathMatch,
  WorkspacePathSearchResult,
} from "../types/session-workspace-inspection";
import {
  claimWebSearchGeneration,
  releaseWebSearchGeneration,
  webSearchGenerationIsCurrent,
  webSearchSkipsPath,
} from "./web-workspace-search-registry";

/**
 * The browser build's Quick Open, with the one refusal Quick Open can actually hit.
 *
 * A path cursor is bound to the query, because a search's ordering is a function of what was typed —
 * the same file scores differently for `main` and for `ma`. A cursor applied to a different query
 * names a rank that ordering never produced, so the page it would return comes from the middle of a
 * different result set. Simulated here so a panel written against this adapter restarts the same way
 * it has to against the native one.
 */

const PATH_CURSOR_VERSION = "web-path-v1";

/** How many matches one page holds when the caller does not say. The native default. */
const DEFAULT_PATH_RESULTS = 25;

function encodePathCursor(query: string, after: number): string {
  return btoa(JSON.stringify({ version: PATH_CURSOR_VERSION, query, after }));
}

/** The offset a cursor resumes at, or `null` when it does not belong to this query. */
function decodePathCursor(encoded: string, query: string): number | null {
  try {
    const parsed = JSON.parse(atob(encoded)) as { version?: string; query?: string; after?: number };
    if (parsed.version !== PATH_CURSOR_VERSION) return null;
    if (parsed.query !== query || typeof parsed.after !== "number") return null;
    return parsed.after;
  } catch {
    return null;
  }
}

export interface WebPathSearchInput {
  query: string;
  searchId: string;
  cursor?: string;
  limit?: number;
}

/**
 * One page of a simulated path search.
 *
 * Shares the generation counter with content search, because the native side shares one registry
 * between them. Two independent counters here would let a Quick Open and a content search look
 * concurrent in the browser and supersede each other on the desktop, which is exactly the kind of
 * difference a panel is written against without noticing.
 */
export async function runWebPathSearch(
  input: WebPathSearchInput,
  fixture: readonly WorkspacePathMatch[],
): Promise<WorkspacePathSearchResult> {
  const generation = claimWebSearchGeneration(input.searchId);
  try {
    const needle = input.query.trim().toLowerCase();
    const after = input.cursor ? decodePathCursor(input.cursor, needle) : 0;
    if (after === null) {
      // A cursor issued for another query names a rank this ordering never produced. An answer
      // rather than a rejection, and the same reason code the native adapter uses, so a caller
      // written against one adapter restarts correctly against the other.
      return {
        generation,
        coverage: { state: "unavailable", reasonCode: "invalid_cursor" },
        matches: [],
      };
    }
    const eligible = fixture.filter(
      (entry) =>
        // The same rule content search applies. Two walks with their own lists is how a file
        // becomes findable by name and not by content, which reads as the search being broken for
        // that one file.
        !webSearchSkipsPath(entry.path) &&
        (!needle || entry.path.toLowerCase().includes(needle)),
    );
    // Yielded once before the answer is decided. A synchronous mock would finish before a second
    // request could be issued, so supersession would be unreachable here and the parity it exists
    // to hold would be untestable — the fixture reporting the mechanism works because it was too
    // fast to use.
    await Promise.resolve();
    const limit = input.limit ?? DEFAULT_PATH_RESULTS;
    const matches = eligible.slice(after, after + limit);
    const next = after + matches.length;
    // Dropped once a newer request holds the id, exactly as the native delivery rule does: these
    // matches are for a query the reader has already retyped, and the cursor beside them names a
    // rank in that older ordering.
    if (!webSearchGenerationIsCurrent(input.searchId, generation)) {
      return {
        generation,
        coverage: { state: "partial", reasonCode: "superseded" },
        matches: [],
      };
    }
    return {
      generation,
      coverage: { state: "complete" },
      matches: [...matches],
      nextCursor: next < eligible.length ? encodePathCursor(needle, next) : undefined,
    };
  } finally {
    releaseWebSearchGeneration(input.searchId, generation);
  }
}
