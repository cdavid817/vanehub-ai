import type { AgentService } from "./agent-service";
import type { SkillEvidenceService } from "./skill-governance-service";

const purgedEvidenceScopes = new Set<string>();

export function resetWebEvidenceForTest() {
  purgedEvidenceScopes.clear();
}

export const webSkillEvidenceClient: SkillEvidenceService = {
  async querySkillEvolutionEvidence(input) {
    const skillId = input.skillId ?? "review";
    const scenario = input.workspace?.startsWith("mock://") ? input.workspace.slice(7) : "healthy";
    const scopeKey = `${input.workspace ?? ""}|${skillId}`;
    const empty = scenario === "empty" || scenario === "disabled" || purgedEvidenceScopes.has(scopeKey);
    const allSignals = empty ? [] : [
      { signalId: "mock-signal-3", sourceKind: "plan_verification", category: "verification_outcome", polarity: "positive", severity: "low", attribution: "verified", attributionRationale: "exact_native_observation", sourceFidelity: "native", sourceAgentId: "onepiece", extractorId: "verification_outcome", extractorVersion: 1, safeSummary: "test verification passed; checks=8", occurredAt: "2026-08-13T10:03:00Z", sanitizerVersion: 1, associationTruncatedCount: 0, sourceLinkTruncatedCount: 0 },
      { signalId: "mock-signal-2", sourceKind: "managed_cli", category: "execution_failure", polarity: "negative", severity: "high", attribution: "correlated", attributionRationale: "binding_and_mount_snapshot", sourceFidelity: "proxied", sourceAgentId: "codex-cli", extractorId: "execution_failure", extractorVersion: 1, safeSummary: "process failed with sandbox classification", occurredAt: "2026-08-13T10:02:00Z", sanitizerVersion: 1, associationTruncatedCount: 1, sourceLinkTruncatedCount: 0 },
      { signalId: "mock-signal-1", sourceKind: "native_execution", category: "execution_failure", polarity: "negative", severity: "high", attribution: "verified", attributionRationale: "exact_native_observation", sourceFidelity: "native", sourceAgentId: "onepiece", extractorId: "execution_failure", extractorVersion: 1, safeSummary: `${skillId} tool failed with sandbox classification`, occurredAt: "2026-08-13T10:01:00Z", sanitizerVersion: 1, associationTruncatedCount: 0, sourceLinkTruncatedCount: 0 },
    ];
    const offset = input.cursor ? Number.parseInt(input.cursor.replace("mock-", ""), 10) || 0 : 0;
    const limit = Math.min(Math.max(input.limit ?? 50, 1), 100);
    const signals = allSignals.slice(offset, offset + limit);
    const distributions: Record<string, Record<string, number>> = empty ? {} : {
      category: { execution_failure: 2, verification_outcome: 1 },
      attribution_strength: { correlated: 1, verified: 2 },
      extractor_id: { execution_failure: 2, verification_outcome: 1 },
      source_agent_id: { "codex-cli": 1, onepiece: 2 },
    };
    return {
      signalCount: allSignals.length,
      seedCount: empty ? 0 : 1,
      firstOccurredAt: allSignals.at(-1)?.occurredAt,
      lastOccurredAt: allSignals[0]?.occurredAt,
      distributions,
      signals,
      seeds: empty ? [] : [{ seedId: "mock-seed-1", category: "execution_failure", readiness: "ready", readinessReason: "verified_recovery_delta", safeSummary: "execution_failure pattern; signals=3 runs=2 recovery=true", signalCount: 3, truncatedSignalCount: 0, independentRunCount: 2, hasRecovery: true, firstOccurredAt: "2026-08-13T10:01:00Z", lastOccurredAt: "2026-08-13T10:03:00Z", createdAt: "2026-08-13T10:04:00Z" }],
      nextCursor: offset + signals.length < allSignals.length ? `mock-${offset + signals.length}` : undefined,
      pipeline: { collectionEnabled: scenario !== "disabled", status: scenario === "disabled" ? "disabled" as const : scenario === "degraded" || scenario === "quota" ? "degraded" as const : "healthy" as const, queueDepth: scenario === "degraded" ? 12 : 0, failureCount: scenario === "degraded" ? 2 : 0 },
      retentionDays: 90,
      signalQuota: 10_000,
      seedQuota: 2_000,
      byteQuota: 64 * 1024 * 1024,
      droppedCount: scenario === "quota" ? 7 : 0,
      expiredCount: 0,
    };
  },

  async getSkillEvolutionSeedLineage(this: AgentService, seedId, input) {
    const overview = await this.querySkillEvolutionEvidence(input);
    const seed = overview.seeds.find((candidate) => candidate.seedId === seedId);
    return seed ? { seed, signals: overview.signals } : null;
  },

  async purgeSkillEvolutionEvidence(this: AgentService, input) {
    if (!input.confirmed) throw new Error("Evidence purge requires confirmation");
    const current = await this.querySkillEvolutionEvidence({ workspace: input.workspace, skillId: input.skillId });
    purgedEvidenceScopes.add(`${input.workspace ?? ""}|${input.skillId}`);
    return {
      operationId: input.operationId,
      deletedSignals: current.signalCount,
      deletedSeeds: current.seedCount,
      deletedFeedback: 0,
    };
  },
};
