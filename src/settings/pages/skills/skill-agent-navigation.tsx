import { Boxes, Cloud, Link2Off } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../../../components/agent-brand-icon";
import { Button } from "../../../components/ui/button";
import { getAgentVisualIdentity } from "../../../lib/agent-visual-identity";
import type { SkillInventoryView } from "../../../lib/skill-management";
import type { SkillCompatibleAgent } from "../../../types/skill";
import { SectionPanel } from "../page-parts";

export function SkillAgentNavigation({
  agents,
  counts,
  selected,
  onSelect,
}: {
  agents: SkillCompatibleAgent[];
  counts: Record<string, number>;
  selected: SkillInventoryView;
  onSelect: (view: SkillInventoryView) => void;
}) {
  const { t } = useTranslation();
  const cliAgents = agents.filter((agent) => agent.kind === "cli");
  const apiAgents = agents.filter((agent) => agent.kind === "api");
  const selectedKey = selected.kind === "agent" ? `agent:${selected.agentId}` : selected.kind;

  function navigationButton(key: string, label: string, icon: ReactNode, view: SkillInventoryView) {
    return (
      <Button className="w-full justify-start gap-2" key={key} onClick={() => onSelect(view)} variant={selectedKey === key ? "default" : "ghost"}>
        {icon}
        <span className="min-w-0 flex-1 truncate text-left">{label}</span>
        <span className="text-xs tabular-nums opacity-70">{counts[key] ?? 0}</span>
      </Button>
    );
  }

  function agentButton(agent: SkillCompatibleAgent) {
    const identity = getAgentVisualIdentity(agent.id);
    return navigationButton(
      `agent:${agent.id}`,
      agent.displayName,
      <span className={`flex h-6 w-6 shrink-0 items-center justify-center rounded border ${identity.tone}`}>
        <AgentBrandIcon agentId={agent.id} className="h-3.5 w-3.5" />
      </span>,
      { kind: "agent", agentId: agent.id, agentKind: agent.kind },
    );
  }

  return (
    <SectionPanel description={t("skills.navigation.description")} title={t("skills.navigation.title")}>
      <nav aria-label={t("skills.navigation.ariaLabel")} className="space-y-4">
        <div className="space-y-1">
          {navigationButton("all", t("skills.navigation.all"), <Boxes className="h-4 w-4" />, { kind: "all" })}
          {navigationButton("unassigned", t("skills.navigation.unassigned"), <Link2Off className="h-4 w-4" />, { kind: "unassigned" })}
        </div>
        {cliAgents.length > 0 ? <div className="space-y-1"><p className="px-2 text-xs font-semibold text-muted-foreground">{t("skills.navigation.cliAgents")}</p>{cliAgents.map(agentButton)}</div> : null}
        {apiAgents.length > 0 ? <div className="space-y-1"><p className="flex items-center gap-1 px-2 text-xs font-semibold text-muted-foreground"><Cloud className="h-3.5 w-3.5" />{t("skills.navigation.apiAgents")}</p>{apiAgents.map(agentButton)}</div> : null}
      </nav>
    </SectionPanel>
  );
}
