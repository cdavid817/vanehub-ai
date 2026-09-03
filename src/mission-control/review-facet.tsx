import { useTranslation } from "react-i18next";
import { formatAppDateTime, formatAppNumber } from "../i18n/format";
import type { MissionControlRunSummary } from "../types/mission-control";
import { AsyncBoundary } from "../ui/async/AsyncBoundary";
import { useMissionControlReview } from "./use-mission-control-review";

function ReviewStat({ label, value }: { label: string; value: string }) {
  return <div><dt className="text-[11px] text-muted-foreground">{label}</dt><dd className="mt-0.5 font-medium tabular-nums">{value}</dd></div>;
}

/**
 * The Review facet: a bounded summary (status, decision, and `ReviewSummary`'s own counts) of the
 * one `CodeReview` linked to this Run — see `use-mission-control-review.ts` for the join. Deliberately
 * not a second implementation of the full Review Center surface: no comment/finding/diff content, no
 * mutation actions. A reader who needs those already reaches them via the "review" action's own
 * navigation (`use-mission-control-actions.ts`) — this facet is what "Run detail" shows in place, not
 * a replacement for that surface.
 */
export function ReviewFacet({ run }: { run: MissionControlRunSummary }) {
  const { t, i18n } = useTranslation();
  const { reload, ...state } = useMissionControlReview(run, t("missionControl.review.noReview"), t("missionControl.review.error"));

  return (
    <div className="mt-4 space-y-3" data-testid="mission-control-review-facet">
      <AsyncBoundary onRetry={reload} state={state} unavailableState={{ title: t("missionControl.review.noReview") }}>
        {(review) => (
          <>
            <div className="flex flex-wrap items-center gap-2 text-xs">
              <span className="rounded-full border border-input px-2 py-0.5 font-medium">{t(`missionControl.review.status.${review.status}`)}</span>
              <span className="rounded-full border border-input px-2 py-0.5 font-medium">{t(`missionControl.review.decision.${review.decision}`)}</span>
              <span className="text-muted-foreground">{formatAppDateTime(review.updatedAt, i18n.language, { dateStyle: "short", timeStyle: "short" })}</span>
            </div>
            <dl className="grid grid-cols-2 gap-3 sm:grid-cols-4">
              <ReviewStat label={t("missionControl.review.changedFiles")} value={formatAppNumber(review.summary.changedFiles, i18n.language)} />
              <ReviewStat label={t("missionControl.review.viewedFiles")} value={formatAppNumber(review.summary.viewedFiles, i18n.language)} />
              <ReviewStat label={t("missionControl.review.unresolvedComments")} value={formatAppNumber(review.summary.unresolvedComments, i18n.language)} />
              <ReviewStat label={t("missionControl.review.unresolvedFindings")} value={formatAppNumber(review.summary.unresolvedFindings, i18n.language)} />
            </dl>
          </>
        )}
      </AsyncBoundary>
    </div>
  );
}
