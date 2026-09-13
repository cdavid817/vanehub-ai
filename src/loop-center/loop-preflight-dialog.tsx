import { useCallback, useEffect, useRef } from "react";
import { AlertCircle, CheckCircle2, Loader2, Play, RefreshCw, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { useLoopAdmission, type LoopAdmissionTarget } from "../hooks/use-loop-admission";
import { useStartLoopMutation } from "../hooks/use-loop-mutations";
import { useLoopReadinessQuery } from "../hooks/use-loop-queries";
import type { LoopControlEnvelope, LoopDefinition, LoopReadinessCheck } from "../types/loop";
import { LoopAuditAcknowledgement, LoopScopeAssessment } from "./loop-scope-assessment";

export function LoopPreflightDialog({
  definition,
  onClose,
  onEdit,
  onStarted,
}: {
  definition: LoopDefinition;
  onClose: () => void;
  onEdit: () => void;
  onStarted: (runId: string) => void;
}) {
  const { t } = useTranslation();
  const panelRef = useRef<HTMLDivElement>(null);
  const readiness = useLoopReadinessQuery(definition.id);
  const start = useStartLoopMutation();
  const startError = start.error instanceof Error ? start.error.message : start.error ? String(start.error) : null;
  const launch = useCallback(async (target: LoopAdmissionTarget, envelope: LoopControlEnvelope) => {
    try {
      const result = await start.mutateAsync({ definitionId: target.definitionId ?? definition.id, envelope });
      onStarted(result.run.id);
    } catch {
      // The authoritative rejection is already on the mutation; readiness is refetched so the
      // dialog explains what changed instead of offering the same start again.
      await readiness.refetch();
    }
  }, [definition.id, onStarted, readiness, start]);
  const admission = useLoopAdmission(launch);
  const busy = start.isPending || admission.busy;

  useEffect(() => {
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    panelRef.current?.querySelector<HTMLElement>("button")?.focus();
    return () => previous?.focus();
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !busy) onClose();
      if (event.key !== "Tab" || !panelRef.current) return;
      const items = [...panelRef.current.querySelectorAll<HTMLElement>('button:not([disabled]), [tabindex]:not([tabindex="-1"])')];
      const first = items[0];
      const last = items.at(-1);
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
      if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [onClose, busy]);

  function requestStart() {
    if (!readiness.data) return;
    start.reset();
    void admission.request({ action: "start", definitionId: definition.id, expectedRevision: readiness.data.definitionRevision });
  }

  const error = startError ?? admission.error;
  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-background/75 p-3 sm:p-4">
      <div aria-labelledby="loop-preflight-title" aria-modal="true" className="ucd-panel grid max-h-[90vh] w-full max-w-xl grid-rows-[auto_minmax(0,1fr)_auto] overflow-hidden rounded-lg shadow-xl" ref={panelRef} role="dialog">
        <header className="flex min-h-14 items-center gap-3 border-b border-border px-4 py-2">
          <div className="min-w-0 flex-1">
            <h2 className="truncate text-sm font-semibold" id="loop-preflight-title">{t("loops.preflight.title")}</h2>
            <p className="truncate text-xs text-muted-foreground">{definition.name}</p>
          </div>
          {readiness.data?.simulated ? <span className="rounded border border-warning/50 px-2 py-1 text-[11px] text-warning">{t("loops.simulated")}</span> : null}
          <Button aria-label={t("loops.preflight.close")} disabled={busy} onClick={onClose} size="icon" title={t("loops.preflight.close")} type="button" variant="ghost"><X aria-hidden="true" /></Button>
        </header>
        <div className="min-h-0 overflow-y-auto p-4">
          {readiness.isLoading ? <LoadingState /> : null}
          {readiness.error ? <p className="text-sm text-destructive" role="alert">{readiness.error instanceof Error ? readiness.error.message : String(readiness.error)}</p> : null}
          {readiness.data ? (
            <div className="grid gap-3">
              <p className="text-sm font-medium">{t(readiness.data.ready ? "loops.preflight.ready" : "loops.preflight.blocked")}</p>
              <ol className="grid gap-1" aria-label={t("loops.preflight.checks")}>
                {readiness.data.checks.map((check) => <ReadinessRow check={check} key={check.code} />)}
              </ol>
              <LoopScopeAssessment assessment={readiness.data.assessment} requestedMode={readiness.data.requestedMode} />
              {admission.pending ? <LoopAuditAcknowledgement admission={admission.pending} busy={busy} onAcknowledge={() => void admission.acknowledge()} onDismiss={admission.dismiss} /> : null}
            </div>
          ) : null}
        </div>
        <footer className="flex min-h-14 flex-wrap items-center justify-between gap-2 border-t border-border px-4 py-2">
          <p aria-live="assertive" className="min-w-0 flex-1 whitespace-pre-line text-xs text-destructive">{error}</p>
          <div className="flex gap-2">
            {!readiness.data?.ready ? <Button disabled={busy} onClick={onEdit} size="sm" type="button" variant="outline">{t("loops.preflight.edit")}</Button> : null}
            <Button disabled={readiness.isFetching || busy} onClick={() => { start.reset(); admission.dismiss(); void readiness.refetch(); }} size="sm" type="button" variant="outline"><RefreshCw aria-hidden="true" className={readiness.isFetching ? "animate-spin" : ""} />{t("loops.preflight.retry")}</Button>
            <Button disabled={!readiness.data?.ready || busy || admission.pending !== null} onClick={requestStart} size="sm" type="button">{busy ? <Loader2 aria-hidden="true" className="animate-spin" /> : <Play aria-hidden="true" />}{t("loops.preflight.start")}</Button>
          </div>
        </footer>
      </div>
    </div>
  );
}

function LoadingState() {
  const { t } = useTranslation();
  return <p className="flex items-center gap-2 text-sm text-muted-foreground"><Loader2 aria-hidden="true" className="h-4 w-4 animate-spin" />{t("loops.preflight.loading")}</p>;
}

function ReadinessRow({ check }: { check: LoopReadinessCheck }) {
  const { t } = useTranslation();
  const passed = check.status === "passed";
  const Icon = passed ? CheckCircle2 : AlertCircle;
  return (
    <li className="grid grid-cols-[auto_minmax(0,1fr)] gap-x-2 border-b border-border/60 py-2 last:border-0">
      <Icon aria-hidden="true" className={passed ? "mt-0.5 h-4 w-4 text-success" : "mt-0.5 h-4 w-4 text-destructive"} />
      <div className="min-w-0">
        <p className="text-sm font-medium">{t(`loops.preflight.check.${check.code}`)}</p>
        {!passed ? <p className="mt-0.5 whitespace-pre-line text-xs text-muted-foreground">{check.detail ?? t(`loops.preflight.remediation.${check.remediationTarget}`)}</p> : null}
      </div>
    </li>
  );
}
