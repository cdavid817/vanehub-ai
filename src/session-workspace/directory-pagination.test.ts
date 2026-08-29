import { describe, expect, it, vi } from "vitest";
import type { DirectoryEntry, DirectoryListing } from "../types/session-workspace";
import type { WorkspaceSearchCoverage } from "../types/session-workspace-inspection";
import {
  collectDirectoryPages,
  MAX_DIRECTORY_PAGES,
  type DirectoryPageFetcher,
} from "./directory-pagination";

const CONTEXT = { availability: "available" as const, rootName: "project", reason: null };

function entry(name: string): DirectoryEntry {
  return { kind: "file", name, path: name, size: 1 };
}

function page(
  items: DirectoryEntry[],
  nextCursor: string | null = null,
  coverage: WorkspaceSearchCoverage = { state: "complete" },
): DirectoryListing {
  return { context: CONTEXT, path: "", items, truncated: Boolean(nextCursor), nextCursor, coverage };
}

describe("collecting a directory across pages", () => {
  it("follows the cursor to the end of the folder", async () => {
    const fetchPage = vi
      .fn<DirectoryPageFetcher>()
      .mockResolvedValueOnce(page([entry("a")], "c1"))
      .mockResolvedValueOnce(page([entry("b")]));

    const listing = await collectDirectoryPages(fetchPage);

    expect(listing.items.map((item) => item.name)).toEqual(["a", "b"]);
    expect(listing.truncated).toBe(false);
    // Consumed here. A caller holding one could resume a listing this already stopped following.
    expect(listing.nextCursor).toBeNull();
    expect(fetchPage.mock.calls).toEqual([[null], ["c1"]]);
  });

  it("stops at its page bound and says there is more", async () => {
    // A generated directory can hold hundreds of thousands of entries, and following it to the end
    // would put all of them in a tree nobody can read.
    const fetchPage = vi
      .fn<DirectoryPageFetcher>()
      .mockImplementation((cursor) => Promise.resolve(page([entry(`e${cursor ?? "0"}`)], "more")));

    const listing = await collectDirectoryPages(fetchPage);

    expect(fetchPage).toHaveBeenCalledTimes(MAX_DIRECTORY_PAGES);
    expect(listing.truncated).toBe(true);
  });

  it("restarts rather than appending across a stale cursor", async () => {
    // The defect this exists for. The folder changed between two pages, so "resume after this entry"
    // no longer names the same position — appending there drops or repeats rows with nothing on
    // screen to say so.
    const fetchPage = vi
      .fn<DirectoryPageFetcher>()
      .mockResolvedValueOnce(page([entry("a")], "c1"))
      .mockResolvedValueOnce(page([], null, { state: "unavailable", reasonCode: "stale_cursor" }))
      .mockResolvedValueOnce(page([entry("a2")], "c2"))
      .mockResolvedValueOnce(page([entry("b2")]));

    const listing = await collectDirectoryPages(fetchPage);

    // Everything from the second attempt, and nothing from the first.
    expect(listing.items.map((item) => item.name)).toEqual(["a2", "b2"]);
    expect(listing.coverage.state).toBe("complete");
    expect(fetchPage.mock.calls).toEqual([[null], ["c1"], [null], ["c2"]]);
  });

  it("gives up after one restart rather than spinning against a folder being written to", async () => {
    const fetchPage = vi
      .fn<DirectoryPageFetcher>()
      .mockImplementation((cursor) =>
        Promise.resolve(
          cursor === null
            ? page([entry("a")], "c1")
            : page([], null, { state: "unavailable", reasonCode: "stale_cursor" }),
        ),
      );

    const listing = await collectDirectoryPages(fetchPage);

    // Two attempts, not an unbounded retry: a build running in that folder would otherwise be asked
    // harder and harder for an answer nothing is going to give.
    expect(fetchPage.mock.calls).toEqual([[null], ["c1"], [null], ["c1"]]);
    // The entries read before the refusal are kept — they are the folder's real contents as far as
    // they go — and the reason says the list is not the whole folder.
    expect(listing.items.map((item) => item.name)).toEqual(["a"]);
    expect(listing.truncated).toBe(true);
    expect(listing.coverage.reasonCode).toBe("stale_cursor");
  });

  it("does not restart on a refusal for the very first request", async () => {
    // Nothing was resumed, so there is no cursor to blame. Asking again would put the identical
    // question and get the identical answer.
    const fetchPage = vi
      .fn<DirectoryPageFetcher>()
      .mockResolvedValue(page([], null, { state: "unavailable", reasonCode: "invalid_cursor" }));

    const listing = await collectDirectoryPages(fetchPage);

    expect(fetchPage).toHaveBeenCalledTimes(1);
    expect(listing.coverage.reasonCode).toBe("invalid_cursor");
  });

  it("carries an incomplete scan through untouched", async () => {
    // A budget stop is not a paging problem and restarting would not help: the scan would spend the
    // same budget over the same prefix and stop in the same place.
    const fetchPage = vi
      .fn<DirectoryPageFetcher>()
      .mockResolvedValue(page([entry("a")], null, {
        state: "partial",
        reasonCode: "entry_budget_exhausted",
      }));

    const listing = await collectDirectoryPages(fetchPage);

    expect(fetchPage).toHaveBeenCalledTimes(1);
    expect(listing.items.map((item) => item.name)).toEqual(["a"]);
    expect(listing.coverage.reasonCode).toBe("entry_budget_exhausted");
  });
});
