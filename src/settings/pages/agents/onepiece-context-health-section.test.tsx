// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../../../i18n";
import type { AgentService } from "../../../services/agent-service";
import type { ContextQualityAssessment, ContextQualitySummary } from "../../../types/context-quality";
import { renderWithAppProviders } from "../../../test/render";
import { SettingsProvider } from "../../settings-provider";
import { OnePieceContextHealthSection } from "./onepiece-context-health-section";

const assessment: ContextQualityAssessment = {
  version: "quality-v1",
  attemptId: "attempt-1",
  sessionCorrelation: "session-1",
  decisionSequence: 1,
  recordedAt: "2026-08-14T08:00:00.000Z",
  outcome: "compacted",
  path: "optimizer",
  reason: null,
  triggerSource: "token-aware",
  beforeCharacters: 1_000,
  afterCharacters: 600,
  savedCharacters: 400,
  beforeTokens: 250,
  afterTokens: 150,
  savedTokens: 100,
  measurementQuality: "reported",
  invariants: { protocolComplete: true, protectedRetained: true, verbatimRetained: true, reinjectionComplete: true },
  contextPolicyVersion: "policy-v1",
  optimizerVersion: "optimizer-v1",
  verifierVersion: "verifier-v1",
};

const summary: ContextQualitySummary = {
  rangeDays: 30,
  evaluated: 2,
  savedCharacters: 800,
  savedTokens: 100,
  tokenMeasurementCount: 1,
  qualityCoverage: { measuredWithTokens: 1, charactersOnly: 1, tokenCoverageBasisPoints: 5_000 },
  outcomes: { compacted: 1, fallback: 1 },
  paths: { optimizer: 1, compatibility: 1 },
  qualities: { reported: 1, "characters-only": 1 },
  reasons: { "verification-failed": 1 },
  policyVersions: { "policy-v1": 2 },
  earliestRecordedAt: assessment.recordedAt,
  latestRecordedAt: assessment.recordedAt,
};

function createService() {
  return {
    getContextQualitySummary: vi.fn().mockResolvedValue(summary),
    listContextQualityHistory: vi.fn().mockResolvedValue({ items: [assessment], nextCursor: null }),
    listContextEvidenceManifests: vi.fn().mockResolvedValue({
      items: [{
        sessionId: "session-1", turnId: "turn-1", generationId: "generation-1",
        policyVersion: "context-engine-v1", evidenceBudget: 4096, occupiedTokens: 768,
        selected: [{ id: "definition", sourceKind: "lsp-definition", sourceRef: "src/lib.rs", startLine: 4, endLine: 12, symbol: "run", tokenEstimate: 200, reasonCodes: ["symbol-relation"] }],
        rejected: [{ id: "memory", reasonCode: "budget-rejected" }],
        sourceOutcomes: { retrieval: "ready", lsp: "unavailable" }, duplicateTokensSaved: 100,
        collectionLatencyBucket: "under-50ms", rankingLatencyBucket: "under-10ms",
        compactionTriggered: false, runtime: "web-mock",
      }],
      nextCursor: null,
    }),
  } as unknown as AgentService;
}

function renderSection(service: AgentService) {
  return renderWithAppProviders(<SettingsProvider><OnePieceContextHealthSection service={service} /></SettingsProvider>);
}

describe("OnePieceContextHealthSection", () => {
  beforeEach(() => window.localStorage.clear());

  it("shows aggregate coverage, distributions, and recent content-safe history", async () => {
    renderSection(createService());

    expect(await screen.findByText("50%")).toBeTruthy();
    expect(screen.getByText("characters-only")).toBeTruthy();
    expect(screen.getByText("policy-v1")).toBeTruthy();
    expect(screen.getByText(/compacted · optimizer/)).toBeTruthy();
    expect(screen.getByText(/提示词和工具内容不会进入历史/)).toBeTruthy();
  });

  it("reloads both resources when the range changes", async () => {
    const service = createService();
    const { user } = renderSection(service);
    await screen.findByText("50%");

    await user.click(screen.getByRole("button", { name: "7 天" }));

    await waitFor(() => expect(service.getContextQualitySummary).toHaveBeenLastCalledWith({ rangeDays: 7 }));
    expect(service.listContextQualityHistory).toHaveBeenLastCalledWith({ rangeDays: 7, cursor: null, limit: 10 });
  });

  it("paginates recent history with the service cursor", async () => {
    const service = createService();
    vi.mocked(service.listContextQualityHistory)
      .mockResolvedValueOnce({ items: [assessment], nextCursor: "attempt-1" })
      .mockResolvedValueOnce({ items: [{ ...assessment, attemptId: "attempt-2" }], nextCursor: null });
    const { user } = renderSection(service);

    await user.click(await screen.findByRole("button", { name: "加载更多" }));

    await waitFor(() => expect(screen.getAllByText(/compacted · optimizer/)).toHaveLength(2));
    expect(service.listContextQualityHistory).toHaveBeenLastCalledWith({ rangeDays: 30, cursor: "attempt-1", limit: 10 });
  });

  it("keeps summary and history failures independent", async () => {
    const service = createService();
    vi.mocked(service.getContextQualitySummary).mockRejectedValueOnce(new Error("summary unavailable"));
    renderSection(service);

    expect((await screen.findByRole("alert")).textContent).toContain("summary unavailable");
    expect(await screen.findByText(/compacted · optimizer/)).toBeTruthy();
  });

  it("shows an explicit empty state without synthesizing success", async () => {
    const service = createService();
    vi.mocked(service.getContextQualitySummary).mockResolvedValueOnce({
      ...summary,
      evaluated: 0,
      savedCharacters: 0,
      savedTokens: 0,
      tokenMeasurementCount: 0,
      qualityCoverage: { measuredWithTokens: 0, charactersOnly: 0, tokenCoverageBasisPoints: 0 },
      outcomes: {},
      paths: {},
      qualities: {},
      reasons: {},
      policyVersions: {},
      earliestRecordedAt: null,
      latestRecordedAt: null,
    });
    vi.mocked(service.listContextQualityHistory).mockResolvedValueOnce({ items: [], nextCursor: null });
    renderSection(service);

    expect(await screen.findByText("此时间范围内尚无上下文决策评估。")).toBeTruthy();
    expect(screen.getAllByText("0").length).toBeGreaterThan(0);
  });

  it("persists a changed retention window", async () => {
    const service = createService();
    const { user } = renderSection(service);
    const select = await screen.findByRole("combobox", { name: "保留期" });

    await user.selectOptions(select, "90");

    await waitFor(() => expect(JSON.parse(window.localStorage.getItem("vanehub.appSettings") ?? "{}"))
      .toMatchObject({ contextQualityRetentionDays: 90 }));
    await waitFor(() => expect(service.listContextQualityHistory).toHaveBeenCalledTimes(2));
    expect(service.getContextQualitySummary).toHaveBeenCalledTimes(2);
  });

  it("keeps the advanced context inspector collapsed until requested", async () => {
    const service = createService();
    const { user } = renderSection(service);
    const trigger = await screen.findByRole("button", { name: "上下文检查器" });
    expect(service.listContextEvidenceManifests).not.toHaveBeenCalled();

    await user.click(trigger);

    expect(await screen.findByText("src/lib.rs:4-12")).toBeTruthy();
    expect(screen.getByText(/memory: budget-rejected/)).toBeTruthy();
    expect(screen.getByText(/lsp: unavailable/)).toBeTruthy();
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
  });

  it("shows a retry control when context evidence loading fails", async () => {
    const service = createService();
    vi.mocked(service.listContextEvidenceManifests).mockRejectedValueOnce(new Error("manifest unavailable"));
    const { user } = renderSection(service);
    await user.click(await screen.findByRole("button", { name: "上下文检查器" }));
    expect((await screen.findByRole("alert")).textContent).toContain("manifest unavailable");
    expect(screen.getByRole("button", { name: "重试加载上下文证据" })).toBeTruthy();
  });
});
