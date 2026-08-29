import { Clock3, FileCheck2, ShieldAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type { CuratorCandidateSummary, CuratorCandidateState, CuratorRisk } from "../../../types/skill-curator";

export function SkillCuratorQueue({
  items,
  onSelect,
  selectedId,
}: {
  items: CuratorCandidateSummary[];
  onSelect: (candidateId: string) => void;
  selectedId: string | null;
}) {
  const { t, i18n } = useTranslation();
  if (items.length === 0) return <div className="rounded-xl border border-dashed border-border p-8 text-center"><FileCheck2 className="mx-auto h-7 w-7 text-muted-foreground" /><p className="mt-3 text-sm font-semibold">{t("skills.curator.empty")}</p><p className="mt-1 text-xs text-muted-foreground">{t("skills.curator.emptyHint")}</p></div>;
  return <div aria-label={t("skills.curator.queue")} className="space-y-2" role="list">
    {items.map((item) => <button
      aria-current={selectedId === item.candidateId ? "true" : undefined}
      className={`w-full rounded-xl border p-3 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${selectedId === item.candidateId ? "border-primary bg-primary/5" : "border-border bg-background hover:bg-muted/40"}`}
      key={item.candidateId}
      onClick={() => onSelect(item.candidateId)}
      role="listitem"
      type="button"
    >
      <span className="flex flex-wrap items-start justify-between gap-2"><span className="min-w-0"><span className="block truncate text-sm font-semibold">{item.targetSkillId}</span><span className="mt-0.5 block truncate font-mono text-[10px] text-muted-foreground">{item.candidateId}</span></span><Badge tone={stateTone(item.state)}>{t(`skills.curator.value.${item.state}`)}</Badge></span>
      <span className="mt-3 flex flex-wrap gap-1.5"><Badge tone={riskTone(item.risk)}><ShieldAlert className="mr-1 h-3 w-3" />{t(`skills.curator.value.${item.risk}`)}</Badge><Badge tone="muted">{t(`skills.curator.value.${item.route}`)}</Badge>{item.draftReady ? <Badge tone="success">{t("skills.curator.draftReady")}</Badge> : null}{item.staleness.length > 0 ? <Badge tone="warning">{t("skills.curator.stale")}</Badge> : null}</span>
      <span className="mt-2 flex items-center gap-1 text-[10px] text-muted-foreground"><Clock3 className="h-3 w-3" />{new Intl.DateTimeFormat(i18n.language, { dateStyle: "medium", timeStyle: "short" }).format(item.updatedAtMs)}</span>
    </button>)}
  </div>;
}

function stateTone(state: CuratorCandidateState) {
  if (state === "applied") return "success" as const;
  if (state === "apply_failed" || state === "rejected") return "danger" as const;
  if (state === "superseded") return "muted" as const;
  return "warning" as const;
}

function riskTone(risk: CuratorRisk) {
  return risk === "high" ? "danger" as const : risk === "medium" ? "warning" as const : "success" as const;
}
