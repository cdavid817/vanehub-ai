import { useEffect, useState } from "react";
import { CheckCircle2, Cloud, Laptop, Pencil, Search, Settings2, Terminal, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../../../components/agent-brand-icon";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { getAgentVisualIdentity } from "../../../lib/agent-visual-identity";
import type { AgentRegistryEntry, InteractionMode } from "../../../types/agent";
import { isCliConfigAgentId, type CliConfigAgentId } from "../../../types/cli-agent-config";
import { StatusPill, TagList } from "../page-parts";
import { AgentToolTrustToggle } from "./agent-tool-trust-toggle";

const modeIcons: Record<InteractionMode, typeof Terminal> = {
  browser: Search,
  "native-desktop": Laptop,
  cli: Terminal,
  api: Cloud,
};

function availabilityTone(agent: AgentRegistryEntry): "success" | "warning" | "muted" {
  if (agent.availabilityState === "available") return "success";
  if (agent.availabilityState === "needs-auth" || agent.availabilityState === "unavailable") return "warning";
  return "muted";
}

export function AgentRuntimeCard({
  agent,
  active,
  activeMode,
  onSelect,
  onEdit,
  onDelete,
  onManageConfigurations,
}: {
  agent: AgentRegistryEntry;
  active: boolean;
  activeMode: InteractionMode | null;
  onSelect: (agent: AgentRegistryEntry, mode: InteractionMode) => void;
  onEdit: (agent: AgentRegistryEntry) => void;
  onDelete: (agent: AgentRegistryEntry) => void;
  onManageConfigurations: (agentId: CliConfigAgentId) => void;
}) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<InteractionMode>(activeMode ?? agent.supportedInteractionModes[0] ?? "cli");
  useEffect(() => {
    if (active && activeMode) setMode(activeMode);
  }, [active, activeMode]);
  const cliConfigAgentId = isCliConfigAgentId(agent.id) ? agent.id : null;

  return (
    <section className="ucd-panel ucd-interactive rounded-lg p-4">
      <div className="mb-4 flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2"><span className={`flex h-7 w-7 shrink-0 items-center justify-center rounded border ${getAgentVisualIdentity(agent.id).tone}`}><AgentBrandIcon agentId={agent.id} className="h-4 w-4" /></span><h3 className="truncate font-semibold">{agent.displayName}</h3></div>
          <p className="mt-1 text-sm text-muted-foreground">{agent.provider}</p>
        </div>
        <Badge tone={availabilityTone(agent)}>{agent.availabilityState}</Badge>
      </div>
      <TagList tags={agent.capabilityTags.slice(0, 3)} />
      <div className="mt-4 flex flex-wrap gap-2">
        {agent.supportedInteractionModes.map((candidate) => {
          const Icon = modeIcons[candidate];
          return <button className={`inline-flex h-8 items-center gap-1 rounded-md border px-2 text-xs ${mode === candidate ? "border-primary bg-primary text-primary-foreground" : "border-border hover:bg-muted"}`} key={candidate} onClick={() => setMode(candidate)} title={t(`agents.mode.${candidate}`)} type="button"><Icon className="h-3.5 w-3.5" />{t(`agents.mode.${candidate}`)}</button>;
        })}
      </div>
      <div className="mt-4 flex items-center justify-between gap-3"><StatusPill status={active ? t("agents.status.running") : t("agents.status.idle")} /><Button variant="outline" onClick={() => onSelect(agent, mode)}><CheckCircle2 className="h-4 w-4" />{t("agents.configure")}</Button></div>
      {agent.launch.kind === "api" ? <div className="mt-2 flex justify-end gap-2"><Button className="h-8 px-3 text-xs" onClick={() => onEdit(agent)} variant="outline"><Pencil className="h-3.5 w-3.5" />{t("agents.edit.action")}</Button><Button className="h-8 px-3 text-xs" onClick={() => onDelete(agent)} variant="outline"><Trash2 className="h-3.5 w-3.5" />{t("agents.delete.action")}</Button></div> : null}
      {agent.launch.kind === "api" ? <AgentToolTrustToggle agent={agent} /> : null}
      {cliConfigAgentId ? <Button className="mt-3 w-full" onClick={() => onManageConfigurations(cliConfigAgentId)} variant="outline"><Settings2 className="h-4 w-4" />{t("agents.globalConfig.manage")}</Button> : null}
    </section>
  );
}
