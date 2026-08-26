import type {
  AgentPersonalizationCapability,
  EffectivePreview,
  MemoryDelivery,
  PersonalizationPolicy,
  PolicyScopeKind,
} from "../../../types/personalization";

// Turns one Agent's capabilities and its resolved preview into the row the Overview renders.
//
// The distinction this exists for: "off" and "the runtime cannot do it" look the same in a boolean
// and mean opposite things to a user. One is a switch they can flip; the other is a fact about the
// Agent, and a page that renders it as a switch invites them to keep trying.

export type ControlState =
  | { kind: "on" }
  | { kind: "off" }
  | { kind: "unavailable"; reason: "runtime_capability" };

export interface PolicySource {
  scopeKind: PolicyScopeKind;
  scopeKey: string;
  revision: number;
}

export interface AgentOverviewRow {
  agentId: string;
  displayName: string;
  /** `off` here means every contributing layer resolved to nothing, not that a switch is down. */
  instructions: ControlState;
  /** Which layers actually contributed the instruction text, in the order they were applied. */
  sources: PolicySource[];
  characters: number;
  memoryRead: ControlState;
  delivery: MemoryDelivery;
  extraction: ControlState;
  warnings: string[];
}

function stateFrom(supported: boolean, enabled: boolean): ControlState {
  if (!supported) return { kind: "unavailable", reason: "runtime_capability" };
  return enabled ? { kind: "on" } : { kind: "off" };
}

export function agentOverviewRow(
  capability: AgentPersonalizationCapability,
  preview: EffectivePreview,
): AgentOverviewRow {
  const sources: PolicySource[] = [];
  for (const segment of preview.includedInstructions) {
    const seen = sources.some(
      (source) => source.scopeKind === segment.scopeKind && source.scopeKey === segment.scopeKey,
    );
    if (!seen) {
      sources.push({
        scopeKind: segment.scopeKind,
        scopeKey: segment.scopeKey,
        revision: segment.policyRevision,
      });
    }
  }

  return {
    agentId: capability.agentId,
    displayName: capability.displayName,
    instructions: stateFrom(
      capability.supportsCustomInstructions,
      preview.includedInstructions.length > 0,
    ),
    sources,
    characters: preview.includedInstructions.reduce(
      (total, segment) => total + segment.characters,
      0,
    ),
    memoryRead: stateFrom(capability.supportsMemoryIndex, preview.memoryRead),
    // Reported from the resolution rather than inferred from the capability: an Agent that can
    // take selected bodies still gets an index only when the policy says so, and claiming bodies
    // were injected because the Agent could accept them would be a lie about this run.
    delivery: preview.memoryDelivery,
    extraction: stateFrom(capability.supportsAutomaticExtraction, preview.automaticExtraction),
    warnings: preview.warnings,
  };
}

export interface OverviewTotals {
  agents: number;
  agentsWithInstructions: number;
  configuredScopes: number;
  globalCharacters: number;
  memoryReadAgents: number;
  extractionAgents: number;
}

export function overviewTotals(
  rows: readonly AgentOverviewRow[],
  policies: readonly PersonalizationPolicy[],
): OverviewTotals {
  const globalPolicy = policies.find((policy) => policy.scopeKind === "global");
  return {
    agents: rows.length,
    agentsWithInstructions: rows.filter((row) => row.instructions.kind === "on").length,
    // A layer that exists but resolves to all-inherit still counts: the user created it, and a
    // count that hid it would make an empty scope indistinguishable from one never opened.
    configuredScopes: policies.length,
    globalCharacters:
      (globalPolicy?.aboutUser.length ?? 0) + (globalPolicy?.styleRules.length ?? 0),
    memoryReadAgents: rows.filter((row) => row.memoryRead.kind === "on").length,
    extractionAgents: rows.filter((row) => row.extraction.kind === "on").length,
  };
}

/** Every distinct warning across the Agents, so the page reports each cause once. */
export function overviewWarnings(rows: readonly AgentOverviewRow[]): string[] {
  return [...new Set(rows.flatMap((row) => row.warnings))];
}
