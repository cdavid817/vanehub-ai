import { ArrowRight, ShieldAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { formatAppDateTime } from "../../../i18n/format";
import type { EvolutionBreakerSummary } from "../../../services/skill-evolution-orchestration-service";

export function SkillEvolutionBreakerPanel({
  acknowledging,
  breakers,
  initialBreakerId,
  onAcknowledge,
  onOpenCurator,
}: {
  acknowledging: boolean;
  breakers: EvolutionBreakerSummary[];
  initialBreakerId?: string;
  onAcknowledge: (breaker: EvolutionBreakerSummary) => void;
  onOpenCurator: () => void;
}) {
  const { i18n, t } = useTranslation();
  return <section aria-labelledby="evolution-breakers-title" className="space-y-3">
    <div><div className="flex items-center gap-2"><ShieldAlert className="h-4 w-4 text-amber-500" /><h3 className="font-semibold" id="evolution-breakers-title">{t("skills.evolution.breakers.title")}</h3></div><p className="mt-1 text-xs text-muted-foreground">{t("skills.evolution.breakers.description")}</p></div>
    {!breakers.length ? <div className="rounded-xl border border-emerald-500/30 bg-emerald-500/10 p-4 text-sm text-emerald-700 dark:text-emerald-300">{t("skills.evolution.breakers.empty")}</div> : <ul className="grid gap-3 lg:grid-cols-2">{breakers.map((breaker) => <li className={`rounded-xl border bg-background p-4 ${breaker.breakerId === initialBreakerId ? "border-primary ring-1 ring-primary" : "border-border"}`} key={breaker.breakerId}><div className="flex flex-wrap items-start justify-between gap-2"><div><p className="font-medium">{breaker.skillId ?? t("skills.evolution.breakers.workspace")}</p><p className="mt-1 font-mono text-xs text-muted-foreground">{breaker.breakerId}</p></div><Badge tone={breaker.status === "closed" ? "success" : "danger"}>{t(`skills.evolution.breakerStatus.${breaker.status}`)}</Badge></div><dl className="mt-3 grid gap-2 text-xs sm:grid-cols-2"><Fact label={t("skills.evolution.breakers.cause")} value={breaker.safeCauseCode ?? t("skills.evolution.notAvailable")} /><Fact label={t("skills.evolution.breakers.health")} value={t(breaker.healthProbePassed ? "skills.evolution.breakers.healthPassed" : "skills.evolution.breakers.healthPending")} /><Fact label={t("skills.evolution.breakers.healthVersion")} value={breaker.healthCheckVersion} /><Fact label={t("skills.evolution.breakers.updated")} value={dateTime(breaker.updatedAtMs, i18n.language)} /></dl><div className="mt-4 flex flex-wrap gap-2"><Button disabled={acknowledging || breaker.status !== "awaiting_acknowledgement" || !breaker.healthProbePassed} onClick={() => onAcknowledge(breaker)} size="sm">{t("skills.evolution.breakers.acknowledge")}</Button><Button onClick={onOpenCurator} size="sm" variant="outline"><ArrowRight />{t("skills.evolution.breakers.review")}</Button></div><p className="mt-3 text-xs text-muted-foreground">{t("skills.evolution.breakers.noRollback")}</p></li>)}</ul>}
  </section>;
}

function Fact({ label, value }: { label: string; value: string }) {
  return <div className="rounded-lg bg-muted/30 p-2"><dt className="text-muted-foreground">{label}</dt><dd className="mt-1 break-all font-medium">{value}</dd></div>;
}

function dateTime(value: number, language: string) {
  return formatAppDateTime(value, language, { dateStyle: "medium", timeStyle: "short" });
}
