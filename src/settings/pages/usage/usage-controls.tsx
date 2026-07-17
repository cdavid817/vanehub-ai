import { CalendarDays, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import { cn } from "../../../lib/utils";
import type { UsageStatisticsRange } from "../../../types/chat";

const ranges: UsageStatisticsRange[] = ["today", "last7Days", "last30Days", "all"];

interface UsageControlsProps {
  range: UsageStatisticsRange;
  isFetching: boolean;
  onRangeChange: (range: UsageStatisticsRange) => void;
  onRefresh: () => void;
}

export function UsageControls({ range, isFetching, onRangeChange, onRefresh }: UsageControlsProps) {
  const { t } = useTranslation();
  return (
    <section className="ucd-panel flex flex-wrap items-center justify-between gap-3 rounded-lg p-4">
      <div className="flex flex-wrap items-center gap-2" aria-label={t("usage.range.label")}>
        {ranges.map((option) => {
          const active = option === range;
          return (
            <button
              aria-pressed={active}
              className={cn(
                "inline-flex h-9 items-center gap-2 rounded-md border px-3 text-sm font-medium transition-colors",
                active
                  ? "border-primary bg-[hsl(var(--nav-active-soft))] text-primary"
                  : "border-border bg-[hsl(var(--panel-muted))] text-muted-foreground hover:text-foreground",
              )}
              key={option}
              onClick={() => onRangeChange(option)}
              type="button"
            >
              <CalendarDays className="h-4 w-4" aria-hidden="true" />
              {t(`usage.range.${option}`)}
            </button>
          );
        })}
      </div>
      <Button disabled={isFetching} onClick={onRefresh} variant="outline">
        <RefreshCw className={cn("h-4 w-4", isFetching && "animate-spin")} aria-hidden="true" />
        {isFetching ? t("usage.refreshing") : t("usage.refresh")}
      </Button>
    </section>
  );
}
