import { ArrowRight, Check, Eye, X } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { formatAppDateTime } from "../../../i18n/format";
import type {
  EvolutionApplicationSummary,
  EvolutionEligibilitySummary,
  EvolutionProbationSummary,
} from "../../../services/skill-evolution-orchestration-service";

export function SkillEvolutionDecisionsPanel({
  applications,
  eligibility,
  initialApplicationId,
  initialProbationId,
  onOpenCurator,
  probations,
}: {
  applications: EvolutionApplicationSummary[];
  eligibility: EvolutionEligibilitySummary[];
  initialApplicationId?: string;
  initialProbationId?: string;
  onOpenCurator: () => void;
  probations: EvolutionProbationSummary[];
}) {
  const { i18n, t } = useTranslation();
  const initialEligibility = useMemo(() => {
    const application = applications.find((item) => item.applicationId === initialApplicationId);
    return application?.eligibilityId ?? eligibility[0]?.eligibilityId ?? null;
  }, [applications, eligibility, initialApplicationId]);
  const [selectedId, setSelectedId] = useState<string | null>(initialEligibility);
  const effectiveSelectedId = selectedId ?? initialEligibility;
  const selected = eligibility.find((item) => item.eligibilityId === effectiveSelectedId) ?? eligibility[0] ?? null;
  return <section aria-labelledby="evolution-decisions-title" className="space-y-4">
    <div><h3 className="font-semibold" id="evolution-decisions-title">{t("skills.evolution.decisions.title")}</h3><p className="mt-1 text-xs text-muted-foreground">{t("skills.evolution.decisions.description")}</p></div>
    <div className="grid min-w-0 gap-4 xl:grid-cols-[minmax(16rem,0.65fr)_minmax(0,1.35fr)]">
      <div className="space-y-2">{eligibility.length ? eligibility.map((item) => <button aria-current={selected?.eligibilityId === item.eligibilityId ? "true" : undefined} className={`w-full rounded-xl border p-3 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${selected?.eligibilityId === item.eligibilityId ? "border-primary bg-primary/10" : "border-border hover:bg-muted/30"}`} key={item.eligibilityId} onClick={() => setSelectedId(item.eligibilityId)} type="button"><div className="flex items-center justify-between gap-2"><span className="truncate text-sm font-medium">{item.targetSkillId}</span><ResultBadge result={item.result} /></div><p className="mt-1 text-xs text-muted-foreground">{dateTime(item.evaluatedAtMs, i18n.language)}</p></button>) : <Empty text={t("skills.evolution.decisions.empty")} />}</div>
      {selected ? <article className="min-w-0 rounded-xl border border-border bg-background p-4"><div className="flex flex-wrap items-center justify-between gap-2"><div><p className="text-xs text-muted-foreground">{selected.targetSkillId}</p><h4 className="font-semibold">{t("skills.evolution.decisions.proofTitle")}</h4></div><ResultBadge result={selected.result} /></div>
        {selected.mockProvenance ? <p className="mt-3 rounded-lg border border-amber-500/30 bg-amber-500/10 p-3 text-xs">{t("skills.evolution.decisions.webSimulation")}</p> : null}
        <dl className="mt-3 grid gap-2 text-xs sm:grid-cols-2"><Fact label={t("skills.evolution.decisions.draftProvenance")} value={t(`skills.evolution.provenance.${selected.draftProvenance}`)} /><Fact label={t("skills.evolution.decisions.preflight")} value={t(`skills.evolution.preflight.${selected.preflightState}`)} /><Fact label={t("skills.evolution.decisions.draftId")} value={selected.draftId} mono /><Fact label={t("skills.evolution.decisions.previewState")} value={selected.overlayPreviewHash ? t("skills.evolution.decisions.previewBound") : t("skills.evolution.decisions.previewAbsent")} /></dl>
        <h5 className="mt-4 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("skills.evolution.decisions.conditions")}</h5><ul className="mt-2 space-y-2">{selected.predicates.map((predicate) => <li className="flex items-start gap-2 rounded-lg border border-border p-3 text-xs" key={predicate.condition}>{predicate.passed ? <Check className="mt-0.5 h-4 w-4 text-emerald-500" /> : <X className="mt-0.5 h-4 w-4 text-destructive" />}<div className="min-w-0"><p className="font-medium">{t(`skills.evolution.condition.${predicate.condition}`, { defaultValue: predicate.condition })}</p><p className="break-all text-muted-foreground">{predicate.safeReasonCode ?? t("skills.evolution.decisions.conditionPassed")}</p></div></li>)}</ul>
        {selected.result === "routed_to_curator" || selected.result === "ineligible" ? <Button className="mt-4" onClick={onOpenCurator} variant="outline"><ArrowRight />{t("skills.evolution.openCurator")}</Button> : null}
      </article> : <Empty text={t("skills.evolution.decisions.select")} />}
    </div>
    <History applications={applications} initialProbationId={initialProbationId} probations={probations} />
  </section>;
}

function History({ applications, initialProbationId, probations }: { applications: EvolutionApplicationSummary[]; initialProbationId?: string; probations: EvolutionProbationSummary[] }) {
  const { i18n, t } = useTranslation();
  const probationByApplication = new Map(probations.map((item) => [item.applicationId, item]));
  return <div><h4 className="font-semibold">{t("skills.evolution.history.title")}</h4><p className="mt-1 text-xs text-muted-foreground">{t("skills.evolution.history.privacy")}</p>{applications.length ? <ul className="mt-3 grid gap-3 lg:grid-cols-2">{applications.map((application) => { const probation = probationByApplication.get(application.applicationId); return <li className={`rounded-xl border bg-background p-4 ${probation?.probationId === initialProbationId ? "border-primary ring-1 ring-primary" : "border-border"}`} key={application.applicationId}><div className="flex items-center justify-between gap-2"><span className="font-medium">{application.targetSkillId}</span><Badge tone="success">{t("skills.evolution.history.applied")}</Badge></div><p className="mt-2 font-mono text-xs text-muted-foreground">{application.applicationId}</p><p className="mt-2 text-xs text-muted-foreground">{dateTime(application.committedAtMs, i18n.language)}</p>{probation ? <div className="mt-3 rounded-lg bg-muted/30 p-3 text-xs"><div className="flex items-center justify-between gap-2"><span className="flex items-center gap-1 font-medium"><Eye className="h-3.5 w-3.5" />{t("skills.evolution.history.probation")}</span><Badge tone={probation.status === "regressed" || probation.status === "suspended" ? "danger" : probation.status === "healthy" ? "success" : "warning"}>{t(`skills.evolution.probation.${probation.status}`)}</Badge></div><p className="mt-1 text-muted-foreground">{t("skills.evolution.history.probationEnds", { date: dateTime(probation.endsAtMs, i18n.language) })}</p></div> : null}</li>; })}</ul> : <Empty text={t("skills.evolution.history.empty")} />}</div>;
}

function ResultBadge({ result }: { result: EvolutionEligibilitySummary["result"] }) {
  const { t } = useTranslation();
  const tone = result === "eligible" ? "success" : result === "would_apply" ? "warning" : result === "routed_to_curator" ? "default" : "danger";
  return <Badge tone={tone}>{t(`skills.evolution.result.${result}`)}</Badge>;
}

function Fact({ label, mono, value }: { label: string; mono?: boolean; value: string }) {
  return <div className="rounded-lg border border-border bg-muted/20 p-3"><dt className="text-muted-foreground">{label}</dt><dd className={`mt-1 break-all font-medium ${mono ? "font-mono" : ""}`}>{value}</dd></div>;
}

function Empty({ text }: { text: string }) {
  return <div className="rounded-xl border border-dashed border-border bg-muted/10 p-6 text-center text-sm text-muted-foreground">{text}</div>;
}

function dateTime(value: number, language: string) {
  return formatAppDateTime(value, language, { dateStyle: "medium", timeStyle: "short" });
}
