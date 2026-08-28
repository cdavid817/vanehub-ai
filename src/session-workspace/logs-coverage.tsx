import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type {
  SessionLogCorrelationFilters,
  SessionLogCoverage,
  SessionLogCoverageState,
} from "../types/session-workspace";

/** Which correlation a chip stands for, in the order a reader narrows by them. */
const CORRELATION_KEYS = [
  "seatId",
  "runId",
  "traceId",
  "spanId",
  "operationId",
  "agentId",
] as const satisfies readonly (keyof SessionLogCorrelationFilters)[];

export type SessionLogCorrelationKey = (typeof CORRELATION_KEYS)[number];

/**
 * What the index is willing to claim about the list below it.
 *
 * Rendered whenever the answer is anything other than `complete`, because `complete` is the only
 * value that licenses the conclusion a reader will otherwise draw for free: that an empty list
 * means nothing happened. The other three each mean something different and specific — still
 * filling in, known to be missing something, or unable to answer at all — and a list that looked
 * identical under all four would let a reader act on the wrong one.
 */
export function LogsCoverageNotice({ coverage }: { coverage: SessionLogCoverage | undefined }) {
  const { t } = useTranslation();
  // Absent coverage is `unavailable`, never `complete`. A runtime that did not report is not a
  // runtime that reported everything.
  const state: SessionLogCoverageState = coverage?.state ?? "unavailable";
  if (state === "complete") return null;

  return (
    <p
      className={cn(
        "rounded border px-2 py-1 text-xs",
        state === "indexing"
          ? "border-border bg-muted text-muted-foreground"
          : "ucd-status-warning",
      )}
      role="status"
    >
      {t(`sessionTabs.logs.coverage.${state}`)}
      {coverage && coverage.droppedCount > 0
        ? ` ${t("sessionTabs.logs.coverage.dropped", { count: coverage.droppedCount })}`
        : null}
    </p>
  );
}

/**
 * The correlations currently narrowing the list, each removable on its own.
 *
 * A reader arrives here from a trace or a run, so the scope was chosen somewhere else and is
 * invisible from this panel. Without the chips the list is silently narrower than the session, and
 * an empty list reads as "this session logged nothing" rather than as "this run logged nothing" —
 * two very different conclusions, reached from the same blank space.
 */
export function LogsScopeChips({
  correlation,
  onClear,
}: {
  correlation: SessionLogCorrelationFilters;
  onClear: (key: SessionLogCorrelationKey) => void;
}) {
  const { t } = useTranslation();
  const active = CORRELATION_KEYS.filter((key) => {
    const value = correlation[key];
    return typeof value === "string" && value.trim().length > 0;
  });
  if (active.length === 0) return null;

  return (
    <div aria-label={t("sessionTabs.logs.scope.label")} className="flex flex-wrap items-center gap-1" role="group">
      {active.map((key) => (
        <button
          className="flex h-6 items-center gap-1 rounded-full border border-border bg-background px-2 text-xs text-muted-foreground hover:bg-muted"
          key={key}
          onClick={() => onClear(key)}
          type="button"
        >
          <span className="font-medium">{t(`sessionTabs.logs.scope.${key}`)}</span>
          {/* Truncated in the middle rather than the end: correlation ids differ in their tails,
              so a head-only preview would render several distinct scopes identically. */}
          <span className="max-w-32 truncate">{correlation[key]}</span>
          <X className="h-3 w-3" aria-hidden="true" />
          <span className="sr-only">{t("sessionTabs.logs.scope.clear")}</span>
        </button>
      ))}
    </div>
  );
}
