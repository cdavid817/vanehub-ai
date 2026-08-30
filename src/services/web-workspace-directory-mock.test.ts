import { afterEach, describe, expect, it } from "vitest";
import type { DirectoryEntry } from "../types/session-workspace";
import {
  changeWebDirectory,
  pageWebDirectory,
  resetWebDirectoryPaging,
} from "./web-workspace-directory-mock";

const CONTEXT = { availability: "available" as const, rootName: "project", reason: null };

const entries: DirectoryEntry[] = ["a", "b", "c", "d"].map((name) => ({
  kind: "file",
  name,
  path: name,
  size: 1,
}));

afterEach(() => {
  resetWebDirectoryPaging();
});

describe("web directory paging", () => {
  it("hands back one page at a time with a cursor for the rest", () => {
    const first = pageWebDirectory(CONTEXT, "", entries, null, 2);

    expect(first.items.map((item) => item.name)).toEqual(["a", "b"]);
    expect(first.truncated).toBe(true);
    expect(first.nextCursor).toBeTruthy();

    const second = pageWebDirectory(CONTEXT, "", entries, first.nextCursor, 2);

    expect(second.items.map((item) => item.name)).toEqual(["c", "d"]);
    expect(second.truncated).toBe(false);
    expect(second.nextCursor).toBeNull();
  });

  it("refuses a cursor issued before the folder changed", () => {
    const first = pageWebDirectory(CONTEXT, "", entries, null, 2);

    changeWebDirectory("");
    const resumed = pageWebDirectory(CONTEXT, "", entries, first.nextCursor, 2);

    // `stale_cursor`, not silence. Continuing here would drop or repeat rows with nothing on screen
    // to say so, which is the failure the whole cursor contract exists to prevent.
    expect(resumed.items).toEqual([]);
    expect(resumed.coverage.reasonCode).toBe("stale_cursor");
    // And the restart works: the recovery is asking again without a cursor.
    expect(pageWebDirectory(CONTEXT, "", entries, null, 2).items).toHaveLength(2);
  });

  it("refuses a cursor from another directory", () => {
    const first = pageWebDirectory(CONTEXT, "", entries, null, 2);

    const elsewhere = pageWebDirectory(CONTEXT, "src", entries, first.nextCursor, 2);

    // A resume position compares perfectly well against another folder's entries, which is exactly
    // why the folder has to be checked rather than trusted.
    expect(elsewhere.coverage.reasonCode).toBe("invalid_cursor");
  });

  it("refuses a cursor nobody issued rather than guessing at it", () => {
    for (const forged of ["", "not-base64!", btoa("{}")]) {
      const refused = pageWebDirectory(CONTEXT, "", entries, forged || "x", 2);
      expect(refused.coverage.reasonCode).toBe("invalid_cursor");
    }
  });

  it("keeps truncation and coverage as separate facts", () => {
    const first = pageWebDirectory(CONTEXT, "", entries, null, 2);

    // The fixture is the whole folder and all of it was read, so the page is `complete` while still
    // being truncated. Collapsing the two would let a bounded read look like an incomplete one.
    expect(first.truncated).toBe(true);
    expect(first.coverage.state).toBe("complete");
    expect(first.coverage.reasonCode).toBeUndefined();
  });
});
