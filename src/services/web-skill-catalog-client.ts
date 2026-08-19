import { i18n } from "../i18n";
import type { AgentService } from "./agent-service";
import type { SkillCatalogService } from "./skill-service";
import type {
  Skill,
  SkillImportInput,
  SkillListResult,
  SkillMetadata,
  SkillMountMigrationReport,
  SkillMutationInput,
  SkillOverview,
  SkillSource,
  SkillUpdateInput,
} from "../types/skill";
import { mockAgents } from "./mock-agent-data";
import { nowIso } from "./web-mock-clock";
import { normalizeWebPath, normalizeWebSkillLocation } from "./web-skill-location";
import { webBuiltinSkillSeeds } from "./web-skill-seeds";
import {
  clearWebBuiltinSkillDeleted,
  deleteWebSkillDocument,
  findWebSkill,
  hydrateSkillBindings,
  isWebBuiltinSkillDeleted,
  listDeletedWebBuiltinSkillIds,
  listWebSkillApiAgentBindings,
  listWebSkillMountPaths,
  listWebSkills,
  markWebBuiltinSkillDeleted,
  mountPathForAgent,
  mutationToSkill,
  nextWebSkillHash,
  replaceWebSkillApiAgentBindings,
  replaceWebSkillMountPaths,
  replaceWebSkills,
  requireAgentKind,
  skillDocumentKey,
  skillScopeMatches,
  upsertWebSkill,
  writeWebSkillDocument,
} from "./web-skill-state";

function validateWebSkillMetadata(metadata: SkillMetadata) {
  if (!/^(?!-)[a-z0-9-]+(?<!-)$/.test(metadata.id)) {
    throw new Error("Skill id must be kebab-case letters, digits, and hyphens");
  }
  if (![metadata.name, metadata.description, metadata.category, metadata.version].every((value) => value.trim())) {
    throw new Error("Skill metadata name, description, category, and version are required");
  }
}

function validateWebSkillMutation(input: SkillMutationInput, allowedSource: SkillSource) {
  validateWebSkillMetadata(input.metadata);
  if (input.id !== input.metadata.id) throw new Error("Skill request id must match metadata id");
  if ((input.source ?? "user") !== allowedSource) {
    throw new Error(`Skill source ${input.source ?? "user"} is invalid for this operation`);
  }
  return { ...input, ...normalizeWebSkillLocation(input), source: allowedSource };
}

function webPathsOverlap(left: string, right: string) {
  const comparable = (value: string) => (/^[a-zA-Z]:/.test(value) ? value.toLocaleLowerCase() : value);
  const leftPath = comparable(left);
  const rightPath = comparable(right);
  return leftPath === rightPath || leftPath.startsWith(`${rightPath}/`) || rightPath.startsWith(`${leftPath}/`);
}

function validateMountPath(mountPath: string) {
  const normalized = mountPath.trim().replaceAll("\\", "/");
  const segments = normalized.split("/");
  if (
    !normalized ||
    normalized.startsWith("/") ||
    /^[a-zA-Z]:/.test(normalized) ||
    segments.some((segment) => !segment || segment === "." || segment === "..") ||
    segments[0]?.toLocaleLowerCase() === ".vanehub"
  ) {
    throw new Error(`Invalid Skill mount path: ${mountPath}`);
  }
  return normalized;
}

function skillStats(skills: Skill[]) {
  return {
    total: skills.length,
    enabled: skills.filter((skill) => skill.enabled).length,
    mounted: skills.filter((skill) => skill.enabled && skill.boundAgentIds.length > 0).length,
  };
}

export const webSkillCatalogClient: SkillCatalogService = {
  async listSkills(input): Promise<SkillListResult> {
    const skills = listWebSkills().filter((skill) => skillScopeMatches(skill, input)).map(hydrateSkillBindings);
    return { skills, stats: skillStats(skills) };
  },

  async getSkillOverview(this: AgentService, input): Promise<SkillOverview> {
    const { skills, stats } = await this.listSkills(input);
    const apiAgentBindings = Object.fromEntries(
      skills.map((skill) => [
        skill.id,
        listWebSkillApiAgentBindings()
          .filter(
            (binding) =>
              binding.skillId === skill.id &&
              binding.scope === skill.scope &&
              binding.workspacePath === skill.workspacePath,
          )
          .map((binding) => binding.agentId),
      ]),
    );
    return {
      skills,
      stats,
      mountPaths: listWebSkillMountPaths().map((path) => ({ ...path })),
      agents: mockAgents.map((agent) => ({
        id: agent.id,
        displayName: agent.displayName,
        kind: agent.launch.kind === "api" ? "api" : "cli",
      })),
      apiAgentBindings,
      drift: await this.detectSkillDrift(input),
      restoreCandidates: input.scope === "global" ? listDeletedWebBuiltinSkillIds().sort() : [],
    };
  },

  async listSkillMountPaths() {
    return listWebSkillMountPaths().map((path) => ({ ...path }));
  },

  async updateSkillMountPath(agentId: string, mountPath: string): Promise<SkillMountMigrationReport> {
    requireAgentKind(agentId, "cli");
    mountPath = validateMountPath(mountPath);
    const existing = listWebSkillMountPaths().find((path) => path.agentId === agentId);
    const oldMountPath = existing?.mountPath ?? mountPathForAgent(agentId);
    replaceWebSkillMountPaths(listWebSkillMountPaths().map((path) =>
      path.agentId === agentId ? { agentId, mountPath, isDefault: false } : path,
    ));
    if (!existing) {
      replaceWebSkillMountPaths([...listWebSkillMountPaths(), { agentId, mountPath, isDefault: false }]);
    }
    const migrated = listWebSkills()
      .filter((skill) => skill.boundAgentIds.includes(agentId) && skill.enabled)
      .map((skill) => skill.id);
    return {
      agentId,
      oldMountPath,
      newMountPath: mountPath,
      migrated,
      removed: migrated.map((skillId) => `${oldMountPath}/${skillId}`),
      overwritten: [],
      backedUp: [],
      failed: [],
    };
  },

  async createSkill(input) {
    const normalized = validateWebSkillMutation(input, "user");
    if (listWebSkills().some((skill) => skill.id === normalized.id && skillScopeMatches(skill, normalized))) {
      throw new Error(`Skill already exists: ${normalized.id}`);
    }
    for (const agentId of normalized.boundAgentIds) requireAgentKind(agentId, "cli");
    const skill = mutationToSkill(normalized);
    return hydrateSkillBindings(upsertWebSkill(skill));
  },

  async updateSkill(skillId, input: SkillUpdateInput) {
    validateWebSkillMetadata(input.metadata);
    if (input.metadata.id !== skillId) {
      throw new Error(i18n.t("web.error.skillIdImmutable"));
    }
    const current = findWebSkill(skillId, input);
    if (current.contentHash !== input.expectedContentHash) {
      throw new Error(`Skill changed since it was loaded: ${skillId}`);
    }
    writeWebSkillDocument(skillDocumentKey(current), input.body);
    const updated: Skill = {
      ...current,
      metadata: {
        ...input.metadata,
        aliases: input.metadata.aliases ?? [],
        type: input.metadata.type ?? "role",
        delivery: input.metadata.delivery ?? "eager",
        compatibilityDefaults: input.metadata.compatibilityDefaults ?? {
          skillType: input.metadata.type == null,
          delivery: input.metadata.delivery == null,
        },
      },
      availability: input.metadata.type === "utility" ? "unsupported" : "available",
      delegationCapability: input.metadata.type === "utility"
        ? { supported: false, reason: "native-runtime-unavailable" }
        : { supported: false, reason: "not-utility" },
      contentHash: nextWebSkillHash(skillId),
      updatedAt: nowIso(),
    };
    return hydrateSkillBindings(upsertWebSkill(updated));
  },

  async deleteSkill(skillId, input) {
    const current = findWebSkill(skillId, input);
    if (current.source === "builtin") {
      markWebBuiltinSkillDeleted(skillId);
    }
    replaceWebSkills(listWebSkills().filter((skill) => !(skill.id === skillId && skillScopeMatches(skill, input))));
    deleteWebSkillDocument(skillDocumentKey(current));
    replaceWebSkillApiAgentBindings(listWebSkillApiAgentBindings().filter(
      (binding) =>
        !(
          binding.skillId === current.id &&
          binding.scope === current.scope &&
          binding.workspacePath === current.workspacePath
        ),
    ));
  },

  async restoreBuiltinSkill(skillId) {
    const seed = webBuiltinSkillSeeds.find((candidate) => candidate.id === skillId);
    if (!seed) {
      throw new Error(`Unknown built-in Skill: ${skillId}`);
    }
    if (!isWebBuiltinSkillDeleted(skillId)) {
      throw new Error(`Built-in Skill is not eligible for restore: ${skillId}`);
    }
    if (listWebSkills().some((skill) => skill.id === skillId && skill.scope === "global")) {
      throw new Error(`Skill already exists: ${skillId}`);
    }
    clearWebBuiltinSkillDeleted(skillId);
    const restored = {
      ...mutationToSkill({
      id: seed.id,
      scope: "global",
      workspacePath: null,
      metadata: {
        id: seed.id,
        name: seed.name,
        description: seed.description,
        category: seed.category,
        version: "1.0.0",
        triggers: seed.triggers,
      },
      body: `Web mock restored content for ${seed.id}.`,
      enabled: true,
      boundAgentIds: [],
      source: "builtin",
      }),
      layer: "system" as const,
      origin: "shipped" as const,
      immutable: true,
    };
    return hydrateSkillBindings(upsertWebSkill(restored));
  },

  async importSkill(input: SkillImportInput) {
    const sourcePath = normalizeWebPath(input.sourcePath, "External Skill directory");
    const id = sourcePath.split("/").at(-1) ?? "";
    const location = normalizeWebSkillLocation(input);
    const destinationRoot = location.scope === "global"
      ? "~/.vanehub/skills"
      : `${location.workspacePath}/.vanehub/skills`;
    const destination = normalizeWebPath(`${destinationRoot}/${id}`, "Managed Skill destination");
    if (webPathsOverlap(sourcePath, destination)) {
      throw new Error("External Skill source overlaps the managed Skill destination");
    }
    const mutation = validateWebSkillMutation({
      id,
      scope: location.scope,
      workspacePath: location.workspacePath,
      metadata: {
        id,
        name: id,
        description: i18n.t("web.skill.importedDescription"),
        category: "imported",
        version: "1.0.0",
        triggers: [],
      },
      body: i18n.t("web.skill.importedBody"),
      enabled: input.enabled,
      boundAgentIds: input.boundAgentIds,
      source: "imported",
    }, "imported");
    if (listWebSkills().some((skill) => skill.id === id && skillScopeMatches(skill, mutation))) {
      throw new Error(`Skill already exists: ${id}`);
    }
    for (const agentId of mutation.boundAgentIds) requireAgentKind(agentId, "cli");
    return hydrateSkillBindings(upsertWebSkill(mutationToSkill(mutation)));
  },
};
