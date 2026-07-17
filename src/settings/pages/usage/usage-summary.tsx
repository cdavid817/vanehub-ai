import { Activity, Binary, Database, MessageSquareText, Sigma } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { UsageStatistics } from "../../../types/chat";
import { StatCard } from "../page-parts";
import { formatUsageNumber } from "./usage-format";

interface UsageSummaryProps {
  stats?: UsageStatistics;
  language: string;
  loading: boolean;
}

export function UsageSummary({ stats, language, loading }: UsageSummaryProps) {
  const { t } = useTranslation();
  const value = (amount: number) => (loading ? t("usage.loading") : formatUsageNumber(amount, language));
  return (
    <div className="space-y-3">
      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
        <StatCard
          hint={t("usage.reported.totalHint")}
          icon={Sigma}
          label={t("usage.reported.total")}
          value={value(stats?.reported.totalTokens ?? 0)}
        />
        <StatCard
          hint={t("usage.reported.inputHint")}
          icon={Binary}
          label={t("usage.reported.input")}
          value={value(stats?.reported.inputTokens ?? 0)}
        />
        <StatCard
          hint={t("usage.reported.outputHint")}
          icon={Activity}
          label={t("usage.reported.output")}
          value={value(stats?.reported.outputTokens ?? 0)}
        />
        <StatCard
          hint={t("usage.reported.cacheHint")}
          icon={Database}
          label={t("usage.reported.cache")}
          value={value(
            (stats?.reported.cacheReadTokens ?? 0) + (stats?.reported.cacheCreationTokens ?? 0),
          )}
        />
      </div>
      <div className="grid gap-3 md:grid-cols-3">
        <StatCard
          hint={t("usage.estimated.totalHint")}
          icon={Sigma}
          label={t("usage.estimated.total")}
          value={value(stats?.estimated.totalCharacters ?? 0)}
        />
        <StatCard
          hint={t("usage.coverage.hint")}
          icon={MessageSquareText}
          label={t("usage.coverage.title")}
          value={loading ? t("usage.loading") : t("usage.coverage.value", {
            percent: formatUsageNumber(stats?.coverage.reportedPercent ?? 0, language),
            reported: formatUsageNumber(stats?.coverage.reportedResponses ?? 0, language),
            total: formatUsageNumber(stats?.coverage.totalResponses ?? 0, language),
          })}
        />
        <StatCard
          hint={t("usage.sessions.hint")}
          icon={Activity}
          label={t("usage.sessions.title")}
          value={value(stats?.countedSessions ?? 0)}
        />
      </div>
    </div>
  );
}
