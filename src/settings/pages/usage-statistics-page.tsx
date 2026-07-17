import { useQuery } from "@tanstack/react-query";
import { BarChart3 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { agentService } from "../../services/runtime-agent-client";
import type { UsageStatisticsRange } from "../../types/chat";
import { PageHeader } from "./page-parts";
import { UsageAccountingNote } from "./usage/usage-accounting-note";
import { UsageAgentBreakdown } from "./usage/usage-agent-breakdown";
import { UsageControls } from "./usage/usage-controls";
import { UsageSummary } from "./usage/usage-summary";
import { UsageTrend } from "./usage/usage-trend";
import { preserveUsageData, usagePollingInterval } from "./usage/usage-query";
import { UsageLoadError } from "./usage/usage-status";

export function UsageStatisticsPage() {
  const { i18n, t } = useTranslation();
  const [range, setRange] = useState<UsageStatisticsRange>("last30Days");
  const usageQuery = useQuery({
    queryKey: ["usage-statistics", range],
    queryFn: () => agentService.getUsageStatistics({ range }),
    placeholderData: preserveUsageData,
    refetchInterval: usagePollingInterval,
  });
  const stats = usageQuery.data;

  return (
    <div className="space-y-4">
      <PageHeader
        description={t("usage.description")}
        icon={BarChart3}
        title={t("usage.title")}
      />
      <UsageControls
        isFetching={usageQuery.isFetching}
        onRangeChange={setRange}
        onRefresh={() => void usageQuery.refetch()}
        range={range}
      />

      {usageQuery.isError ? (
        <UsageLoadError error={usageQuery.error} />
      ) : null}

      <UsageSummary language={i18n.language} loading={!stats && usageQuery.isPending} stats={stats} />

      {stats && stats.coverage.totalResponses === 0 ? (
        <div className="ucd-panel rounded-lg p-8 text-center text-sm text-muted-foreground">
          {t("usage.empty")}
        </div>
      ) : null}
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.4fr)_minmax(300px,0.6fr)]">
        <UsageTrend daily={stats?.daily ?? []} language={i18n.language} />
        <UsageAgentBreakdown agents={stats?.byAgent ?? []} language={i18n.language} />
      </div>

      <UsageAccountingNote generatedAt={stats?.generatedAt} language={i18n.language} />
    </div>
  );
}
