import { activityPayloadIcons, activityPayloadPresentation } from "./activity-presentation-registry";
import type {
  ActivityMetricCode, ActivityPayload, ActivityStatus,
} from "./activity-contracts";
import { openActivityNavigation, type ActivityNavigator } from "./activity-navigation";

export interface ActivityPayloadRendererProps {
  payload: ActivityPayload;
  translate: (key: string, values?: Record<string, string | number>) => string;
  onNavigate?: ActivityNavigator;
}

export function ActivityPayloadRenderer({
  payload,
  translate,
  onNavigate,
}: ActivityPayloadRendererProps) {
  const presentation = activityPayloadPresentation[payload.schema];
  const Icon = activityPayloadIcons[payload.schema];
  return (
    <section
      aria-label={translate(presentation.accessibleLabelKey)}
      className="rounded-xl border border-slate-200/80 bg-slate-50/80 p-3 text-slate-800 shadow-sm dark:border-slate-700/80 dark:bg-slate-900/60 dark:text-slate-100"
      data-payload-schema={payload.schema}
    >
      <div className="mb-2 flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">
        <Icon aria-hidden="true" className="h-4 w-4" />
        <span>{translate(presentation.accessibleLabelKey)}</span>
      </div>
      {renderPayload(payload, translate, onNavigate)}
    </section>
  );
}

function renderPayload(
  payload: ActivityPayload,
  translate: ActivityPayloadRendererProps["translate"],
  onNavigate: ActivityPayloadRendererProps["onNavigate"],
) {
  switch (payload.schema) {
    case "status_card":
      return (
        <dl className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3">
          <dt className="text-sm text-slate-600 dark:text-slate-300">
            {translate(`systemActivity.label.${payload.labelCode}`)}
          </dt>
          <dd className="rounded-full bg-slate-200/70 px-2.5 py-1 text-xs font-semibold dark:bg-slate-700">
            {translate(`systemActivity.value.${payload.valueCode}`)}
          </dd>
        </dl>
      );
    case "stage_timeline":
      return (
        <ol className="space-y-2">
          {payload.stages.map((stage, index) => (
            <li className="flex items-center gap-3 text-sm" key={`${stage.code}-${index}`}>
              <StatusDot status={stage.status} />
              <span className="min-w-0 flex-1 truncate">
                {translate(`systemActivity.stage.${stage.code}`)}
              </span>
              <span className="text-xs text-slate-500 dark:text-slate-400">
                {translate(`systemActivity.status.${stage.status}.title`)}
              </span>
            </li>
          ))}
        </ol>
      );
    case "check_summary":
      return (
        <dl className="grid grid-cols-3 gap-2 text-center">
          <SummaryMetric label={translate("systemActivity.check.passed")} value={payload.passed} />
          <SummaryMetric label={translate("systemActivity.check.failed")} value={payload.failed} />
          <SummaryMetric label={translate("systemActivity.check.review")} value={payload.review} />
        </dl>
      );
    case "metric_summary":
      return (
        <dl className="grid gap-2 sm:grid-cols-2">
          {metricEntries(payload.metrics).map(([code, value]) => (
            <div className="flex items-center justify-between gap-3 text-sm" key={code}>
              <dt className="text-slate-600 dark:text-slate-300">
                {translate(`systemActivity.metric.${code}`)}
              </dt>
              <dd className="font-mono font-semibold tabular-nums">{value}</dd>
            </div>
          ))}
        </dl>
      );
    case "navigation_list":
      return (
        <ul className="space-y-1.5">
          {payload.links.map((link) => (
            <li key={`${link.kind}:${link.stableId}:${link.childId ?? ""}`}>
              <button
                className="w-full rounded-lg px-2.5 py-2 text-left text-sm text-sky-700 transition-colors hover:bg-sky-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-500 disabled:cursor-default disabled:text-slate-500 dark:text-sky-300 dark:hover:bg-sky-950/40 dark:disabled:text-slate-400"
                disabled={!onNavigate}
                onClick={() => {
                  if (onNavigate) openActivityNavigation(link, onNavigate);
                }}
                type="button"
              >
                {translate(`systemActivity.navigation.${link.kind}`, { id: link.stableId })}
              </button>
            </li>
          ))}
        </ul>
      );
    case "supersession_notice":
      return (
        <p className="text-sm text-slate-600 dark:text-slate-300">
          {translate("systemActivity.supersession.notice", { id: payload.priorEventId })}
        </p>
      );
  }
}

function SummaryMetric({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-lg bg-white/80 p-2 dark:bg-slate-800/80">
      <dd className="text-lg font-semibold tabular-nums">{value}</dd>
      <dt className="text-xs text-slate-500 dark:text-slate-400">{label}</dt>
    </div>
  );
}

function StatusDot({ status }: { status: ActivityStatus }) {
  const tone = status === "succeeded"
    ? "bg-emerald-500"
    : status === "failed" || status === "blocked"
      ? "bg-rose-500"
      : status === "running"
        ? "bg-sky-500"
        : "bg-slate-400";
  return <span aria-hidden="true" className={`h-2.5 w-2.5 shrink-0 rounded-full ${tone}`} />;
}

function metricEntries(metrics: Partial<Record<ActivityMetricCode, number>>) {
  return Object.entries(metrics)
    .filter((entry): entry is [ActivityMetricCode, number] => typeof entry[1] === "number")
    .sort(([left], [right]) => left.localeCompare(right));
}
