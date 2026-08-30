import { useQuery } from "@tanstack/react-query";
import { RefreshCw, Sparkles } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { agentService } from "../../../services/runtime-agent-client";
import type { SkillGenerationService } from "../../../services/skill-generation-service";
import { SkillGenerationJobDetail } from "./skill-generation-job-detail";
import { SkillGenerationJobList } from "./skill-generation-job-list";
import { SkillGenerationPolicyPanel } from "./skill-generation-policy-panel";

export function SkillGenerationWorkspace({
  initialWorkspaceId = "",
  initialJobId,
  onOpenCurator,
  service = agentService,
}: {
  initialWorkspaceId?: string;
  initialJobId?: string;
  onOpenCurator: (workspaceId: string) => void;
  service?: SkillGenerationService;
}) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState(initialWorkspaceId);
  const [workspaceId, setWorkspaceId] = useState(initialWorkspaceId);
  const [selectedId, setSelectedId] = useState<string | null>(initialJobId ?? null);
  const queryInput = useMemo(() => ({ workspaceId, limit: 100 }), [workspaceId]);
  const jobs = useQuery({
    enabled: Boolean(workspaceId),
    queryKey: ["skill-generation-jobs", queryInput],
    queryFn: () => service.listGenerationJobs(queryInput),
    refetchInterval: (query) => query.state.data?.items.some((job) =>
      ["requested", "queued", "running", "cancel_requested"].includes(job.status)) ? 2_000 : false,
  });
  const quarantine = useQuery({
    enabled: Boolean(workspaceId),
    queryKey: ["skill-generation-quarantine", workspaceId],
    queryFn: () => service.listGenerationQuarantine({ workspaceId, limit: 100 }),
  });
  return <section aria-labelledby="skill-generation-title" className="space-y-4">
    <header className="overflow-hidden rounded-xl border border-border bg-gradient-to-br from-violet-500/15 via-background to-cyan-500/10 p-4 shadow-sm sm:p-5">
      <div className="flex flex-wrap items-start justify-between gap-3"><div><div className="flex items-center gap-2"><Sparkles className="h-5 w-5 text-violet-500" /><h2 className="text-lg font-semibold" id="skill-generation-title">{t("skills.generation.title")}</h2></div><p className="mt-1 max-w-3xl text-xs leading-5 text-muted-foreground">{t("skills.generation.description")}</p></div><Badge tone="muted">{t("skills.generation.manualOnly")}</Badge></div>
      <form className="mt-4 flex flex-col gap-2 sm:flex-row" onSubmit={(event) => { event.preventDefault(); setWorkspaceId(draft.trim()); setSelectedId(null); }}><label className="min-w-0 flex-1 text-xs text-muted-foreground"><span>{t("skills.generation.workspace")}</span><input className="mt-1 h-9 w-full rounded-md border border-border bg-background px-3 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => setDraft(event.target.value)} placeholder={t("skills.generation.workspacePlaceholder")} value={draft} /></label><Button className="sm:mt-5" type="submit">{t("skills.generation.openWorkspace")}</Button></form>
    </header>
    {!workspaceId ? <Empty text={t("skills.generation.workspaceRequired")} /> : <>
      <SkillGenerationPolicyPanel service={service} workspaceId={workspaceId} />
      {jobs.isLoading ? <p className="rounded-xl border border-border p-4 text-sm text-muted-foreground" role="status">{t("skills.generation.loading")}</p> : null}
      {jobs.isError ? <div className="rounded-xl border border-destructive/40 bg-destructive/10 p-4 text-sm text-destructive" role="alert"><p>{jobs.error.message}</p><Button className="mt-2" onClick={() => void jobs.refetch()} size="sm" variant="outline"><RefreshCw />{t("featureLoad.retry")}</Button></div> : null}
      {jobs.data ? <div className="grid min-w-0 gap-4 xl:grid-cols-[minmax(18rem,0.72fr)_minmax(0,1.28fr)]"><SkillGenerationJobList items={jobs.data.items} onSelect={setSelectedId} selectedId={selectedId} quarantineCount={quarantine.data?.items.length ?? 0} /><div className="min-w-0">{selectedId ? <SkillGenerationJobDetail jobId={selectedId} onChanged={() => jobs.refetch()} onOpenCurator={() => onOpenCurator(workspaceId)} service={service} /> : <Empty text={t("skills.generation.selectJob")} />}</div></div> : null}
    </>}
  </section>;
}

function Empty({ text }: { text: string }) {
  return <div className="rounded-xl border border-dashed border-border bg-muted/10 p-8 text-center text-sm text-muted-foreground">{text}</div>;
}
