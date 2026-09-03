import type { DirectoryEntry, DirectoryListing, SessionWorkspaceContext } from "../types/session-workspace";

/**
 * The browser build's directory paging, with the same cursor rules as the desktop one.
 *
 * A mock that returned every entry in one page and no cursor would let the tree be written as though
 * a listing cannot be refused — and the first time a real cursor went stale, the tree would append a
 * page from a directory that had changed underneath it and silently drop or repeat rows.
 *
 * So the two refusals exist here as well, and mean the same things. `invalid_cursor`: this token was
 * not issued for this listing. `stale_cursor`: it was, and the directory has changed since.
 */

/** Cursors are opaque so a caller cannot hand-craft one, exactly as on the native side. */
const CURSOR_VERSION = "web-v2";

interface WebDirectoryCursor {
  version: string;
  path: string;
  fingerprint: number;
  after: number;
}

/**
 * What each directory currently looks like.
 *
 * A counter rather than a timestamp: a fixture has no filesystem to read an mtime from, and a
 * counter changes exactly when something says the directory changed — which is the only property
 * the cursor rules actually use.
 */
const fingerprints = new Map<string, number>();

function fingerprintOf(path: string): number {
  return fingerprints.get(path) ?? 1;
}

/** Marks a directory as changed, so cursors issued before this go stale. */
export function changeWebDirectory(path: string): void {
  fingerprints.set(path, fingerprintOf(path) + 1);
}

export function resetWebDirectoryPaging(): void {
  fingerprints.clear();
}

function encodeCursor(cursor: WebDirectoryCursor): string {
  return btoa(JSON.stringify(cursor));
}

function decodeCursor(encoded: string): WebDirectoryCursor | null {
  try {
    const parsed = JSON.parse(atob(encoded)) as Partial<WebDirectoryCursor>;
    if (parsed.version !== CURSOR_VERSION) return null;
    if (typeof parsed.path !== "string") return null;
    if (typeof parsed.fingerprint !== "number" || typeof parsed.after !== "number") return null;
    return parsed as WebDirectoryCursor;
  } catch {
    return null;
  }
}

function refusal(
  context: SessionWorkspaceContext,
  path: string,
  reasonCode: string,
): DirectoryListing {
  return {
    context,
    path,
    items: [],
    truncated: false,
    nextCursor: null,
    // `unavailable` rather than `partial`: nothing was examined, so there is no partial answer to
    // describe — only a request that could not be served.
    coverage: { state: "unavailable", reasonCode },
  };
}

/**
 * One page of a fixture directory.
 *
 * The page is a window on a fixed list, so ordering is whatever the fixture declares and does not
 * need re-deriving. What is simulated is the part a tree can get wrong: where the next page starts,
 * and what happens when the token naming that place no longer applies.
 */
export function pageWebDirectory(
  context: SessionWorkspaceContext,
  path: string,
  entries: DirectoryEntry[],
  cursor: string | null,
  limit: number,
): DirectoryListing {
  const fingerprint = fingerprintOf(path);
  let after = 0;

  if (cursor) {
    const decoded = decodeCursor(cursor);
    if (!decoded || decoded.path !== path) return refusal(context, path, "invalid_cursor");
    if (decoded.fingerprint !== fingerprint) return refusal(context, path, "stale_cursor");
    after = decoded.after;
  }

  const page = entries.slice(after, after + limit);
  const next = after + page.length;
  const truncated = next < entries.length;
  return {
    context,
    path,
    items: page,
    truncated,
    nextCursor: truncated
      ? encodeCursor({ version: CURSOR_VERSION, path, fingerprint, after: next })
      : null,
    // The fixture is the whole directory and it was all read, so `complete` is the honest answer.
    // Truncation is a separate fact and travels on its own flag above.
    coverage: { state: "complete" },
  };
}
