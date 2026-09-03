import { useTranslation } from "react-i18next";
import { formatAppDateTime, formatAppNumber } from "../i18n/format";
import type { MissionControlRunSummary } from "../types/mission-control";
import type { ModelInvocation } from "../types/token-usage";
import { AsyncBoundary } from "../ui/async/AsyncBoundary";
import { useMissionControlUsage, type MissionControlUsageData } from "./use-mission-control-usage";

function UsageStat({ label, value }: { label: string; value: string }) {
  return <div><dt className="text-[11px] text-muted-foreground">{label}</dt><dd className="mt-0.5 font-medium tabular-nums">{value}</dd></div>;
}

/**
 * The Usage facet: renders `useMissionControlUsage`'s own `AsyncViewState<MissionControlUsageData>`
 * through the shared `AsyncBoundary` (16.11) instead of a bespoke loading/error/empty/ready union.
 */
export function UsageFacet({ run }: { run: MissionControlRunSummary }) {
  const { t } = useTranslation();
  const { reload, ...state } = useMissionControlUsage(run, t("missionControl.usage.empty"), t("missionControl.usage.error"));

  return (
    <div className="mt-4 space-y-3" data-testid="mission-control-usage-facet">
      <AsyncBoundary onRetry={reload} state={state} unavailableState={{ title: t("missionControl.usage.empty") }}>
        {(data) => <UsageFacetContent invocations={data.invocations} summary={data.summary} />}
      </AsyncBoundary>
    </div>
  );
}

function UsageFacetContent({ invocations, summary }: MissionControlUsageData) {
  const { t, i18n } = useTranslation();
  const language = i18n.language;
  const number = (value: number | null) => value === null ? t("usage.unknown") : formatAppNumber(value, language, { maximumFractionDigits: 1 });
  return (
    <>
      <dl className="grid grid-cols-2 gap-3 sm:grid-cols-3">
        <UsageStat label={t("usage.reported.total")} value={number(summary.totals.reported.headlineTotal)} />
        <UsageStat label={t("usage.derived.title")} value={number(summary.totals.reportedDerived.headlineTotal)} />
        <UsageStat label={t("usage.estimated.total")} value={number(summary.totals.estimated.headlineTotal)} />
        <UsageStat label={t("usage.calls.title")} value={formatAppNumber(summary.counts.calls, language)} />
        <UsageStat label={t("usage.generations.title")} value={formatAppNumber(summary.counts.generations, language)} />
        <UsageStat label={t("usage.sessions.title")} value={formatAppNumber(summary.counts.sessions, language)} />
      </dl>
      <div>
        <h3 className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">{t("missionControl.usage.invocations")}</h3>
        {invocations.length ? (
          <ul className="mt-2 space-y-1" data-testid="mission-control-usage-invocations">
            {invocations.map((invocation: ModelInvocation) => (
              <li className="rounded-md border border-border bg-card px-2 py-1.5 text-xs" key={invocation.id}>
                <div className="flex flex-wrap items-center justify-between gap-1">
                  <span className="font-medium">{invocation.modelId ?? invocation.providerId ?? t("missionControl.usage.unknownModel")}</span>
                  <span className="text-muted-foreground">{formatAppDateTime(invocation.startedAt, language, { dateStyle: "short", timeStyle: "short" })}</span>
                </div>
                <p className="mt-0.5 text-muted-foreground">{t(`usage.status.${invocation.status}`)} · {t(`usage.purpose.${invocation.purpose}`)}</p>
              </li>
            ))}
          </ul>
        ) : <p className="mt-2 text-xs text-muted-foreground">{t("missionControl.usage.noInvocations")}</p>}
      </div>
    </>
  );
}
