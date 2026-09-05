import type { DirectoryEntry, DirectoryListing } from "../types/session-workspace";
import { isCursorRefusal } from "./search-coverage";

/**
 * How many pages one directory is followed for.
 *
 * A bound rather than "until the cursor runs out", because a generated directory can hold hundreds
 * of thousands of entries and following it to the end would put all of them in a tree nobody can
 * read. Stopping says so through `truncated`, which is what that flag is for.
 */
export const MAX_DIRECTORY_PAGES = 4;

/**
 * How many times a refused cursor is answered by starting over.
 *
 * Once. A directory being written to while it is read can refuse a second time, and a loop that
 * restarted on every refusal would spin against a build running in that folder — asking harder and
 * harder for an answer nothing is going to give. After that the refusal is reported, which is at
 * least a fact the reader can act on.
 */
export const MAX_DIRECTORY_RESTARTS = 1;

export type DirectoryPageFetcher = (cursor: string | null) => Promise<DirectoryListing>;

/**
 * A whole directory, assembled from as many pages as the bound allows.
 *
 * The restart is the point. A cursor is a resume point in an ordering, and a directory that changed
 * between two pages is one where "resume after this entry" no longer names the same position —
 * appending the next page there would drop or repeat rows with nothing on screen to say so. So a
 * refusal discards what was collected and starts again, rather than keeping a half-listing that
 * looks complete.
 *
 * Merging here rather than in the component because the two facts a page carries have to survive the
 * merge separately: `truncated` is "this stopped before the end", coverage is "part of it was never
 * examined". A merge that kept only one of them would let a bounded read look like a finished one.
 */
export async function collectDirectoryPages(
  fetchPage: DirectoryPageFetcher,
): Promise<DirectoryListing> {
  let restarts = 0;

  for (;;) {
    const items: DirectoryEntry[] = [];
    let cursor: string | null = null;
    let page: DirectoryListing | null = null;
    let refused = false;

    for (let index = 0; index < MAX_DIRECTORY_PAGES; index += 1) {
      page = await fetchPage(cursor);
      // Only a resumed page can be refused; a refusal on the first request is about the listing
      // itself and restarting would ask the identical question again.
      if (cursor !== null && isCursorRefusal(page.coverage)) {
        refused = true;
        break;
      }
      items.push(...page.items);
      cursor = page.nextCursor;
      if (!cursor) break;
    }

    if (refused && restarts < MAX_DIRECTORY_RESTARTS) {
      restarts += 1;
      continue;
    }
    if (!page) throw new Error("a directory listing produced no page");

    return {
      ...page,
      // The entries collected before the refusal, not the refusing page's empty list. They were read
      // under a scope that held at the time and are the folder's real contents as far as they go;
      // throwing them away would show an empty directory to a reader looking at a full one. What is
      // forbidden is *appending* across the refusal, and nothing here does.
      items,
      // Still more to come: the last page said so, the page bound stopped the follow, or a refusal
      // ended it part way.
      truncated: refused || Boolean(cursor) || page.truncated,
      // Consumed here. A caller holding one could resume a listing this function has already decided
      // to stop following, which would append a page to a set it never assembled.
      nextCursor: null,
    };
  }
}
