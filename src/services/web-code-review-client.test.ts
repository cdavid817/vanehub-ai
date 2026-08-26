import { describe, expect, it, vi } from "vitest";
import type { GitStatusResult } from "../types/session-workspace";
import { createWebCodeReviewClient } from "./web-code-review-client";

function workspace() {
  const initialStatus: GitStatusResult = {
    context: { availability: "available", rootName: "demo", reason: null },
    isGit: true,
    branch: "main",
    items: [{ path: "src/main.ts", previousPath: null, index: "unmodified", worktree: "modified" }],
    truncated: false,
    nextCursor: null,
  };
  return {
    getSessionGitStatus: vi.fn(async (): Promise<GitStatusResult> => initialStatus),
    getSessionGitDiff: vi.fn(async () => ({
      context: { availability: "available" as const, rootName: "demo", reason: null },
      source: "working" as const,
      files: [{
        oldPath: "src/main.ts",
        newPath: "src/main.ts",
        binary: false,
        oversized: false,
        hunks: [{
          header: "@@ -1 +1 @@",
          oldStart: 1,
          oldLines: 1,
          newStart: 1,
          newLines: 1,
          lines: [{ kind: "addition" as const, content: "safe", oldLineNumber: null, newLineNumber: 1 }],
        }],
      }],
      truncated: false,
    })),
  };
}

describe("web code review parity", () => {
  it("recovers reviews, persists comments, and labels revert as simulated", async () => {
    const client = createWebCodeReviewClient(workspace());
    const review = await client.openCodeReview("session-1");
    expect((await client.openCodeReview("session-1")).id).toBe(review.id);
    const diff = await client.loadCodeReviewFile("session-1", "src/main.ts", review.fingerprint);
    await client.addCodeReviewComment({
      reviewId: review.id,
      body: "Please cover this branch",
      anchor: {
        filePath: "src/main.ts",
        side: "new",
        startLine: 1,
        endLine: 1,
        hunkFingerprint: diff.hunks[0].fingerprint,
        contextFingerprint: diff.hunks[0].contextFingerprints[0],
      },
    });
    expect((await client.getCodeReview(review.id)).comments).toHaveLength(1);
    const receipt = await client.revertCodeReviewChange({
      sessionId: "session-1",
      path: "src/main.ts",
      expectedSnapshot: review.fingerprint,
      confirmed: true,
    });
    expect(receipt.simulated).toBe(true);
  });

  it("rejects stale snapshots and unconfirmed destructive actions", async () => {
    const client = createWebCodeReviewClient(workspace());
    const review = await client.openCodeReview("session-1");
    await expect(client.loadCodeReviewFile("session-1", "src/main.ts", "stale")).rejects.toThrow("stale");
    await expect(client.revertCodeReviewChange({
      sessionId: "session-1",
      path: "src/main.ts",
      expectedSnapshot: review.fingerprint,
      confirmed: false,
    })).rejects.toThrow("confirmation");
  });

  // Accepting a hunk used to call the review-level mutation, so approving one block of a diff
  // marked the entire Review Session accepted and closed it.
  it("keeps a hunk decision from touching the review decision or repository content", async () => {
    const source = workspace();
    const client = createWebCodeReviewClient(source);
    const review = await client.openCodeReview("session-1");
    const diff = await client.loadCodeReviewFile("session-1", "src/main.ts", review.fingerprint);

    const receipt = await client.setCodeReviewHunkDecision({
      reviewId: review.id,
      relativePath: "src/main.ts",
      hunkFingerprint: diff.hunks[0].fingerprint,
      expectedSnapshotFingerprint: review.fingerprint,
      decision: "accepted",
    });

    expect(receipt).toEqual({
      reviewId: review.id,
      relativePath: "src/main.ts",
      hunkFingerprint: diff.hunks[0].fingerprint,
      decision: "accepted",
      simulated: true,
    });
    const after = await client.getCodeReview(review.id);
    expect(after.decision).toBe("pending");
    expect(after.status).toBe("active");
    // Nothing in the mock reads or writes Git; asserting the read-only source stayed untouched is
    // the strongest available statement that no index or working-tree mutation was attempted.
    expect(source.getSessionGitDiff).toHaveBeenCalledTimes(1);
  });

  it("keeps the review decision from rewriting hunk decisions", async () => {
    const client = createWebCodeReviewClient(workspace());
    const review = await client.openCodeReview("session-1");
    const diff = await client.loadCodeReviewFile("session-1", "src/main.ts", review.fingerprint);
    await client.setCodeReviewHunkDecision({
      reviewId: review.id,
      relativePath: "src/main.ts",
      hunkFingerprint: diff.hunks[0].fingerprint,
      expectedSnapshotFingerprint: review.fingerprint,
      decision: "changes-requested",
    });

    const accepted = await client.setCodeReviewDecision(review.id, "accepted");

    expect(accepted.decision).toBe("accepted");
    const again = await client.setCodeReviewHunkDecision({
      reviewId: review.id,
      relativePath: "src/main.ts",
      hunkFingerprint: diff.hunks[0].fingerprint,
      expectedSnapshotFingerprint: review.fingerprint,
      decision: "changes-requested",
    });
    expect(again.decision).toBe("changes-requested");
  });

  it("refuses a hunk decision witnessed against an older snapshot", async () => {
    const client = createWebCodeReviewClient(workspace());
    const review = await client.openCodeReview("session-1");

    await expect(client.setCodeReviewHunkDecision({
      reviewId: review.id,
      relativePath: "src/main.ts",
      hunkFingerprint: "hunk-1",
      expectedSnapshotFingerprint: "an-older-snapshot",
      decision: "accepted",
    })).rejects.toThrow("stale_witness");
    expect((await client.getCodeReview(review.id)).decision).toBe("pending");
  });

  it("witnesses a viewed mark to the file rather than to the review snapshot", async () => {
    const client = createWebCodeReviewClient(workspace());
    const review = await client.openCodeReview("session-1");

    const receipt = await client.setCodeReviewFileViewed({
      reviewId: review.id,
      relativePath: "src/main.ts",
      expectedSnapshotFingerprint: review.fingerprint,
      viewed: true,
    });

    // A review snapshot covers every changed file, so a mark witnessed to it would be cleared by
    // an edit to a different file. The fixture reproduces the distinction rather than papering
    // over it, because a mock that agreed with itself would let the desktop path drift.
    expect(receipt.viewed).toBe(true);
    expect(receipt.fileWitness).not.toBe(review.fingerprint);
    expect(receipt.viewedAt).toBeTruthy();
    expect(receipt.simulated).toBe(true);
  });

  it("leaves no moment on a file the reviewer unmarks", async () => {
    const client = createWebCodeReviewClient(workspace());
    const review = await client.openCodeReview("session-1");
    await client.setCodeReviewFileViewed({
      reviewId: review.id,
      relativePath: "src/main.ts",
      expectedSnapshotFingerprint: review.fingerprint,
      viewed: true,
    });

    const cleared = await client.setCodeReviewFileViewed({
      reviewId: review.id,
      relativePath: "src/main.ts",
      expectedSnapshotFingerprint: review.fingerprint,
      viewed: false,
    });

    expect(cleared.viewed).toBe(false);
    expect(cleared.viewedAt).toBeUndefined();
  });

  it("refuses a viewed mark witnessed against an older snapshot", async () => {
    const client = createWebCodeReviewClient(workspace());
    const review = await client.openCodeReview("session-1");

    await expect(
      client.setCodeReviewFileViewed({
        reviewId: review.id,
        relativePath: "src/main.ts",
        expectedSnapshotFingerprint: "an-older-snapshot",
        viewed: true,
      }),
    ).rejects.toThrow("stale_witness");
  });

  it("marks an anchor stale when its file disappears from a later snapshot", async () => {
    const source = workspace();
    const client = createWebCodeReviewClient(source);
    const review = await client.openCodeReview("session-1");
    await client.addCodeReviewComment({
      reviewId: review.id,
      body: "Keep this anchor",
      anchor: { filePath: "src/main.ts", side: "new", startLine: 1, endLine: 1, hunkFingerprint: "h", contextFingerprint: "c" },
    });
    source.getSessionGitStatus.mockResolvedValue({
      context: { availability: "available", rootName: "demo", reason: null },
      isGit: true,
      branch: "main",
      items: [{ path: "other.ts", previousPath: null, index: "added", worktree: "unmodified" }],
      truncated: false,
      nextCursor: null,
    });
    expect((await client.openCodeReview("session-1")).comments[0].anchor.state).toBe("stale");
  });
});
