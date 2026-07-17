import { useQuery } from "@tanstack/react-query";
import { AlertTriangle, BarChart3, CalendarDays, Hash, MessageSquareText, RefreshCw, Sigma, TrendingDown, TrendingUp } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { cn } from "../../lib/utils";
import { agentService } from "../../services/runtime-agent-client";
import type { UsageStatisticsRange } from "../../types/chat";
import { PageHeader, SectionPanel, StatCard } from "./page-parts";

const rangeOptions: UsageStatisticsRange[] = ["today", "last7Days", "last30Days", "all"];

function formatNumber(value: number, language: string) {
  return new Intl.NumberFormat(language).format(value);
}

function formatGeneratedAt(value: string | undefined, language: string) {
  if (!value) return "";
  return new Intl.DateTimeFormat(language, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

export function UsageStatisticsPage() {
  const { i18n, t } = useTranslation();
  const [range, setRange] = useState<UsageStatisticsRange>("last30Days");
  const usageQuery = useQuery({
    queryKey: ["usage-statistics", range],
    queryFn: () => agentService.getUsageStatistics({ range }),
  });
  const stats = usageQuery.data;
  const language = i18n.language;

  const actions = (
    <Button disabled={usageQuery.isFetching} onClick={() => void usageQuery.refetch()} variant="outline">
      <RefreshCw className={usageQuery.isFetching ? "h-4 w-4 animate-spin" : "h-4 w-4"} aria-hidden="true" />
      {usageQuery.isFetching ? t("usage.refreshing") : t("usage.refresh")}
    </Button>
  );

  return (
    <div className="space-y-4">
      <PageHeader actions={actions} description={t("usage.description")} icon={BarChart3} title={t("usage.title")} />

      <section className="ucd-panel rounded-lg p-4">
        <div className="flex flex-wrap items-center gap-2">
          {rangeOptions.map((option) => {
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
                onClick={() => setRange(option)}
                type="button"
              >
                <CalendarDays className="h-4 w-4" aria-hidden="true" />
                {t(`usage.range.${option}`)}
              </button>
            );
          })}
        </div>
      </section>

      {usageQuery.isError ? (
        <div className="rounded-lg border p-4 text-sm ucd-status-danger">
          {t("usage.error", { message: usageQuery.error instanceof Error ? usageQuery.error.message : String(usageQuery.error) })}
        </div>
      ) : null}

      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
        <StatCard
          hint={t("usage.stats.totalHint")}
          icon={Sigma}
          label={t("usage.stats.total")}
          value={formatNumber(stats?.totalTokens ?? 0, language)}
        />
        <StatCard
          hint={t("usage.stats.inputHint")}
          icon={TrendingDown}
          label={t("usage.stats.input")}
          value={formatNumber(stats?.inputTokens ?? 0, language)}
        />
        <StatCard
          hint={t("usage.stats.outputHint")}
          icon={TrendingUp}
          label={t("usage.stats.output")}
          value={formatNumber(stats?.outputTokens ?? 0, language)}
        />
        <StatCard
          hint={t("usage.stats.messagesHint")}
          icon={MessageSquareText}
          label={t("usage.stats.messages")}
          value={formatNumber(stats?.countedMessages ?? 0, language)}
        />
      </div>

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_360px]">
        <SectionPanel title={t("usage.breakdown.title")} description={t("usage.breakdown.description")}>
          <div className="grid gap-3 md:grid-cols-2">
            <div className="rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3">
              <div className="flex items-center gap-2 text-sm font-medium">
                <Hash className="h-4 w-4 text-primary" aria-hidden="true" />
                {t("usage.breakdown.sessions")}
              </div>
              <div className="mt-2 text-2xl font-semibold text-primary">
                {formatNumber(stats?.countedSessions ?? 0, language)}
              </div>
              <p className="mt-1 text-xs leading-5 text-muted-foreground">{t("usage.breakdown.sessionsHint")}</p>
            </div>
            <div className="rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3">
              <div className="flex items-center gap-2 text-sm font-medium">
                <CalendarDays className="h-4 w-4 text-primary" aria-hidden="true" />
                {t("usage.breakdown.generatedAt")}
              </div>
              <div className="mt-2 text-sm font-semibold text-foreground">
                {stats ? formatGeneratedAt(stats.generatedAt, language) : t("usage.loading")}
              </div>
              <p className="mt-1 text-xs leading-5 text-muted-foreground">{t("usage.breakdown.generatedAtHint")}</p>
            </div>
          </div>
        </SectionPanel>

        <SectionPanel title={t("usage.constraints.title")} description={t("usage.constraints.description")}>
          <div className="flex gap-3 rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3">
            <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-primary" aria-hidden="true" />
            <div className="space-y-2 text-sm leading-6 text-muted-foreground">
              <p>{t("usage.constraints.current")}</p>
              <p>{t("usage.constraints.future")}</p>
            </div>
          </div>
        </SectionPanel>
      </div>
    </div>
  );
}
