import { AlertTriangle, CheckCircle2, Loader2, ShieldAlert, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { formatAppDateTime } from "../i18n/format";
import type { LoopAdmission, LoopExecutionAssessment, LoopRequestedMode } from "../types/loop";

/** The always-visible coverage facts. Limitations are listed even when the mode is satisfied so nobody mistakes an audited run for a contained one. */
export function LoopScopeAssessment({ assessment, requestedMode }: { assessment: LoopExecutionAssessment | null; requestedMode: LoopRequestedMode | null }) {
  const { t } = useTranslation();
  if (!assessment) return <p className="text-xs text-muted-foreground">{t("loops.assessment.unavailable")}</p>;
  const satisfied = assessment.satisfiesRequestedMode;
  return (
    <section aria-labelledby="loop-assessment-title" className="grid gap-2 rounded-md border border-border/70 p-3">
      <div className="flex flex-wrap items-center gap-2">
        {satisfied ? <ShieldCheck aria-hidden="true" className="h-4 w-4 text-success" /> : <ShieldAlert aria-hidden="true" className="h-4 w-4 text-destructive" />}
        <h3 className="text-xs font-semibold" id="loop-assessment-title">{t("loops.assessment.title")}</h3>
        <span className="rounded border border-border px-1.5 py-0.5 text-[11px]">{t(`loops.mode.${requestedMode ?? assessment.requestedMode}`)}</span>
        {assessment.simulated ? <span className="rounded border border-warning/50 px-1.5 py-0.5 text-[11px] text-warning">{t("loops.simulated")}</span> : null}
      </div>
      <p className={satisfied ? "text-xs text-success" : "text-xs text-destructive"}>{t(satisfied ? "loops.assessment.satisfied" : "loops.assessment.blocked", { mode: t(`loops.mode.${assessment.requestedMode}`) })}</p>
      <ul aria-label={t("loops.assessment.surfaces")} className="grid gap-1">
        {assessment.surfaces.map((surface) => (
          <li className="grid grid-cols-[auto_minmax(0,1fr)] gap-x-2 text-xs" key={surface.surface}>
            {surface.coverage === "complete-enforcement" ? <CheckCircle2 aria-hidden="true" className="mt-0.5 h-3.5 w-3.5 text-success" /> : <AlertTriangle aria-hidden="true" className={surface.blocking ? "mt-0.5 h-3.5 w-3.5 text-destructive" : "mt-0.5 h-3.5 w-3.5 text-warning"} />}
            <div className="min-w-0"><p><span className="font-medium">{t(`loops.assessment.surface.${surfaceKey(surface.surface)}`, { id: surfaceId(surface.surface) })}</span> · {t(`loops.coverage.${surface.coverage}`)}</p><p className="text-muted-foreground">{surface.detail}</p></div>
          </li>
        ))}
      </ul>
      {assessment.blockers.length > 0 ? <div><p className="text-[11px] font-semibold uppercase text-destructive">{t("loops.assessment.blockers")}</p><ul className="list-inside list-disc text-xs text-destructive">{assessment.blockers.map((blocker) => <li key={blocker}>{blocker}</li>)}</ul></div> : null}
      <div>
        <p className="text-[11px] font-semibold uppercase text-muted-foreground">{t("loops.assessment.limitations")}</p>
        {assessment.limitations.length > 0 ? <ul className="list-inside list-disc text-xs text-muted-foreground">{assessment.limitations.map((limitation) => <li key={limitation}>{limitation}</li>)}</ul> : <p className="text-xs text-muted-foreground">{t("loops.assessment.noLimitations")}</p>}
      </div>
    </section>
  );
}

/** Per-operation audit acknowledgement. The receipt it produces is valid for one transition of one target at one revision. */
export function LoopAuditAcknowledgement({ admission, busy, onAcknowledge, onDismiss }: { admission: LoopAdmission; busy: boolean; onAcknowledge: () => void; onDismiss: () => void }) {
  const { i18n, t } = useTranslation();
  return (
    <div aria-describedby="loop-audit-description" aria-labelledby="loop-audit-title" className="grid gap-3 rounded-md border border-warning/50 bg-warning/5 p-3" role="alertdialog">
      <div>
        <p className="text-xs font-semibold" id="loop-audit-title">{t("loops.audit.title", { action: t(`loops.audit.action.${admission.action}`) })}</p>
        <p className="mt-1 text-xs text-muted-foreground" id="loop-audit-description">{t("loops.audit.description")}</p>
      </div>
      <ul className="list-inside list-disc text-xs">{admission.assessment.limitations.map((limitation) => <li key={limitation}>{limitation}</li>)}</ul>
      <dl className="grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
        <dt>{t("loops.audit.scopeDigest")}</dt><dd className="break-all">{admission.scopeDigest}</dd>
        <dt>{t("loops.audit.revision")}</dt><dd>{admission.expectedRevision}</dd>
        {admission.expiresAt ? <><dt>{t("loops.audit.expires")}</dt><dd>{formatAppDateTime(admission.expiresAt, i18n.resolvedLanguage ?? "en", { timeStyle: "short" })}</dd></> : null}
      </dl>
      <div className="grid grid-cols-2 gap-2">
        <Button disabled={busy} onClick={onDismiss} size="sm" type="button" variant="ghost">{t("loops.controls.dismiss")}</Button>
        <Button disabled={busy || !admission.challengeId} onClick={onAcknowledge} size="sm" type="button">{busy ? <Loader2 aria-hidden="true" className="animate-spin" /> : null}{t("loops.audit.confirm", { action: t(`loops.audit.action.${admission.action}`) })}</Button>
      </div>
    </div>
  );
}

function surfaceKey(surface: string) {
  return surface.startsWith("verification:") ? "verification" : surface;
}

function surfaceId(surface: string) {
  return surface.startsWith("verification:") ? surface.slice("verification:".length) : "";
}
