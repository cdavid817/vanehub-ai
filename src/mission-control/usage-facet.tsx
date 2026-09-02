import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { formatAppDateTime, formatAppNumber } from "../i18n/format";
import { agentService } from "../services/runtime-agent-client";
import { executionObservabilityService } from "../services/runtime-execution-observability-client";
import type { MissionControlRunSummary } from "../types/mission-control";
import type { ModelInvocation, TokenUsageSummary } from "../types/token-usage";
import { resolveSessionExecutionContext } from "./session-execution-context";

// A run detail panel, not a dashboard — enough to show real recent activity without paging.
const INVOCATION_LIST_LIMIT = 20;

type UsageFacetState =
  | { status: "loading" }
  | { status: "empty" }
  | { status: "error" }
  | { status: "ready"; summary: TokenUsageSummary; invocations: ModelInvocation[] };

function UsageStat({ label, value }: { label: string; value: string }) {
  return <div><dt className="text-[11px] text-muted-foreground">{label}</dt><dd className="mt-0.5 font-medium tabular-nums">{value}</dd></div>;
}

/**
 * The Usage facet: joins this Mission Control run to its execution-observability run via
 * `resolveSessionExecutionContext`, then renders real token-usage content for that session.
 *
 * The backend's own `MissionControlFacetAvailability` for "usage" gates whether this component is
 * even mounted (see `mission-control-facets.tsx`) — this component does not re-decide availability,
 * it only decides what to show once mounted, including an honest empty state when the resolver
 * itself cannot find anything (e.g. the concurrent-runs edge case documented on the resolver).
 */
export function UsageFacet({ run }: { run: MissionControlRunSummary }) {
  const { t, i18n } = useTranslation();
  const [state, setState] = useState<UsageFacetState>({ status: "loading" });

  useEffect(() => {
    let cancelled = false;
    setState({ status: "loading" });
    void (async () => {
      const resolved = await resolveSessionExecutionContext(run, executionObservabilityService);
      const sessionId = resolved?.sessionId;
      if (!sessionId) { if (!cancelled) setState({ status: "empty" }); return; }
      const [summary, details] = await Promise.all([
        agentService.getTokenUsageSummary({ sessionId }),
        agentService.getTokenUsageDetails({ sessionId, limit: INVOCATION_LIST_LIMIT }),
      ]);
      if (!cancelled) setState({ status: "ready", summary, invocations: details.invocations });
    })().catch(() => { if (!cancelled) setState({ status: "error" }); });
    return () => { cancelled = true; };
  }, [run]);

  return (
    <div className="mt-4 space-y-3" data-testid="mission-control-usage-facet">
      {state.status === "loading" ? <p className="text-xs text-muted-foreground">{t("usage.loading")}</p> : null}
      {state.status === "error" ? <p className="text-xs text-destructive">{t("missionControl.usage.error")}</p> : null}
      {state.status === "empty" ? <p className="text-xs text-muted-foreground">{t("missionControl.usage.empty")}</p> : null}
      {state.status === "ready" ? <UsageFacetContent invocations={state.invocations} language={i18n.language} summary={state.summary} /> : null}
    </div>
  );
}

function UsageFacetContent({ invocations, language, summary }: { invocations: ModelInvocation[]; language: string; summary: TokenUsageSummary }) {
  const { t } = useTranslation();
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
            {invocations.map((invocation) => (
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
