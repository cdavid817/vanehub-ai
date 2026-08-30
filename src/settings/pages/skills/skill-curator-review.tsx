import { useQuery } from "@tanstack/react-query";
import { CheckCircle2, CircleAlert, GitBranch, History, ShieldCheck, XCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { SkillCuratorService } from "../../../services/skill-curator-service";
import type { CuratorCandidateDetail, CuratorQualityCheck } from "../../../types/skill-curator";
import { SkillCuratorActions } from "./skill-curator-actions";
import { SkillCuratorDraftEditor } from "./skill-curator-draft-editor";
import { SkillCuratorPreview } from "./skill-curator-preview";

export function SkillCuratorReview({ candidateId, onQueueChanged, service }: { candidateId: string; onQueueChanged: () => Promise<unknown>; service: SkillCuratorService }) {
  const { t } = useTranslation();
  const candidate = useQuery({
    queryKey: ["skill-curator-candidate", candidateId],
    queryFn: () => service.getSkillCuratorCandidate(candidateId),
  });
  const audit = useQuery({
    queryKey: ["skill-curator-audit", candidateId],
    queryFn: () => service.querySkillCuratorAudit(candidateId),
  });
  if (candidate.isLoading) return <Status text={t("skills.curator.loadingCandidate")} />;
  if (candidate.isError || !candidate.data?.ok) return <ErrorState onRetry={() => void candidate.refetch()} text={t("skills.curator.candidateError")} />;
  const detail = candidate.data.value;
  const onChanged = async () => { await Promise.all([candidate.refetch(), audit.refetch(), onQueueChanged()]); };
  return <article className="space-y-4" aria-labelledby="curator-candidate-title">
    <CandidateHeader detail={detail} />
    <ReviewFacts detail={detail} />
    <QualityChecks checks={detail.qualityChecks} />
    <Lineage detail={detail} />
    <DraftHistory detail={detail} />
    <SkillCuratorDraftEditor detail={detail} onChanged={onChanged} service={service} />
    <SkillCuratorPreview detail={detail} onChanged={onChanged} service={service} />
    <SkillCuratorActions detail={detail} onChanged={onChanged} service={service} />
    <AuditTimeline error={audit.isError || Boolean(audit.data && !audit.data.ok)} events={audit.data?.ok ? audit.data.value.items : []} loading={audit.isLoading} />
  </article>;
}

function CandidateHeader({ detail }: { detail: CuratorCandidateDetail }) {
  const { t } = useTranslation();
  return <header className={`rounded-xl border p-4 ${detail.risk === "high" ? "border-destructive/50 bg-destructive/5" : "border-border bg-background"}`}>
    <div className="flex flex-wrap items-start justify-between gap-3"><div><p className="text-xs font-medium text-primary">{t("skills.curator.review")}</p><h3 className="mt-1 text-lg font-semibold" id="curator-candidate-title">{detail.targetSkillId}</h3><p className="mt-1 break-all font-mono text-[10px] text-muted-foreground">{detail.candidateId}</p></div><div className="flex flex-wrap gap-1.5"><Badge tone={detail.risk === "high" ? "danger" : "warning"}>{t(`skills.curator.value.${detail.risk}`)}</Badge><Badge tone="muted">{t(`skills.curator.value.${detail.state}`)}</Badge></div></div>
    {detail.risk === "high" ? <p className="mt-3 flex gap-2 rounded-md bg-destructive/10 p-3 text-xs leading-5 text-destructive"><CircleAlert className="mt-0.5 h-4 w-4 shrink-0" />{t("skills.curator.highRiskBoundary")}</p> : null}
  </header>;
}

function ReviewFacts({ detail }: { detail: CuratorCandidateDetail }) {
  const { t } = useTranslation();
  const facts = [
    ["route", detail.route], ["confidence", detail.confidence], ["assessment", detail.assessmentAttemptId],
    ["assessmentRevision", detail.assessmentRevision], ["targetRevision", detail.targetRevision],
    ["overlayScope", detail.overlayScope], ["skillState", detail.application?.status ?? "unchanged"],
  ];
  return <section className="rounded-xl border border-border bg-background p-4"><h4 className="flex items-center gap-2 text-sm font-semibold"><GitBranch className="h-4 w-4 text-primary" />{t("skills.curator.assessmentAndTarget")}</h4><dl className="mt-3 grid gap-2 text-xs sm:grid-cols-2">{facts.map(([label, value]) => <div className="rounded-md bg-muted/40 p-2" key={label}><dt className="text-muted-foreground">{t(`skills.curator.field.${label}`)}</dt><dd className="mt-1 break-all font-medium">{value}</dd></div>)}</dl></section>;
}

function QualityChecks({ checks }: { checks: CuratorQualityCheck[] }) {
  const { t } = useTranslation();
  return <section className="rounded-xl border border-border bg-background p-4"><div className="flex items-center justify-between gap-2"><h4 className="flex items-center gap-2 text-sm font-semibold"><ShieldCheck className="h-4 w-4 text-primary" />{t("skills.curator.qualityChecks")}</h4><Badge tone={checks.length === 9 ? "success" : "danger"}>{t("skills.curator.checkCount", { count: checks.length })}</Badge></div><div className="mt-3 grid gap-2 sm:grid-cols-2 xl:grid-cols-3">{checks.map((check) => <div className="rounded-md border border-border p-2 text-xs" key={check.code}><p className="flex items-center gap-1.5 font-medium">{check.result === "pass" ? <CheckCircle2 className="h-3.5 w-3.5 text-success" /> : check.result === "fail" ? <XCircle className="h-3.5 w-3.5 text-destructive" /> : <CircleAlert className="h-3.5 w-3.5 text-warning" />}{check.code}</p><p className="mt-1 text-muted-foreground">{check.reasonCode}</p></div>)}</div></section>;
}

function Lineage({ detail }: { detail: CuratorCandidateDetail }) {
  const { t } = useTranslation();
  return <section className="rounded-xl border border-border bg-background p-4"><h4 className="text-sm font-semibold">{t("skills.curator.sanitizedLineage")}</h4><p className="mt-1 text-xs text-muted-foreground">{t("skills.curator.lineageHint", { seedId: detail.seedId })}</p><div className="mt-3 space-y-2">{detail.evidenceSources.map((source) => <dl className="grid gap-1 rounded-md bg-muted/40 p-2 font-mono text-[10px] sm:grid-cols-3" key={`${source.evidenceId}-${source.evidenceRevision}`}><div><dt className="text-muted-foreground">ID</dt><dd>{source.evidenceId}</dd></div><div><dt className="text-muted-foreground">{t("skills.curator.field.revision")}</dt><dd>{source.evidenceRevision}</dd></div><div><dt className="text-muted-foreground">Hash</dt><dd className="break-all">{source.lineageHash}</dd></div></dl>)}</div></section>;
}

function DraftHistory({ detail }: { detail: CuratorCandidateDetail }) {
  const { t, i18n } = useTranslation();
  return <section className="rounded-xl border border-border bg-background p-4"><h4 className="text-sm font-semibold">{t("skills.curator.draftHistory")}</h4>{detail.drafts.length === 0 ? <p className="mt-3 text-xs text-muted-foreground">{t("skills.curator.noDrafts")}</p> : <div className="mt-3 space-y-2">{detail.drafts.map((draft) => <details className="rounded-md border border-border p-3" key={`${draft.draftId}-${draft.revision}`}><summary className="cursor-pointer text-xs font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">v{draft.revision} · {t(`skills.curator.value.${draft.kind}`)} · {new Intl.DateTimeFormat(i18n.language).format(draft.createdAtMs)}</summary><div className="mt-3 space-y-2 border-t border-border pt-3 text-xs"><p>{draft.rationale}</p><p className="text-muted-foreground">{draft.expectedEffectiveChange}</p><pre className="max-h-48 overflow-auto whitespace-pre-wrap rounded bg-muted/50 p-2 font-mono text-[11px]">{JSON.stringify(draft.mutation, null, 2)}</pre></div></details>)}</div>}</section>;
}

function AuditTimeline({ error, events, loading }: { error: boolean; events: Array<{ sequence: number; eventKind: string; actorClass: string; occurredAtMs: number; reasonCode?: string }>; loading: boolean }) {
  const { t, i18n } = useTranslation();
  return <section className="rounded-xl border border-border bg-background p-4"><h4 className="flex items-center gap-2 text-sm font-semibold"><History className="h-4 w-4 text-primary" />{t("skills.curator.audit")}</h4>{loading ? <Status text={t("skills.curator.loadingAudit")} /> : null}{error ? <p className="mt-3 text-xs text-destructive" role="alert">{t("skills.curator.auditError")}</p> : null}<ol className="mt-3 space-y-2">{events.map((event) => <li className="border-l-2 border-primary/30 pl-3 text-xs" key={event.sequence}><p className="font-medium">{event.eventKind}</p><p className="mt-0.5 text-muted-foreground">{event.actorClass} · {new Intl.DateTimeFormat(i18n.language, { dateStyle: "medium", timeStyle: "short" }).format(event.occurredAtMs)}{event.reasonCode ? ` · ${event.reasonCode}` : ""}</p></li>)}</ol></section>;
}

function Status({ text }: { text: string }) { return <p className="rounded-md border border-border bg-muted/30 p-3 text-xs text-muted-foreground" role="status">{text}</p>; }
function ErrorState({ onRetry, text }: { onRetry: () => void; text: string }) { const { t } = useTranslation(); return <div className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-xs text-destructive" role="alert"><p>{text}</p><Button className="mt-2" onClick={onRetry} size="sm" variant="outline">{t("featureLoad.retry")}</Button></div>; }
