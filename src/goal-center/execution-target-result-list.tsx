import { useTranslation } from "react-i18next";
import { EmptyState } from "../ui/empty-state/EmptyState";
import { ExecutionTargetOptionSummary } from "./execution-target-option-summary";
import type { ExecutionTargetKind, ExecutionTargetOption } from "./execution-target-providers";

export interface ExecutionTargetResultsProps {
  kind: ExecutionTargetKind;
  options: ExecutionTargetOption[];
  loading: boolean;
  error: string | null;
  query: string;
  onSelect: (option: ExecutionTargetOption) => void;
}

/**
 * Loading only suppresses this list while there is nothing to show yet -- once a first page of
 * options has rendered, a later in-flight re-search (retyping) leaves the previous results visible
 * rather than flashing to empty, the same trade-off use-command-center-search.ts's own hook leaves
 * to its caller.
 */
export function ExecutionTargetResults({ error, kind, loading, onSelect, options, query }: ExecutionTargetResultsProps) {
  const { t } = useTranslation();

  if (error) {
    return <p className="rounded border border-destructive/50 bg-destructive/10 p-2 text-xs text-destructive" role="alert">{error}</p>;
  }
  if (loading && options.length === 0) {
    return <p className="p-2 text-xs text-muted-foreground" role="status">{t("goals.picker.loading")}</p>;
  }
  if (options.length === 0) {
    return (
      <EmptyState
        className="min-h-0 p-3"
        description={query.trim() ? t("goals.picker.emptyNoMatch", { query }) : t("goals.picker.emptyNoQuery")}
        title={t("goals.picker.emptyTitle")}
        variant={query.trim() ? "no-filter-match" : "no-data"}
      />
    );
  }
  return (
    <ul aria-label={t("goals.picker.resultsLabel")} className="grid max-h-56 gap-1 overflow-y-auto">
      {options.map((option) => (
        <li key={option.id}>
          <button
            className="w-full rounded-md border border-border px-2 py-1.5 text-left hover:bg-muted/40"
            onClick={() => onSelect(option)}
            type="button"
          >
            <ExecutionTargetOptionSummary kind={kind} option={option} />
          </button>
        </li>
      ))}
    </ul>
  );
}
