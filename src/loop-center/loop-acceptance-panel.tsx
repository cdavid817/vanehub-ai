import { AlertTriangle, CheckCircle2, CircleDashed } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { LoopInspectionTarget, LoopRun } from "../types/loop";
import { LoopInspectionActions } from "./loop-inspection-actions";
import { selectChangeStatistics, selectRequiredCheckOutcomes } from "./loop-presentation";
import { LoopRunControls } from "./loop-run-controls";

export function LoopAcceptancePanel({ onInspect, run }: { onInspect?: (target: LoopInspectionTarget) => void; run: LoopRun }) {
  const { t } = useTranslation();
  const iteration = run.iterations.at(-1);
  if (run.status !== "awaiting-acceptance") return null;
  if (!iteration) return (
    <section aria-labelledby="loop-acceptance-title" className="grid gap-4 border-y border-warning/40 bg-warning/5 px-3 py-4 sm:px-4">
      <div><h3 className="text-sm font-semibold" id="loop-acceptance-title">{t("loops.acceptance.title")}</h3><p className="mt-1 text-xs text-muted-foreground">{t("loops.acceptance.notEvaluated")}</p></div>
      <LoopRunControls run={run} />
    </section>
  );
  const checks = selectRequiredCheckOutcomes(run);
  const changes = selectChangeStatistics(iteration);
  return (
    <section aria-labelledby="loop-acceptance-title" className="grid gap-4 border-y border-warning/40 bg-warning/5 px-3 py-4 sm:px-4">
      <div><h3 className="text-sm font-semibold" id="loop-acceptance-title">{t("loops.acceptance.title")}</h3><p className="mt-1 text-xs text-muted-foreground">{t("loops.acceptance.description")}</p></div>
      <div className="grid gap-4 sm:grid-cols-2">
        <AcceptanceSection title={t("loops.definition.acceptance")}>
          <ul className="grid gap-2">{run.definitionSnapshot.acceptanceCriteria.map((criterion) => <li className="flex gap-2 text-xs" key={criterion}><CircleDashed aria-hidden="true" className="mt-0.5 h-3.5 w-3.5 shrink-0 text-muted-foreground" /><span><span className="block">{criterion}</span><span className="text-[11px] text-muted-foreground">{t("loops.acceptance.notEvaluated")}</span></span></li>)}</ul>
        </AcceptanceSection>
        <AcceptanceSection title={t("loops.iterations.checks")}>
          <ul className="grid gap-2">{checks.map((check) => <li className="flex items-center gap-2 text-xs" key={check.commandId}>{check.outcome === "passed" ? <CheckCircle2 aria-hidden="true" className="h-3.5 w-3.5 text-success" /> : <AlertTriangle aria-hidden="true" className="h-3.5 w-3.5 text-warning" />}<span>{check.commandId}: {t(`loops.acceptance.outcome.${check.outcome}`)}</span></li>)}</ul>
        </AcceptanceSection>
        <AcceptanceSection title={t("loops.iterations.verifier")}>
          <p className="text-xs font-medium">{iteration.verifierRecommendation ? t(`loops.recommendation.${iteration.verifierRecommendation}`) : t("loops.acceptance.notEvaluated")}</p>
          {iteration.verifierFindings.length ? <ul className="mt-2 list-inside list-disc text-xs text-muted-foreground">{iteration.verifierFindings.map((finding) => <li key={finding}>{finding}</li>)}</ul> : null}
          <LoopInspectionActions onInspect={onInspect} sessionId={iteration.verifierSessionId} />
        </AcceptanceSection>
        <AcceptanceSection title={t("loops.acceptance.changesAndRisks")}>
          <p className="text-xs">{changes ? t("loops.iterations.diffSummary", { additions: changes.additions, deletions: changes.deletions, files: changes.changedFiles }) : t("loops.acceptance.changesUnknown")}</p>
          <p className="mt-2 text-xs text-muted-foreground">{iteration.decisionReason ?? t("loops.acceptance.risksUnknown")}</p>
          <LoopInspectionActions onInspect={onInspect} sessionId={iteration.workerSessionId ?? iteration.verifierSessionId} surfaces={["changes", "files"]} />
        </AcceptanceSection>
      </div>
      <LoopRunControls run={run} />
    </section>
  );
}

function AcceptanceSection({ children, title }: { children: React.ReactNode; title: string }) {
  return <section><h4 className="mb-2 text-[11px] font-semibold uppercase text-muted-foreground">{title}</h4>{children}</section>;
}
