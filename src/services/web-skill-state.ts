import type {
  Skill,
  SkillAccessRefusalReason,
  SkillAgentMountPath,
  SkillLoadInput,
  SkillLoadOutcome,
  SkillMutationInput,
  SkillScope,
  SkillScopeInput,
} from "../types/skill";
import { normalizeWebPath, normalizeWebSkillLocation } from "./web-skill-location";
import { mockAgents } from "./mock-agent-data";
import { nowIso } from "./web-mock-clock";
import {
  createWebSkillDocumentSeeds,
  createWebSkillMountPathSeeds,
  createWebSkillResourceDocumentSeeds,
  createWebSkillSeeds,
} from "./web-skill-seeds";

/** Mock non-mount Skill-to-API-agent bindings (`add-agent-skill-support`), separate from the CLI
 * mount-path `boundAgentIds` a `Skill` carries. */
export interface WebSkillApiAgentBinding {
  skillId: string;
  scope: SkillScope;
  workspacePath: string | null;
  agentId: string;
}

// Owned here and never exported, for the same reason as the session bindings: an exported mutable
// binding read from two modules can fork into two divergent copies of the mock world.
let webSkills: Skill[] = createWebSkillSeeds();
let webSkillMountPaths: SkillAgentMountPath[] = createWebSkillMountPathSeeds();
let webSkillApiAgentBindings: WebSkillApiAgentBinding[] = [];
let nextWebSkillRevision = 1;
const webSkillDocuments = createWebSkillDocumentSeeds(webSkills);
const webSkillResourceDocuments = createWebSkillResourceDocumentSeeds();
const deletedBuiltinSkillIds = new Set<string>();

export function listWebSkills(): Skill[] {
  return webSkills;
}

export function replaceWebSkills(next: Skill[]): void {
  webSkills = next;
}

export function listWebSkillMountPaths(): SkillAgentMountPath[] {
  return webSkillMountPaths;
}

export function replaceWebSkillMountPaths(next: SkillAgentMountPath[]): void {
  webSkillMountPaths = next;
}

export function listWebSkillApiAgentBindings(): WebSkillApiAgentBinding[] {
  return webSkillApiAgentBindings;
}

export function replaceWebSkillApiAgentBindings(next: WebSkillApiAgentBinding[]): void {
  webSkillApiAgentBindings = next;
}

export function readWebSkillDocument(key: string): string | undefined {
  return webSkillDocuments.get(key);
}

export function writeWebSkillDocument(key: string, body: string): void {
  webSkillDocuments.set(key, body);
}

export function deleteWebSkillDocument(key: string): void {
  webSkillDocuments.delete(key);
}

export function readWebSkillResourceDocument(uri: string): string | undefined {
  return webSkillResourceDocuments.get(uri);
}

export function markWebBuiltinSkillDeleted(skillId: string): void {
  deletedBuiltinSkillIds.add(skillId);
}

export function isWebBuiltinSkillDeleted(skillId: string): boolean {
  return deletedBuiltinSkillIds.has(skillId);
}

export function clearWebBuiltinSkillDeleted(skillId: string): void {
  deletedBuiltinSkillIds.delete(skillId);
}

export function listDeletedWebBuiltinSkillIds(): string[] {
  return [...deletedBuiltinSkillIds];
}

export function skillDocumentKey(skill: Pick<Skill, "id" | "scope" | "workspacePath">): string {
  return `${skill.scope}:${skill.workspacePath ?? ""}:${skill.id}`;
}

export function skillScopeMatches(skill: Skill, input: SkillScopeInput): boolean {
  const location = normalizeWebSkillLocation(input);
  return (
    skill.scope === location.scope &&
    (location.scope === "global" || skill.workspacePath === location.workspacePath)
  );
}

/** Shared by the catalogue and binding clients, so it stays with the skill domain's other
 * cross-cutting helpers rather than being duplicated into both. */
export function requireAgentKind(agentId: string, kind: "cli" | "api"): void {
  const agent = mockAgents.find((candidate) => candidate.id === agentId);
  if (!agent || agent.launch.kind !== kind) {
    throw new Error(`Unknown ${kind.toUpperCase()} Agent id: ${agentId}`);
  }
}

export function mountPathForAgent(agentId: string): string {
  return webSkillMountPaths.find((path) => path.agentId === agentId)?.mountPath ?? ".skills";
}

export function hydrateSkillBindings(skill: Skill): Skill {
  const bindings = skill.boundAgentIds.map((agentId) => {
    const mountPath = mountPathForAgent(agentId);
    const root = skill.scope === "global" ? "~" : (skill.workspacePath ?? ".");
    return {
      agentId,
      mountPath,
      mountedPath: `${root}/${mountPath}/${skill.id}`,
      mounted: skill.enabled,
    };
  });
  return { ...skill, bindings };
}

export function buildSkillContent(skill: Skill): string {
  const triggers = skill.metadata.triggers.map((trigger) => `  - ${trigger}`).join("\n");
  const body = webSkillDocuments.get(skillDocumentKey(skill)) ?? "";
  return `---\nid: ${skill.metadata.id}\nname: ${skill.metadata.name}\ndescription: ${skill.metadata.description}\ncategory: ${skill.metadata.category}\nversion: ${skill.metadata.version}\ntriggers:\n${triggers}\n---\n\n# ${skill.metadata.name}\n\n${body.trim()}\n`;
}

export function webSkillResources(skillId: string) {
  const entries = [...webSkillResourceDocuments.entries()]
    .filter(([uri]) => uri.startsWith(`skill://${skillId}/`))
    .map(([uri, content]) => ({
      uri,
      relativePath: uri.slice(`skill://${skillId}/`.length),
      sizeBytes: new TextEncoder().encode(content).byteLength,
    }));
  const inDirectory = (directory: string) => entries.filter((entry) => entry.relativePath.startsWith(`${directory}/`));
  return {
    scripts: inDirectory("scripts"),
    references: inDirectory("references"),
    templates: inDirectory("templates"),
    assets: inDirectory("assets"),
    truncated: false,
  };
}

export type WebSkillRefusalOutcome = Extract<SkillLoadOutcome, { status: "refused" }>;

function webSkillRefusal(
  requested: string,
  reason: SkillAccessRefusalReason,
  canonicalId: string | null = null,
): WebSkillRefusalOutcome {
  return { status: "refused", refusal: { requested, canonicalId, reason, conflictingIds: [] } };
}

export function findProgressiveWebSkill(input: SkillLoadInput): WebSkillRefusalOutcome | Skill {
  const workspacePath = input.workspacePath ? normalizeWebPath(input.workspacePath, "Workspace path") : null;
  const candidates = webSkills.filter((skill) =>
    skill.scope === "global" || (workspacePath != null && skill.workspacePath === workspacePath),
  );
  const exact = candidates.find((skill) => skill.id === input.idOrAlias);
  const aliases = exact == null
    ? candidates.filter((skill) => skill.metadata.aliases?.includes(input.idOrAlias))
    : [];
  if (aliases.length > 1) {
    return {
      status: "refused",
      refusal: {
        requested: input.idOrAlias,
        canonicalId: null,
        reason: "ambiguous-alias",
        conflictingIds: aliases.map((skill) => skill.id).sort(),
      },
    };
  }
  const skill = exact ?? aliases[0];
  if (!skill) return webSkillRefusal(input.idOrAlias, "not-found");
  if (!skill.enabled) return webSkillRefusal(input.idOrAlias, "disabled", skill.id);
  if (skill.metadata.type === "utility") {
    return webSkillRefusal(input.idOrAlias, "utility-not-loadable", skill.id);
  }
  if (skill.availability !== "available") {
    return webSkillRefusal(input.idOrAlias, skill.availability, skill.id);
  }
  return skill;
}

export function findWebSkill(skillId: string, input: SkillScopeInput): Skill {
  const skill = webSkills.find((candidate) => candidate.id === skillId && skillScopeMatches(candidate, input));
  if (!skill) {
    throw new Error(`Skill not found: ${skillId}`);
  }
  return skill;
}

export function upsertWebSkill(skill: Skill): Skill {
  const index = webSkills.findIndex(
    (candidate) =>
      candidate.id === skill.id &&
      candidate.scope === skill.scope &&
      candidate.workspacePath === skill.workspacePath,
  );
  if (index === -1) {
    webSkills = [...webSkills, skill];
    return skill;
  }
  webSkills = webSkills.map((candidate, candidateIndex) => (candidateIndex === index ? skill : candidate));
  return skill;
}

export function nextWebSkillHash(skillId: string): string {
  const revision = nextWebSkillRevision;
  nextWebSkillRevision += 1;
  return `web-${skillId}-${revision}`;
}

export function mutationToSkill(input: SkillMutationInput): Skill {
  const location = normalizeWebSkillLocation(input);
  const timestamp = nowIso();
  const root = location.scope === "global" ? "~/.vanehub/skills" : `${location.workspacePath}/.vanehub/skills`;
  const skill: Skill = {
    id: input.id,
    scope: location.scope,
    workspacePath: location.workspacePath ?? null,
    source: input.source ?? "user",
    enabled: input.enabled,
    skillDir: `${root}/${input.id}`,
    skillMdPath: `${root}/${input.id}/SKILL.md`,
    contentHash: nextWebSkillHash(input.id),
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
    boundAgentIds: [...input.boundAgentIds],
    bindings: [],
    createdAt: timestamp,
    updatedAt: timestamp,
    layer: location.scope === "workspace" ? "project" : "user",
    origin: input.source === "imported" ? "imported" : "created",
    trust: input.source === "imported" ? "untrusted" : "trusted",
    availability: input.metadata.type === "utility" ? "unsupported" : "available",
    delegationCapability: input.metadata.type === "utility"
      ? { supported: false, reason: "native-runtime-unavailable" }
      : { supported: false, reason: "not-utility" },
    immutable: false,
    shadowedDefinitions: [],
    usage: {
      viewCount: 0,
      useCount: 0,
      lastViewedAt: null,
      lastUsedAt: null,
      revisionWitness: "web-usage-1",
    },
  };
  webSkillDocuments.set(skillDocumentKey(skill), input.body);
  return skill;
}
