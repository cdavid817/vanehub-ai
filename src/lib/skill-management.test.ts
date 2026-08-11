import { describe, expect, it } from "vitest";
import type { Session } from "../types/agent";
import type { Skill, SkillOverview } from "../types/skill";
import {
  deriveSessionSkillGroups,
  effectiveSkillInventory,
  filterGlobalSkillInventory,
  getSkillBindingState,
  isSkillAssignedToAgent,
  partitionSkillsForAgent,
  resolveSessionSkillWorkspace,
  skillIdentity,
} from "./skill-management";

function makeSkill(id: string, scope: Skill["scope"], boundAgentIds: string[] = []): Skill {
  return {
    id,
    scope,
    workspacePath: scope === "workspace" ? "D:/project" : null,
    source: "user",
    enabled: true,
    skillDir: id,
    skillMdPath: `${id}/SKILL.md`,
    contentHash: id,
    metadata: { id, name: id, description: id, category: "dev", version: "1", triggers: [] },
    boundAgentIds,
    bindings: [],
    createdAt: "2026-01-01",
    updatedAt: "2026-01-01",
    layer: scope === "workspace" ? "project" : "user",
    origin: "created",
    trust: "trusted",
    availability: "available",
    immutable: false,
    shadowedDefinitions: [],
    usage: { viewCount: 0, useCount: 0, lastViewedAt: null, lastUsedAt: null, revisionWitness: null },
  };
}

function overview(skills: Skill[]): SkillOverview {
  return {
    skills,
    stats: { total: skills.length, enabled: skills.length, mounted: 0 },
    agents: [
      { id: "codex-cli", displayName: "Codex", kind: "cli" },
      { id: "api-one", displayName: "API", kind: "api" },
    ],
    apiAgentBindings: { api: ["api-one"] },
    mountPaths: [],
    drift: { scope: skills[0]?.scope ?? "global", workspacePath: null, issues: [], driftHash: "clean" },
    restoreCandidates: [],
  };
}

describe("skill management presentation", () => {
  it("keeps the first effective row for each canonical Skill id", () => {
    const effective = { ...makeSkill("same", "global"), layer: "user" as const };
    const duplicate = { ...makeSkill("same", "global"), layer: "system" as const, immutable: true };

    expect(effectiveSkillInventory([effective, duplicate])).toEqual([effective]);
    expect(filterGlobalSkillInventory(
      overview([effective, duplicate]),
      { kind: "all" },
      { category: "all", query: "", sort: "name", source: "all", status: "all" },
    )).toEqual([effective]);
  });
  it("keeps identical ids distinct by scope and resolves worktree before project", () => {
    expect(skillIdentity(makeSkill("same", "global"))).not.toBe(skillIdentity(makeSkill("same", "workspace")));
    const session = { worktreePath: "D:/worktree", projectPath: "D:/project" } as Session;
    expect(resolveSessionSkillWorkspace(session)).toBe("D:/worktree");
    expect(resolveSessionSkillWorkspace({ ...session, worktreePath: null })).toBe("D:/project");
  });

  it("uses CLI mount bindings and API prompt bindings independently", () => {
    const cli = makeSkill("cli", "global", ["codex-cli"]);
    const api = makeSkill("api", "global");
    const data = overview([cli, api]);
    expect(isSkillAssignedToAgent(cli, data.agents[0], data.apiAgentBindings)).toBe(true);
    expect(isSkillAssignedToAgent(api, data.agents[1], data.apiAgentBindings)).toBe(true);
    expect(getSkillBindingState(cli, data.agents[0], data.apiAgentBindings)).toBe("configured");
    expect(getSkillBindingState(api, data.agents[1], data.apiAgentBindings)).toBe("api-prompt");
    expect(partitionSkillsForAgent(data.skills, data.agents[0], data.apiAgentBindings)).toEqual({ assigned: [cli], available: [api] });
    expect(filterGlobalSkillInventory(data, { kind: "unassigned" }, {
      category: "all", query: "", sort: "name", source: "all", status: "all",
    })).toEqual([]);
  });

  it("derives mounted and paused states for CLI and API assignments", () => {
    const mounted = {
      ...makeSkill("mounted", "global", ["codex-cli"]),
      bindings: [{
        agentId: "codex-cli",
        mountPath: ".codex/skills",
        mountedPath: ".codex/skills/mounted",
        mounted: true,
      }],
    } satisfies Skill;
    const pausedCli = { ...mounted, id: "paused-cli", enabled: false } satisfies Skill;
    const pausedApi = { ...makeSkill("api", "global"), enabled: false } satisfies Skill;
    const data = overview([mounted, pausedCli, pausedApi]);

    expect(getSkillBindingState(mounted, data.agents[0], data.apiAgentBindings)).toBe("mounted");
    expect(getSkillBindingState(pausedCli, data.agents[0], data.apiAgentBindings)).toBe("paused");
    expect(getSkillBindingState(pausedApi, data.agents[1], data.apiAgentBindings)).toBe("paused");
  });

  it("derives effective, global and complete project groups", () => {
    const global = overview([makeSkill("global", "global", ["codex-cli"])]);
    const project = overview([makeSkill("assigned", "workspace", ["codex-cli"]), makeSkill("other", "workspace")]);
    const groups = deriveSessionSkillGroups({ agent: global.agents[0], globalOverview: global, projectOverview: project });
    expect(groups.effective.map((skill) => skill.id)).toEqual(["global", "assigned"]);
    expect(groups.global.map((skill) => skill.id)).toEqual(["global"]);
    expect(groups.project.map((skill) => skill.id)).toEqual(["assigned", "other"]);
  });

  it("preserves same-id Skills from global and project scopes", () => {
    const global = overview([makeSkill("same", "global", ["codex-cli"])]);
    const project = overview([makeSkill("same", "workspace", ["codex-cli"])]);
    const groups = deriveSessionSkillGroups({ agent: global.agents[0], globalOverview: global, projectOverview: project });
    expect(groups.effective.map(skillIdentity)).toEqual(["global::same", "workspace:D:/project:same"]);
  });

  it("matches a localized source label supplied by the presentation layer", () => {
    const builtin = { ...makeSkill("builtin", "global"), source: "builtin" as const };
    const data = overview([builtin]);
    expect(filterGlobalSkillInventory(data, { kind: "all" }, {
      category: "all", query: "Built-in", sort: "name", source: "all", status: "all",
    }, { builtin: "Built-in" })).toEqual([builtin]);
  });
});
