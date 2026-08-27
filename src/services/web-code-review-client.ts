import type {
  AddReviewCommentInput,
  CodeReview,
  ReviewAction,
  ReviewComment,
  ReviewDecision,
  ReviewDiffFile,
  GetReviewPatchInput,
  ReviewFileSummary,
  ReviewFileViewedReceipt,
  ReviewHunkDecisionReceipt,
  ReviewPatch,
  ReviewRevertReceipt,
  ReviewSummary,
  RevertReviewChangeInput,
  SetReviewFileViewedInput,
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
  /** Keyed by review and path, holding the witness the mark was made against. */
  const fileViews = new Map<string, { fileWitness: string; viewed: boolean; viewedAt?: string }>();
  let sequence = 0;
  /**
   * The witness a Viewed mark has to still match, computed the way the native side computes it.
   *
   * The fixture reproduces the rule rather than storing a boolean, because the rule is the
   * behaviour: a mark survives an edit to another file and does not survive an edit to its own.
   */
  const witnessOf = (file: ReviewFileSummary) =>
    fingerprint(
      [file.path, file.previousPath ?? "", file.changeType, file.oldHash ?? "", file.newHash ?? ""].join("\u0000"),
    );
  /** Recomputed on every read: the marks and the files both move, and a stored count would not. */
  const summarize = (review: CodeReview): ReviewSummary => ({
    changedFiles: review.files.length,
    viewedFiles: review.files.filter((file) => {
      const mark = fileViews.get(JSON.stringify([review.id, file.path]));
      return Boolean(mark?.viewed) && mark?.fileWitness === witnessOf(file);
    }).length,
    unresolvedComments: review.comments.filter((comment) => comment.status !== "resolved").length,
    unresolvedFindings: review.findings.filter((finding) => !finding.resolved).length,
  });
  const withSummary = (review: CodeReview) => {
    const clone = structuredClone(review);
    clone.summary = summarize(clone);
    return clone;
  };
  const find = (reviewId: string) => {
    const review = reviews.get(reviewId);
    if (!review) throw new Error("review-not-found");
    return withSummary(review);
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
        return withSummary(recovered);
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
        // Replaced on the way out by `withSummary`. Present here only because the type requires a
        // review to carry one, and a review with no files has read none of them.
        summary: { changedFiles: files.length, viewedFiles: 0, unresolvedComments: 0, unresolvedFindings: 0 },
      };
      reviews.set(review.id, review);
      return withSummary(review);
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
    async setCodeReviewFileViewed(input: SetReviewFileViewedInput): Promise<ReviewFileViewedReceipt> {
      const review = reviews.get(input.reviewId);
      if (!review) throw new Error("review-not-found");
      if (review.fingerprint !== input.expectedSnapshotFingerprint) throw new Error("stale_witness");
      const file = review.files.find((entry) => entry.path === input.relativePath);
      if (!file) throw new Error("review-file-not-found");
      // Witnessed to the file, not to the review, so the fixture reproduces the behaviour that
      // matters: an edit to one file leaves the marks on the others standing.
      const fileWitness = witnessOf(file);
      const viewedAt = input.viewed ? new Date().toISOString() : undefined;
      fileViews.set(JSON.stringify([input.reviewId, input.relativePath]), { fileWitness, viewed: input.viewed, viewedAt });
      return { reviewId: input.reviewId, relativePath: input.relativePath, fileWitness, viewed: input.viewed, viewedAt, simulated: true };
    },
    async getCodeReviewPatch(input: GetReviewPatchInput): Promise<ReviewPatch> {
      const review = [...reviews.values()].find((value) => value.sessionId === input.sessionId && value.status === "active");
      if (!review || review.fingerprint !== input.expectedSnapshot) throw new Error("stale_witness");
      const diff = await this.loadCodeReviewFile(input.sessionId, input.path, input.expectedSnapshot);
      const selected = input.hunkFingerprint
        ? diff.hunks.filter((hunk) => hunk.fingerprint === input.hunkFingerprint)
        : diff.hunks;
      if (selected.length !== (input.hunkFingerprint ? 1 : selected.length) || selected.length === 0) {
        throw new Error("review-hunk-unavailable");
      }
      // Real headers even in the fixture. A mock that returned the displayed lines would let the
      // difference between "readable" and "appliable" disappear on the one adapter where nothing
      // can run `git apply` to notice.
      const body = selected
        .map((hunk) => [hunk.header, ...hunk.lines.map((line) => `${line.kind === "addition" ? "+" : line.kind === "deletion" ? "-" : " "}${line.content}`)].join("\n"))
        .join("\n");
      return {
        path: input.path,
        snapshot: review.fingerprint,
        hunks: selected.length,
        patch: `diff --git a/${input.path} b/${input.path}\n--- a/${input.path}\n+++ b/${input.path}\n${body}\n`,
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
