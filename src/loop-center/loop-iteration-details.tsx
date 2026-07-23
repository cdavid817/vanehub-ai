import { AlertCircle, CheckCircle2, Circle, Clock3 } from "lucide-react";
import { useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { LoopEvidence, LoopInspectionTarget, LoopIteration } from "../types/loop";
import { LoopInspectionActions } from "./loop-inspection-actions";
import { evidenceDetailNumber } from "./loop-monitoring";

export function LoopIterationDetails({ iteration, onInspect, open }: { iteration: LoopIteration; onInspect?: (target: LoopInspectionTarget) => void; open: boolean }) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(open);
  const workerEvidence = iteration.evidence.find((evidence) => evidence.kind === "worker");
  const checks = iteration.evidence.filter((evidence) => evidence.kind === "verification");
  const changedFiles = evidenceDetailNumber(workerEvidence, "changedFiles");
  const additions = evidenceDetailNumber(workerEvidence, "additions");
  const deletions = evidenceDetailNumber(workerEvidence, "deletions");
  return (
    <details className="group rounded-md border border-border/70 bg-background/30" onToggle={(event) => setExpanded(event.currentTarget.open)} open={expanded}>
      <summary className="flex min-h-12 cursor-pointer list-none items-center gap-3 px-3 py-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring [&::-webkit-details-marker]:hidden">
        <StatusIcon status={iteration.status} />
        <span className="min-w-0 flex-1">
          <span className="block truncate text-sm font-medium">{t("loops.iterations.number", { number: iteration.sequence })}</span>
          <span className="block truncate text-[11px] text-muted-foreground">{t("loops.iterations.evidence", { count: iteration.evidence.length })}</span>
        </span>
        {iteration.verifierRecommendation ? <span className="hidden text-[11px] text-muted-foreground sm:block">{t(`loops.recommendation.${iteration.verifierRecommendation}`)}</span> : null}
        <span className="text-xs text-muted-foreground">{t(`loops.status.${iteration.status}`)}</span>
      </summary>
      <div className="grid gap-4 border-t border-border/60 p-3">
        {iteration.workerSummary || iteration.workerSessionId ? <DetailSection label={t("loops.iterations.workerSummary")}><p>{iteration.workerSummary}</p><LoopInspectionActions onInspect={onInspect} sessionId={iteration.workerSessionId} /></DetailSection> : null}
        {changedFiles !== null || iteration.diffFingerprint ? <DetailSection label={t("loops.iterations.changes")}><p>{t("loops.iterations.diffSummary", { additions: additions ?? 0, deletions: deletions ?? 0, files: changedFiles ?? 0 })}</p>{iteration.diffFingerprint ? <code className="mt-1 block break-all text-[11px] text-muted-foreground">{iteration.diffFingerprint}</code> : null}<LoopInspectionActions onInspect={onInspect} sessionId={iteration.workerSessionId ?? iteration.verifierSessionId} surfaces={["changes", "files"]} /></DetailSection> : null}
        {checks.length > 0 ? <DetailSection label={t("loops.iterations.checks")}><div className="grid gap-2">{checks.map((evidence) => <EvidenceRow evidence={evidence} key={evidence.id} onInspect={onInspect} sessionId={iteration.workerSessionId ?? iteration.verifierSessionId} />)}</div></DetailSection> : null}
        {iteration.verifierRecommendation || iteration.verifierFindings.length > 0 || iteration.verifierSessionId ? <DetailSection label={t("loops.iterations.verifier")}>
          {iteration.verifierRecommendation ? <p className="font-medium">{t(`loops.recommendation.${iteration.verifierRecommendation}`)}</p> : null}
          {iteration.verifierFindings.length > 0 ? <ul className="mt-1 list-inside list-disc text-muted-foreground">{iteration.verifierFindings.map((finding, index) => <li key={`${iteration.id}-finding-${index}`}>{finding}</li>)}</ul> : null}
          <LoopInspectionActions onInspect={onInspect} sessionId={iteration.verifierSessionId} />
        </DetailSection> : null}
        {iteration.decisionReason ? <DetailSection label={t("loops.iterations.decision")}><p>{iteration.decisionReason}</p></DetailSection> : null}
        {iteration.userFeedback ? <DetailSection label={t("loops.iterations.feedback")}><p>{iteration.userFeedback}</p></DetailSection> : null}
        <DetailSection label={t("loops.iterations.allEvidence")}><div className="grid gap-2">{iteration.evidence.map((evidence) => <EvidenceRow evidence={evidence} key={evidence.id} onInspect={onInspect} sessionId={evidence.kind === "verifier" ? iteration.verifierSessionId : iteration.workerSessionId ?? iteration.verifierSessionId} />)}{iteration.evidence.length === 0 ? <p className="text-muted-foreground">{t("loops.iterations.noEvidence")}</p> : null}</div></DetailSection>
      </div>
    </details>
  );
}

function EvidenceRow({ evidence, onInspect, sessionId }: { evidence: LoopEvidence; onInspect?: (target: LoopInspectionTarget) => void; sessionId: string | null }) {
  const { t } = useTranslation();
  return <div className="grid grid-cols-[auto_minmax(0,1fr)] gap-x-2 gap-y-1 border-l-2 border-border pl-2 text-xs">
    <StatusIcon status={evidence.status} />
    <div className="min-w-0"><p className="break-words"><span className="font-medium">{t(`loops.evidence.kind.${evidence.kind}`)}</span>: {evidence.summary}</p><p className="mt-0.5 flex flex-wrap gap-x-3 text-[11px] text-muted-foreground"><span>{t(`loops.evidence.status.${evidence.status}`)}</span>{evidence.commandId ? <span>{evidence.commandId}</span> : null}{evidence.exitCode !== null ? <span>{t("loops.evidence.exitCode", { code: evidence.exitCode })}</span> : null}{evidence.durationMs !== null ? <span>{t("loops.evidence.duration", { duration: evidence.durationMs })}</span> : null}{evidence.operationId ? <span className="break-all">{t("loops.evidence.operation", { id: evidence.operationId })}</span> : null}</p>{evidence.operationId ? <LoopInspectionActions onInspect={onInspect} sessionId={sessionId} surfaces={["logs"]} /> : null}</div>
  </div>;
}

function DetailSection({ children, label }: { children: ReactNode; label: string }) {
  return <section className="min-w-0 text-xs leading-5"><h5 className="mb-1 text-[11px] font-semibold uppercase text-muted-foreground">{label}</h5>{children}</section>;
}

function StatusIcon({ status }: { status: LoopEvidence["status"] | LoopIteration["status"] }) {
  if (status === "passed" || status === "succeeded") return <CheckCircle2 aria-hidden="true" className="mt-0.5 h-3.5 w-3.5 shrink-0 text-success" />;
  if (status === "failed" || status === "blocked" || status === "cancelled") return <AlertCircle aria-hidden="true" className="mt-0.5 h-3.5 w-3.5 shrink-0 text-destructive" />;
  if (status === "running" || status === "queued") return <Clock3 aria-hidden="true" className="mt-0.5 h-3.5 w-3.5 shrink-0 text-primary" />;
  return <Circle aria-hidden="true" className={cn("mt-0.5 h-3.5 w-3.5 shrink-0 text-muted-foreground")} />;
}
