import { useMutation, useQuery } from "@tanstack/react-query";
import { ArrowRight, Ban, RefreshCw, ShieldCheck } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { SkillGenerationService } from "../../../services/skill-generation-service";
import { SkillGenerationDossier } from "./skill-generation-dossier";

export function SkillGenerationJobDetail({ jobId, onChanged, onOpenCurator, service }: {
  jobId: string; onChanged: () => void; onOpenCurator: () => void; service: SkillGenerationService;
}) {
  const { t } = useTranslation();
  const [view, setView] = useState<"overview" | "dossier" | "provenance">("overview");
  const detail = useQuery({ queryKey: ["skill-generation-job", jobId], queryFn: () => service.getGenerationJob(jobId) });
  const provenance = useQuery({ enabled: view === "provenance", queryKey: ["skill-generation-provenance", jobId], queryFn: () => service.getGenerationProvenance(jobId) });
  const prior = useQuery({ enabled: Boolean(detail.data?.supersedesJobId), queryKey: ["skill-generation-prior-job", detail.data?.supersedesJobId], queryFn: () => service.getGenerationJob(detail.data!.supersedesJobId!) });
  const action = useMutation({
    mutationFn: async (kind: "cancel" | "regenerate" | "handoff") => {
      if (kind === "cancel") return service.cancelGenerationJob(jobId);
      if (kind === "handoff") return service.handoffGenerationPackage(jobId);
      const witness = detail.data?.inputWitnessHash;
      if (!witness) throw new Error("generation-input-witness-unavailable");
      return service.regenerateGenerationJob({ jobId, expectedInputWitnessHash: witness, requestId: `${jobId}-regenerate-${Date.now()}` });
    },
    onSuccess: (job) => { void detail.refetch(); onChanged(); if (job.jobId !== jobId) setView("overview"); },
  });
  if (detail.isLoading) return <p className="rounded-xl border border-border p-4 text-sm text-muted-foreground" role="status">{t("skills.generation.loadingDetail")}</p>;
  if (!detail.data) return <p className="rounded-xl border border-destructive/40 bg-destructive/10 p-4 text-sm text-destructive" role="alert">{detail.error?.message ?? t("skills.generation.jobMissing")}</p>;
  const job = detail.data;
  const cancellable = ["requested", "queued", "running"].includes(job.status);
  const regenerable = ["completed", "cancelled", "failed"].includes(job.status) && Boolean(job.inputWitnessHash);
  return <article className="space-y-3"><section className="rounded-xl border border-border bg-background p-4 shadow-sm"><div className="flex flex-wrap items-start justify-between gap-3"><div><div className="flex flex-wrap items-center gap-2"><h3 className="text-base font-semibold">{job.seedId}</h3><Badge tone="muted">{job.artifactKind ?? job.status}</Badge></div><p className="mt-1 font-mono text-[11px] text-muted-foreground">{job.jobId}</p></div><Badge tone="success"><ShieldCheck className="h-3 w-3" />{t("skills.generation.manualOnly")}</Badge></div>
    <div className="mt-4 flex flex-wrap gap-2" role="tablist">{(["overview", "dossier", "provenance"] as const).map((tab) => <button aria-selected={view === tab} className={`rounded-md px-3 py-1.5 text-xs font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${view === tab ? "bg-primary text-primary-foreground" : "bg-muted text-muted-foreground"}`} key={tab} onClick={() => setView(tab)} role="tab" type="button">{t(`skills.generation.tab.${tab}`)}</button>)}</div></section>
    {view === "overview" ? <Overview job={job} prior={prior.data ?? undefined} /> : null}
    {view === "dossier" ? job.dossierId ? <SkillGenerationDossier dossierId={job.dossierId} service={service} /> : <Empty text={t("skills.generation.noDossier")} /> : null}
    {view === "provenance" ? <Provenance loading={provenance.isLoading} value={provenance.data} /> : null}
    <div className="sticky bottom-3 flex flex-wrap justify-end gap-2 rounded-xl border border-border bg-background/95 p-3 shadow-lg backdrop-blur"><Button disabled={!cancellable || action.isPending} onClick={() => action.mutate("cancel")} variant="outline"><Ban />{t("skills.generation.cancel")}</Button><Button disabled={!regenerable || action.isPending} onClick={() => action.mutate("regenerate")} variant="outline"><RefreshCw />{t("skills.generation.regenerate")}</Button><Button disabled={job.status !== "completed" || action.isPending} onClick={() => action.mutate("handoff")}><ArrowRight />{t("skills.generation.sendToCurator")}</Button>{job.handoffStatus === "delivered" ? <Button onClick={onOpenCurator} variant="outline">{t("skills.generation.openCurator")}</Button> : null}</div>
    {action.isError ? <p className="rounded-lg bg-destructive/10 p-3 text-xs text-destructive" role="alert">{action.error.message}</p> : null}
  </article>;
}

function Overview({ job, prior }: { job: NonNullable<Awaited<ReturnType<SkillGenerationService["getGenerationJob"]>>>; prior?: Awaited<ReturnType<SkillGenerationService["getGenerationJob"]>> }) {
  const { t } = useTranslation();
  return <>{prior ? <section className="rounded-xl border border-border bg-background p-4"><h4 className="text-sm font-semibold">{t("skills.generation.comparison")}</h4><div className="mt-3 grid grid-cols-2 gap-2"><Compare label={t("skills.generation.priorAttempt")} job={prior} /><Compare label={t("skills.generation.currentAttempt")} job={job} /></div></section> : null}<section className="rounded-xl border border-border bg-background p-4"><h4 className="text-sm font-semibold">{t("skills.generation.stages")}</h4><ol className="mt-3 space-y-2">{job.stages.map((stage, index) => <li className="grid grid-cols-[1.5rem_minmax(0,1fr)_auto] items-center gap-2" key={stage.attemptId}><span className={`flex h-6 w-6 items-center justify-center rounded-full text-[10px] font-semibold ${stage.status === "succeeded" ? "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400" : "bg-muted text-muted-foreground"}`}>{index + 1}</span><div className="min-w-0"><p className="truncate text-xs font-medium">{t(`skills.generation.stage.${stage.stage}`)}</p><p className="text-[11px] text-muted-foreground">{t("skills.generation.attempt", { count: stage.attempt })} · {stage.usage.inputTokens + stage.usage.outputTokens} tok</p></div><Badge tone="muted">{stage.status}</Badge></li>)}</ol></section>
    {job.draft ? <section className="rounded-xl border border-border bg-background p-4"><div className="flex items-center justify-between gap-2"><h4 className="text-sm font-semibold">{t("skills.generation.renderedDraft")}</h4><Badge tone="muted">{job.draft.artifactKind}</Badge></div><pre className="mt-3 max-h-72 overflow-auto whitespace-pre-wrap break-words rounded-lg border border-border bg-muted/30 p-3 font-mono text-xs leading-5">{job.draft.renderedContent}</pre>{job.draft.citations.length ? <div className="mt-3 flex flex-wrap gap-2" aria-label={t("skills.generation.citations")}>{job.draft.citations.map((citation) => <Badge key={`${citation.claimId}-${citation.sourceId}`} tone="muted">{citation.dossierSection} · {citation.sourceId}</Badge>)}</div> : null}</section> : null}
    {job.validation ? <section className="rounded-xl border border-border bg-background p-4"><h4 className="text-sm font-semibold">{t("skills.generation.validation")}</h4><div className="mt-3 overflow-x-auto"><table className="w-full text-left text-xs"><thead className="text-muted-foreground"><tr><th className="pb-2 pr-3">{t("skills.generation.check")}</th><th className="pb-2">{t("skills.generation.result")}</th></tr></thead><tbody>{job.validation.checks.map((check, index) => { const value = check as { code?: string; status?: string }; return <tr className="border-t border-border" key={`${value.code ?? "check"}-${index}`}><td className="py-2 pr-3 font-mono">{value.code ?? "—"}</td><td className="py-2">{value.status ?? "—"}</td></tr>; })}</tbody></table></div></section> : null}</>;
}

function Compare({ job, label }: { job: NonNullable<Awaited<ReturnType<SkillGenerationService["getGenerationJob"]>>>; label: string }) { return <div className="rounded-lg bg-muted/40 p-3"><p className="text-[11px] text-muted-foreground">{label}</p><p className="mt-1 truncate text-xs font-medium">{job.status} · {job.artifactKind ?? "—"}</p><p className="mt-1 text-[11px] tabular-nums text-muted-foreground">{job.usage.inputTokens + job.usage.outputTokens} tok · {job.usage.validationRepairs} repair</p></div>; }

function Provenance({ loading, value }: { loading: boolean; value?: Awaited<ReturnType<SkillGenerationService["getGenerationProvenance"]>> }) { const { t } = useTranslation(); if (loading) return <Empty text={t("skills.generation.loading")} />; return <section className="grid gap-3 sm:grid-cols-3">{[["modelCalls", value?.modelCalls.length ?? 0], ["toolReceipts", value?.toolReceipts.length ?? 0], ["validations", value?.validations.length ?? 0]].map(([label, count]) => <div className="rounded-xl border border-border bg-background p-4" key={label}><p className="text-xs text-muted-foreground">{t(`skills.generation.${label}`)}</p><p className="mt-1 text-2xl font-semibold tabular-nums">{count}</p></div>)}</section>; }
function Empty({ text }: { text: string }) { return <div className="rounded-xl border border-dashed border-border p-8 text-center text-sm text-muted-foreground">{text}</div>; }
