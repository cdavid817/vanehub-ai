import { describe, expect, it } from "vitest";
import type { AgentRun } from "../../types/agent-run";
import { agentRunElapsed } from "./agent-run-elapsed";

const run = (state: AgentRun["state"]): AgentRun => ({
  id: "run-1", owner: { ownerType: "session_generation", ownerId: "message-1" }, links: [],
  parentRunId: null, state, recoveryPolicy: "not_recoverable", retryCount: 0, maxRetries: 0,
  reasonCode: null, createdAt: "2026-08-27T00:00:00Z", updatedAt: "2026-08-27T00:00:00Z",
  version: 1, lastWitness: "fixture",
});

describe("agentRunElapsed", () => {
  it("advances a running Run against the current clock even when updatedAt is unchanged", () => {
    expect(agentRunElapsed(run("running"), Date.parse("2026-08-27T00:01:05Z"))).toBe("1:05");
  });

  it("freezes a terminal Run against its persisted update timestamp", () => {
    expect(agentRunElapsed({ ...run("completed"), updatedAt: "2026-08-27T00:00:42Z" }, Date.parse("2026-08-27T00:05:00Z"))).toBe("0:42");
  });

  it("clamps invalid or future timestamps", () => {
    expect(agentRunElapsed({ ...run("running"), createdAt: "invalid" })).toBe("0:00");
    expect(agentRunElapsed(run("running"), Date.parse("2026-08-26T23:59:00Z"))).toBe("0:00");
  });
});
