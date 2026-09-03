import { AlertCircle, CheckCircle2, ChevronDown, Circle, Clock3 } from "lucide-react";
import { useId, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { LoopEvidence, LoopInspectionTarget, LoopIteration } from "../types/loop";
import { LoopInspectionActions } from "./loop-inspection-actions";
import { evidenceDetailNumber, formatLoopDuration } from "./loop-monitoring";
import { compareConsecutiveIterations } from "./loop-presentation";

/**
 * Task 20.9: clears the sticky `LoopRunHeader` + (while awaiting acceptance) `LoopAcceptancePanel`
 * "Decision Panel" stack above this list's own scroll container (`loop-center.tsx`'s
 * `overflow-y-auto` region) so Tab-focusing a row's toggle never leaves it focused-but-visually-
 * hidden underneath them -- browsers scroll a newly focused element into view using *that
 * element's own* `scroll-margin`, not an ancestor's, which is why this lives on the toggle buttons
 * themselves rather than on `<li>`/`<ol>`. Unlike `LoopAcceptancePanel`'s own `top` offsets
 * (loop-acceptance-panel.tsx), which only need to clear the header's fixed-shape metrics, the
 * panel's *own* height also varies with how many acceptance criteria/checks/findings a run has, so
 * there is no single exact number here. These values are a deliberately generous estimate (the
 * header's own documented height plus a typical, not worst-case, panel height) rather than a tight
 * one -- safe to over-shoot, since `scroll-margin-top` only changes how far a browser scrolls to
 * reveal a newly focused element, never layout, so a run with an unusually long acceptance panel
 * degrades to "scrolls a bit less far than ideal," not a regression from today's zero clearance.
 */
const ITERATION_FOCUS_SCROLL_MARGIN = "scroll-mt-[360px] sm:scroll-mt-[300px] lg:scroll-mt-[260px]";

/**
 * Task 17.10: replaces the former literal `<details>`/`<summary>` accordion
 * (loop-iteration-details.tsx) with a compact, verdict-first row. The row itself always shows
 * the decision-relevant facts (outcome, status, verifier recommendation, duration, a checks
 * pass/fail tally, evidence count) without requiring a click; the full breakdown (comparison,
 * worker summary, diff stats, per-check evidence, verifier findings, decision, feedback,
 * recovery, raw evidence) stays exactly as before, just moved behind one explicit expand toggle
 * instead of being a wall of always-present collapsed sections. Every fact the old accordion
 * rendered is still rendered somewhere here -- the former standalone "Outcome" section is the one
 * exception, and it is a relocation, not a removal: its value (`decisionReason ?? workerSummary ??
 * status`) is now the row's own always-visible verdict line, so it is strictly more available
 * than before (previously it required opening the accordion at all).
 */
export function LoopIterationRow({ iteration, onInspect, open, previousIteration }: { iteration: LoopIteration; onInspect?: (target: LoopInspectionTarget) => void; open: boolean; previousIteration: LoopIteration | null }) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(open);
  const detailId = useId();
  const workerEvidence = iteration.evidence.find((evidence) => evidence.kind === "worker");
  const checks = iteration.evidence.filter((evidence) => evidence.kind === "verification");
  const passedChecks = checks.filter((evidence) => evidence.status === "passed").length;
  const changedFiles = evidenceDetailNumber(workerEvidence, "changedFiles");
  const additions = evidenceDetailNumber(workerEvidence, "additions");
  const deletions = evidenceDetailNumber(workerEvidence, "deletions");
  const comparison = previousIteration ? compareConsecutiveIterations(previousIteration, iteration) : null;
  const recovery = iteration.evidence.filter((evidence) => evidence.kind === "recovery");
  const verdict = iteration.decisionReason ?? iteration.workerSummary ?? t(`loops.status.${iteration.status}`);
  const duration = formatLoopDuration(Date.parse(iteration.completedAt ?? new Date().toISOString()) - Date.parse(iteration.startedAt));
  return (
    <li className="rounded-md border border-border/70 bg-background/30">
      <button aria-controls={detailId} aria-expanded={expanded} className={cn("flex min-h-12 w-full items-center gap-3 px-3 py-2 text-left focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring", ITERATION_FOCUS_SCROLL_MARGIN)} onClick={() => setExpanded((current) => !current)} type="button">
        <StatusIcon status={iteration.status} />
        <span className="min-w-0 flex-1">
          <span className="flex items-baseline gap-2">
            <span className="shrink-0 text-sm font-medium">{t("loops.iterations.number", { number: iteration.sequence })}</span>
            <span className="min-w-0 truncate text-xs text-muted-foreground">{verdict}</span>
          </span>
          <span className="mt-0.5 flex flex-wrap items-center gap-x-3 gap-y-0.5 text-[11px] text-muted-foreground">
            <span>{t(`loops.status.${iteration.status}`)}</span>
            {iteration.verifierRecommendation ? <span>{t(`loops.recommendation.${iteration.verifierRecommendation}`)}</span> : null}
            <span>{duration}</span>
            {checks.length > 0 ? <span>{t("loops.iterations.checks")} {passedChecks}/{checks.length}</span> : null}
            <span>{t("loops.iterations.evidence", { count: iteration.evidence.length })}</span>
          </span>
        </span>
        <ChevronDown aria-hidden="true" className={cn("h-4 w-4 shrink-0 text-muted-foreground transition-transform", expanded && "rotate-180")} />
      </button>
      {expanded ? (
        <div className="grid gap-4 border-t border-border/60 p-3" id={detailId}>
          {comparison ? <DetailSection label={t("loops.iterations.comparison")}><p>{comparison.resolvedFailures.length ? t("loops.iterations.resolvedFailures", { checks: comparison.resolvedFailures.join(", ") }) : t("loops.iterations.noResolvedFailures")}</p><p>{comparison.newFailures.length ? t("loops.iterations.newFailures", { checks: comparison.newFailures.join(", ") }) : t("loops.iterations.noNewFailures")}</p>{comparison.changeDelta ? <p>{t("loops.iterations.changeDelta", { additions: comparison.changeDelta.additions, deletions: comparison.changeDelta.deletions, files: comparison.changeDelta.changedFiles })}</p> : <p className="text-muted-foreground">{t("loops.iterations.changeDeltaUnknown")}</p>}</DetailSection> : null}
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
          {recovery.length ? <DetailSection label={t("loops.iterations.recovery")}>{recovery.map((evidence) => <p key={evidence.id}>{evidence.summary}</p>)}</DetailSection> : null}
          <AllEvidenceDisclosure evidence={iteration.evidence} onInspect={onInspect} verifierSessionId={iteration.verifierSessionId} workerSessionId={iteration.workerSessionId} />
        </div>
      ) : null}
    </li>
  );
}

/** The raw per-evidence dump: every fact in it is already surfaced above by kind (worker summary,
 *  changes, checks, verifier findings), so it stays a second, nested, collapsed-by-default
 *  disclosure -- same "opt in to the full wall of evidence" behavior the old nested `<details>`
 *  had, just as a real controlled toggle instead of a native sub-accordion. */
function AllEvidenceDisclosure({ evidence, onInspect, verifierSessionId, workerSessionId }: { evidence: LoopEvidence[]; onInspect?: (target: LoopInspectionTarget) => void; verifierSessionId: string | null; workerSessionId: string | null }) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const contentId = useId();
  return (
    <section className="border-t border-border/60 pt-3">
      <button aria-controls={contentId} aria-expanded={open} className={cn("flex min-h-11 items-center gap-1 text-[11px] font-semibold uppercase text-muted-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring", ITERATION_FOCUS_SCROLL_MARGIN)} onClick={() => setOpen((current) => !current)} type="button">
        <ChevronDown aria-hidden="true" className={cn("h-3 w-3 shrink-0 transition-transform", open && "rotate-180")} />
        {t("loops.iterations.allEvidence")}
      </button>
      {open ? (
        <div className="mt-3 grid gap-2" id={contentId}>
          {evidence.map((item) => <EvidenceRow evidence={item} key={item.id} onInspect={onInspect} sessionId={item.kind === "verifier" ? verifierSessionId : workerSessionId ?? verifierSessionId} />)}
          {evidence.length === 0 ? <p className="text-muted-foreground">{t("loops.iterations.noEvidence")}</p> : null}
        </div>
      ) : null}
    </section>
  );
}

function EvidenceRow({ evidence, onInspect, sessionId }: { evidence: LoopEvidence; onInspect?: (target: LoopInspectionTarget) => void; sessionId: string | null }) {
  const { t } = useTranslation();
  return <div className="grid grid-cols-[auto_minmax(0,1fr)] gap-x-2 gap-y-1 border-l-2 border-border pl-2 text-xs">
    <StatusIcon status={evidence.status} />
    <div className="min-w-0"><p className="wrap-break-word"><span className="font-medium">{t(`loops.evidence.kind.${evidence.kind}`)}</span>: {evidence.summary}</p><p className="mt-0.5 flex flex-wrap gap-x-3 text-[11px] text-muted-foreground"><span>{t(`loops.evidence.status.${evidence.status}`)}</span>{evidence.commandId ? <span>{evidence.commandId}</span> : null}{evidence.exitCode !== null ? <span>{t("loops.evidence.exitCode", { code: evidence.exitCode })}</span> : null}{evidence.durationMs !== null ? <span>{t("loops.evidence.duration", { duration: evidence.durationMs })}</span> : null}{evidence.operationId ? <span className="break-all">{t("loops.evidence.operation", { id: evidence.operationId })}</span> : null}</p>{evidence.operationId ? <LoopInspectionActions onInspect={onInspect} sessionId={sessionId} surfaces={["logs"]} /> : null}</div>
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
