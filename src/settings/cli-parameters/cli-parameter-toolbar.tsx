import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import type { CliLaunchScope } from "../../types/cli-parameter";
import { cliParameterFilters, type CliParameterFilter } from "./view-model";

export interface CliParameterToolbarProps {
  scope: CliLaunchScope;
  onScopeChange: (scope: CliLaunchScope) => void;
  query: string;
  onQueryChange: (query: string) => void;
  filter: CliParameterFilter;
  onFilterChange: (filter: CliParameterFilter) => void;
}

const scopes: readonly CliLaunchScope[] = ["chat", "interactive"];

/** Scope is an explicit control rather than an implicit "whatever chat renders", because the same
 * parameter can belong to one launch and not the other and the preview has to say which. */
export function CliParameterToolbar({
  scope,
  onScopeChange,
  query,
  onQueryChange,
  filter,
  onFilterChange,
}: CliParameterToolbarProps) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-wrap items-center gap-3">
      <div aria-label={t("cliParameters.scopeSelector.label")} className="flex gap-1" role="group">
        {scopes.map((candidate) => (
          <Button
            aria-pressed={scope === candidate}
            key={candidate}
            onClick={() => onScopeChange(candidate)}
            size="sm"
            variant={scope === candidate ? "default" : "outline"}
          >
            {t(`cliParameters.scope.${candidate}Short`)}
          </Button>
        ))}
      </div>

      <label className="flex min-w-[12rem] flex-1 items-center gap-2 text-sm">
        <span className="sr-only">{t("cliParameters.search.label")}</span>
        <input
          className="min-h-9 w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
          onChange={(event) => onQueryChange(event.currentTarget.value)}
          placeholder={t("cliParameters.search.placeholder")}
          type="search"
          value={query}
        />
      </label>

      <div aria-label={t("cliParameters.filters.label")} className="flex flex-wrap gap-1" role="group">
        {cliParameterFilters.map((candidate) => (
          <Button
            aria-pressed={filter === candidate}
            key={candidate}
            onClick={() => onFilterChange(candidate)}
            size="sm"
            variant={filter === candidate ? "default" : "ghost"}
          >
            {t(`cliParameters.filters.${candidate}`)}
          </Button>
        ))}
      </div>
    </div>
  );
}
