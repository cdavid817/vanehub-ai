import type { AgentRegistryEntry, AvailabilityState } from "../types/agent";

/** The real, checkable "status" axis for 18.5's filters: `availabilityState` is the only field on
 *  `AgentRegistryEntry` that states whether an Agent can currently do anything at all. */
export type EvaluationAgentStatusFilter = "all" | AvailabilityState;

/** Declaration order also drives the status filter's own `<select>` options and the
 *  `evaluation.agentStatus.*` i18n keys, matching `createSession.agentAvailability.*`'s order. */
export const EVALUATION_AGENT_STATUSES: AvailabilityState[] = ["available", "unavailable", "needs-auth", "unknown"];

/** The real "capability" axis: `capabilityTags` has no fixed enum anywhere in `AgentRegistryEntry`
 *  or the Rust catalog (`agent_capability_tags` is a free-form join table), so "all capabilities
 *  currently in the roster" -- not an invented fixed list -- is what a filter can honestly offer. */
export type EvaluationAgentCapabilityFilter = "all" | string;

export interface EvaluationAgentFilters {
  query: string;
  status: EvaluationAgentStatusFilter;
  capability: EvaluationAgentCapabilityFilter;
}

export const EVALUATION_AGENT_FILTERS_DEFAULT: EvaluationAgentFilters = { query: "", status: "all", capability: "all" };

/** Mirrors the Rust-side `MAX_ARENA_ATTEMPTS` (`evaluation_engine.rs`) and the Web mock's own
 *  identical `> 8` guard (`web-evaluation-client.ts`) -- a real, server-enforced cap, not a
 *  client-invented one. */
export const MAX_EVALUATION_AGENTS = 8;

/** Every distinct capability tag currently present across `agents`, sorted for a stable filter
 *  option order. Built from the roster actually passed in rather than a hard-coded catalog. */
export function collectEvaluationCapabilityTags(agents: AgentRegistryEntry[]): string[] {
  return Array.from(new Set(agents.flatMap((agent) => agent.capabilityTags))).sort();
}

function matchesQuery(agent: AgentRegistryEntry, query: string): boolean {
  const trimmed = query.trim().toLowerCase();
  if (!trimmed) return true;
  return `${agent.displayName} ${agent.id} ${agent.provider}`.toLowerCase().includes(trimmed);
}

export function filterEvaluationAgents(agents: AgentRegistryEntry[], filters: EvaluationAgentFilters): AgentRegistryEntry[] {
  return agents.filter((agent) =>
    matchesQuery(agent, filters.query)
    && (filters.status === "all" || agent.availabilityState === filters.status)
    && (filters.capability === "all" || agent.capabilityTags.includes(filters.capability)));
}

/**
 * 18.5's own "incompatibility reason" concept, investigated rather than assumed. `capabilityTags`
 * values actually assigned anywhere in this codebase (`mock-agent-data.ts`, the Rust fixtures under
 * `contexts/agent_runtime`) are runtime-shape descriptors -- "coding", "cli", "api", "agent",
 * "native", "browser", "open-source" -- and `EvaluationTask.category` is a task-domain enum --
 * "bugfix", "feature", "refactor", "tests", "code_review", "tool_use", "context", "planning". The
 * two vocabularies share zero values anywhere they are assigned, so a tag-vs-category compatibility
 * heuristic would be fabricated: there is no real per-task incompatibility concept to build.
 *
 * What *is* real and checkable, task-independent: `availabilityState` plus the Agent's own
 * `unavailableReason`, the exact pair `create-session-agent-section.tsx` already renders for this
 * same registry field. An incompatible Agent stays selectable rather than disabled here: the Rust
 * dispatch path (`evaluation_api.rs` -> `NativeEvaluationAgentAdapter`) already tolerates dispatching
 * to one and records a real, modeled `agent_failed` outcome with its own diagnostic rather than
 * rejecting the request (see `WEB_DISPATCH_DIAGNOSTIC` in the Web mock), and this page's own
 * pre-existing default-selection effect (`evaluation-center.tsx`) already falls back to *every*
 * Agent once none are "available" -- disabling selection here would make that fallback unusable.
 */
export function isEvaluationAgentIncompatible(agent: AgentRegistryEntry): boolean {
  return agent.availabilityState !== "available";
}
