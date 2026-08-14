import { CalendarDays, RefreshCw, RotateCcw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import { cn } from "../../../lib/utils";
import type { UsageStatisticsRange } from "../../../types/chat";
import type {
  TokenUsageFilters,
  UsagePurpose,
  UsageQuality,
  UsageStatus,
} from "../../../types/token-usage";

const ranges: UsageStatisticsRange[] = ["today", "last7Days", "last30Days", "all"];
const purposes: UsagePurpose[] = ["assistant-initial", "tool-continuation", "context-compaction", "memory-extraction", "retry", "terminal-interval"];
const qualities: UsageQuality[] = ["reported", "reported-derived", "estimated"];
const statuses: UsageStatus[] = ["running", "succeeded", "failed", "cancelled"];

export type UsageFilterState = Omit<TokenUsageFilters, "sessionId">;

interface UsageControlsProps {
  range: UsageStatisticsRange;
  filters: UsageFilterState;
  options: { agents: string[]; providers: string[]; models: string[] };
  isFetching: boolean;
  onFiltersChange: (filters: UsageFilterState) => void;
  onRangeChange: (range: UsageStatisticsRange) => void;
  onRefresh: () => void;
}

function FilterSelect({ label, onChange, options, value }: {
  label: string;
  onChange: (value: string | undefined) => void;
  options: Array<{ label: string; value: string }>;
  value?: string;
}) {
  const { t } = useTranslation();
  return (
    <label className="grid min-w-0 gap-1 text-xs font-medium text-muted-foreground">
      <span>{label}</span>
      <select
        className="h-10 min-w-0 rounded-md border border-border bg-background px-3 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        onChange={(event) => onChange(event.target.value || undefined)}
        value={value ?? ""}
      >
        <option value="">{t("usage.filters.all")}</option>
        {options.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
      </select>
    </label>
  );
}

export function UsageControls(props: UsageControlsProps) {
  const { t } = useTranslation();
  const setFilter = (key: keyof UsageFilterState, value: string | undefined) => {
    props.onFiltersChange({ ...props.filters, [key]: value });
  };
  return (
    <section className="ucd-panel space-y-4 rounded-lg p-4" aria-label={t("usage.filters.title")}>
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex flex-wrap items-center gap-2" aria-label={t("usage.range.label")}>
          {ranges.map((option) => (
            <button
              aria-pressed={option === props.range}
              className={cn(
                "inline-flex min-h-10 items-center gap-2 rounded-md border px-3 text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                option === props.range
                  ? "border-primary bg-[hsl(var(--nav-active-soft))] text-primary"
                  : "border-border bg-[hsl(var(--panel-muted))] text-muted-foreground hover:text-foreground",
              )}
              key={option}
              onClick={() => props.onRangeChange(option)}
              type="button"
            >
              <CalendarDays className="h-4 w-4" aria-hidden="true" />
              {t(`usage.range.${option}`)}
            </button>
          ))}
        </div>
        <Button disabled={props.isFetching} onClick={props.onRefresh} variant="outline">
          <RefreshCw className={cn("h-4 w-4", props.isFetching && "animate-spin motion-reduce:animate-none")} aria-hidden="true" />
          {props.isFetching ? t("usage.refreshing") : t("usage.refresh")}
        </Button>
      </div>
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
        <FilterSelect label={t("usage.filters.agent")} onChange={(value) => setFilter("agentId", value)} options={props.options.agents.map((value) => ({ label: value, value }))} value={props.filters.agentId} />
        <FilterSelect label={t("usage.filters.provider")} onChange={(value) => setFilter("providerId", value)} options={props.options.providers.map((value) => ({ label: value, value }))} value={props.filters.providerId} />
        <FilterSelect label={t("usage.filters.model")} onChange={(value) => setFilter("modelId", value)} options={props.options.models.map((value) => ({ label: value, value }))} value={props.filters.modelId} />
        <FilterSelect label={t("usage.filters.purpose")} onChange={(value) => setFilter("purpose", value)} options={purposes.map((value) => ({ label: t(`usage.purpose.${value}`), value }))} value={props.filters.purpose} />
        <FilterSelect label={t("usage.filters.quality")} onChange={(value) => setFilter("quality", value)} options={qualities.map((value) => ({ label: t(`usage.quality.${value}`), value }))} value={props.filters.quality} />
        <FilterSelect label={t("usage.filters.status")} onChange={(value) => setFilter("status", value)} options={statuses.map((value) => ({ label: t(`usage.status.${value}`), value }))} value={props.filters.status} />
      </div>
      {Object.values(props.filters).some(Boolean) ? (
        <Button onClick={() => props.onFiltersChange({})} variant="ghost">
          <RotateCcw className="h-4 w-4" aria-hidden="true" />{t("usage.filters.reset")}
        </Button>
      ) : null}
    </section>
  );
}
