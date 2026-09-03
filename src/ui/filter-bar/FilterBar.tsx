import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";

export interface FilterDefinition {
  id: string;
  /** Already-localized label, e.g. "Status". */
  label: string;
  /**
   * Already-localized rendering of a value for this filter's chip. Takes `unknown` because a
   * page's filter set is necessarily heterogeneous (a status enum, a date range, ...) — each
   * definition is authored with its own concrete value type in scope, so the narrowing inside
   * `formatValue` stays sound without this interface itself needing (or being allowed) `any`.
   */
  formatValue: (value: unknown) => string;
}

export interface ActiveFilter {
  definitionId: string;
  value: unknown;
}

export interface FilterBarProps {
  definitions: FilterDefinition[];
  active: ActiveFilter[];
  onClearOne: (definitionId: string) => void;
  onClearAll: () => void;
  resultCount: number;
  /** When supplied and different from `resultCount`, renders "N of M results" instead of "N results". */
  totalCount?: number;
  className?: string;
}

export function FilterBar({ definitions, active, onClearOne, onClearAll, resultCount, totalCount, className }: FilterBarProps) {
  const { t } = useTranslation();
  const definitionById = new Map(definitions.map((definition) => [definition.id, definition]));

  return (
    <div className={cn("flex flex-wrap items-center gap-2 text-sm", className)}>
      {active.map((filter) => {
        const definition = definitionById.get(filter.definitionId);
        if (!definition) return null;
        return (
          <span className="ucd-status-neutral inline-flex items-center gap-1 rounded-sm border px-2 py-0.5 text-xs" key={filter.definitionId}>
            {definition.label}: {definition.formatValue(filter.value)}
            <button
              aria-label={t("workbenchUi.filterBar.clearFilter", { label: definition.label })}
              className="ucd-focus-ring rounded-sm"
              onClick={() => onClearOne(filter.definitionId)}
              type="button"
            >
              <X aria-hidden="true" className="h-3 w-3" />
            </button>
          </span>
        );
      })}
      {active.length > 0 ? (
        <button className="ucd-focus-ring rounded-sm text-xs font-medium text-muted-foreground hover:underline" onClick={onClearAll} type="button">
          {t("workbenchUi.filterBar.clearAll")}
        </button>
      ) : null}
      <span className="ml-auto text-xs text-muted-foreground">
        {totalCount !== undefined && totalCount !== resultCount
          ? t("workbenchUi.filterBar.resultCountOfTotal", { count: resultCount, total: totalCount })
          : t("workbenchUi.filterBar.resultCount", { count: resultCount })}
      </span>
    </div>
  );
}
