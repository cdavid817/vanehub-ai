import { useCallback, useEffect, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import type { CodeReview, ReviewDiffFile } from "../types/code-review";

export function useCodeReview(sessionId: string) {
  const [review, setReview] = useState<CodeReview | null>(null);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [diff, setDiff] = useState<ReviewDiffFile | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const open = useCallback(async () => {
    setRefreshing(Boolean(review));
    setLoading(!review);
    setError(null);
    try {
      const next = await agentService.openCodeReview(sessionId);
      setReview(next);
      setSelectedPath((current) => current && next.files.some((file) => file.path === current)
        ? current
        : next.files[0]?.path ?? null);
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, [review, sessionId]);

  useEffect(() => {
    setReview(null);
    setSelectedPath(null);
    setDiff(null);
    void open();
  // A new session owns a distinct review lifecycle.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId]);

  useEffect(() => {
    if (!review || !selectedPath) {
      setDiff(null);
      return;
    }
    let active = true;
    setRefreshing(true);
    agentService.loadCodeReviewFile(sessionId, selectedPath, review.fingerprint)
      .then((next) => { if (active) setDiff(next); })
      .catch((reason: unknown) => { if (active) setError(reason instanceof Error ? reason.message : String(reason)); })
      .finally(() => { if (active) setRefreshing(false); });
    return () => { active = false; };
  }, [review, selectedPath, sessionId]);

  const replaceReview = useCallback((next: CodeReview) => setReview(next), []);
  return { review, selectedPath, setSelectedPath, diff, loading, refreshing, error, open, replaceReview };
}
