import { useQuery } from "@tanstack/react-query";
import { ChevronDown, ChevronUp, Gauge } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { formatAppNumber } from "../i18n/format";
import { agentService } from "../services/runtime-agent-client";
import type { SessionLifecycleState } from "../types/agent";
import type { UsageQualityTotals } from "../types/token-usage";

function Metric({ label, language, value }: { label: string; language: string; value: number | null }) {
  const { t } = useTranslation();
  return (
    <div className="min-w-0">
      <dt className="truncate text-xs text-muted-foreground">{label}</dt>
      <dd className="mt-1 text-lg font-semibold tabular-nums text-primary">{value === null ? t("usage.unknown") : formatAppNumber(value, language)}</dd>
    </div>
  );
}

function QualityMetrics({ language, totals }: { language: string; totals: UsageQualityTotals }) {
  const { t } = useTranslation();
  return (
    <dl className="grid grid-cols-2 gap-2">
      <Metric label={t("usage.quality.reported")} language={language} value={totals.reported.headlineTotal} />
      <Metric label={t("usage.quality.reported-derived")} language={language} value={totals.reportedDerived.headlineTotal} />
      <Metric label={t("usage.quality.estimated")} language={language} value={totals.estimated.headlineTotal} />
      <Metric label={t("usage.calls.title")} language={language} value={totals.reported.callCount + totals.reportedDerived.callCount + totals.estimated.callCount} />
    </dl>
  );
}

function EmptyState({ children }: { children: React.ReactNode }) {
  return <p className="p-3 text-center text-xs text-muted-foreground">{children}</p>;
}

export function SessionTokenUsagePane({ active = true, lifecycle, sessionId }: {
  /**
   * Whether this pane is the one on screen.
   *
   * Mounted either way — these panes hold local form state, and a reader who typed something,
   * checked another tab, and came back must find it still there. What stops is the reading: a
   * hidden pane polling its own service costs a request per pane per session open, for answers
   * nobody is looking at.
   *
   * Mutations are unaffected. React Query runs one to completion regardless of this flag, so a
   * write that was in flight when the reader switched away still finishes and still invalidates.
   */
  active?: boolean;
  lifecycle?: SessionLifecycleState;
  sessionId: string | null;
}) {
  const { i18n, t } = useTranslation();
  const [detailsOpen, setDetailsOpen] = useState(false);
  const summary = useQuery({
    enabled: active && Boolean(sessionId),
    queryKey: ["token-usage-summary", "session", sessionId],
    queryFn: () => agentService.getTokenUsageSummary({ sessionId: sessionId ?? "", breakdownLimit: 4 }),
    refetchInterval: lifecycle === "running" ? 5000 : false,
  });
  const details = useQuery({
    enabled: active && detailsOpen && Boolean(sessionId),
    queryKey: ["token-usage-details", sessionId],
    queryFn: () => agentService.getTokenUsageDetails({ sessionId: sessionId ?? "", limit: 10 }),
  });
  if (summary.isLoading) return <EmptyState>{t("layout.info.loading")}</EmptyState>;
  if (summary.isError) return <EmptyState>{t("usage.error", { message: summary.error.message })}</EmptyState>;
  if (!summary.data || summary.data.counts.calls === 0) return <EmptyState>{t("layout.info.noUsage")}</EmptyState>;
  const purposeBreakdown = summary.data.breakdowns.find(({ dimension }) => dimension === "purpose");

  return (
    <div className="grid gap-3">
      <section className="ucd-muted-panel rounded-lg p-3">
        <h3 className="mb-3 flex items-center gap-2 text-sm font-semibold"><Gauge className="h-4 w-4 text-primary" aria-hidden="true" />{t("layout.info.usage.total")}</h3>
        <QualityMetrics language={i18n.language} totals={summary.data.totals} />
      </section>
      <section className="ucd-muted-panel rounded-lg p-3">
        <h3 className="mb-3 text-sm font-semibold">{t("usage.purpose.title")}</h3>
        <div className="flex flex-wrap gap-2">
          {purposeBreakdown?.entries.map((entry) => <Badge key={entry.key} tone="muted">{t(`usage.purpose.${entry.key}`)} · {formatAppNumber(entry.counts.calls, i18n.language)}</Badge>)}
        </div>
      </section>
      <Button aria-expanded={detailsOpen} onClick={() => setDetailsOpen((open) => !open)} variant="outline">
        {detailsOpen ? <ChevronUp className="h-4 w-4" aria-hidden="true" /> : <ChevronDown className="h-4 w-4" aria-hidden="true" />}
        {t("usage.details.toggle")}
      </Button>
      {detailsOpen ? (
        <section aria-live="polite" className="space-y-2">
          {details.isLoading ? <EmptyState>{t("usage.loading")}</EmptyState> : null}
          {details.isError ? <EmptyState>{t("usage.error", { message: details.error.message })}</EmptyState> : null}
          {details.data?.invocations.map((invocation) => {
            const observation = details.data.observations.find((item) => item.invocationId === invocation.id);
            return (
              <article className="rounded-md border border-border bg-background p-3 text-xs" key={invocation.id}>
                <div className="flex flex-wrap items-center gap-1.5"><Badge tone="muted">{t(`usage.purpose.${invocation.purpose}`)}</Badge><Badge tone={invocation.status === "failed" ? "danger" : "muted"}>{t(`usage.status.${invocation.status}`)}</Badge></div>
                <p className="mt-2 wrap-break-word text-muted-foreground">{invocation.providerId ?? t("usage.unknown")} · {invocation.modelId ?? t("usage.unknown")}</p>
                <p className="mt-1 font-medium tabular-nums">{observation?.dimensions.providerTotal ?? t("usage.unknown")} {observation?.unit === "characters" ? t("usage.units.characters") : t("usage.units.tokens")}</p>
              </article>
            );
          })}
        </section>
      ) : null}
    </div>
  );
}
