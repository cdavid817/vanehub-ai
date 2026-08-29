import { Ban, CheckCircle2, CircleDashed, Clock3, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { formatAppDateTime } from "../../../i18n/format";
import type {
  EvolutionRunBudgetProjection,
  EvolutionRunSummary,
  SkillEvolutionOrchestrationService,
} from "../../../services/skill-evolution-orchestration-service";

export function SkillEvolutionRunsPanel({
  cancelling,
  initialRunId,
  onCancel,
  runs,
  service,
}: {
  cancelling: boolean;
  initialRunId?: string;
  onCancel: (run: EvolutionRunSummary) => void;
  runs: EvolutionRunSummary[];
  service: SkillEvolutionOrchestrationService;
}) {
  const { i18n, t } = useTranslation();
  const [selectedId, setSelectedId] = useState(initialRunId ?? runs[0]?.runId ?? null);
  useEffect(() => {
    if (initialRunId) setSelectedId(initialRunId);
    else if (!selectedId && runs[0]) setSelectedId(runs[0].runId);
  }, [initialRunId, runs, selectedId]);
  const detail = useQuery({
    enabled: Boolean(selectedId),
    queryKey: ["skill-evolution-run-detail", selectedId],
    queryFn: () => service.getEvolutionRun(selectedId!),
    refetchInterval: (query) => query.state.data && isActive(query.state.data) ? 2_000 : false,
  });
  return <section aria-labelledby="evolution-runs-title" className="space-y-3">
    <div><h3 className="font-semibold" id="evolution-runs-title">{t("skills.evolution.runs.title")}</h3><p className="mt-1 text-xs text-muted-foreground">{t("skills.evolution.runs.description")}</p></div>
    {!runs.length ? <Empty text={t("skills.evolution.runs.empty")} /> : <div className="grid min-w-0 gap-3 xl:grid-cols-[minmax(17rem,0.7fr)_minmax(0,1.3fr)]">
      <div aria-label={t("skills.evolution.runs.listLabel")} className="space-y-2" role="list">{runs.map((run) => <button aria-current={selectedId === run.runId ? "true" : undefined} className={`w-full rounded-xl border p-3 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${selectedId === run.runId ? "border-primary bg-primary/10" : "border-border bg-background hover:bg-muted/30"}`} key={run.runId} onClick={() => setSelectedId(run.runId)} role="listitem" type="button"><div className="flex items-center justify-between gap-2"><span className="truncate font-mono text-xs">{run.runId}</span><StatusBadge status={run.status} /></div><div className="mt-2 flex items-center justify-between gap-2 text-[11px] text-muted-foreground"><span>{run.currentStage ?? t("skills.evolution.runs.noStage")}</span><time dateTime={new Date(run.updatedAtMs).toISOString()}>{dateTime(run.updatedAtMs, i18n.language)}</time></div>{run.safeFailureCode ? <p className="mt-2 truncate text-xs text-destructive">{run.safeFailureCode}</p> : null}</button>)}</div>
      <div className="min-w-0">{detail.isLoading ? <Loading /> : detail.isError ? <div className="rounded-xl border border-destructive/40 bg-destructive/10 p-4 text-sm text-destructive" role="alert"><p>{detail.error.message}</p><Button className="mt-2" onClick={() => void detail.refetch()} size="sm" variant="outline"><RefreshCw />{t("featureLoad.retry")}</Button></div> : detail.data ? <RunDetail cancelling={cancelling} onCancel={onCancel} run={detail.data} /> : null}</div>
    </div>}
  </section>;
}

function RunDetail({ cancelling, onCancel, run }: { cancelling: boolean; onCancel: (run: EvolutionRunSummary) => void; run: Awaited<ReturnType<SkillEvolutionOrchestrationService["getEvolutionRun"]>> }) {
  const { i18n, t } = useTranslation();
  return <article className="rounded-xl border border-border bg-background p-4">
    <div className="flex flex-wrap items-start justify-between gap-3"><div><p className="font-mono text-xs text-muted-foreground">{run.runId}</p><h4 className="mt-1 font-semibold">{t("skills.evolution.runs.detailTitle")}</h4></div>{isActive(run) ? <Button disabled={cancelling || run.status === "cancel_requested"} onClick={() => onCancel(run)} size="sm" variant="outline"><Ban />{t("skills.evolution.runs.cancel")}</Button> : <StatusBadge status={run.status} />}</div>
    <p className="mt-3 rounded-lg bg-muted/30 p-3 text-xs text-muted-foreground">{t("skills.evolution.runs.cancelExplanation")}</p>
    {run.safeFailureCode ? <p className="mt-3 rounded-lg border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive" role="alert">{t("skills.evolution.runs.safeFailure", { code: run.safeFailureCode })}</p> : null}
    <h5 className="mt-4 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("skills.evolution.runs.budget")}</h5><BudgetGrid budget={run.budget} usage={run.usage} />
    <h5 className="mt-4 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("skills.evolution.runs.stages")}</h5>{run.stages.length ? <ol className="mt-2 space-y-2">{run.stages.map((stage) => <li className="flex items-start gap-2 rounded-lg border border-border p-3" key={stage.stageId}>{stage.status === "committed" || stage.status === "completed" ? <CheckCircle2 className="mt-0.5 h-4 w-4 text-emerald-500" /> : <CircleDashed className="mt-0.5 h-4 w-4 text-amber-500" />}<div className="min-w-0"><p className="text-sm font-medium">{t(`skills.evolution.stage.${stage.stage}`, { defaultValue: stage.stage })}</p><p className="text-xs text-muted-foreground">{t("skills.evolution.runs.attempt", { count: stage.attempt })} · {stage.status}</p>{stage.safeFailureCode ? <p className="text-xs text-destructive">{stage.safeFailureCode}</p> : null}</div></li>)}</ol> : <Empty text={t("skills.evolution.runs.noStages")} />}
    <h5 className="mt-4 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("skills.evolution.runs.checkpoints")}</h5>{run.checkpoints.length ? <ul className="mt-2 space-y-2">{run.checkpoints.map((checkpoint) => <li className="rounded-lg border border-border p-3 text-xs" key={checkpoint.checkpointId}><div className="flex items-center justify-between gap-2"><span className="font-medium">{checkpoint.stage}</span><Badge tone="muted">{checkpoint.status}</Badge></div><p className="mt-1 text-muted-foreground"><Clock3 className="mr-1 inline h-3 w-3" />{dateTime(checkpoint.committedAtMs, i18n.language)}</p></li>)}</ul> : <Empty text={t("skills.evolution.runs.noCheckpoints")} />}
  </article>;
}

function BudgetGrid({ budget, usage }: { budget: EvolutionRunBudgetProjection; usage: EvolutionRunSummary["usage"] }) {
  const { t } = useTranslation();
  const keys = ["wallTimeMs", "evidenceItems", "seedGroups", "assessments", "modelCalls", "notifications", "automaticMutations"] as const;
  return <dl className="mt-2 grid grid-cols-2 gap-2 sm:grid-cols-4">{keys.map((key) => <div className="rounded-lg border border-border bg-muted/20 p-2" key={key}><dt className="text-[10px] text-muted-foreground">{t(`skills.evolution.budget.${key}`)}</dt><dd className="mt-1 text-sm font-semibold tabular-nums">{usage[key === "wallTimeMs" ? "elapsedMs" : key]} / {budget[key]}</dd></div>)}</dl>;
}

function StatusBadge({ status }: { status: EvolutionRunSummary["status"] }) {
  const { t } = useTranslation();
  const tone = status === "completed" ? "success" : status === "failed" || status === "cancelled" ? "danger" : status === "running" ? "default" : "warning";
  return <Badge tone={tone}>{t(`skills.evolution.status.${status}`)}</Badge>;
}

function isActive(run: EvolutionRunSummary) {
  return ["requested", "waiting_idle", "running", "partial", "cancel_requested", "recovered"].includes(run.status);
}

function Loading() {
  const { t } = useTranslation();
  return <p className="rounded-xl border border-border p-4 text-sm text-muted-foreground" role="status">{t("skills.evolution.orchestrationLoading")}</p>;
}

function Empty({ text }: { text: string }) {
  return <p className="mt-2 rounded-xl border border-dashed border-border bg-muted/10 p-6 text-center text-sm text-muted-foreground">{text}</p>;
}

function dateTime(value: number, language: string) {
  return formatAppDateTime(value, language, { dateStyle: "medium", timeStyle: "short" });
}
