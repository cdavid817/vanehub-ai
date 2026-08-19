import type { SkillBindingService } from "./skill-service";
import type {
  SkillLoadInput,
  SkillLoadOutcome,
  SkillPreview,
  SkillResourceReadInput,
  SkillResourceReadOutcome,
} from "../types/skill";
import { nowIso } from "./web-mock-clock";
import {
  buildSkillContent,
  findProgressiveWebSkill,
  findWebSkill,
  hydrateSkillBindings,
  listWebSkillApiAgentBindings,
  readWebSkillDocument,
  readWebSkillResourceDocument,
  replaceWebSkillApiAgentBindings,
  requireAgentKind,
  skillDocumentKey,
  upsertWebSkill,
  webSkillResources,
} from "./web-skill-state";

export const webSkillBindingClient: SkillBindingService = {
  async setSkillEnabled(skillId, input, enabled) {
    const current = findWebSkill(skillId, input);
    const availability = !enabled
      ? "disabled" as const
      : current.metadata.type === "utility" ? "unsupported" as const : "available" as const;
    const updated = { ...current, enabled, availability, updatedAt: nowIso() };
    return hydrateSkillBindings(upsertWebSkill(updated));
  },

  async setSkillAgentBindings(skillId, input, agentIds) {
    for (const agentId of agentIds) requireAgentKind(agentId, "cli");
    const current = findWebSkill(skillId, input);
    const updated = { ...current, boundAgentIds: [...agentIds], updatedAt: nowIso() };
    return hydrateSkillBindings(upsertWebSkill(updated));
  },

  async bindSkillToCliAgent(skillId, input, agentId) {
    requireAgentKind(agentId, "cli");
    const current = findWebSkill(skillId, input);
    if (current.boundAgentIds.includes(agentId)) return hydrateSkillBindings(current);
    const updated = {
      ...current,
      boundAgentIds: [...current.boundAgentIds, agentId].sort(),
      updatedAt: nowIso(),
    };
    return hydrateSkillBindings(upsertWebSkill(updated));
  },

  async unbindSkillFromCliAgent(skillId, input, agentId) {
    requireAgentKind(agentId, "cli");
    const current = findWebSkill(skillId, input);
    const updated = {
      ...current,
      boundAgentIds: current.boundAgentIds.filter((id) => id !== agentId),
      updatedAt: nowIso(),
    };
    return hydrateSkillBindings(upsertWebSkill(updated));
  },

  async bindSkillToApiAgent(skillId, input, agentId) {
    requireAgentKind(agentId, "api");
    const skill = findWebSkill(skillId, input);
    const alreadyBound = listWebSkillApiAgentBindings().some(
      (binding) =>
        binding.skillId === skill.id &&
        binding.scope === skill.scope &&
        binding.workspacePath === skill.workspacePath &&
        binding.agentId === agentId,
    );
    if (!alreadyBound) {
      replaceWebSkillApiAgentBindings([
        ...listWebSkillApiAgentBindings(),
        { skillId: skill.id, scope: skill.scope, workspacePath: skill.workspacePath, agentId },
      ]);
    }
  },

  async unbindSkillFromApiAgent(skillId, input, agentId) {
    requireAgentKind(agentId, "api");
    const skill = findWebSkill(skillId, input);
    replaceWebSkillApiAgentBindings(listWebSkillApiAgentBindings().filter(
      (binding) =>
        !(
          binding.skillId === skill.id &&
          binding.scope === skill.scope &&
          binding.workspacePath === skill.workspacePath &&
          binding.agentId === agentId
        ),
    ));
  },

  async listSkillApiAgentBindings(skillId, input) {
    const skill = findWebSkill(skillId, input);
    return listWebSkillApiAgentBindings()
      .filter(
        (binding) =>
          binding.skillId === skill.id &&
          binding.scope === skill.scope &&
          binding.workspacePath === skill.workspacePath,
      )
      .map((binding) => binding.agentId);
  },

  async previewSkill(skillId, input): Promise<SkillPreview> {
    const skill = hydrateSkillBindings(findWebSkill(skillId, input));
    return {
      id: skill.id,
      scope: skill.scope,
      workspacePath: skill.workspacePath,
      path: skill.skillMdPath,
      content: buildSkillContent(skill),
      layer: skill.layer,
      origin: skill.origin,
      availability: skill.availability,
      immutable: skill.immutable,
      shadowedDefinitions: skill.shadowedDefinitions.map((definition) => ({ ...definition })),
    };
  },

  async loadSkill(input: SkillLoadInput): Promise<SkillLoadOutcome> {
    const resolved = findProgressiveWebSkill(input);
    if ("status" in resolved) return resolved;
    const body = readWebSkillDocument(skillDocumentKey(resolved)) ?? "";
    const baseUri = `skill://${resolved.id}/`;
    const expanded = body.replaceAll("{skill_base_dir}", baseUri);
    const characters = [...expanded];
    const timestamp = nowIso();
    upsertWebSkill({
      ...resolved,
      usage: {
        ...resolved.usage,
        viewCount: resolved.usage.viewCount + 1,
        lastViewedAt: timestamp,
        revisionWitness: `${resolved.usage.revisionWitness ?? "web-usage"}-view`,
      },
    });
    return {
      status: "loaded",
      result: {
        id: resolved.id,
        name: resolved.metadata.name,
        content: characters.slice(0, 12_000).join(""),
        truncated: characters.length > 12_000,
        revision: resolved.contentHash,
        baseUri,
        resources: webSkillResources(resolved.id),
      },
    };
  },

  async readSkillResource(input: SkillResourceReadInput): Promise<SkillResourceReadOutcome> {
    const match = /^skill:\/\/([a-z0-9]+(?:-[a-z0-9]+)*)\/(.+)$/.exec(input.uri);
    if (!match) {
      return {
        status: "refused",
        refusal: { requested: input.uri, canonicalId: null, reason: "invalid-uri", conflictingIds: [] },
      };
    }
    const skillId = match[1];
    const resolved = findProgressiveWebSkill({ idOrAlias: skillId, workspacePath: input.workspacePath });
    if ("status" in resolved) return resolved;
    if (resolved.contentHash !== input.revision) {
      return {
        status: "refused",
        refusal: { requested: input.uri, canonicalId: skillId, reason: "stale-revision", conflictingIds: [] },
      };
    }
    const content = readWebSkillResourceDocument(input.uri);
    if (content == null || !webSkillResources(skillId).references.concat(
      webSkillResources(skillId).templates,
      webSkillResources(skillId).scripts,
      webSkillResources(skillId).assets,
    ).some((entry) => entry.uri === input.uri)) {
      return {
        status: "refused",
        refusal: { requested: input.uri, canonicalId: skillId, reason: "unindexed-resource", conflictingIds: [] },
      };
    }
    return {
      status: "read",
      result: {
        id: skillId,
        uri: input.uri,
        revision: input.revision,
        content,
        sizeBytes: new TextEncoder().encode(content).byteLength,
      },
    };
  },
};
