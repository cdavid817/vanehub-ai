import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";

import { Badge } from "../../../components/ui/badge";
import { agentService } from "../../../services/runtime-agent-client";
import type { EvidenceQueryInput, EvidenceSeedSummary, EvidenceSignalSummary } from "../../../services/agent-service";

export function SkillEvolutionLineage({ seed, scope }: { seed: EvidenceSeedSummary; scope: EvidenceQueryInput }) {
  const { t } = useTranslation();
  const query = useQuery({
    queryKey: ["skill-evolution-lineage", seed.seedId, scope.workspace ?? "", scope.skillId ?? ""],
    queryFn: () => agentService.getSkillEvolutionSeedLineage(seed.seedId, scope),
  });
  return <div className="mt-3 rounded-lg border border-border bg-muted/20 p-3" role="region" aria-label={t("skills.evolution.lineageTitle")}>
    <div className="flex flex-wrap items-center gap-2">
      <Badge tone={seed.readiness === "ready" ? "success" : "warning"}>{seed.readiness}</Badge>
      <span className="text-xs text-muted-foreground">{t("skills.evolution.runs", { count: seed.independentRunCount })}</span>
      <span className="text-xs text-muted-foreground">{seed.readinessReason}</span>
    </div>
    <p className="mt-2 text-xs leading-5">{seed.safeSummary}</p>
    {query.isLoading ? <p className="mt-3 text-xs text-muted-foreground" role="status">{t("skills.evolution.loadingLineage")}</p> : null}
    {query.isError ? <p className="mt-3 text-xs text-destructive" role="alert">{t("skills.evolution.lineageError")}</p> : null}
    {query.data ? <ol className="mt-3 space-y-2 border-t border-border pt-3">{query.data.signals.map((signal) => <SignalLineage key={signal.signalId} signal={signal} />)}</ol> : null}
  </div>;
}

function SignalLineage({ signal }: { signal: EvidenceSignalSummary }) {
  const { t } = useTranslation();
  const truncated = signal.associationTruncatedCount + signal.sourceLinkTruncatedCount;
  return <li className="rounded-md border border-border bg-background p-3 text-xs">
    <div className="flex flex-wrap gap-2"><Badge tone={attributionTone(signal.attribution)}>{signal.attribution}</Badge><span>{signal.category}</span><span className="text-muted-foreground">{signal.sourceKind}</span></div>
    <p className="mt-2 leading-5">{signal.safeSummary}</p>
    <dl className="mt-2 grid gap-1 text-muted-foreground sm:grid-cols-2">
      <div><dt className="inline">{t("skills.evolution.extractor")}: </dt><dd className="inline">{signal.extractorId} v{signal.extractorVersion}</dd></div>
      <div><dt className="inline">{t("skills.evolution.sanitizer")}: </dt><dd className="inline">v{signal.sanitizerVersion}</dd></div>
      <div><dt className="inline">{t("skills.evolution.fidelity")}: </dt><dd className="inline">{signal.sourceFidelity}</dd></div>
      <div><dt className="inline">{t("skills.evolution.occurred")}: </dt><dd className="inline">{signal.occurredAt}</dd></div>
    </dl>
    {truncated > 0 ? <p className="mt-2 text-warning" role="note">{t("skills.evolution.truncated", { count: truncated })}</p> : null}
  </li>;
}

export function attributionTone(attribution: string): "success" | "warning" | "muted" {
  if (attribution === "verified") return "success";
  if (attribution === "correlated") return "warning";
  return "muted";
}
