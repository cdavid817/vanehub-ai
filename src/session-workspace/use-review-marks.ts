import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { CodeReview, ReviewDecision } from "../types/code-review";

/**
 * The two things a reviewer records that are not the review's own decision.
 *
 * Marking a file read and deciding about a hunk are separate operations with separate authority,
 * and neither touches the review-level decision. They share a hook because they share everything
 * else: both are witnessed to the snapshot on screen, both are refused when it has moved, and both
 * need the review re-read afterwards so the header's counts and the decision list come back
 * agreeing with what just happened.
 */
export function useReviewMarks(
  review: CodeReview,
  replaceReview: (next: CodeReview) => void,
) {
  const { t } = useTranslation();
  const [error, setError] = useState<string | null>(null);

  /** Whether one hunk's recorded decision still applies, matched the way the backend sends it. */
  const decisionFor = useCallback(
    (relativePath: string, hunkFingerprint: string): ReviewDecision =>
      review.hunkDecisions.find(
        (recorded) =>
          recorded.relativePath === relativePath && recorded.hunkFingerprint === hunkFingerprint,
      )?.decision ?? "pending",
    [review.hunkDecisions],
  );

  /** Whether the file's Viewed mark is current, which only the backend can answer. */
  const isViewed = useCallback(
    (relativePath: string) =>
      Boolean(review.files.find((file) => file.path === relativePath)?.viewed),
    [review.files],
  );

  const run = useCallback(
    async (work: () => Promise<unknown>) => {
      setError(null);
      try {
        await work();
        // Re-read rather than patched locally. The header's counts are derived from two stores on
        // the other side, and a local edit would leave them agreeing with the click rather than
        // with what was recorded.
        replaceReview(await agentService.getCodeReview(review.id));
      } catch (reason: unknown) {
        setError(
          t(
            String(reason).includes("stale_witness")
              ? "sessionTabs.review.markStale"
              : "sessionTabs.review.markFailed",
          ),
        );
      }
    },
    [replaceReview, review.id, t],
  );

  const setViewed = useCallback(
    (relativePath: string, viewed: boolean) =>
      run(() =>
        agentService.setCodeReviewFileViewed({
          expectedSnapshotFingerprint: review.fingerprint,
          relativePath,
          reviewId: review.id,
          viewed,
        }),
      ),
    [review.fingerprint, review.id, run],
  );

  const setHunkDecision = useCallback(
    (relativePath: string, hunkFingerprint: string, decision: ReviewDecision) =>
      run(() =>
        agentService.setCodeReviewHunkDecision({
          decision,
          expectedSnapshotFingerprint: review.fingerprint,
          hunkFingerprint,
          relativePath,
          reviewId: review.id,
        }),
      ),
    [review.fingerprint, review.id, run],
  );

  return { decisionFor, error, isViewed, setHunkDecision, setViewed };
}
