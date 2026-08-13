import { Activity, CircleGauge, MessagesSquare, Sigma, Workflow } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { TokenUsageSummary } from "../../../types/token-usage";
import { StatCard } from "../page-parts";
import { formatUsageNumber } from "./usage-format";
import { dimensionRows, reportedCoverage, tokenTotal } from "./usage-presentation";

interface UsageSummaryProps {
  stats?: TokenUsageSummary;
  language: string;
  loading: boolean;
}

export function UsageSummary({ stats, language, loading }: UsageSummaryProps) {
  const { t } = useTranslation();
  const value = (amount: number | null | undefined) => {
    if (loading) return t("usage.loading");
    return amount === null || amount === undefined ? t("usage.unknown") : formatUsageNumber(amount, language);
  };
  const coverage = stats ? reportedCoverage(stats.totals) : { reported: 0, total: 0, percent: 0 };
  return (
    <div className="space-y-3" aria-live="polite">
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <StatCard hint={t("usage.total.hint")} icon={Sigma} label={t("usage.total.title")} value={value(stats ? tokenTotal(stats.totals) : 0)} />
        <StatCard hint={t("usage.reported.totalHint")} icon={CircleGauge} label={t("usage.reported.total")} value={value(stats ? stats.totals.reported.headlineTotal : 0)} />
        <StatCard hint={t("usage.derived.hint")} icon={Workflow} label={t("usage.derived.title")} value={value(stats ? stats.totals.reportedDerived.headlineTotal : 0)} />
        <StatCard hint={t("usage.estimated.totalHint")} icon={Activity} label={t("usage.estimated.total")} value={value(stats ? stats.totals.estimated.headlineTotal : 0)} />
      </div>
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
        <StatCard hint={t("usage.userResponse.hint")} icon={MessagesSquare} label={t("usage.userResponse.title")} value={value(stats ? tokenTotal(stats.userResponse) : 0)} />
        <StatCard hint={t("usage.internal.hint")} icon={Workflow} label={t("usage.internal.title")} value={value(stats ? tokenTotal(stats.internal) : 0)} />
        <StatCard hint={t("usage.calls.hint")} icon={Activity} label={t("usage.calls.title")} value={value(stats?.counts.calls ?? 0)} />
        <StatCard hint={t("usage.generations.hint")} icon={Workflow} label={t("usage.generations.title")} value={value(stats?.counts.generations ?? 0)} />
        <StatCard hint={t("usage.sessions.hint")} icon={MessagesSquare} label={t("usage.sessions.title")} value={value(stats?.counts.sessions ?? 0)} />
        <StatCard
          hint={t("usage.coverage.hint")}
          icon={CircleGauge}
          label={t("usage.coverage.title")}
          value={loading ? t("usage.loading") : t("usage.coverage.value", {
            percent: formatUsageNumber(coverage.percent, language),
            reported: formatUsageNumber(coverage.reported, language),
            total: formatUsageNumber(coverage.total, language),
          })}
        />
      </div>
      <section className="ucd-panel rounded-lg p-4">
        <h3 className="text-sm font-semibold">{t("usage.dimensions.title")}</h3>
        <p className="mt-1 text-xs leading-5 text-muted-foreground">{t("usage.dimensions.hint")}</p>
        <dl className="mt-3 grid gap-3 sm:grid-cols-2 xl:grid-cols-6">
          {dimensionRows(stats?.totals.reported.dimensions ?? { input: 0, output: 0, cachedInput: 0, cacheWriteInput: 0, reasoningOutput: 0, providerTotal: 0 }).map(([key, amount]) => (
            <div className="min-w-0" key={key}><dt className="truncate text-xs text-muted-foreground">{t(`usage.dimensions.${key}`)}</dt><dd className="mt-1 font-medium tabular-nums">{value(amount)}</dd></div>
          ))}
        </dl>
      </section>
    </div>
  );
}
