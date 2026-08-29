// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import "../../../i18n";
import type { EvidenceOverview } from "../../../services/agent-service";
import type { Skill } from "../../../types/skill";

const service = vi.hoisted(() => ({
  getSkillEvolutionSeedLineage: vi.fn(),
  purgeSkillEvolutionEvidence: vi.fn(),
  querySkillEvolutionEvidence: vi.fn(),
}));
vi.mock("../../../services/runtime-agent-client", () => ({ agentService: service }));

import { SkillEvolutionEvidence } from "./skill-evolution-evidence";

const skill = {
  id: "review",
  scope: "global",
  workspacePath: null,
  source: "user",
  enabled: true,
  skillDir: "review",
  skillMdPath: "review/SKILL.md",
  contentHash: "hash",
  metadata: { id: "review", name: "Review", description: "Review skill", version: "1", category: "quality", triggers: [] },
  boundAgentIds: [],
  bindings: [],
  createdAt: "2026-08-13T00:00:00Z",
  updatedAt: "2026-08-13T00:00:00Z",
  layer: "user",
  origin: "created",
  trust: "trusted",
  availability: "available",
  immutable: false,
  shadowedDefinitions: [],
  usage: { viewCount: 0, useCount: 0, lastViewedAt: null, lastUsedAt: null, revisionWitness: null },
} satisfies Skill;

const healthy: EvidenceOverview = {
  signalCount: 1,
  seedCount: 1,
  firstOccurredAt: "2026-08-13T10:00:00Z",
  lastOccurredAt: "2026-08-13T10:00:00Z",
  distributions: { category: { execution_failure: 1 }, attribution_strength: { correlated: 1 } },
  signals: [{ signalId: "signal-1", sourceKind: "managed_cli", category: "execution_failure", polarity: "negative", severity: "high", attribution: "correlated", attributionRationale: "binding_and_mount_snapshot", sourceFidelity: "proxied", sourceAgentId: "codex-cli", extractorId: "execution_failure", extractorVersion: 1, safeSummary: "process failed with sandbox classification", occurredAt: "2026-08-13T10:00:00Z", sanitizerVersion: 1, associationTruncatedCount: 1, sourceLinkTruncatedCount: 0 }],
  seeds: [{ seedId: "seed-1", category: "execution_failure", readiness: "human_review_only", readinessReason: "correlated_evidence", safeSummary: "bounded pattern", signalCount: 1, truncatedSignalCount: 0, independentRunCount: 1, hasRecovery: false, firstOccurredAt: "2026-08-13T10:00:00Z", lastOccurredAt: "2026-08-13T10:00:00Z", createdAt: "2026-08-13T10:01:00Z" }],
  pipeline: { collectionEnabled: true, status: "healthy", queueDepth: 0, failureCount: 0 },
  retentionDays: 90,
  signalQuota: 10_000,
  seedQuota: 2_000,
  byteQuota: 64 * 1024 * 1024,
  droppedCount: 0,
  expiredCount: 0,
};

function renderEvidence() {
  return render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })}><SkillEvolutionEvidence skill={skill} /></QueryClientProvider>);
}

beforeEach(() => {
  vi.clearAllMocks();
  service.querySkillEvolutionEvidence.mockResolvedValue(healthy);
  service.getSkillEvolutionSeedLineage.mockResolvedValue({ seed: healthy.seeds[0], signals: healthy.signals });
  service.purgeSkillEvolutionEvidence.mockResolvedValue({ operationId: "purge", deletedSignals: 1, deletedSeeds: 1, deletedFeedback: 0 });
});

describe("SkillEvolutionEvidence", () => {
  it("renders evidence-only fidelity and sanitized lineage details", async () => {
    const user = userEvent.setup();
    renderEvidence();
    expect(await screen.findByText("演进证据")).toBeTruthy();
    expect(screen.getByText(/脱敏证据保持只读/)).toBeTruthy();
    expect((await screen.findAllByText("correlated")).length).toBeGreaterThan(0);
    await user.click(screen.getByRole("button", { name: "查看谱系" }));
    expect(await screen.findByRole("region", { name: "候选种子谱系" })).toBeTruthy();
    expect(screen.getAllByText(/execution_failure/).length).toBeGreaterThan(0);
    expect(document.body.textContent).not.toMatch(/prompt|tool arguments|tool output/i);
  });

  it("renders empty and degraded collection states without fabricated metrics", async () => {
    service.querySkillEvolutionEvidence.mockResolvedValueOnce({
      ...healthy,
      signalCount: 0,
      seedCount: 0,
      signals: [],
      seeds: [],
      distributions: {},
      droppedCount: 4,
      pipeline: { collectionEnabled: true, status: "degraded", queueDepth: 2, failureCount: 1 },
    });
    renderEvidence();
    expect(await screen.findByText("此 Skill 暂无保留证据")).toBeTruthy();
    expect(screen.getByText(/失败 1 次，丢弃 4 条/)).toBeTruthy();
    expect(screen.queryByText("可供审查")).toBeNull();
  });

  it("confirms scoped purge, preserves source wording, and restores trigger focus", async () => {
    const user = userEvent.setup();
    renderEvidence();
    const trigger = await screen.findByRole("button", { name: "清除此 Skill 的证据" });
    await user.click(trigger);
    const dialog = screen.getByRole("dialog", { name: "清除演进证据？" });
    expect(within(dialog).getByText(/源对话、跟踪、日志、用量、Skill 和 Overlay/)).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "清除证据" }));
    await waitFor(() => expect(service.purgeSkillEvolutionEvidence).toHaveBeenCalledWith(expect.objectContaining({ skillId: "review", confirmed: true })));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(document.activeElement).toBe(trigger);
  });
});
