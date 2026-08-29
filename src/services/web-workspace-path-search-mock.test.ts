import { afterEach, describe, expect, it } from "vitest";
import type { WorkspacePathMatch } from "../types/session-workspace-inspection";
import { runWebPathSearch } from "./web-workspace-path-search-mock";
import { resetWebWorkspaceSearch } from "./web-workspace-search-registry";

const fixture: WorkspacePathMatch[] = [
  "src/main.rs",
  "src/lib.rs",
  "docs/main.md",
  "README.md",
].map((path) => ({ kind: "file", name: path.split("/").pop() ?? path, path }));

afterEach(() => {
  resetWebWorkspaceSearch();
});

describe("web path search", () => {
  it("pages the matches and stops issuing a cursor when they run out", async () => {
    const first = await runWebPathSearch(
      { query: "main", searchId: "q", limit: 1 },
      fixture,
    );

    expect(first.matches.map((match) => match.path)).toEqual(["src/main.rs"]);
    expect(first.nextCursor).toBeTruthy();

    const second = await runWebPathSearch(
      { query: "main", searchId: "q", limit: 1, cursor: first.nextCursor },
      fixture,
    );

    expect(second.matches.map((match) => match.path)).toEqual(["docs/main.md"]);
    // A cursor for an exhausted result set would invite a caller to fetch a page that is always
    // empty, and an empty page reads as a search that just stopped finding things.
    expect(second.nextCursor).toBeUndefined();
  });

  it("refuses a cursor issued for another query", async () => {
    const first = await runWebPathSearch({ query: "main", searchId: "q", limit: 1 }, fixture);

    const refused = await runWebPathSearch(
      { query: "lib", searchId: "q", limit: 1, cursor: first.nextCursor },
      fixture,
    );

    // The same file ranks differently under a different query, so this cursor names a position the
    // new ordering never produced. `invalid_cursor` is the same code the native adapter uses, which
    // is what lets one panel restart correctly against either.
    expect(refused.matches).toEqual([]);
    expect(refused.coverage.reasonCode).toBe("invalid_cursor");
    expect(refused.nextCursor).toBeUndefined();
  });

  it("refuses a cursor nobody issued", async () => {
    const refused = await runWebPathSearch(
      { query: "main", searchId: "q", cursor: "not-a-cursor" },
      fixture,
    );

    expect(refused.coverage.reasonCode).toBe("invalid_cursor");
  });

  it("shares one generation counter with content search", async () => {
    const first = await runWebPathSearch({ query: "main", searchId: "q" }, fixture);
    const second = await runWebPathSearch({ query: "lib", searchId: "q" }, fixture);

    // One registry for both kinds, because the native side has one. Two counters here would let a
    // Quick Open and a content search look concurrent in the browser and supersede each other on
    // the desktop.
    expect(second.generation).toBeGreaterThan(first.generation);
  });

  it("drops a superseded page rather than returning a cursor into an old ordering", async () => {
    // Started and then replaced before it resolves. The matches are for a query the reader has
    // already retyped, and the cursor beside them names a rank in that older ordering — following
    // it would page the new query's list from a position it never had.
    const stale = runWebPathSearch({ query: "main", searchId: "q", limit: 1 }, fixture);
    const fresh = runWebPathSearch({ query: "lib", searchId: "q", limit: 1 }, fixture);

    const late = await stale;
    await fresh;

    expect(late.coverage.reasonCode).toBe("superseded");
    expect(late.matches).toEqual([]);
    expect(late.nextCursor).toBeUndefined();
  });
});
