import { useQuery } from "@tanstack/react-query";
import { BarChart3 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { agentService } from "../../services/runtime-agent-client";
import type { UsageStatisticsRange } from "../../types/chat";
import type { SettingsPageStatus } from "../settings-page-types";
import { PageHeader } from "./page-parts";
import { UsageAccountingNote } from "./usage/usage-accounting-note";
import { UsageBreakdowns } from "./usage/usage-breakdowns";
import { UsageControls, type UsageFilterState } from "./usage/usage-controls";
import { UsageSummary } from "./usage/usage-summary";
import { UsageTrend } from "./usage/usage-trend";
import { preserveUsageData, usageRefetchInterval } from "./usage/usage-query";
import { UsageEmptyState, UsageLoadError } from "./usage/usage-status";
import { breakdownKeys, usageRangeQuery } from "./usage/usage-presentation";

export function UsageStatisticsPage({
  isActive = true,
  onStatusChange,
}: {
  isActive?: boolean;
  onStatusChange?: (status: SettingsPageStatus | null) => void;
}) {
  const { i18n, t } = useTranslation();
  const [range, setRange] = useState<UsageStatisticsRange>("last30Days");
  const [filters, setFilters] = useState<UsageFilterState>({});
  const rangeQuery = useMemo(() => usageRangeQuery(range), [range]);
  const optionsQuery = useQuery({
    queryKey: ["token-usage-options", range],
    queryFn: () => agentService.getTokenUsageSummary({ ...rangeQuery, breakdownLimit: 50 }),
    refetchInterval: usageRefetchInterval(isActive),
  });
  const usageQuery = useQuery({
    queryKey: ["token-usage-summary", range, filters],
    queryFn: () => agentService.getTokenUsageSummary({ ...rangeQuery, ...filters, breakdownLimit: 10 }),
    placeholderData: preserveUsageData,
    refetchInterval: usageRefetchInterval(isActive),
  });
  const stats = usageQuery.data;

  // Task 12.16: the same condition the banner below already renders from (`UsageLoadError`),
  // reported so this page's nav entry can flag it while the user is looking at a different page.
  useEffect(() => {
    onStatusChange?.(usageQuery.isError ? { kind: "error", labelKey: "usage.status.error" } : null);
    return () => onStatusChange?.(null);
  }, [onStatusChange, usageQuery.isError]);

  return (
    <div className="space-y-4">
      <PageHeader
        description={t("usage.description")}
        icon={BarChart3}
        title={t("usage.title")}
      />
      <UsageControls
        filters={filters}
        isFetching={usageQuery.isFetching}
        onFiltersChange={setFilters}
        onRangeChange={setRange}
        onRefresh={() => { void usageQuery.refetch(); void optionsQuery.refetch(); }}
        options={{
          agents: breakdownKeys(optionsQuery.data?.breakdowns ?? [], "agent"),
          providers: breakdownKeys(optionsQuery.data?.breakdowns ?? [], "provider"),
          models: breakdownKeys(optionsQuery.data?.breakdowns ?? [], "model"),
        }}
        range={range}
      />

      {usageQuery.isError ? (
        <UsageLoadError error={usageQuery.error} />
      ) : null}

      <UsageSummary language={i18n.language} loading={!stats && usageQuery.isPending} stats={stats} />

      {stats && stats.counts.calls === 0 ? (
        <UsageEmptyState />
      ) : null}
      <UsageTrend daily={stats?.daily ?? []} language={i18n.language} />
      <UsageBreakdowns breakdowns={stats?.breakdowns ?? []} language={i18n.language} />

      <UsageAccountingNote generatedAt={stats?.generatedAt} language={i18n.language} />
    </div>
  );
}
