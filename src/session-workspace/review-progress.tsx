import { useTranslation } from "react-i18next";
import type { CodeReview } from "../types/code-review";

/**
 * What is left to do in this review, and the one action that changes it.
 *
 * The counts come from the backend rather than being folded here. `viewedFiles` cannot be worked
 * out on this side at all — the marks live in a store the review does not carry, and whether one
 * still applies depends on comparing its witness with the file's current one — and computing the
 * other three locally while reading that one remotely would put two sources behind one line of
 * text.
 *
 * Unviewed is rendered as the subtraction rather than sent as a fifth number, so it cannot
 * disagree with the two it came from.
 */
export function ReviewProgress({
  onToggleViewed,
  review,
  selectedPath,
  viewed,
}: {
  /** Absent while nothing is selected, in which case the action is not offered. */
  onToggleViewed?: (viewed: boolean) => void;
  review: CodeReview;
  selectedPath: string | null;
  /** Whether the selected file's mark is current. */
  viewed: boolean;
}) {
  const { t } = useTranslation();
  const { changedFiles, unresolvedComments, unresolvedFindings, viewedFiles } = review.summary;
  return (
    <div className="flex flex-wrap items-center gap-2 border-b border-border px-2 py-1 text-xs text-muted-foreground">
      <span role="status">
        {t("sessionTabs.review.progress", { count: changedFiles, viewed: viewedFiles })}
      </span>
      {/* Zero is not rendered as "0 unresolved": a review with nothing outstanding should read as
          having nothing outstanding, and a row of zeroes is three numbers a reader has to check
          before learning that. */}
      {unresolvedComments > 0 ? (
        <span>{t("sessionTabs.review.unresolvedComments", { count: unresolvedComments })}</span>
      ) : null}
      {unresolvedFindings > 0 ? (
        <span>{t("sessionTabs.review.unresolvedFindings", { count: unresolvedFindings })}</span>
      ) : null}
      {selectedPath && onToggleViewed ? (
        <button
          aria-pressed={viewed}
          className="ml-auto rounded border border-border px-2 py-1"
          onClick={() => onToggleViewed(!viewed)}
          type="button"
        >
          {t(viewed ? "sessionTabs.review.markUnviewed" : "sessionTabs.review.markViewed")}
        </button>
      ) : null}
    </div>
  );
}
