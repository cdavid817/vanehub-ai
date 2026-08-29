import { AlertTriangle, CheckCircle2, CircleDashed, Clock3 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type { GenerationJobSummary } from "../../../services/skill-generation-service";

export function SkillGenerationJobList({ items, onSelect, quarantineCount, selectedId }: {
  items: GenerationJobSummary[]; onSelect: (id: string) => void; quarantineCount: number; selectedId: string | null;
}) {
  const { t } = useTranslation();
  return <section aria-label={t("skills.generation.jobs")} className="space-y-3"><div className="grid grid-cols-2 gap-2"><Metric label={t("skills.generation.jobs")} value={items.length} /><Metric label={t("skills.generation.quarantined")} value={quarantineCount} /></div>{items.length ? <div className="space-y-2">{items.map((job) => <button aria-pressed={selectedId === job.jobId} className={`w-full rounded-xl border p-3 text-left transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${selectedId === job.jobId ? "border-primary bg-primary/5 shadow-sm" : "border-border bg-background hover:border-primary/40"}`} key={job.jobId} onClick={() => onSelect(job.jobId)} type="button"><div className="flex items-center justify-between gap-2"><span className="truncate text-sm font-medium">{job.seedId}</span><Status status={job.status} /></div><p className="mt-1 truncate font-mono text-[11px] text-muted-foreground">{job.jobId}</p><div className="mt-3 flex items-center justify-between text-[11px] text-muted-foreground"><span>{job.currentStage ? t(`skills.generation.stage.${job.currentStage}`) : t(`skills.generation.status.${job.status}`)}</span><span className="tabular-nums">{job.usage.inputTokens + job.usage.outputTokens} tok</span></div>{job.supersedesJobId ? <p className="mt-2 truncate text-[11px] text-amber-600 dark:text-amber-400">{t("skills.generation.supersedes", { id: job.supersedesJobId })}</p> : null}</button>)}</div> : <div className="rounded-xl border border-dashed border-border p-6 text-center text-sm text-muted-foreground">{t("skills.generation.noJobs")}</div>}</section>;
}

function Status({ status }: { status: GenerationJobSummary["status"] }) {
  const Icon = status === "completed" ? CheckCircle2 : status === "failed" ? AlertTriangle : status === "running" ? Clock3 : CircleDashed;
  return <Badge tone={status === "completed" ? "success" : status === "failed" ? "danger" : "muted"}><Icon className="h-3 w-3" />{status}</Badge>;
}
function Metric({ label, value }: { label: string; value: number }) { return <div className="rounded-xl border border-border bg-background p-3"><p className="text-[11px] text-muted-foreground">{label}</p><p className="mt-1 text-xl font-semibold tabular-nums">{value}</p></div>; }
