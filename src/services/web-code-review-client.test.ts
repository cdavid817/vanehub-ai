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
