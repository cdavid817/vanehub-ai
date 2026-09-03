import { describe, expect, it } from "vitest";
import type { AgentRegistryEntry } from "../types/agent";
import {
  collectEvaluationCapabilityTags,
  EVALUATION_AGENT_STATUSES,
  filterEvaluationAgents,
  isEvaluationAgentIncompatible,
  MAX_EVALUATION_AGENTS,
} from "./evaluation-agent-filters";

function buildAgent(id: string, displayName: string, overrides: Partial<AgentRegistryEntry> = {}): AgentRegistryEntry {
  return {
    id, displayName, provider: id, launch: { kind: "cli" }, supportedInteractionModes: ["cli"],
    availabilityState: "available", capabilityTags: [], agentOrigin: "builtin",
    ...overrides,
  };
}

const CLAUDE = buildAgent("claude-code", "Claude Code", { capabilityTags: ["coding", "cli"] });
const CODEX = buildAgent("codex-cli", "Codex CLI", {
  provider: "OpenAI", capabilityTags: ["coding", "api"], availabilityState: "needs-auth", unavailableReason: "Sign in to OpenAI.",
});
const ONEPIECE = buildAgent("onepiece", "OnePiece", {
  provider: "VaneHub", capabilityTags: ["api", "native"], availabilityState: "unavailable", unavailableReason: "OnePiece requires provider configuration.",
});

describe("collectEvaluationCapabilityTags", () => {
  it("returns every distinct tag across the roster, sorted", () => {
    expect(collectEvaluationCapabilityTags([CLAUDE, CODEX, ONEPIECE])).toEqual(["api", "cli", "coding", "native"]);
  });

  it("returns an empty list for an empty roster", () => {
    expect(collectEvaluationCapabilityTags([])).toEqual([]);
  });
});

describe("filterEvaluationAgents", () => {
  const agents = [CLAUDE, CODEX, ONEPIECE];

  it("matches every Agent under the default filters", () => {
    expect(filterEvaluationAgents(agents, { query: "", status: "all", capability: "all" })).toEqual(agents);
  });

  it("matches query against displayName, id, and provider, case-insensitively", () => {
    expect(filterEvaluationAgents(agents, { query: "openai", status: "all", capability: "all" })).toEqual([CODEX]);
    expect(filterEvaluationAgents(agents, { query: "ONEPIECE", status: "all", capability: "all" })).toEqual([ONEPIECE]);
    expect(filterEvaluationAgents(agents, { query: "claude-code", status: "all", capability: "all" })).toEqual([CLAUDE]);
  });

  it("narrows by status", () => {
    expect(filterEvaluationAgents(agents, { query: "", status: "needs-auth", capability: "all" })).toEqual([CODEX]);
    expect(filterEvaluationAgents(agents, { query: "", status: "available", capability: "all" })).toEqual([CLAUDE]);
  });

  it("narrows by capability", () => {
    expect(filterEvaluationAgents(agents, { query: "", status: "all", capability: "native" })).toEqual([ONEPIECE]);
  });

  it("combines query, status, and capability with AND semantics", () => {
    expect(filterEvaluationAgents(agents, { query: "codex", status: "needs-auth", capability: "api" })).toEqual([CODEX]);
    expect(filterEvaluationAgents(agents, { query: "codex", status: "unavailable", capability: "api" })).toEqual([]);
  });
});

describe("isEvaluationAgentIncompatible", () => {
  it("is false only for an available Agent", () => {
    expect(isEvaluationAgentIncompatible(CLAUDE)).toBe(false);
  });

  it("is true for needs-auth, unavailable, and unknown", () => {
    expect(isEvaluationAgentIncompatible(CODEX)).toBe(true);
    expect(isEvaluationAgentIncompatible(ONEPIECE)).toBe(true);
    expect(isEvaluationAgentIncompatible(buildAgent("x", "X", { availabilityState: "unknown" }))).toBe(true);
  });
});

describe("EVALUATION_AGENT_STATUSES / MAX_EVALUATION_AGENTS", () => {
  it("lists every AvailabilityState exactly once", () => {
    expect(EVALUATION_AGENT_STATUSES).toEqual(["available", "unavailable", "needs-auth", "unknown"]);
  });

  it("caps at 8, matching the Rust/mock server-side limit", () => {
    expect(MAX_EVALUATION_AGENTS).toBe(8);
  });
});
