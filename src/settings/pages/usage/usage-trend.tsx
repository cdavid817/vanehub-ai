import { TrendingUp } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { UsageDailyPoint } from "../../../types/token-usage";
import { SectionPanel } from "../page-parts";
import { formatUsageDate, formatUsageNumber } from "./usage-format";

interface UsageTrendProps { daily: UsageDailyPoint[]; language: string }

function pathFor(values: number[]) {
  const maximum = Math.max(...values, 1);
  return values.map((value, index) => {
    const x = values.length === 1 ? 50 : (index / (values.length - 1)) * 100;
    const y = 36 - (value / maximum) * 30;
    return `${index === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`;
  }).join(" ");
}

export function UsageTrend({ daily, language }: UsageTrendProps) {
  const { t } = useTranslation();
  const series = [
    { key: "reported", values: daily.map((point) => point.totals.reported.headlineTotal), tone: "stroke-primary" },
    { key: "reported-derived", values: daily.map((point) => point.totals.reportedDerived.headlineTotal), tone: "stroke-chart-2" },
    { key: "estimated", values: daily.map((point) => point.totals.estimated.headlineTotal), tone: "stroke-muted-foreground" },
  ];
  return (
    <SectionPanel description={t("usage.trend.description")} title={t("usage.trend.title")} variant="plain">
      {daily.length === 0 ? <p className="py-8 text-center text-sm text-muted-foreground">{t("usage.trend.empty")}</p> : (
        <div className="space-y-4">
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
            {series.map((item) => {
              const knownValues = item.values.filter((value): value is number => value !== null);
              const known = knownValues.length === item.values.length;
              return <TrendLine key={item.key} label={t(`usage.quality.${item.key}`)} language={language} path={known ? pathFor(knownValues) : ""} tone={item.tone} total={known ? knownValues.reduce((sum, value) => sum + value, 0) : null} />;
            })}
          </div>
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>{formatUsageDate(daily[0].localDate, language)}</span>
            <span>{formatUsageDate(daily[daily.length - 1].localDate, language)}</span>
          </div>
        </div>
      )}
    </SectionPanel>
  );
}

function TrendLine({ label, path, total, language, tone }: { label: string; path: string; total: number | null; language: string; tone: string }) {
  const { t } = useTranslation();
  return (
    <div className="rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3">
      <div className="flex items-center justify-between gap-2 text-sm font-medium"><span className="flex items-center gap-2"><TrendingUp className="h-4 w-4 text-primary" aria-hidden="true" />{label}</span><span>{total === null ? t("usage.unknown") : formatUsageNumber(total, language)}</span></div>
      <svg aria-label={label} className="mt-2 h-24 w-full" preserveAspectRatio="none" role="img" viewBox="0 0 100 40">
        <path className="stroke-border" d="M 0 36 L 100 36" fill="none" strokeWidth="0.6" />
        {path ? <path className={tone} d={path} fill="none" strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" vectorEffect="non-scaling-stroke" /> : null}
      </svg>
    </div>
  );
}
