import { ArrowUpCircle, RefreshCw, Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";

/** Search, the two filters, the attention toggle, and the two page-level actions. */
export function CliToolbar({
  search,
  sourceFilter,
  sourceIds,
  attentionOnly,
  refreshing,
  bulkEligibleCount,
  bulkPending,
  onSearchChange,
  onSourceFilterChange,
  onAttentionOnlyChange,
  onRefreshAll,
  onBulkUpgrade,
}: {
  search: string;
  sourceFilter: string;
  sourceIds: readonly string[];
  attentionOnly: boolean;
  refreshing: boolean;
  bulkEligibleCount: number;
  bulkPending: boolean;
  onSearchChange: (value: string) => void;
  onSourceFilterChange: (value: string) => void;
  onAttentionOnlyChange: (value: boolean) => void;
  onRefreshAll: () => void;
  onBulkUpgrade: () => void;
}) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-wrap items-center gap-2">
      <div className="relative min-w-48 flex-1">
        <Search
          aria-hidden="true"
          className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground"
        />
        <input
          aria-label={t("cli.filter.search")}
          className="ucd-input h-9 w-full rounded pl-8 pr-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
          placeholder={t("cli.filter.searchPlaceholder")}
          type="search"
          value={search}
          onChange={(event) => onSearchChange(event.target.value)}
        />
      </div>

      <select
        aria-label={t("cli.filter.source")}
        className="ucd-input h-9 rounded px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
        value={sourceFilter}
        onChange={(event) => onSourceFilterChange(event.target.value)}
      >
        <option value="all">{t("cli.filter.allSources")}</option>
        {sourceIds.map((sourceId) => (
          <option key={sourceId} value={sourceId}>
            {t(`cli.source.${sourceId}`)}
          </option>
        ))}
      </select>

      <label className="flex h-9 items-center gap-2 rounded px-2 text-sm">
        <input
          checked={attentionOnly}
          className="h-4 w-4 rounded border-border focus-visible:ring-2 focus-visible:ring-ring"
          type="checkbox"
          onChange={(event) => onAttentionOnlyChange(event.target.checked)}
        />
        {t("cli.filter.attentionOnly")}
      </label>

      <Button disabled={refreshing} variant="outline" onClick={onRefreshAll}>
        <RefreshCw aria-hidden="true" className={refreshing ? "h-4 w-4 animate-spin" : "h-4 w-4"} />
        {t(refreshing ? "cli.refreshing" : "cli.refresh")}
      </Button>

      <Button disabled={bulkPending || bulkEligibleCount === 0} onClick={onBulkUpgrade}>
        <ArrowUpCircle
          aria-hidden="true"
          className={bulkPending ? "h-4 w-4 animate-spin" : "h-4 w-4"}
        />
        {t("cli.upgradeAll", { count: bulkEligibleCount })}
      </Button>
    </div>
  );
}
