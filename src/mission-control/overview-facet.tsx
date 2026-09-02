import { useTranslation } from "react-i18next";
import { formatAppDateTime, formatAppNumber } from "../i18n/format";
import type { MissionControlRunSummary } from "../types/mission-control";

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0">
      <dt className="truncate text-[11px] text-muted-foreground">{label}</dt>
      <dd className="mt-0.5 truncate text-xs font-medium" title={value}>{value}</dd>
    </div>
  );
}

/**
 * The Overview facet's own detail expansion — deliberately only the `MissionControlRunSummary`
 * fields the adjoining `RunCard` does not already render (title/state/agent/verification/elapsed
 * are visible there already; repeating them here would make this facet a duplicate rather than a
 * genuine expansion). Needs no join to execution-observability, unlike the Usage facet below it.
 */
export function OverviewFacet({ run }: { run: MissionControlRunSummary }) {
  const { t, i18n } = useTranslation();
  const none = t("missionControl.overview.none");
  const timestamp = (value: string | null) =>
    value ? formatAppDateTime(value, i18n.language, { dateStyle: "medium", timeStyle: "short" }) : none;
  const attentionLabel = run.attention ? t(`missionControl.attentionKind.${run.attention}`) : none;
  // reasonCode reuses the same `runner.reason.*` namespace RunCard's own inline warning text does,
  // so a reason renders identically wherever it appears rather than acquiring a second translation.
  const reason = run.reasonCode ? t(`runner.reason.${run.reasonCode}`, { defaultValue: run.reasonCode }) : null;

  return (
    <dl className="mt-4 grid grid-cols-2 gap-3 text-xs" data-testid="mission-control-overview-facet">
      <Field label={t("missionControl.overview.project")} value={run.projectId ?? none} />
      <Field label={t("missionControl.overview.workspace")} value={run.workspace ?? none} />
      <Field label={t("missionControl.overview.phase")} value={run.phase ?? none} />
      <Field label={t("missionControl.overview.attention")} value={reason ? `${attentionLabel} · ${reason}` : attentionLabel} />
      <Field label={t("missionControl.overview.tokens")} value={run.tokens === null ? none : formatAppNumber(run.tokens, i18n.language)} />
      <Field label={t("missionControl.overview.cost")} value={run.cost === null ? none : formatAppNumber(run.cost, i18n.language, { maximumFractionDigits: 4 })} />
      <Field label={t("missionControl.overview.createdAt")} value={timestamp(run.createdAt)} />
      <Field label={t("missionControl.overview.updatedAt")} value={timestamp(run.updatedAt)} />
      <Field label={t("missionControl.overview.endedAt")} value={timestamp(run.endedAt)} />
    </dl>
  );
}
