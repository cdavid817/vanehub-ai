import { describe, expect, it } from "vitest";
import type { DirectoryEntry, GitDiffFile } from "../types/session-workspace";
import { buildSplitRows, buildUnifiedRows } from "./diff-view";
import { flattenFileRows } from "./files-tab";
import { gitStatusPresentation } from "./git-status-presentation";

const directory: DirectoryEntry = { name: "src", path: "src", kind: "directory", size: null };
const file: DirectoryEntry = { name: "main.ts", path: "src/main.ts", kind: "file", size: 20 };
const diff: GitDiffFile = {
  oldPath: "src/main.ts", newPath: "src/main.ts", binary: false, oversized: false,
  hunks: [{
    header: "@@ -1,2 +1,2 @@", oldStart: 1, oldLines: 2, newStart: 1, newLines: 2,
    lines: [
      { kind: "deletion", content: "old", oldLineNumber: 1, newLineNumber: null },
      { kind: "addition", content: "new", oldLineNumber: null, newLineNumber: 1 },
      { kind: "context", content: "same", oldLineNumber: 2, newLineNumber: 2 },
    ],
  }],
};

describe("session workspace presentation models", () => {
  it("retains nested file rows only while their parent is expanded", () => {
    const entries = { "": [directory], src: [file] };
    expect(flattenFileRows(entries, new Set()).map((row) => row.entry.path)).toEqual(["src"]);
    expect(flattenFileRows(entries, new Set(["src"]))).toEqual([
      { entry: directory, depth: 0 }, { entry: file, depth: 1 },
    ]);
  });

  it("constructs unified and aligned split diff rows from one model", () => {
    expect(buildUnifiedRows(diff).map((row) => row.kind)).toEqual(["deletion", "addition", "context"]);
    const split = buildSplitRows(diff);
    expect(split[0].left.content).toBe("old");
    expect(split[0].right.content).toBe("");
    expect(split[1].left.content).toBe("");
    expect(split[1].right.content).toBe("new");
    expect(split[2].left.number).toBe(2);
    expect(split[2].right.number).toBe(2);
  });

  it("uses conventional two-column Git codes and preserves both localized status kinds", () => {
    expect(gitStatusPresentation({ path: "new.ts", previousPath: null, index: "added", worktree: "modified" })).toEqual({
      code: "AM",
      kinds: ["added", "modified"],
    });
    expect(gitStatusPresentation({ path: "new.ts", previousPath: "old.ts", index: "renamed", worktree: "unmodified" })).toEqual({
      code: "R ",
      kinds: ["renamed"],
    });
    expect(gitStatusPresentation({ path: "new.ts", previousPath: null, index: "untracked", worktree: "untracked" })).toEqual({
      code: "??",
      kinds: ["untracked"],
    });
  });
});
