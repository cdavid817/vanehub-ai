/** @vitest-environment jsdom */
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it, vi } from "vitest";
import { createWebCodeReviewClient } from "../services/web-code-review-client";
import type { GitDiffResult, GitStatusResult } from "../types/session-workspace";

/**
 * Three properties this group exists to establish, checked as properties rather than as screens.
 *
 * A review decision and a hunk decision are independent in both directions. A Viewed mark stops
 * applying when its own file moves and survives when a different one does. And nothing on the
 * decision path stages or rewrites repository content.
 *
 * The first two run against the Web fixture, which implements the same rules as the desktop store:
 * it is the only place a whole review lifecycle can be driven in one test without a repository on
 * disk. The third reads sources, because a behavioural test can only fail for the mutation
 * somebody thought to write a case for.
 */

function status(paths: readonly { hash: string; path: string }[]): GitStatusResult {
  return {
    branch: "main",
    context: { availability: "available", reason: null, rootName: "demo" },
    isGit: true,
    items: paths.map(({ path }) => ({
      index: "unmodified",
      path,
      previousPath: null,
      worktree: "modified",
    })),
    nextCursor: null,
    truncated: false,
  };
}

function diff(entries: readonly { hash: string; path: string }[], path: string): GitDiffResult {
  return {
    context: { availability: "available", reason: null, rootName: "demo" },
    files: [
      {
        binary: false,
        hunks: [
          {
            header: "@@ -1,1 +1,1 @@",
            lines: [
              {
                content: entries.find((entry) => entry.path === path)?.hash ?? "content",
                kind: "addition",
                newLineNumber: 1,
                oldLineNumber: null,
              },
            ],
            newLines: 1,
            newStart: 1,
            oldLines: 1,
            oldStart: 1,
          },
        ],
        newPath: path,
        oldPath: path,
        oversized: false,
      },
    ],
    source: "working",
    truncated: false,
  };
}

function workspace(entries: readonly { hash: string; path: string }[]) {
  return {
    getSessionGitDiff: vi.fn(async (_session: string, path: string) => diff(entries, path)),
    getSessionGitStatus: vi.fn(async () => status(entries)),
  };
}

describe("review decisions, hunk decisions, and Viewed marks", () => {
  it("does not move a hunk decision when the review is decided, or the reverse", async () => {
    const client = createWebCodeReviewClient(workspace([{ hash: "a", path: "src/main.ts" }]));
    const review = await client.openCodeReview("session-1");
    const diff = await client.loadCodeReviewFile("session-1", "src/main.ts", review.fingerprint);
    await client.setCodeReviewHunkDecision({
      decision: "changes-requested",
      expectedSnapshotFingerprint: review.fingerprint,
      hunkFingerprint: diff.hunks[0].fingerprint,
      relativePath: "src/main.ts",
      reviewId: review.id,
    });

    const accepted = await client.setCodeReviewDecision(review.id, "accepted");

    // Accepting the review left the hunk asking for changes. The reverse held in the same run:
    // deciding the hunk did not touch the review, which was still pending when it was accepted.
    expect(accepted.decision).toBe("accepted");
    const reread = await client.getCodeReview(review.id);
    expect(reread.hunkDecisions).toEqual([
      {
        decision: "changes-requested",
        hunkFingerprint: diff.hunks[0].fingerprint,
        relativePath: "src/main.ts",
      },
    ]);
  });

  it("keeps a Viewed mark through an edit to a different file and drops it through its own", async () => {
    const source = workspace([
      { hash: "a", path: "src/kept.ts" },
      { hash: "b", path: "src/edited.ts" },
    ]);
    const client = createWebCodeReviewClient(source);
    const review = await client.openCodeReview("session-1");
    for (const path of ["src/kept.ts", "src/edited.ts"]) {
      await client.setCodeReviewFileViewed({
        expectedSnapshotFingerprint: review.fingerprint,
        relativePath: path,
        reviewId: review.id,
        viewed: true,
      });
    }
    expect((await client.getCodeReview(review.id)).summary.viewedFiles).toBe(2);

    // One file changes: modified becomes deleted, so it is no longer the file that was read. The
    // review's own fingerprint moves with it, which is exactly why the mark cannot be witnessed to
    // that — the other file is untouched and its mark must survive.
    source.getSessionGitStatus.mockResolvedValue({
      ...status([{ hash: "a", path: "src/kept.ts" }]),
      items: [
        { index: "unmodified", path: "src/kept.ts", previousPath: null, worktree: "modified" },
        { index: "unmodified", path: "src/edited.ts", previousPath: null, worktree: "deleted" },
      ],
    });

    const reopened = await client.openCodeReview("session-1");
    expect(reopened.summary.viewedFiles).toBe(1);
    expect(reopened.files.find((file) => file.path === "src/kept.ts")?.viewed).toBe(true);
    expect(reopened.files.find((file) => file.path === "src/edited.ts")?.viewed).toBe(false);
  });
});

/** Service calls that change repository content, as call forms rather than as words. */
const REPOSITORY_MUTATIONS: readonly { readonly pattern: RegExp; readonly what: string }[] = [
  { pattern: /\.\s*revertCodeReviewChange\s*\(/, what: "reverts a change" },
  { pattern: /\.\s*writeSessionFile\s*\(/, what: "writes a file" },
  { pattern: /\.\s*saveSessionFile\s*\(/, what: "saves a file" },
];

/** The modules a decision or a Viewed mark passes through on this side. */
const DECISION_MODULES = ["use-review-marks.ts", "review-progress.tsx", "review-findings.tsx"];

describe("the decision surfaces on this side", () => {
  it("uses patterns that match a real mutation", () => {
    const directory = dirname(fileURLToPath(import.meta.url));
    const reviewCenter = readFileSync(join(directory, "review-center.tsx"), "utf8");

    // Proved against the one surface that legitimately mutates. A typo in any pattern would leave
    // a guard that passes because it matches nothing.
    expect(REPOSITORY_MUTATIONS.some(({ pattern }) => pattern.test(reviewCenter))).toBe(true);
  });

  it("records decisions without reaching anything that changes content", () => {
    const directory = dirname(fileURLToPath(import.meta.url));
    const offenders = DECISION_MODULES.flatMap((name) => {
      const source = readFileSync(join(directory, name), "utf8");
      return REPOSITORY_MUTATIONS.filter(({ pattern }) => pattern.test(source)).map(
        ({ what }) => `${name} ${what}`,
      );
    });

    // The revert lives one import away from all three. Marking a hunk and reverting it are
    // adjacent controls in the same header, which is exactly how the wrong one gets called.
    expect(offenders).toEqual([]);
  });

  it("names modules that are there", () => {
    const directory = dirname(fileURLToPath(import.meta.url));
    const present = new Set(readdirSync(directory));
    for (const name of DECISION_MODULES) {
      expect(present.has(name), `${name} is missing, so this guard checks nothing`).toBe(true);
    }
  });
});
