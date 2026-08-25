import { LoaderCircle, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { ReportGroupBy } from "../types/session-workspace-evidence";
import type { ReportScopeSelection } from "./use-session-run-report";

const GROUP_BY_OPTIONS: readonly ReportGroupBy[] = ["run", "agent", "seat", "model", "tool"];

/**
 * The ranges a reader actually asks for, as durations back from now.
 *
 * Absolute pickers were the alternative and are worse here: a report is read right after the work
 * happened, and "the last hour" is the question. A date picker makes the reader compute the answer
 * to a question they did not have.
 */
export const REPORT_RANGES = [
  { key: "all", minutes: null },
  { key: "hour", minutes: 60 },
  { key: "day", minutes: 60 * 24 },
  { key: "week", minutes: 60 * 24 * 7 },
] as const;

export type ReportRangeKey = (typeof REPORT_RANGES)[number]["key"];

/**
 * The `from` bound for a preset, or nothing for the whole session.
 *
 * `now` is passed in rather than read here so the value a control produces is a function of its
 * inputs — a component that called the clock itself could not be tested without freezing time.
 */
export function reportRangeStart(key: ReportRangeKey, now: Date): string | undefined {
  const option = REPORT_RANGES.find((range) => range.key === key);
  if (!option || option.minutes === null) return undefined;
  return new Date(now.getTime() - option.minutes * 60_000).toISOString();
}

export function ReportScopeControls({
  isRefreshing,
  onClearCorrelation,
  onGroupByChange,
  onRangeChange,
  range,
  scope,
}: {
  /** True while a newer report is in flight and the previous one is still on screen. */
  isRefreshing: boolean;
  onClearCorrelation: () => void;
  onGroupByChange: (groupBy: ReportGroupBy) => void;
  onRangeChange: (range: ReportRangeKey) => void;
  range: ReportRangeKey;
  scope: ReportScopeSelection;
}) {
  const { t } = useTranslation();
  const correlations = [...scope.runIds, ...scope.seatIds];

  return (
    <div className="flex flex-col gap-2 border-b border-border pb-2">
      <div className="flex flex-wrap items-center gap-1.5">
        <span className="text-[11px] text-muted-foreground">{t("sessionTabs.report.groupBy")}</span>
        <div aria-label={t("sessionTabs.report.groupBy")} className="flex flex-wrap gap-1" role="group">
          {GROUP_BY_OPTIONS.map((option) => (
            <button
              aria-pressed={scope.groupBy === option}
              className={cn(
                "h-6 rounded-full border border-border px-2 text-[11px]",
                scope.groupBy === option
                  ? "bg-primary text-primary-foreground"
                  : "bg-background hover:bg-muted",
              )}
              key={option}
              onClick={() => onGroupByChange(option)}
              type="button"
            >
              {t(`sessionTabs.report.groupBy.${option}`)}
            </button>
          ))}
        </div>
        {/* Beside the controls rather than replacing the report: the previous answer stays
            readable while a narrower one is fetched. */}
        {isRefreshing ? (
          <span className="flex items-center gap-1 text-[11px] text-muted-foreground" role="status">
            <LoaderCircle aria-hidden="true" className="h-3 w-3 animate-spin" />
            {t("sessionTabs.report.refreshing")}
          </span>
        ) : null}
      </div>

      <div className="flex flex-wrap items-center gap-1.5">
        <span className="text-[11px] text-muted-foreground">{t("sessionTabs.report.range")}</span>
        <div aria-label={t("sessionTabs.report.range")} className="flex flex-wrap gap-1" role="group">
          {REPORT_RANGES.map((option) => (
            <button
              aria-pressed={range === option.key}
              className={cn(
                "h-6 rounded-full border border-border px-2 text-[11px]",
                range === option.key
                  ? "bg-primary text-primary-foreground"
                  : "bg-background hover:bg-muted",
              )}
              key={option.key}
              onClick={() => onRangeChange(option.key)}
              type="button"
            >
              {t(`sessionTabs.report.range.${option.key}`)}
            </button>
          ))}
        </div>
      </div>

      {/* The run and seat come from wherever the reader navigated in, so they are invisible from
          this panel. Without saying so, a narrower report reads as a quieter session. */}
      {correlations.length > 0 ? (
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="text-[11px] text-muted-foreground">{t("sessionTabs.report.scope")}</span>
          {correlations.map((value) => (
            <span
              className="flex h-6 items-center rounded-full border border-border bg-background px-2 font-mono text-[11px] text-muted-foreground"
              key={value}
            >
              {value}
            </span>
          ))}
          <button
            className="flex h-6 items-center gap-1 rounded-full border border-border bg-background px-2 text-[11px] text-muted-foreground hover:bg-muted"
            onClick={onClearCorrelation}
            type="button"
          >
            <X aria-hidden="true" className="h-3 w-3" />
            {t("sessionTabs.report.wholeSession")}
          </button>
        </div>
      ) : null}
    </div>
  );
}
