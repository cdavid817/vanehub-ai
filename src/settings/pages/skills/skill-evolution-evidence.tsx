import { useQuery } from "@tanstack/react-query";
import { Activity, Database, ShieldCheck, Trash2, TriangleAlert } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { EvidenceOverview, EvidenceSignalSummary } from "../../../services/agent-service";
import { agentService } from "../../../services/runtime-agent-client";
import type { Skill } from "../../../types/skill";
import { SkillEvolutionLineage, attributionTone } from "./skill-evolution-lineage";
import { SkillEvolutionPurgeDialog } from "./skill-evolution-purge-dialog";

export function SkillEvolutionEvidence({ skill }: { skill: Skill }) {
  const { t } = useTranslation();
  const [selectedSeed, setSelectedSeed] = useState<string | null>(null);
  const [purgeTrigger, setPurgeTrigger] = useState<HTMLElement | null>(null);
  const [filters, setFilters] = useState({ attribution: "all", category: "all", readiness: "all" });
  const scope = useMemo(() => ({ skillId: skill.id, workspace: skill.workspacePath ?? undefined, limit: 50 }), [skill.id, skill.workspacePath]);
  const query = useQuery({
    queryKey: ["skill-evolution-evidence", scope.workspace ?? "", scope.skillId],
    queryFn: () => agentService.querySkillEvolutionEvidence(scope),
  });
  const overview = query.data;
  const signals = overview?.signals.filter((signal) =>
    (filters.attribution === "all" || signal.attribution === filters.attribution)
    && (filters.category === "all" || signal.category === filters.category)) ?? [];
  const seeds = overview?.seeds.filter((seed) => filters.readiness === "all" || seed.readiness === filters.readiness) ?? [];

  return <section aria-labelledby="skill-evolution-heading" className="rounded-lg border border-border bg-muted/10 p-3 sm:p-4">
    <div className="flex flex-wrap items-start justify-between gap-3">
      <div>
        <div className="flex items-center gap-2"><Activity className="h-4 w-4 text-primary" /><h4 className="text-sm font-semibold" id="skill-evolution-heading">{t("skills.evolution.title")}</h4></div>
        <p className="mt-1 text-xs leading-5 text-muted-foreground">{t("skills.evolution.evidenceOnly")}</p>
      </div>
      {overview ? <PipelineBadge overview={overview} /> : null}
    </div>
    {query.isLoading ? <p className="mt-4 text-xs text-muted-foreground" role="status">{t("skills.evolution.loading")}</p> : null}
    {query.isError ? <div className="mt-4 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-xs text-destructive" role="alert"><p>{t("skills.evolution.loadError")}</p><Button className="mt-2" onClick={() => void query.refetch()} size="sm" variant="outline">{t("skills.evolution.retry")}</Button></div> : null}
    {overview ? <>
      <EvidenceStatus overview={overview} />
      {overview.signalCount === 0 ? <EmptyEvidence retentionDays={overview.retentionDays} /> : <>
        <EvidenceMetrics overview={overview} />
        <EvidenceDistributions overview={overview} />
        <EvidenceFilters filters={filters} overview={overview} onChange={setFilters} />
        <div className="mt-4 space-y-4">
          <EvidenceSignals signals={signals} />
          <div>
            <h5 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("skills.evolution.seeds")}</h5>
            <div className="mt-2 space-y-2">{seeds.map((seed) => <div className="rounded-lg border border-border bg-background p-3" key={seed.seedId}>
              <div className="flex flex-wrap items-start justify-between gap-2"><div><Badge tone={seed.readiness === "ready" ? "success" : "warning"}>{seed.readiness}</Badge><p className="mt-2 text-xs leading-5">{seed.safeSummary}</p></div><Button aria-expanded={selectedSeed === seed.seedId} onClick={() => setSelectedSeed((current) => current === seed.seedId ? null : seed.seedId)} size="sm" variant="outline">{t("skills.evolution.inspectLineage")}</Button></div>
              {selectedSeed === seed.seedId ? <SkillEvolutionLineage scope={scope} seed={seed} /> : null}
            </div>)}</div>
          </div>
        </div>
      </>}
      <PrivacySummary overview={overview} />
      <div className="mt-4 flex justify-end border-t border-border pt-3"><Button onClick={(event) => setPurgeTrigger(event.currentTarget)} size="sm" variant="outline"><Trash2 />{t("skills.evolution.purgeAction")}</Button></div>
    </> : null}
    {purgeTrigger ? <SkillEvolutionPurgeDialog onClose={() => setPurgeTrigger(null)} onPurged={() => query.refetch()} returnFocus={purgeTrigger} skillId={skill.id} workspace={skill.workspacePath ?? undefined} /> : null}
  </section>;
}

function PipelineBadge({ overview }: { overview: EvidenceOverview }) {
  const { t } = useTranslation();
  const tone = overview.pipeline.status === "healthy" ? "success" : overview.pipeline.status === "disabled" ? "muted" : "warning";
  return <Badge tone={tone}>{t(`skills.evolution.pipeline.${overview.pipeline.status}`)}</Badge>;
}

function EvidenceStatus({ overview }: { overview: EvidenceOverview }) {
  const { t } = useTranslation();
  if (overview.pipeline.status === "healthy" && overview.droppedCount === 0) return null;
  return <div className="mt-4 flex gap-2 rounded-md border border-warning/40 bg-warning/10 p-3 text-xs leading-5" role="status"><TriangleAlert className="mt-0.5 h-4 w-4 shrink-0" /><span>{overview.pipeline.status === "disabled" ? t("skills.evolution.disabled") : t("skills.evolution.degraded", { failures: overview.pipeline.failureCount, dropped: overview.droppedCount })}</span></div>;
}

function EvidenceMetrics({ overview }: { overview: EvidenceOverview }) {
  const { t } = useTranslation();
  const ready = overview.seeds.filter((seed) => seed.readiness === "ready").length;
  return <dl className="mt-4 grid grid-cols-2 gap-2 sm:grid-cols-3">
    {[[t("skills.evolution.signals"), overview.signalCount], [t("skills.evolution.seeds"), overview.seedCount], [t("skills.evolution.ready"), ready]].map(([label, value]) => <div className="rounded-md border border-border bg-background p-3" key={label}><dt className="text-[11px] text-muted-foreground">{label}</dt><dd className="mt-1 text-lg font-semibold tabular-nums">{value}</dd></div>)}
  </dl>;
}

function EvidenceDistributions({ overview }: { overview: EvidenceOverview }) {
  const { t } = useTranslation();
  const groups = [
    [t("skills.evolution.categories"), "category"],
    [t("skills.evolution.attributionFidelity"), "attribution_strength"],
    [t("skills.evolution.sourceAgents"), "source_agent_id"],
    [t("skills.evolution.revisions"), "skill_revision"],
  ] as const;
  return <div className="mt-4 grid gap-2 sm:grid-cols-2">{groups.map(([label, key]) => <div className="rounded-md border border-border bg-background p-3" key={key}><h5 className="text-[11px] font-medium text-muted-foreground">{label}</h5><div className="mt-2 flex flex-wrap gap-1.5">{Object.entries(overview.distributions[key] ?? {}).map(([value, count]) => <Badge key={value} tone={key === "attribution_strength" ? attributionTone(value) : "muted"}>{value} · {count}</Badge>)}</div></div>)}</div>;
}

function EvidenceFilters({ filters, overview, onChange }: { filters: { attribution: string; category: string; readiness: string }; overview: EvidenceOverview; onChange: (value: { attribution: string; category: string; readiness: string }) => void }) {
  const { t } = useTranslation();
  const options = (key: string) => Object.keys(overview.distributions[key] ?? {});
  return <fieldset className="mt-4 grid gap-2 sm:grid-cols-3"><legend className="sr-only">{t("skills.evolution.filters")}</legend>
    <Filter label={t("skills.evolution.attribution")} options={options("attribution_strength")} value={filters.attribution} onChange={(attribution) => onChange({ ...filters, attribution })} />
    <Filter label={t("skills.evolution.category")} options={options("category")} value={filters.category} onChange={(category) => onChange({ ...filters, category })} />
    <Filter label={t("skills.evolution.readiness")} options={[...new Set(overview.seeds.map((seed) => seed.readiness))]} value={filters.readiness} onChange={(readiness) => onChange({ ...filters, readiness })} />
  </fieldset>;
}

function Filter({ label, options, value, onChange }: { label: string; options: string[]; value: string; onChange: (value: string) => void }) {
  const { t } = useTranslation();
  return <label className="text-xs text-muted-foreground"><span>{label}</span><select className="mt-1 h-9 w-full rounded-md border border-border bg-background px-2 text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => onChange(event.target.value)} value={value}><option value="all">{t("skills.evolution.all")}</option>{options.map((option) => <option key={option} value={option}>{option}</option>)}</select></label>;
}

function EvidenceSignals({ signals }: { signals: EvidenceSignalSummary[] }) {
  const { t } = useTranslation();
  return <div><h5 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("skills.evolution.recentActivity")}</h5><div className="mt-2 space-y-2">{signals.map((signal) => <details className="rounded-lg border border-border bg-background p-3" key={signal.signalId}><summary className="cursor-pointer list-none text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"><span className="flex flex-wrap items-center gap-2"><Badge tone={attributionTone(signal.attribution)}>{signal.attribution}</Badge><span className="font-medium">{signal.category}</span><time className="ml-auto text-muted-foreground">{signal.occurredAt}</time></span><span className="mt-2 block leading-5">{signal.safeSummary}</span></summary><dl className="mt-3 grid gap-2 border-t border-border pt-3 text-xs text-muted-foreground sm:grid-cols-2"><Field label={t("skills.evolution.source")} value={`${signal.sourceKind} · ${signal.sourceAgentId ?? "—"}`} /><Field label={t("skills.evolution.extractor")} value={`${signal.extractorId} v${signal.extractorVersion}`} /><Field label={t("skills.evolution.fidelity")} value={`${signal.attributionRationale} · ${signal.sourceFidelity}`} /><Field label={t("skills.evolution.sanitizer")} value={`v${signal.sanitizerVersion}`} /></dl></details>)}</div></div>;
}

function Field({ label, value }: { label: string; value: string }) { return <div><dt>{label}</dt><dd className="mt-0.5 break-words text-foreground">{value}</dd></div>; }

function EmptyEvidence({ retentionDays }: { retentionDays: number }) { const { t } = useTranslation(); return <div className="mt-4 rounded-lg border border-dashed border-border p-4 text-center"><Database className="mx-auto h-5 w-5 text-muted-foreground" /><p className="mt-2 text-sm font-medium">{t("skills.evolution.emptyTitle")}</p><p className="mt-1 text-xs leading-5 text-muted-foreground">{t("skills.evolution.emptyDescription", { days: retentionDays })}</p></div>; }

function PrivacySummary({ overview }: { overview: EvidenceOverview }) { const { t } = useTranslation(); return <details className="mt-4 rounded-lg border border-border bg-background p-3"><summary className="flex cursor-pointer list-none items-center gap-2 text-xs font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"><ShieldCheck className="h-4 w-4 text-primary" />{t("skills.evolution.privacyTitle")}</summary><p className="mt-2 text-xs leading-5 text-muted-foreground">{t("skills.evolution.privacyDescription", { days: overview.retentionDays, quota: overview.signalQuota, expired: overview.expiredCount })}</p></details>; }
