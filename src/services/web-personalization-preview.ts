import type {
  EffectivePreview,
  EffectivePreviewInput,
  ExcludedSegment,
  MemoryDelivery,
  MemoryExclusion,
  MemoryExclusionReason,
  PersonalizationPolicy,
  PersonalizationWarning,
  PolicyToggle,
  PreviewSegment,
  RecallAvailability,
} from "../types/personalization";
import type { MemoryDetail } from "../types/personalization-memory";
import { policyScopeKey } from "./web-personalization-state";
import { MOCK_AGENT_CAPABILITIES } from "./web-personalization-rules";

/** The injection page bound the native snapshot applies; recall considers the whole eligible set. */
export const WEB_INJECTION_REF_LIMIT = 200;

/**
 * What the preview command decides beside the resolution: whether the session id named a stored
 * session, and whether an embedding source is configured. Both are facts the mock has to be told.
 */
export interface PreviewChannelOptions {
  boundSession?: boolean;
  retrievalConfigured?: boolean;
}

interface ResolvedReadPolicy {
  read: boolean;
  explicitSave: boolean;
  automaticExtraction: boolean;
  globalMemory: boolean;
}

/**
 * The same layering the native resolver applies: built-in safe defaults, then global, Agent,
 * workspace and workspace-Agent overrides in that order, each `inherit` leaving the lower layer's
 * answer alone. The session mode is applied last and can only narrow.
 */
function resolveReadPolicy(
  input: EffectivePreviewInput,
  policies: PersonalizationPolicy[],
): ResolvedReadPolicy {
  const layers = [
    { scopeKind: "global" as const },
    { scopeKind: "agent" as const, agentId: input.agentId },
    ...(input.workspaceKey
      ? [
          { scopeKind: "workspace" as const, workspaceKey: input.workspaceKey },
          { scopeKind: "workspace-agent" as const, workspaceKey: input.workspaceKey, agentId: input.agentId },
        ]
      : []),
  ];
  const resolved: ResolvedReadPolicy = {
    read: true,
    explicitSave: true,
    automaticExtraction: true,
    globalMemory: true,
  };
  const apply = (current: boolean, toggle: PolicyToggle | undefined): boolean =>
    toggle === undefined || toggle === "inherit" ? current : toggle === "enabled";
  for (const layer of layers) {
    const policy = policies.find(
      (entry) => entry.scopeKind === layer.scopeKind && entry.scopeKey === policyScopeKey(layer),
    );
    if (!policy) continue;
    resolved.read = apply(resolved.read, policy.memoryReadMode);
    resolved.explicitSave = apply(resolved.explicitSave, policy.explicitSaveMode);
    resolved.automaticExtraction = apply(resolved.automaticExtraction, policy.automaticExtractionMode);
    resolved.globalMemory = apply(resolved.globalMemory, policy.globalMemoryAccessMode);
  }
  return resolved;
}

/**
 * What the mock would apply for one Agent in one session.
 *
 * Honours the whole read-related policy stack, the session mode as a hard narrowing, exact Agent
 * audience membership, and the difference between "may read but nothing is eligible" and "may not
 * read" -- the last being what the previous mock got wrong by inferring the switch from the count.
 * Everything here is simulated over local mock state; nothing is read from disk or the network.
 */
export function previewFor(
  input: EffectivePreviewInput,
  policies: PersonalizationPolicy[],
  memories: MemoryDetail[],
  options: PreviewChannelOptions = {},
): EffectivePreview {
  const capability = MOCK_AGENT_CAPABILITIES.find((entry) => entry.agentId === input.agentId);
  const warnings: PersonalizationWarning[] = capability ? [] : ["unknown-agent"];
  const mode = input.sessionMode ?? "standard";
  const globalPolicy = policies.find((entry) => entry.scopeKind === "global") ?? null;
  const policy = resolveReadPolicy(input, policies);
  const included: PreviewSegment[] = [];
  const excluded: ExcludedSegment[] = [];

  for (const field of ["about_user", "style_rules"] as const) {
    const text = field === "about_user" ? (globalPolicy?.aboutUser ?? "") : (globalPolicy?.styleRules ?? "");
    if (!capability?.supportsCustomInstructions) {
      excluded.push({ field, scopeKind: "global", scopeKey: "", reason: "runtime_capability" });
    } else if (!text) {
      excluded.push({ field, scopeKind: "global", scopeKey: "", reason: "empty_field" });
    } else {
      included.push({
        field,
        scopeKind: "global",
        scopeKey: "",
        policyRevision: globalPolicy?.revision ?? 0,
        mergeAction: globalPolicy?.instructionMergeMode === "replace" ? "replaced" : "appended",
        redactedText: text,
        characters: text.length,
      });
    }
  }

  // Why nothing may be read, when nothing may: the first restriction to fire is the one a user
  // has to change, so the order is the resolver's -- registry, then mode, then capability, then
  // policy.
  const readBlockReason = !capability
    ? "no_validated_policy"
    : mode === "temporary"
      ? "session_mode"
      : !capability.supportsMemoryIndex
        ? "runtime_capability"
        : !policy.read
          ? "read_disabled"
          : null;
  const readAllowed = readBlockReason === null;

  const exclusions = new Map<MemoryExclusionReason, number>();
  const count = (reason: MemoryExclusionReason) => {
    exclusions.set(reason, (exclusions.get(reason) ?? 0) + 1);
  };

  // One outcome per record, decided in the native order: lifecycle, then the read switch, then
  // scope, then exact audience. Provenance never enters.
  const eligible = memories.filter((memory) => {
    if (memory.status === "candidate") return count("pending_candidate"), false;
    if (memory.status === "archived") return count("archived"), false;
    if (memory.scopeKind !== "global" && memory.scopeKind !== "workspace") {
      return count("invalid_record"), false;
    }
    if (!capability) return count("unsafe_maintenance_state"), false;
    if (mode === "temporary") return count("temporary_session"), false;
    if (!capability.supportsMemoryIndex) return count("runtime_capability"), false;
    if (!policy.read) return count("memory_read_disabled"), false;
    if (memory.scopeKind === "global" && mode === "project-only") {
      return count("project_only_session"), false;
    }
    if (memory.scopeKind === "global" && !policy.globalMemory) {
      return count("global_memory_disabled"), false;
    }
    if (memory.scopeKind === "workspace" && (!input.workspaceKey || memory.workspaceKey !== input.workspaceKey)) {
      return count("other_workspace"), false;
    }
    if (memory.audienceAgentIds && !memory.audienceAgentIds.includes(input.agentId)) {
      return count("agent_audience"), false;
    }
    return true;
  });

  const memoryExclusions: MemoryExclusion[] = [...exclusions].map(([reason, value]) => ({
    reason,
    count: value,
  }));
  const indexEntryCount = Math.min(eligible.length, WEB_INJECTION_REF_LIMIT);
  const knownCharacters =
    included.reduce((total, segment) => total + segment.characters, 0)
    + eligible
      .slice(0, indexEntryCount)
      .reduce((total, memory) => total + memory.name.length + memory.description.length, 0);

  return {
    revisionToken: `${globalPolicy?.revision ?? 0}:${input.agentId}:${mode}:web-mock`,
    previewKind: options.boundSession ? "bound_session" : "hypothetical",
    instructionMode: globalPolicy?.instructionMergeMode ?? "inherit",
    includedInstructions: included,
    excludedInstructions: excluded,
    memoryDelivery: deliveryFor(readAllowed, capability?.supportsSelectedMemoryBodies),
    memoryRead: readAllowed,
    memoryReadAllowed: readAllowed,
    readBlockReason,
    explicitSave: readAllowed && policy.explicitSave,
    automaticExtraction:
      mode !== "temporary"
      && policy.automaticExtraction
      && (capability?.supportsAutomaticExtraction ?? false),
    candidateCreation: mode !== "temporary" && capability !== undefined,
    retrievalWrite: mode === "standard" && capability !== undefined,
    eligibleMemoryCount: eligible.length,
    consideredMemoryCount: memories.length,
    indexEntryCount,
    indexTruncated: eligible.length > indexEntryCount,
    recallAvailability: recallAvailabilityFor(
      readAllowed,
      capability?.supportsSelectedMemoryBodies ?? false,
      options.retrievalConfigured ?? false,
    ),
    memoryExclusions,
    warnings,
    approximateTokens: Math.ceil(knownCharacters / 4),
    knownCharacters,
    selectedBodyBudgetMax: 5,
    excludedSurfaces: ["cli_internal_context"],
    estimatorVersion: "web-mock-2",
    // VaneHub never manages a CLI's own compaction, on either runtime.
    cliInternalCompactionManaged: false,
  };
}

/** Capability-driven, never count-driven: an allowed empty pool still delivers an (empty) index. */
function deliveryFor(readAllowed: boolean, supportsBodies: boolean | undefined): MemoryDelivery {
  if (!readAllowed) return "none";
  return supportsBodies ? "index_with_selected_bodies" : "index_only";
}

/**
 * The five answers the native command gives, in its order: no reading means no channel; an
 * index-only runtime never has one; then the embedder configuration. The mock has no health state,
 * so `unavailable` is never produced here.
 */
function recallAvailabilityFor(
  readAllowed: boolean,
  supportsRecallChannel: boolean,
  retrievalConfigured: boolean,
): RecallAvailability {
  if (!readAllowed) return "disabled";
  if (!supportsRecallChannel) return "unsupported";
  if (!retrievalConfigured) return "unconfigured";
  return "available";
}
