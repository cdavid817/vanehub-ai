import { useTranslation } from "react-i18next";
import type { QueryCoverage } from "../types/session-workspace-evidence";
import { WorkspaceState } from "./workspace-state";

/**
 * What an execution-record list is showing when it is showing no rows.
 *
 * Every one of these renders differently on purpose. "Nothing happened", "nothing matched your
 * filter", "the index has not caught up", "the retained window does not reach back that far", and
 * "this runtime cannot answer" are five different facts, and a shared empty state would let a
 * reader act on the first when the truth was one of the other four.
 */
export type TerminalHistoryEmptyState =
  | "loading"
  | "complete-empty"
  | "no-filter-match"
  | "partial"
  | "indexing"
  | "unavailable";

export function emptyStateFor({
  coverage,
  filtered,
  hasError,
  loading,
}: {
  coverage: QueryCoverage | null;
  filtered: boolean;
  hasError: boolean;
  loading: boolean;
}): TerminalHistoryEmptyState {
  if (hasError) return "unavailable";
  if (coverage === null) return loading ? "loading" : "unavailable";
  if (coverage.state === "unavailable") return "unavailable";
  if (coverage.state === "indexing") return "indexing";
  // A definitive "no match" needs coverage that can support it. Under partial coverage the list
  // has not seen everything, so "your filter matched nothing" is a claim about rows it never read.
  if (coverage.state === "partial") return "partial";
  if (filtered) return "no-filter-match";
  return loading ? "loading" : "complete-empty";
}

export function TerminalHistoryEmpty({ state }: { state: TerminalHistoryEmptyState }) {
  const { t } = useTranslation();
  if (state === "loading") return <WorkspaceState kind="loading" />;
  return (
    <WorkspaceState
      kind={state === "unavailable" ? "unavailable" : "empty"}
      message={t(`executionRecords.empty.${state}`)}
    />
  );
}

/**
 * States the coverage of a list that does have rows.
 *
 * A partial or indexing list is still worth reading; what it must not do is let the rows on screen
 * stand for the whole corpus.
 */
export function CoverageNotice({ coverage }: { coverage: QueryCoverage | null }) {
  const { t } = useTranslation();
  if (coverage === null || coverage.state === "complete") return null;
  return (
    <p
      className="rounded border border-border bg-muted px-2 py-1 text-xs text-muted-foreground"
      data-testid={`execution-records-coverage-${coverage.state}`}
      role="status"
    >
      {t(`executionRecords.coverage.${coverage.state}`)}
      {coverage.droppedCount !== undefined && coverage.droppedCount > 0
        ? ` ${t("executionRecords.coverage.dropped", { count: coverage.droppedCount })}`
        : ""}
    </p>
  );
}

/**
 * Says where legacy activity came from, every time it is shown.
 *
 * The rows look like execution records because they are rendered by the same list, and without
 * this the reader would have no way to know that one list was observed and the other reconstructed
 * from what an assistant said it was doing.
 */
export function LegacySourceNotice({ coverage }: { coverage: QueryCoverage }) {
  const { t } = useTranslation();
  return (
    <p
      className="rounded border border-border bg-muted px-2 py-1 text-xs text-muted-foreground"
      data-testid="legacy-source-notice"
      role="status"
    >
      {t("executionRecords.legacy.source")}
      {coverage.truncated ? ` ${t("executionRecords.legacy.windowPartial")}` : ""}
    </p>
  );
}

/** A failed continuation, offered with a retry that resumes from the same boundary. */
export function PageErrorNotice({
  message,
  onRetry,
}: {
  message: string;
  onRetry: () => void;
}) {
  const { t } = useTranslation();
  return (
    <div
      className="flex items-center justify-between gap-2 rounded border border-border bg-muted px-2 py-1 text-xs"
      data-testid="execution-records-page-error"
      role="alert"
    >
      <span className="text-muted-foreground">{message}</span>
      <button
        className="h-6 shrink-0 rounded border border-border px-2 hover:bg-background"
        onClick={onRetry}
        type="button"
      >
        {t("executionRecords.retry")}
      </button>
    </div>
  );
}
