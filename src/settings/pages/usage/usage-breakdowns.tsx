import { Boxes } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type { UsageBreakdown } from "../../../types/token-usage";
import { SectionPanel } from "../page-parts";
import { formatUsageNumber } from "./usage-format";
import { tokenTotal } from "./usage-presentation";

export function UsageBreakdowns({ breakdowns, language }: {
  breakdowns: UsageBreakdown[];
  language: string;
}) {
  const { t } = useTranslation();
  const populated = breakdowns.filter((breakdown) => breakdown.entries.length > 0);
  return (
    <SectionPanel description={t("usage.breakdowns.description")} icon={Boxes} title={t("usage.breakdowns.title")} variant="plain">
      {populated.length === 0 ? <p className="py-8 text-center text-sm text-muted-foreground">{t("usage.breakdowns.empty")}</p> : (
        <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          {populated.map((breakdown) => (
            <section className="min-w-0 rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3" key={breakdown.dimension}>
              <h4 className="mb-3 text-sm font-semibold">{t(`usage.filters.${breakdown.dimension}`)}</h4>
              <div className="space-y-2">
                {breakdown.entries.map((entry) => {
                  const tokens = tokenTotal(entry.totals);
                  const characters = entry.totals.estimated.headlineTotal;
                  return (
                    <article className="rounded-md border border-border bg-background p-3" key={entry.key}>
                      <div className="flex min-w-0 items-center justify-between gap-2">
                        <Badge className="max-w-full truncate" tone="muted">{entry.key === "unknown" ? t("usage.unknown") : entry.key}</Badge>
                        <span className="shrink-0 text-xs text-muted-foreground">{t("usage.calls.value", { count: entry.counts.calls })}</span>
                      </div>
                      <dl className="mt-2 grid grid-cols-2 gap-2 text-xs">
                        <div><dt className="text-muted-foreground">{t("usage.units.tokens")}</dt><dd className="mt-0.5 font-medium tabular-nums">{tokens === null ? t("usage.unknown") : formatUsageNumber(tokens, language)}</dd></div>
                        <div><dt className="text-muted-foreground">{t("usage.units.characters")}</dt><dd className="mt-0.5 font-medium tabular-nums">{characters === null ? t("usage.unknown") : formatUsageNumber(characters, language)}</dd></div>
                      </dl>
                    </article>
                  );
                })}
              </div>
            </section>
          ))}
        </div>
      )}
    </SectionPanel>
  );
}
