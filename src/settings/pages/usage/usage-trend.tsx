import { TrendingUp } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { UsageStatisticsPoint } from "../../../types/chat";
import { SectionPanel } from "../page-parts";
import { formatUsageDate, formatUsageNumber } from "./usage-format";

interface UsageTrendProps {
  daily: UsageStatisticsPoint[];
  language: string;
}

function pathFor(values: number[]) {
  const maximum = Math.max(...values, 1);
  return values
    .map((value, index) => {
      const x = values.length === 1 ? 50 : (index / (values.length - 1)) * 100;
      const y = 36 - (value / maximum) * 30;
      return `${index === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`;
    })
    .join(" ");
}

export function UsageTrend({ daily, language }: UsageTrendProps) {
  const { t } = useTranslation();
  const tokenValues = daily.map((point) => point.reported.totalTokens);
  const characterValues = daily.map((point) => point.estimated.totalCharacters);
  return (
    <SectionPanel description={t("usage.trend.description")} title={t("usage.trend.title")}>
      {daily.length === 0 ? (
        <p className="py-8 text-center text-sm text-muted-foreground">{t("usage.trend.empty")}</p>
      ) : (
        <div className="space-y-4">
          <div className="grid gap-3 sm:grid-cols-2">
            <TrendLine
              label={t("usage.trend.reported")}
              path={pathFor(tokenValues)}
              total={tokenValues.reduce((sum, value) => sum + value, 0)}
              language={language}
              variant="primary"
            />
            <TrendLine
              label={t("usage.trend.estimated")}
              path={pathFor(characterValues)}
              total={characterValues.reduce((sum, value) => sum + value, 0)}
              language={language}
              variant="muted"
            />
          </div>
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>{formatUsageDate(daily[0].date, language)}</span>
            <span>{formatUsageDate(daily[daily.length - 1].date, language)}</span>
          </div>
        </div>
      )}
    </SectionPanel>
  );
}

function TrendLine({ label, path, total, language, variant }: {
  label: string;
  path: string;
  total: number;
  language: string;
  variant: "primary" | "muted";
}) {
  return (
    <div className="rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3">
      <div className="flex items-center justify-between gap-2 text-sm font-medium">
        <span className="flex items-center gap-2"><TrendingUp className="h-4 w-4 text-primary" />{label}</span>
        <span>{formatUsageNumber(total, language)}</span>
      </div>
      <svg aria-label={label} className="mt-2 h-24 w-full" preserveAspectRatio="none" role="img" viewBox="0 0 100 40">
        <path className="stroke-border" d="M 0 36 L 100 36" fill="none" strokeWidth="0.6" />
        <path
          className={variant === "primary" ? "stroke-primary" : "stroke-muted-foreground"}
          d={path}
          fill="none"
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth="2"
          vectorEffect="non-scaling-stroke"
        />
      </svg>
    </div>
  );
}
