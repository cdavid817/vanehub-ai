import { CheckCircle2, CircleDashed, ShieldAlert, Target } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { AssessmentCheck, AssessmentDetail, AssessmentSummary, AssessmentTarget } from "../../../services/skill-assessment-service";

export function AssessmentExplanation({ detail }: { detail: AssessmentDetail }) {
  const { t } = useTranslation();
  const passed = detail.checks.filter((check) => check.result === "pass").length;
  const provenance = detail.provenance.modelConsulted ? "model_assisted" : detail.provenance.fallbackReason ? "fallback" : "deterministic";
  const cards = [
    [t("skills.assessment.card.target"), detail.classification ? t(`skills.assessment.classification.${detail.classification}`) : "—"],
    [t("skills.assessment.card.checks"), t("skills.assessment.checkCount", { passed, total: detail.checks.length })],
    [t("skills.assessment.card.confidence"), detail.confidence ? t(`skills.assessment.confidence.${detail.confidence}`) : "—"],
    [t("skills.assessment.card.risk"), detail.risk ? t(`skills.assessment.risk.${detail.risk}`) : "—"],
    [t("skills.assessment.card.route"), detail.route ? t(`skills.assessment.route.${detail.route}`) : "—"],
    [t("skills.assessment.card.provenance"), t(`skills.assessment.provenance.${provenance}`)],
  ];
  return <div className="mt-4 space-y-4">
    <dl className="grid grid-cols-2 gap-2 sm:grid-cols-3 xl:grid-cols-6">{cards.map(([label, value]) => <div className="min-w-0 rounded-lg border border-border bg-background p-3" key={label}><dt className="text-[11px] text-muted-foreground">{label}</dt><dd className="mt-1 break-words text-sm font-semibold">{value}</dd></div>)}</dl>
    {detail.route === "advance" ? <p className="rounded-md border border-primary/30 bg-primary/5 p-3 text-xs leading-5">{t("skills.assessment.advanceExplanation")}</p> : null}
    {detail.provenance.modelConsulted ? <p className="rounded-md border border-border bg-muted/30 p-3 text-xs leading-5 text-muted-foreground">{t("skills.assessment.modelProvenance")}: {[detail.provenance.providerProtocol, detail.provenance.modelId, detail.provenance.templateVersion, detail.provenance.responseSchemaVersion].filter(Boolean).join(" · ")}</p> : null}
    {detail.provenance.fallbackReason ? <p className="rounded-md border border-warning/40 bg-warning/10 p-3 text-xs leading-5">{t("skills.assessment.fallback", { reason: detail.provenance.fallbackReason })}</p> : null}
    <div className="grid gap-4 xl:grid-cols-2"><TargetDetails detail={detail} /><CheckDetails checks={detail.checks} /></div>
    <VersionWitnesses detail={detail} />
  </div>;
}

function TargetDetails({ detail }: { detail: AssessmentDetail }) {
  const { t } = useTranslation();
  return <section aria-labelledby="assessment-targets-heading" className="rounded-lg border border-border bg-background p-3">
    <div className="flex items-center gap-2"><Target className="h-4 w-4 text-primary" /><h5 className="text-xs font-semibold" id="assessment-targets-heading">{t("skills.assessment.targetsTitle")}</h5></div>
    {detail.selectionThreshold ? <dl className="mt-3 grid grid-cols-3 gap-2 rounded-md bg-muted/40 p-2 text-xs"><Field label={t("skills.assessment.leadingScore")} value={String(detail.selectionThreshold.leadingScore)} /><Field label={t("skills.assessment.margin")} value={`${detail.selectionThreshold.margin}/${detail.selectionThreshold.requiredMargin}`} /><Field label={t("skills.assessment.threshold")} value={`${detail.selectionThreshold.ambiguousMinimum}/${detail.selectionThreshold.selectedMinimum}`} /></dl> : null}
    <ol className="mt-3 space-y-2">{detail.targets.map((target) => <TargetRow key={`${target.skillId}-${target.revisionHash}`} target={target} />)}</ol>
    {detail.targets.length === 0 ? <p className="mt-3 text-xs text-muted-foreground">{t("skills.assessment.noTargets")}</p> : null}
  </section>;
}

function TargetRow({ target }: { target: AssessmentTarget }) {
  const { t } = useTranslation();
  return <li><details className="group rounded-md border border-border p-3"><summary className="cursor-pointer list-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"><span className="flex flex-wrap items-center gap-2"><span className="font-mono text-xs text-muted-foreground">#{target.ordinal + 1}</span><span className="text-sm font-semibold">{target.skillId}</span><Badge tone={target.attributionUncertain ? "warning" : "success"}>{target.score}</Badge><Badge tone={target.attributionUncertain ? "warning" : "success"}>{t(`skills.assessment.attribution.${target.attribution}`)}</Badge>{target.attributionUncertain ? <Badge tone="warning">{t("skills.assessment.uncertain")}</Badge> : null}</span><span className="mt-1 block text-xs text-muted-foreground">{t(`skills.layer.${target.scope === "built_in" ? "system" : target.scope}`)} · {t(`skills.type.${target.skillType === "unknown" ? "role" : target.skillType}`)} · {target.revisionHash}</span></summary>
    <div className="mt-3 border-t border-border pt-3"><dl className="grid grid-cols-2 gap-2 text-xs sm:grid-cols-3">{target.components.map((component) => <Field key={component.component} label={t(`skills.assessment.component.${component.component}`)} value={String(component.score)} />)}<Field label={t("skills.assessment.lifecycle")} value={target.lifecycle} /><Field label={t("skills.assessment.trust")} value={target.trust} /></dl><p className="mt-3 text-xs text-muted-foreground">{t("skills.assessment.matched")}: {target.matchedFeatureClasses.join(", ") || t("skills.assessment.none")}</p></div></details></li>;
}

function CheckDetails({ checks }: { checks: AssessmentCheck[] }) {
  const { t } = useTranslation();
  return <section aria-labelledby="assessment-checks-heading" className="rounded-lg border border-border bg-background p-3">
    <div className="flex items-center gap-2"><ShieldAlert className="h-4 w-4 text-primary" /><h5 className="text-xs font-semibold" id="assessment-checks-heading">{t("skills.assessment.checksTitle")}</h5></div>
    <ol className="mt-3 space-y-2">{checks.map((check) => <li key={check.kind}><details className="rounded-md border border-border p-3"><summary className="cursor-pointer list-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"><span className="flex flex-wrap items-center gap-2">{check.result === "pass" ? <CheckCircle2 className="h-4 w-4 text-success" /> : <CircleDashed className="h-4 w-4 text-warning" />}<span className="text-xs font-medium">{t(`skills.assessment.check.${check.kind}`)}</span><Badge tone={checkTone(check)}>{t(`skills.assessment.checkResult.${check.result}`)}</Badge><Badge tone={riskTone(check.severity)}>{t(`skills.assessment.risk.${check.severity}`)}</Badge></span></summary><dl className="mt-3 grid gap-2 border-t border-border pt-3 text-xs sm:grid-cols-2"><Field label={t("skills.assessment.reason")} value={check.reasonCode} /><Field label={t("skills.assessment.evidenceReferences")} value={check.evidenceIds.join(", ") || t("skills.assessment.none")} /><Field label={t("skills.assessment.routingEffect")} value={check.routeConstraints.map((route) => t(`skills.assessment.route.${route}`)).join(", ") || t("skills.assessment.none")} /></dl></details></li>)}</ol>
  </section>;
}

function VersionWitnesses({ detail }: { detail: AssessmentDetail }) {
  const { t } = useTranslation();
  const witness = detail.versionWitnesses;
  return <details className="rounded-lg border border-border bg-background p-3"><summary className="cursor-pointer list-none text-xs font-semibold focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">{t("skills.assessment.versionsTitle")}</summary><dl className="mt-3 grid gap-2 border-t border-border pt-3 text-xs sm:grid-cols-2 xl:grid-cols-3"><Field label={t("skills.assessment.seedRevision")} value={detail.seedRevision} /><Field label={t("skills.assessment.witnessHash")} value={witness.witnessHash} /><Field label={t("skills.assessment.lineageHash")} value={witness.lineageHash} /><Field label={t("skills.assessment.targetUniverseHash")} value={witness.targetUniverseHash} /><Field label={t("skills.assessment.sanitizerVersion")} value={witness.sanitizerVersion} /><Field label={t("skills.assessment.policyVersions")} value={`${witness.selectorPolicyVersion} · ${witness.gatePolicyVersion} · ${witness.routingPolicyVersion} · ${witness.confidencePolicyVersion}`} /></dl></details>;
}

export function AssessmentHistory({ items, onSelect, selectedId }: { items: AssessmentSummary[]; onSelect: (id: string) => void; selectedId?: string }) {
  const { t, i18n } = useTranslation();
  return <section aria-labelledby="assessment-history-heading" className="rounded-lg border border-border bg-background p-3"><h5 className="text-xs font-semibold" id="assessment-history-heading">{t("skills.assessment.historyTitle")}</h5><p className="mt-1 text-xs leading-5 text-muted-foreground">{t("skills.assessment.historyHint")}</p><ol className="mt-3 space-y-2">{items.map((item) => <li className="rounded-md border border-border p-2" key={item.attemptId}><div className="flex flex-wrap items-center gap-2"><Badge tone={item.isCurrent ? "success" : "muted"}>{t(item.isCurrent ? "skills.assessment.current" : "skills.assessment.superseded")}</Badge><time className="text-xs text-muted-foreground">{new Intl.DateTimeFormat(i18n.language, { dateStyle: "medium", timeStyle: "short" }).format(item.createdAtMs)}</time><Button aria-pressed={selectedId === item.attemptId} className="ml-auto" onClick={() => onSelect(item.attemptId)} size="sm" variant="ghost">{t("skills.assessment.inspectAttempt")}</Button></div>{item.supersessionReason ? <p className="mt-2 break-words text-xs text-muted-foreground">{t("skills.assessment.supersessionReason", { reason: item.supersessionReason })}</p> : null}</li>)}</ol></section>;
}

function Field({ label, value }: { label: string; value: string }) { return <div className="min-w-0"><dt className="text-muted-foreground">{label}</dt><dd className="mt-0.5 break-words font-medium text-foreground">{value}</dd></div>; }
function checkTone(check: AssessmentCheck) { return check.result === "pass" ? "success" as const : check.result === "fail" ? "danger" as const : check.result === "review" ? "warning" as const : "muted" as const; }
function riskTone(risk: string) { return risk === "high" ? "danger" as const : risk === "medium" ? "warning" as const : "success" as const; }
