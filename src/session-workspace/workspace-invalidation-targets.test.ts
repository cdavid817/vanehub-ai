import { describe, expect, it } from "vitest";
import type { WorkspaceInvalidationNotice } from "../types/session-workspace-inspection";
import {
  filterMatchesKey,
  invalidationFiltersFor,
  isWithinDirectory,
  parentDirectoryOf,
  selectionStillExists,
} from "./workspace-invalidation-targets";
import { workspaceQueryKeys } from "./workspace-query-keys";

function notice(overrides: Partial<WorkspaceInvalidationNotice> = {}): WorkspaceInvalidationNotice {
  return {
    sessionId: "session-1",
    source: "execution-evidence",
    scope: "path",
    relativePath: "src/main.rs",
    change: "modified",
    sequence: 1,
    occurredAt: "2026-08-26T09:00:00Z",
    ...overrides,
  };
}

function keysOf(value: WorkspaceInvalidationNotice) {
  return invalidationFiltersFor(value).map((filter) => JSON.stringify(filter.queryKey));
}

describe("parentDirectoryOf", () => {
  it("puts a top-level entry in the root's own listing", () => {
    // The root's key is the empty path, so this has to produce exactly that string rather than
    // something that merely looks empty.
    expect(parentDirectoryOf("readme.md")).toBe("");
    expect(parentDirectoryOf("src/main.rs")).toBe("src");
    expect(parentDirectoryOf("a/b/c.txt")).toBe("a/b");
  });
});

describe("isWithinDirectory", () => {
  it("does not treat a sibling with a shared prefix as a child", () => {
    // The failure this prevents reads like a typo: `src-generated` refreshing whenever `src`
    // changes, with nothing on screen to say why.
    expect(isWithinDirectory("src", "src-generated/main.rs")).toBe(false);
    expect(isWithinDirectory("src", "src/main.rs")).toBe(true);
    expect(isWithinDirectory("src", "src")).toBe(true);
  });

  it("treats the root as containing everything", () => {
    expect(isWithinDirectory("", "anything/at/all.txt")).toBe(true);
  });
});

describe("invalidationFiltersFor", () => {
  it("refreshes the parent listing a changed file appears in", () => {
    const keys = keysOf(notice({ relativePath: "src/main.rs" }));

    expect(keys).toContain(JSON.stringify(workspaceQueryKeys.directory("session-1", "src")));
    expect(keys).toContain(
      JSON.stringify(workspaceQueryKeys.preview("session-1", "src/main.rs")),
    );
  });

  it("leaves unrelated directories alone", () => {
    const keys = keysOf(notice({ relativePath: "src/main.rs" }));

    // The whole point of targeting. Refreshing `docs` here would collapse it in the tree and lose
    // the reader's place on every agent write.
    expect(keys).not.toContain(JSON.stringify(workspaceQueryKeys.directory("session-1", "docs")));
    expect(keys).not.toContain(JSON.stringify(workspaceQueryKeys.session("session-1")));
  });

  it("refreshes git status and the review for any change", () => {
    const keys = keysOf(notice());

    // A file changing is exactly what moves a repository between clean and dirty. A Changes tab
    // that stayed clean after an edit is wrong in the way a reader acts on: they conclude nothing
    // happened.
    expect(keys).toContain(JSON.stringify(workspaceQueryKeys.gitStatus("session-1")));
    expect(keys).toContain(JSON.stringify(workspaceQueryKeys.review("session-1")));
  });

  it("refreshes a changed directory's own listing rather than its parent", () => {
    const keys = keysOf(notice({ scope: "directory", relativePath: "src", change: undefined }));

    expect(keys).toContain(JSON.stringify(workspaceQueryKeys.directory("session-1", "src")));
    // Its entries changed; it did not. Refreshing the parent instead would reread a listing that
    // is still correct and leave the one that is not.
    expect(keys).not.toContain(JSON.stringify(workspaceQueryKeys.directory("session-1", "")));
  });

  it("narrows a directory notice to the previews inside it", () => {
    const filters = invalidationFiltersFor(
      notice({ scope: "directory", relativePath: "src", change: undefined }),
    );
    const previews = filters.find(
      (filter) =>
        JSON.stringify(filter.queryKey) ===
        JSON.stringify(workspaceQueryKeys.previews("session-1")),
    );

    expect(previews).toBeDefined();
    expect(filterMatchesKey(previews!, workspaceQueryKeys.preview("session-1", "src/main.rs"))).toBe(
      true,
    );
    // A rename inside `src` says nothing about a file in `docs`, and refetching it would be a read
    // nobody asked for.
    expect(filterMatchesKey(previews!, workspaceQueryKeys.preview("session-1", "docs/a.md"))).toBe(
      false,
    );
  });

  it("falls back to the whole session only when the notice cannot say where", () => {
    const filters = invalidationFiltersFor(
      notice({ scope: "workspace", relativePath: undefined, change: undefined }),
    );

    expect(filters).toHaveLength(1);
    expect(filters[0]?.queryKey).toEqual(workspaceQueryKeys.session("session-1"));
  });

  it("keeps a workspace-wide notice inside its own session", () => {
    const filters = invalidationFiltersFor(
      notice({ scope: "workspace", relativePath: undefined, change: undefined }),
    );

    // Another session's panels saw nothing happen. Refreshing them would spend a second
    // workspace's reads — over SSH, a second host's — on this one's uncertainty.
    expect(JSON.stringify(filters[0]?.queryKey)).not.toBe(JSON.stringify(workspaceQueryKeys.all()));
  });

  it("covers both diff sources of a changed file", () => {
    const keys = keysOf(notice({ relativePath: "src/main.rs" }));
    const prefix = JSON.stringify(workspaceQueryKeys.gitDiffsFor("session-1", "src/main.rs"));

    // A prefix, not one source: the staged and unstaged diffs are separate entries, and an edit
    // can change what either of them shows.
    expect(keys).toContain(prefix);
  });
});

describe("filterMatchesKey", () => {
  it("matches everything when the filter names no directory", () => {
    expect(
      filterMatchesKey(
        { queryKey: workspaceQueryKeys.gitStatus("session-1") },
        workspaceQueryKeys.gitStatus("session-1"),
      ),
    ).toBe(true);
  });

  it("refuses a key with no path where one was expected", () => {
    // A key from another family has no path segment. Treating that as a match would invalidate
    // whatever happened to be next to it in the cache.
    expect(
      filterMatchesKey(
        { queryKey: workspaceQueryKeys.previews("session-1"), pathWithin: "src" },
        workspaceQueryKeys.previews("session-1"),
      ),
    ).toBe(false);
  });
});

describe("selectionStillExists", () => {
  it("keeps a selection while its parent has not answered yet", () => {
    // Absence of data is not evidence of absence. Dropping the selection here would move a reader
    // off the file during its own refetch, when nothing has happened to it at all.
    expect(selectionStillExists("src/main.rs", undefined)).toBe(true);
  });

  it("keeps a selection the refreshed listing still holds", () => {
    expect(selectionStillExists("src/main.rs", [{ path: "src/main.rs" }, { path: "src/a.rs" }])).toBe(
      true,
    );
  });

  it("drops a selection the refreshed listing no longer holds", () => {
    expect(selectionStillExists("src/main.rs", [{ path: "src/a.rs" }])).toBe(false);
  });

  it("has nothing to say when nothing is selected", () => {
    expect(selectionStillExists(null, [])).toBe(true);
  });
});
