import type { SessionLogEntry, SessionLogPage } from "../types/session-workspace";

export const maxTimestampSeekPages = 10;

export interface TimestampSeekResult {
  entries: SessionLogEntry[];
  hasMore: boolean;
  matchIndex: number;
  nextCursor: string | null;
  status: "continue" | "found" | "not-found";
}

export function appendUniqueLogs(
  current: readonly SessionLogEntry[],
  incoming: readonly SessionLogEntry[],
) {
  const ids = new Set(current.map((entry) => entry.id));
  const appended: SessionLogEntry[] = [];
  for (const entry of incoming) {
    if (ids.has(entry.id)) continue;
    ids.add(entry.id);
    appended.push(entry);
  }
  return [...current, ...appended];
}

export function findLogIndexAtOrBefore(
  entries: readonly SessionLogEntry[],
  targetTimestamp: number,
) {
  return entries.findIndex((entry) => {
    const timestamp = Date.parse(entry.timestamp);
    return Number.isFinite(timestamp) && timestamp <= targetTimestamp;
  });
}

export function isTimestampNewerThanLogs(
  entries: readonly SessionLogEntry[],
  targetTimestamp: number,
) {
  const newestTimestamp = Date.parse(entries[0]?.timestamp ?? "");
  return Number.isFinite(newestTimestamp) && targetTimestamp > newestTimestamp;
}

export function parseTimestampInput(value: string) {
  if (!value.trim()) return null;
  const timestamp = Date.parse(value);
  return Number.isFinite(timestamp) ? timestamp : null;
}

export async function seekLogsByTimestamp({
  entries,
  hasMore,
  loadPage,
  maxPages = maxTimestampSeekPages,
  nextCursor,
  targetTimestamp,
}: {
  entries: readonly SessionLogEntry[];
  hasMore: boolean;
  loadPage: (cursor: string) => Promise<SessionLogPage>;
  maxPages?: number;
  nextCursor: string | null;
  targetTimestamp: number;
}): Promise<TimestampSeekResult> {
  let combinedEntries = [...entries];
  let cursor = nextCursor;
  let more = hasMore;
  let matchIndex = findLogIndexAtOrBefore(combinedEntries, targetTimestamp);

  for (let pageIndex = 0; matchIndex < 0 && more && cursor && pageIndex < maxPages; pageIndex += 1) {
    const page = await loadPage(cursor);
    combinedEntries = appendUniqueLogs(combinedEntries, page.items);
    cursor = page.nextCursor;
    more = page.truncated;
    matchIndex = findLogIndexAtOrBefore(combinedEntries, targetTimestamp);
  }

  return {
    entries: combinedEntries,
    hasMore: more,
    matchIndex,
    nextCursor: cursor,
    status: matchIndex >= 0 ? "found" : more && cursor ? "continue" : "not-found",
  };
}
