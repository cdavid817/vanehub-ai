import { useTranslation } from "react-i18next";
import { getSkillBindingState, isSkillAssigned, isSkillAssignedToAgent, partitionSkillsForAgent, type SkillInventoryView } from "../../../lib/skill-management";
import type { SkillOverview } from "../../../types/skill";

export function SkillInventorySummary({ overview, view }: { overview: SkillOverview; view: SkillInventoryView }) {
  const { t } = useTranslation();
  const items = buildItems(overview, view);

  return (
    <dl aria-label={t("skills.summary.ariaLabel")} className="flex flex-wrap gap-x-4 gap-y-2 text-xs">
      {items.map(({ key, labelKey, value }) => (
        <div className="flex items-baseline gap-1" key={key}>
          <dt className="text-muted-foreground">{t(labelKey)}</dt>
          <dd className="font-semibold tabular-nums text-foreground">{value}</dd>
        </div>
      ))}
    </dl>
  );
}

function buildItems(overview: SkillOverview, view: SkillInventoryView) {
  if (view.kind === "all") {
    const cliAgents = overview.agents.filter((agent) => agent.kind === "cli");
    const apiAgents = overview.agents.filter((agent) => agent.kind === "api");
    return [
      item("total", "skills.stats.total", overview.skills.length),
      item("enabled", "skills.stats.enabled", overview.skills.filter((skill) => skill.enabled).length),
      item("cli", "skills.summary.cliBound", overview.skills.filter((skill) => cliAgents.some((agent) => isSkillAssignedToAgent(skill, agent, overview.apiAgentBindings))).length),
      item("api", "skills.summary.apiBound", overview.skills.filter((skill) => apiAgents.some((agent) => isSkillAssignedToAgent(skill, agent, overview.apiAgentBindings))).length),
      item("unassigned", "skills.navigation.unassigned", overview.skills.filter((skill) => !isSkillAssigned(skill, overview)).length),
    ];
  }

  if (view.kind === "agent") {
    const agent = overview.agents.find((candidate) => candidate.id === view.agentId)
      ?? { id: view.agentId, displayName: view.agentId, kind: view.agentKind };
    const partition = partitionSkillsForAgent(overview.skills, agent, overview.apiAgentBindings);
    const states = partition.assigned.map((skill) => getSkillBindingState(skill, agent, overview.apiAgentBindings));
    const active = states.filter((state) => state === "mounted" || state === "api-prompt").length;
    const configured = states.filter((state) => state === "configured").length;
    const paused = states.filter((state) => state === "paused").length;
    return [
      item("assigned", "skills.assignment.assigned", partition.assigned.length),
      item("active", "skills.summary.active", active),
      item("configured", "skills.binding.configured", configured),
      item("paused", "skills.binding.paused", paused),
      item("available", "skills.assignment.available", partition.available.length),
    ];
  }

  const unassigned = overview.skills.filter((skill) => !isSkillAssigned(skill, overview));
  return [
    item("unassigned", "skills.navigation.unassigned", unassigned.length),
    item("enabled", "skills.stats.enabled", unassigned.filter((skill) => skill.enabled).length),
    item("disabled", "basic.disabled", unassigned.filter((skill) => !skill.enabled).length),
  ];
}

function item(key: string, labelKey: string, value: number) {
  return { key, labelKey, value };
}
