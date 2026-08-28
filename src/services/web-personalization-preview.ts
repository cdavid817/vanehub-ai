import type {
  EffectivePreview,
  EffectivePreviewInput,
  ExcludedSegment,
  MemoryDelivery,
  MemoryExclusion,
  MemoryExclusionReason,
  PersonalizationPolicy,
  PersonalizationWarning,
  PreviewSegment,
} from "../types/personalization";
import type { MemoryDetail } from "../types/personalization-memory";
import { MOCK_AGENT_CAPABILITIES } from "./web-personalization-rules";

/**
 * What the mock would apply for one Agent in one session.
 *
 * The session mode is honoured rather than reported: a mock that returned the same memory count
 * for `temporary` as for `standard` would let a screen ship a mode selector that changes nothing,
 * and the discrepancy would only appear on a real desktop.
 */
export function previewFor(
  input: EffectivePreviewInput,
  policy: PersonalizationPolicy | null,
  memories: MemoryDetail[],
): EffectivePreview {
  const capability = MOCK_AGENT_CAPABILITIES.find((entry) => entry.agentId === input.agentId);
  const warnings: PersonalizationWarning[] = capability ? [] : ["unknown-agent"];
  const mode = input.sessionMode ?? "standard";
  const included: PreviewSegment[] = [];
  const excluded: ExcludedSegment[] = [];

  for (const field of ["about_user", "style_rules"] as const) {
    const text = field === "about_user" ? (policy?.aboutUser ?? "") : (policy?.styleRules ?? "");
    if (!capability?.supportsCustomInstructions) {
      excluded.push({ field, scopeKind: "global", scopeKey: "", reason: "runtime_capability" });
    } else if (!text) {
      excluded.push({ field, scopeKind: "global", scopeKey: "", reason: "empty_field" });
    } else {
      included.push({
        field,
        scopeKind: "global",
        scopeKey: "",
        policyRevision: policy?.revision ?? 0,
        mergeAction: policy?.instructionMergeMode === "replace" ? "replaced" : "appended",
        redactedText: text,
        characters: text.length,
      });
    }
  }

  const considered = memories.filter((memory) => memory.status !== "candidate");
  const exclusions = new Map<MemoryExclusionReason, number>();
  const count = (reason: MemoryExclusionReason) => {
    exclusions.set(reason, (exclusions.get(reason) ?? 0) + 1);
  };

  const eligible = considered.filter((memory) => {
    if (!capability?.supportsMemoryIndex) return count("runtime_capability"), false;
    if (mode === "temporary") return count("temporary_session"), false;
    if (memory.status === "archived") return count("archived"), false;
    if (mode === "project-only" && memory.scopeKind === "global") {
      return count("project_only_session"), false;
    }
    if (memory.workspaceKey && memory.workspaceKey !== input.workspaceKey) {
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
  const knownCharacters =
    included.reduce((total, segment) => total + segment.characters, 0)
    + eligible.reduce((total, memory) => total + memory.name.length + memory.description.length, 0);

  return {
    revisionToken: `${policy?.revision ?? 0}:${input.agentId}:${mode}`,
    instructionMode: policy?.instructionMergeMode ?? "inherit",
    includedInstructions: included,
    excludedInstructions: excluded,
    memoryDelivery: deliveryFor(mode, eligible.length, capability?.supportsSelectedMemoryBodies),
    memoryRead: eligible.length > 0,
    explicitSave: mode !== "temporary" && policy?.explicitSaveMode !== "disabled",
    automaticExtraction:
      mode !== "temporary"
      && policy?.automaticExtractionMode !== "disabled"
      && (capability?.supportsAutomaticExtraction ?? false),
    candidateCreation: mode !== "temporary",
    retrievalWrite: mode === "standard",
    eligibleMemoryCount: eligible.length,
    consideredMemoryCount: considered.length,
    memoryExclusions,
    warnings,
    approximateTokens: Math.ceil(knownCharacters / 4),
    knownCharacters,
    selectedBodyBudgetMax: 5,
    excludedSurfaces: ["cli_internal_context"],
    estimatorVersion: "web-mock-1",
    // VaneHub never manages a CLI's own compaction, on either runtime.
    cliInternalCompactionManaged: false,
  };
}

function deliveryFor(
  mode: string,
  eligible: number,
  supportsBodies: boolean | undefined,
): MemoryDelivery {
  if (mode === "temporary" || eligible === 0) return "none";
  return supportsBodies ? "index_with_selected_bodies" : "index_only";
}
