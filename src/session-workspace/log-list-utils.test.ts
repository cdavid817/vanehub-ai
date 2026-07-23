import { describe, expect, it, vi } from "vitest";
import type { SessionLogEntry } from "../types/session-workspace";
import {
  appendUniqueLogs,
  findLogIndexAtOrBefore,
  isTimestampNewerThanLogs,
  maxTimestampSeekPages,
  parseTimestampInput,
  seekLogsByTimestamp,
} from "./log-list-utils";

describe("session log list utilities", () => {
  it("de-duplicates appended pages and locates the first entry at or before a timestamp", () => {
    const entries = [log("3", 3), log("2", 2)];
    expect(appendUniqueLogs(entries, [log("2", 2), log("1", 1)]).map((entry) => entry.id))
      .toEqual(["3", "2", "1"]);
    expect(findLogIndexAtOrBefore(entries, timestamp(2))).toBe(1);
    expect(isTimestampNewerThanLogs(entries, timestamp(4))).toBe(true);
    expect(parseTimestampInput("not-a-date")).toBeNull();
  });

  it("stops after ten pages and returns a continuation state", async () => {
    const loadPage = vi.fn(async (cursor: string) => ({
      items: [log(cursor, 99 - Number(cursor))],
      nextCursor: String(Number(cursor) + 1),
      truncated: true,
    }));

    const result = await seekLogsByTimestamp({
      entries: [log("initial", 100)],
      hasMore: true,
      loadPage,
      nextCursor: "0",
      targetTimestamp: timestamp(1),
    });

    expect(maxTimestampSeekPages).toBe(10);
    expect(loadPage).toHaveBeenCalledTimes(10);
    expect(result.status).toBe("continue");
    expect(result.entries).toHaveLength(11);
  });

  it("returns found and exhausted no-match results without extra reads", async () => {
    const foundLoader = vi.fn();
    const found = await seekLogsByTimestamp({
      entries: [log("3", 3), log("2", 2)],
      hasMore: true,
      loadPage: foundLoader,
      nextCursor: "next",
      targetTimestamp: timestamp(2),
    });
    expect(found.status).toBe("found");
    expect(found.matchIndex).toBe(1);
    expect(foundLoader).not.toHaveBeenCalled();

    const exhausted = await seekLogsByTimestamp({
      entries: [log("3", 3)],
      hasMore: true,
      loadPage: async () => ({ items: [log("2", 2)], nextCursor: null, truncated: false }),
      nextCursor: "next",
      targetTimestamp: timestamp(1),
    });
    expect(exhausted.status).toBe("not-found");
    expect(exhausted.hasMore).toBe(false);
  });
});

function log(id: string, minute: number): SessionLogEntry {
  return {
    id,
    timestamp: new Date(timestamp(minute)).toISOString(),
    level: "info",
    category: "test",
    message: id,
    context: {},
  };
}

function timestamp(minute: number) {
  return Date.UTC(2026, 0, 1, 0, minute);
}
