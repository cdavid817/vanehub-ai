import type {
  AddReviewCommentInput,
  CodeReview,
  ReviewAction,
  ReviewComment,
  ReviewDecision,
  ReviewDiffFile,
  ReviewHunkDecisionReceipt,
  ReviewRevertReceipt,
  RevertReviewChangeInput,
  SetReviewHunkDecisionInput,
} from "../types/code-review";
import type { GitDiffResult, GitDiffSource, GitStatusResult } from "../types/session-workspace";
import { createWebMockOperation } from "./web-operation-client";

interface WorkspaceReviewSource {
  getSessionGitStatus(sessionId: string): Promise<GitStatusResult>;
  getSessionGitDiff(sessionId: string, path: string, source: GitDiffSource): Promise<GitDiffResult>;
}

function fingerprint(value: string) {
  let hash = 2166136261;
  for (const character of value) hash = Math.imul(hash ^ character.charCodeAt(0), 16777619);
  return `web-${(hash >>> 0).toString(16).padStart(8, "0")}`;
}

export function createWebCodeReviewClient(workspace: WorkspaceReviewSource) {
  const reviews = new Map<string, CodeReview>();
  // Keyed by review, path, and hunk fingerprint so a decision cannot leak across hunks.
  const hunkDecisions = new Map<string, ReviewDecision>();
  let sequence = 0;
  const find = (reviewId: string) => {
    const review = reviews.get(reviewId);
    if (!review) throw new Error("review-not-found");
    return structuredClone(review);
  };
  return {
    async openCodeReview(sessionId: string) {
      const status = await workspace.getSessionGitStatus(sessionId);
      const files = status.items.map((item) => ({
        path: item.path,
        previousPath: item.previousPath ?? undefined,
        changeType: item.worktree !== "unmodified" ? item.worktree : item.index,
      }));
      const snapshot = fingerprint(JSON.stringify(files));
      const recovered = [...reviews.values()].find((review) => review.sessionId === sessionId && review.status === "active");
      if (recovered) {
        if (recovered.fingerprint !== snapshot) {
          recovered.comments.forEach((comment) => {
            comment.anchor.state = files.some((file) => file.path === comment.anchor.filePath)
              ? "relocated"
              : "stale";
          });
          recovered.files = files;
          recovered.fingerprint = snapshot;
          recovered.updatedAt = new Date().toISOString();
        }
        return structuredClone(recovered);
      }
      const now = new Date().toISOString();
      const review: CodeReview = {
        id: `web-review-${++sequence}`,
        sessionId,
        workspaceId: status.branch ?? `web-${sessionId}`,
        fingerprint: snapshot,
        status: "active",
        decision: "pending",
        createdAt: now,
        updatedAt: now,
        files,
        comments: [],
        findings: [],
      };
      reviews.set(review.id, review);
      return structuredClone(review);
    },
    async getCodeReview(reviewId: string) {
      return find(reviewId);
    },
    async loadCodeReviewFile(sessionId: string, path: string, expectedSnapshot: string): Promise<ReviewDiffFile> {
      const review = [...reviews.values()].find((value) => value.sessionId === sessionId && value.status === "active");
      if (!review || review.fingerprint !== expectedSnapshot) throw new Error("stale-review-snapshot");
      const working = await workspace.getSessionGitDiff(sessionId, path, "working");
      const diff = working.files.length > 0
        ? working
        : await workspace.getSessionGitDiff(sessionId, path, "staged");
      const file = diff.files[0];
      if (!file) throw new Error("review-file-not-found");
      return {
        path: file.newPath,
        changeType: review.files.find((value) => value.path === path)?.changeType ?? "modified",
        binary: file.binary,
        oversized: file.oversized,
        truncated: diff.truncated,
        acceptedBytes: file.hunks.reduce((total, hunk) => total + hunk.lines.reduce((size, line) => size + line.content.length, 0), 0),
        hunks: file.hunks.map((hunk) => ({
          ...hunk,
          lines: hunk.lines.map((line) => ({
            ...line,
            oldLineNumber: line.oldLineNumber ?? undefined,
            newLineNumber: line.newLineNumber ?? undefined,
          })),
          fingerprint: fingerprint(JSON.stringify(hunk)),
          contextFingerprints: hunk.lines.map((line) => fingerprint(`${line.kind}:${line.content}`)),
        })),
      };
    },
    async addCodeReviewComment(input: AddReviewCommentInput): Promise<ReviewComment> {
      const review = reviews.get(input.reviewId);
      if (!review) throw new Error("review-not-found");
      if (!input.body.trim() || input.body.length > 8192) throw new Error("invalid-review-comment");
      const comment: ReviewComment = {
        id: `web-comment-${++sequence}`,
        anchor: { ...input.anchor, state: "current" },
        body: input.body,
        status: "active",
        selected: true,
      };
      review.comments.push(comment);
      review.updatedAt = new Date().toISOString();
      return structuredClone(comment);
    },
    async resolveCodeReviewComment(reviewId: string, commentId: string) {
      const review = reviews.get(reviewId);
      const comment = review?.comments.find((value) => value.id === commentId);
      if (!review || !comment) throw new Error("review-comment-not-found");
      comment.status = "resolved";
      return find(reviewId);
    },
    async selectCodeReviewComment(reviewId: string, commentId: string, selected: boolean) {
      const review = reviews.get(reviewId);
      const comment = review?.comments.find((value) => value.id === commentId);
      if (!review || !comment) throw new Error("review-comment-not-found");
      comment.selected = selected;
      return find(reviewId);
    },
    async setCodeReviewDecision(reviewId: string, decision: ReviewDecision) {
      const review = reviews.get(reviewId);
      if (!review) throw new Error("review-not-found");
      review.decision = decision;
      review.status = decision === "accepted" ? "completed" : "active";
      return find(reviewId);
    },
    async setCodeReviewHunkDecision(input: SetReviewHunkDecisionInput): Promise<ReviewHunkDecisionReceipt> {
      const review = reviews.get(input.reviewId);
      if (!review) throw new Error("review-not-found");
      // Refusing a stale witness here keeps the mock honest about the one guarantee that matters:
      // a decision belongs to the snapshot the reviewer was actually looking at.
      if (review.fingerprint !== input.expectedSnapshotFingerprint) throw new Error("stale_witness");
      if (!review.files.some((file) => file.path === input.relativePath)) throw new Error("review-file-not-found");
      // Only this key changes. The review decision, status, and every other hunk are untouched,
      // and no Git index or working tree exists to touch in Web mode.
      hunkDecisions.set(JSON.stringify([input.reviewId, input.relativePath, input.hunkFingerprint]), input.decision);
      return {
        reviewId: input.reviewId,
        relativePath: input.relativePath,
        hunkFingerprint: input.hunkFingerprint,
        decision: input.decision,
        simulated: true,
      };
    },
    async revertCodeReviewChange(input: RevertReviewChangeInput): Promise<ReviewRevertReceipt> {
      if (!input.confirmed) throw new Error("review-revert-confirmation-required");
      const resultingSnapshot = fingerprint(`${input.expectedSnapshot}:${input.path}:${input.hunkFingerprint ?? "file"}`);
      return { path: input.path, previousSnapshot: input.expectedSnapshot, resultingSnapshot, revertedHunks: 1, simulated: true };
    },
    async sendCodeReviewFeedback(reviewId: string, acknowledgeStale: boolean) {
      const review = find(reviewId);
      if (review.comments.some((comment) => comment.selected && comment.anchor.state === "stale") && !acknowledgeStale) {
        throw new Error("stale-review-acknowledgement-required");
      }
      return { messageId: `web-feedback-${++sequence}` };
    },
    async startCodeReviewAction(reviewId: string, action: ReviewAction) {
      find(reviewId);
      const operationId = `web-review-${action}-${++sequence}`;
      createWebMockOperation({
        id: operationId,
        kind: "workspace",
        relatedEntityId: reviewId,
        message: `Running ${action}`,
        terminalStatus: "succeeded",
        error: null,
        result: { findings: [] },
      });
      return { operationId };
    },
  };
}
