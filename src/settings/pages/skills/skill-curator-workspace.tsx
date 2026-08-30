import { useQuery } from "@tanstack/react-query";
import { RefreshCw, Scale } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { agentService } from "../../../services/runtime-agent-client";
import type { SkillCuratorService } from "../../../services/skill-curator-service";
import type { CuratorQueueQuery } from "../../../types/skill-curator";
import { defaultCuratorFilters, SkillCuratorFilters, type CuratorFilters } from "./skill-curator-filters";
import { SkillCuratorQueue } from "./skill-curator-queue";
import { SkillCuratorReview } from "./skill-curator-review";
import { SkillCuratorPolicyPanel } from "./skill-curator-policy";

export function SkillCuratorWorkspace({
  initialCandidateId,
  initialWorkspaceId = "",
  service = agentService,
}: {
  initialCandidateId?: string;
  initialWorkspaceId?: string;
  service?: SkillCuratorService;
}) {
  const { t } = useTranslation();
  const [workspaceDraft, setWorkspaceDraft] = useState(initialWorkspaceId);
  const [workspaceId, setWorkspaceId] = useState(initialWorkspaceId);
  const [filters, setFilters] = useState<CuratorFilters>(defaultCuratorFilters);
  const [selectedId, setSelectedId] = useState<string | null>(initialCandidateId ?? null);
  const input = useMemo(() => buildQuery(workspaceId, filters), [filters, workspaceId]);
  const queue = useQuery({
    enabled: Boolean(workspaceId.trim()),
    queryKey: ["skill-curator-queue", input],
    queryFn: () => service.querySkillCuratorQueue(input),
  });
  const page = queue.data?.ok ? queue.data.value : undefined;
  return <section aria-labelledby="skill-curator-title" className="space-y-4">
    <header className="rounded-xl border border-border bg-gradient-to-br from-primary/10 via-background to-background p-4 shadow-sm sm:p-5"><div className="flex flex-wrap items-start justify-between gap-3"><div><div className="flex items-center gap-2"><Scale className="h-5 w-5 text-primary" /><h2 className="text-lg font-semibold" id="skill-curator-title">{t("skills.curator.title")}</h2></div><p className="mt-1 max-w-3xl text-xs leading-5 text-muted-foreground">{t("skills.curator.description")}</p></div>{page ? <Badge tone="muted">{t("skills.curator.total", { count: page.totalCount })}</Badge> : null}</div>
      <form className="mt-4 flex flex-col gap-2 sm:flex-row" onSubmit={(event) => { event.preventDefault(); setWorkspaceId(workspaceDraft.trim()); setSelectedId(null); }}><label className="min-w-0 flex-1 text-xs text-muted-foreground"><span>{t("skills.curator.workspace")}</span><input className="mt-1 h-9 w-full rounded-md border border-border bg-background px-3 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => setWorkspaceDraft(event.target.value)} placeholder={t("skills.curator.workspacePlaceholder")} value={workspaceDraft} /></label><Button className="sm:mt-5" type="submit">{t("skills.curator.openWorkspace")}</Button></form>
    </header>
    {!workspaceId ? <div className="rounded-xl border border-dashed border-border p-8 text-center text-sm text-muted-foreground">{t("skills.curator.workspaceRequired")}</div> : <>
      <SkillCuratorFilters filters={filters} onChange={setFilters} />
      <SkillCuratorPolicyPanel service={service} workspaceId={workspaceId} />
      {queue.isLoading ? <p className="rounded-xl border border-border p-4 text-sm text-muted-foreground" role="status">{t("skills.curator.loading")}</p> : null}
      {queue.isError || (queue.data && !queue.data.ok) ? <div className="rounded-xl border border-destructive/40 bg-destructive/10 p-4 text-sm text-destructive" role="alert"><p>{t("skills.curator.loadError")}</p><Button className="mt-2" onClick={() => void queue.refetch()} size="sm" variant="outline"><RefreshCw />{t("featureLoad.retry")}</Button></div> : null}
      {page ? <><QueueCounts items={page.items} total={page.totalCount} /><div className="grid min-w-0 gap-4 xl:grid-cols-[minmax(17rem,0.7fr)_minmax(0,1.3fr)]"><SkillCuratorQueue items={page.items} onSelect={setSelectedId} selectedId={selectedId} /><div className="min-w-0">{selectedId ? <SkillCuratorReview candidateId={selectedId} onQueueChanged={() => queue.refetch()} service={service} /> : <div className="rounded-xl border border-dashed border-border p-8 text-center text-sm text-muted-foreground">{t("skills.curator.selectCandidate")}</div>}</div></div></> : null}
    </>}
  </section>;
}

function buildQuery(workspaceId: string, filters: CuratorFilters): CuratorQueueQuery {
  const ageDays = { day: 1, week: 7, month: 30 } as const;
  return {
    workspaceId,
    ...(filters.skillId.trim() ? { skillId: filters.skillId.trim() } : {}),
    ...(filters.state === "all" ? {} : { states: [filters.state] }),
    ...(filters.route === "all" ? {} : { routes: [filters.route] }),
    ...(filters.risk === "all" ? {} : { risks: [filters.risk] }),
    ...(filters.readiness === "all" ? {} : { draftReady: filters.readiness === "ready" }),
    ...(filters.stale === "all" ? {} : { stale: filters.stale === "stale" }),
    ...(filters.notification === "all" ? {} : { notificationPending: filters.notification === "pending" }),
    ...(filters.age === "all" ? {} : { updatedBeforeMs: Date.now() - ageDays[filters.age] * 86_400_000 }),
    limit: 100,
  };
}

function QueueCounts({ items, total }: { items: Array<{ state: string; risk: string }>; total: number }) {
  const { t } = useTranslation();
  const metrics = [["total", total], ["ready", items.filter((item) => item.state === "ready_for_review").length], ["highRisk", items.filter((item) => item.risk === "high").length], ["failed", items.filter((item) => item.state === "apply_failed").length]] as const;
  return <dl className="grid grid-cols-2 gap-2 sm:grid-cols-4">{metrics.map(([key, value]) => <div className="rounded-xl border border-border bg-background p-3" key={key}><dt className="text-[11px] text-muted-foreground">{t(`skills.curator.count.${key}`)}</dt><dd className="mt-1 text-xl font-semibold tabular-nums">{value}</dd></div>)}</dl>;
}
