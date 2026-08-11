import type { Session } from "../types/agent";
import type { Skill, SkillCompatibleAgent, SkillOverview, SkillSource } from "../types/skill";
import type { SkillScopeInput } from "../types/skill";

export type SkillInventoryView =
  | { kind: "all" }
  | { kind: "unassigned" }
  | { kind: "agent"; agentId: string; agentKind: SkillCompatibleAgent["kind"] };

export type SkillStatusFilter = "all" | "enabled" | "disabled";
export type SkillSourceFilter = "all" | SkillSource;
export type SkillSort = "name" | "updated" | "source";
export type SkillBindingState = "available" | "configured" | "mounted" | "paused" | "api-prompt";

export interface SkillInventoryFilters {
  category: string;
  query: string;
  source: SkillSourceFilter;
  status: SkillStatusFilter;
  sort: SkillSort;
}

export function skillOverviewQueryKey(input: SkillScopeInput) {
  return ["skill-overview", { scope: input.scope, workspacePath: input.workspacePath ?? null }] as const;
}

export function skillIdentity(skill: Skill) {
  return `${skill.scope}:${skill.workspacePath ?? ""}:${skill.id}`;
}

export function effectiveSkillInventory(skills: Skill[]) {
  const byCanonicalId = new Map<string, Skill>();
  for (const skill of skills) {
    if (!byCanonicalId.has(skill.id)) byCanonicalId.set(skill.id, skill);
  }
  return [...byCanonicalId.values()];
}

export function resolveSessionSkillWorkspace(session: Session | null) {
  return session?.worktreePath ?? session?.projectPath ?? null;
}

export function isSkillAssignedToAgent(
  skill: Skill,
  agent: Pick<SkillCompatibleAgent, "id" | "kind">,
  apiBindingsBySkillId: Record<string, string[]>,
) {
  if (agent.kind === "api") return (apiBindingsBySkillId[skill.id] ?? []).includes(agent.id);
  return skill.boundAgentIds.includes(agent.id)
    || skill.bindings.some((binding) => binding.agentId === agent.id && binding.mounted);
}

export function isSkillAssigned(skill: Skill, overview: Pick<SkillOverview, "agents" | "apiAgentBindings">) {
  return overview.agents.some((agent) => isSkillAssignedToAgent(skill, agent, overview.apiAgentBindings));
}

export function getSkillBindingState(
  skill: Skill,
  agent: Pick<SkillCompatibleAgent, "id" | "kind">,
  apiBindingsBySkillId: Record<string, string[]>,
): SkillBindingState {
  const assigned = isSkillAssignedToAgent(skill, agent, apiBindingsBySkillId);
  if (!assigned) return "available";
  if (!skill.enabled) return "paused";
  if (agent.kind === "api") return "api-prompt";
  return skill.bindings.some((binding) => binding.agentId === agent.id && binding.mounted)
    ? "mounted"
    : "configured";
}

export function partitionSkillsForAgent(
  skills: Skill[],
  agent: Pick<SkillCompatibleAgent, "id" | "kind">,
  apiBindingsBySkillId: Record<string, string[]>,
) {
  const assigned: Skill[] = [];
  const available: Skill[] = [];
  for (const skill of skills) {
    (isSkillAssignedToAgent(skill, agent, apiBindingsBySkillId) ? assigned : available).push(skill);
  }
  return { assigned, available };
}

export function filterGlobalSkillInventory(
  overview: Pick<SkillOverview, "skills" | "agents" | "apiAgentBindings">,
  view: SkillInventoryView,
  filters: SkillInventoryFilters,
  sourceLabels: Partial<Record<SkillSource, string>> = {},
) {
  const needles = filters.query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  const agent = view.kind === "agent"
    ? { id: view.agentId, kind: view.agentKind }
    : null;
  return effectiveSkillInventory(overview.skills)
    .filter((skill) => {
      if (view.kind === "unassigned" && isSkillAssigned(skill, overview)) return false;
      if (agent && !isSkillAssignedToAgent(skill, agent, overview.apiAgentBindings)) return false;
      if (filters.category !== "all" && skill.metadata.category !== filters.category) return false;
      if (filters.source !== "all" && skill.source !== filters.source) return false;
      if (filters.status !== "all" && skill.enabled !== (filters.status === "enabled")) return false;
      const haystack = [
        skill.id,
        skill.metadata.name,
        skill.metadata.description,
        skill.metadata.category,
        ...skill.metadata.triggers,
        skill.source,
        sourceLabels[skill.source] ?? "",
      ].join(" ").toLocaleLowerCase();
      return needles.every((needle) => haystack.includes(needle));
    })
    .sort((left, right) => {
      if (filters.sort === "updated") return right.updatedAt.localeCompare(left.updatedAt);
      if (filters.sort === "source") return left.source.localeCompare(right.source) || left.metadata.name.localeCompare(right.metadata.name);
      return left.metadata.name.localeCompare(right.metadata.name);
    });
}

export function deriveSessionSkillGroups({
  agent,
  globalOverview,
  projectOverview,
}: {
  agent: Pick<SkillCompatibleAgent, "id" | "kind"> | null;
  globalOverview?: SkillOverview;
  projectOverview?: SkillOverview;
}) {
  const assigned = (overview?: SkillOverview) => overview && agent
    ? overview.skills.filter((skill) => isSkillAssignedToAgent(skill, agent, overview.apiAgentBindings))
    : [];
  const global = assigned(globalOverview);
  const project = projectOverview?.skills ?? [];
  const effective = [...global, ...assigned(projectOverview)]
    .filter((skill) => skill.enabled);
  return {
    effective: [...new Map(effective.map((skill) => [skillIdentity(skill), skill])).values()],
    global,
    project,
  };
}
