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

/**
 * Review Center's service boundary, split out of `AgentService` so the review surface can grow
 * without pushing the composite interface further past its size budget.
 *
 * `setCodeReviewDecision` and `setCodeReviewHunkDecision` are deliberately separate operations
 * with different authority: one records a judgement about the whole Review Session, the other
 * about a single witnessed hunk. Neither stages, commits, or rewrites repository content.
 */
export interface CodeReviewService {
  openCodeReview(sessionId: string): Promise<CodeReview>;
  getCodeReview(reviewId: string): Promise<CodeReview>;
  loadCodeReviewFile(sessionId: string, path: string, expectedSnapshot: string): Promise<ReviewDiffFile>;
  addCodeReviewComment(input: AddReviewCommentInput): Promise<ReviewComment>;
  resolveCodeReviewComment(reviewId: string, commentId: string): Promise<CodeReview>;
  selectCodeReviewComment(reviewId: string, commentId: string, selected: boolean): Promise<CodeReview>;
  setCodeReviewDecision(reviewId: string, decision: ReviewDecision): Promise<CodeReview>;
  setCodeReviewHunkDecision(input: SetReviewHunkDecisionInput): Promise<ReviewHunkDecisionReceipt>;
  revertCodeReviewChange(input: RevertReviewChangeInput): Promise<ReviewRevertReceipt>;
  sendCodeReviewFeedback(reviewId: string, acknowledgeStale: boolean): Promise<{ messageId: string }>;
  startCodeReviewAction(reviewId: string, action: ReviewAction): Promise<{ operationId: string }>;
}
