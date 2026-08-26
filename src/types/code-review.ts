export type ReviewDecision = "pending" | "accepted" | "changes-requested";
export type ReviewAnchorState = "current" | "relocated" | "stale";
export type ReviewAction = "review-agent" | "tests" | "security";

export interface ReviewAnchor {
  filePath: string;
  side: "old" | "new";
  startLine: number;
  endLine: number;
  hunkFingerprint: string;
  contextFingerprint: string;
  state: ReviewAnchorState;
}

export interface ReviewFileSummary {
  path: string;
  previousPath?: string;
  changeType: string;
  oldHash?: string;
  newHash?: string;
}

export interface ReviewComment {
  id: string;
  anchor: ReviewAnchor;
  body: string;
  status: "active" | "resolved";
  selected: boolean;
}

export interface ReviewFinding {
  id: string;
  source: ReviewAction;
  title: string;
  severity: "info" | "warning" | "error";
  anchor?: ReviewAnchor;
  operationId: string;
  resolved: boolean;
}

export interface CodeReview {
  id: string;
  sessionId: string;
  workspaceId: string;
  baseRevision?: string;
  headRevision?: string;
  fingerprint: string;
  status: "active" | "completed";
  decision: ReviewDecision;
  createdAt: string;
  updatedAt: string;
  files: ReviewFileSummary[];
  comments: ReviewComment[];
  findings: ReviewFinding[];
}

/**
 * A decision about one witnessed hunk. Separate from the review-level decision on purpose:
 * accepting a hunk used to call the review mutation, so approving one block of a diff marked the
 * entire review accepted.
 */
export interface SetReviewHunkDecisionInput {
  reviewId: string;
  relativePath: string;
  hunkFingerprint: string;
  /** The review snapshot the user was looking at. A later snapshot rejects the decision. */
  expectedSnapshotFingerprint: string;
  decision: ReviewDecision;
}

export interface SetReviewFileViewedInput {
  reviewId: string;
  relativePath: string;
  /** The review snapshot the user was looking at. A later snapshot rejects the mark. */
  expectedSnapshotFingerprint: string;
  viewed: boolean;
}

/**
 * What was recorded about a file being read.
 *
 * `fileWitness` is what the mark is about, and it is not the review's snapshot fingerprint. A
 * review snapshot covers every changed file, so witnessing Viewed to it would clear all twelve
 * marks because an agent wrote to one — a progress count that resets on unrelated work is one
 * nobody can act on. The witness moves only when that file moves.
 */
export interface ReviewFileViewedReceipt {
  reviewId: string;
  relativePath: string;
  fileWitness: string;
  viewed: boolean;
  /** Absent when the file is not viewed, because there is no moment at which it was. */
  viewedAt?: string;
  /** True when only fixture memory changed: no review row, Git index, or working tree was touched. */
  simulated: boolean;
}

export interface ReviewHunkDecisionReceipt {
  reviewId: string;
  relativePath: string;
  hunkFingerprint: string;
  decision: ReviewDecision;
  /** True when only fixture memory changed: no review row, Git index, or working tree was touched. */
  simulated: boolean;
}

export interface ReviewDiffLine {
  kind: string;
  content: string;
  oldLineNumber?: number;
  newLineNumber?: number;
}

export interface ReviewDiffHunk {
  fingerprint: string;
  contextFingerprints: string[];
  header: string;
  oldStart: number;
  oldLines: number;
  newStart: number;
  newLines: number;
  lines: ReviewDiffLine[];
}

export interface ReviewDiffFile {
  path: string;
  changeType: string;
  binary: boolean;
  oversized: boolean;
  truncated: boolean;
  acceptedBytes: number;
  hunks: ReviewDiffHunk[];
}

export interface AddReviewCommentInput {
  reviewId: string;
  anchor: Omit<ReviewAnchor, "state">;
  body: string;
}

export interface RevertReviewChangeInput {
  sessionId: string;
  path: string;
  expectedSnapshot: string;
  hunkFingerprint?: string;
  confirmed: boolean;
}

export interface ReviewRevertReceipt {
  path: string;
  previousSnapshot: string;
  resultingSnapshot: string;
  revertedHunks: number;
  simulated: boolean;
}
