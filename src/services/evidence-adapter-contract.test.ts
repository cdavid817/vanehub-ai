import { afterEach, describe, expect, it } from "vitest";

import { tauriAgentClient } from "./tauri-agent-client";
import { resetWebEvidenceForTest, webAgentClient } from "./web-agent-client";

afterEach(resetWebEvidenceForTest);

describe("skill evolution evidence adapter contract", () => {
  it("keeps both adapters on the same service boundary", () => {
    expect(typeof tauriAgentClient.querySkillEvolutionEvidence).toBe("function");
    expect(typeof tauriAgentClient.getSkillEvolutionSeedLineage).toBe("function");
    expect(typeof webAgentClient.querySkillEvolutionEvidence).toBe("function");
    expect(typeof webAgentClient.getSkillEvolutionSeedLineage).toBe("function");
    expect(typeof tauriAgentClient.purgeSkillEvolutionEvidence).toBe("function");
    expect(typeof webAgentClient.purgeSkillEvolutionEvidence).toBe("function");
  });

  it("purges only the requested simulated scope after confirmation", async () => {
    await expect(webAgentClient.purgeSkillEvolutionEvidence({
      operationId: "purge-1",
      workspace: "workspace-a",
      skillId: "review",
      confirmed: true,
    })).resolves.toMatchObject({ deletedSignals: 3, deletedSeeds: 1 });
    expect((await webAgentClient.querySkillEvolutionEvidence({ workspace: "workspace-a", skillId: "review" })).signalCount).toBe(0);
    expect((await webAgentClient.querySkillEvolutionEvidence({ workspace: "workspace-b", skillId: "review" })).signalCount).toBe(3);
    await expect(webAgentClient.purgeSkillEvolutionEvidence({
      operationId: "purge-2",
      skillId: "review",
      confirmed: false,
    })).rejects.toThrow(/confirmation/i);
  });

  it("provides deterministic pagination and sanitized lineage", async () => {
    const first = await webAgentClient.querySkillEvolutionEvidence({ skillId: "review", limit: 1 });
    const repeated = await webAgentClient.querySkillEvolutionEvidence({ skillId: "review", limit: 1 });
    expect(first).toEqual(repeated);
    expect(first.nextCursor).toBe("mock-1");
    const second = await webAgentClient.querySkillEvolutionEvidence({
      skillId: "review",
      limit: 1,
      cursor: first.nextCursor,
    });
    expect(second.signals[0]?.signalId).not.toBe(first.signals[0]?.signalId);
    const lineage = await webAgentClient.getSkillEvolutionSeedLineage("mock-seed-1", { skillId: "review" });
    expect(lineage?.seed.independentRunCount).toBe(2);
    expect(JSON.stringify(lineage)).not.toMatch(/prompt|toolArguments|toolOutput|thinking/i);
  });

  it.each([
    ["mock://empty", "healthy", true, 0],
    ["mock://disabled", "disabled", false, 0],
    ["mock://degraded", "degraded", true, 3],
    ["mock://quota", "degraded", true, 3],
  ] as const)("simulates %s", async (workspace, status, collectionEnabled, signalCount) => {
    const overview = await webAgentClient.querySkillEvolutionEvidence({ workspace });
    expect(overview.pipeline).toMatchObject({ status, collectionEnabled });
    expect(overview.signalCount).toBe(signalCount);
  });
});
