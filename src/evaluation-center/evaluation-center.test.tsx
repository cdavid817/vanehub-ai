// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { i18n } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import type { EvaluationArena, EvaluationCheck } from "../types/evaluation";
import { EvaluationCenter } from "./evaluation-center";

beforeAll(() => {
  // jsdom does not implement ResizeObserver; this repo's convention (DataTable.test.tsx) is a
  // no-op stub, which also pins `useTableCompactMode` to its non-compact default for these tests.
  globalThis.ResizeObserver = class {
    observe() {}
    disconnect() {}
  } as unknown as typeof ResizeObserver;
});

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

/**
 * 18.4 moved task/Agent configuration out of the header and into `EvaluationRunWizard`'s own
 * guided Sheet (task -> Agents -> Review). Every test here that used to click the header's own
 * Run button directly now opens that wizard first and advances to Review before clicking the same
 * (relocated) Run action -- `customizeAgentStep` is the hook the one test that actually changes
 * the Agent selection (rather than accepting the page's own default) uses to do that from inside
 * the wizard's own Agent step.
 */
async function openWizardAndRun(customizeAgentStep?: () => void) {
  const configure = await screen.findByRole("button", { name: "配置评测" });
  await waitFor(() => expect((configure as HTMLButtonElement).disabled).toBe(false));
  fireEvent.click(configure);
  fireEvent.click(screen.getByRole("button", { name: "下一步" }));
  customizeAgentStep?.();
  fireEvent.click(screen.getByRole("button", { name: "下一步" }));
  fireEvent.click(screen.getByRole("button", { name: "运行竞技场" }));
}

describe("EvaluationCenter", () => {
  it("configures, compares, filters, inspects, and exports a mock arena", async () => {
    await i18n.changeLanguage("zh-CN");
    const exportSpy = vi.spyOn(agentService, "exportEvaluation");
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => undefined);
    vi.stubGlobal("URL", { createObjectURL: () => "blob:evaluation", revokeObjectURL: vi.fn() });
    render(<EvaluationCenter />);
    await openWizardAndRun(() => {
      expect(screen.getByTestId("evaluation-agent-opencode")).toBeTruthy();
      // The Agent picker's own display name -- unlike the results table below, which identifies
      // an Agent by its raw id (`evaluation-results-table.tsx`'s Agent column).
      expect(screen.getByText("Codex CLI")).toBeTruthy();
      for (const agentId of ["claude-code", "opencode", "gemini-cli", "antigravity-cli"]) {
        fireEvent.click(screen.getByTestId(`evaluation-agent-${agentId}`));
      }
    });
    expect(await screen.findByText("onepiece")).toBeTruthy();
    expect(screen.getByText("codex-cli")).toBeTruthy();
    fireEvent.change(screen.getByLabelText("筛选结果"), { target: { value: "codex-cli" } });
    expect(document.querySelectorAll("tbody tr")).toHaveLength(1);
    fireEvent.click(document.querySelector("tbody tr")!);
    expect(screen.getByText("验证")).toBeTruthy();
    expect(screen.getByText("指标与来源")).toBeTruthy();
    expect(screen.getByText(/reported · provider/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "导出 JSON" }));
    await waitFor(() => expect(exportSpy).toHaveBeenCalled());
    expect(click).toHaveBeenCalled();
    vi.unstubAllGlobals();
  });

  it("shows running, cancellation, terminal, and error states", async () => {
    await i18n.changeLanguage("zh-CN");
    let resolveStart: ((arena: EvaluationArena) => void) | undefined;
    const queued = arena("queued");
    vi.spyOn(agentService, "startEvaluation").mockReturnValue(new Promise((resolve) => { resolveStart = resolve; }));
    vi.spyOn(agentService, "cancelEvaluation").mockResolvedValue(arena("cancelled"));
    render(<EvaluationCenter />);
    await openWizardAndRun();
    expect((screen.getByRole("button", { name: "运行中" }) as HTMLButtonElement).disabled).toBe(true);
    resolveStart?.(queued);
    // Scoped to the results table body: 18.12 added an outcome badge to the detail pane too, and
    // `start()` auto-selects the arena's first attempt, so an unscoped lookup would find this same
    // text in both places once that pane renders.
    const tableBody = document.querySelector("tbody") as HTMLElement;
    fireEvent.click((await within(tableBody).findByText("已排队")).closest("tr")!);
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    // Scoped to the detail pane -- this is what the test is actually about (the pane following the
    // polled state, not the table row, which updates independently either way).
    await waitFor(() => expect(within(screen.getByTestId("evaluation-detail")).getByText("已取消")).toBeTruthy());
    cleanup();
    vi.spyOn(agentService, "listEvaluationTasks").mockRejectedValue(new Error("secret must not surface"));
    render(<EvaluationCenter />);
    expect((await screen.findByRole("alert")).textContent).toBe("无法加载评测数据。");
  });

  // The detail pane used to hold the attempt object captured at click time. Polling replaced the
  // arena around it, so the pane kept reporting `queued` -- and kept offering Cancel -- next to a
  // row that had already settled.
  it("follows the polled arena instead of the attempt captured when the row was clicked", async () => {
    await i18n.changeLanguage("zh-CN");
    const list = vi.spyOn(agentService, "listEvaluationArenas").mockResolvedValue({ items: [], nextCursor: null });
    vi.spyOn(agentService, "startEvaluation").mockResolvedValue(arena("queued"));
    render(<EvaluationCenter />);
    await openWizardAndRun();
    await waitFor(() => expect(screen.getByTestId("evaluation-detail").dataset.selectedOutcome).toBe("queued"));
    expect(screen.getByTestId("evaluation-cancel")).toBeTruthy();

    list.mockResolvedValue({ items: [arena("cancelled")], nextCursor: null });
    await waitFor(
      () => expect(screen.getByTestId("evaluation-detail").dataset.selectedOutcome).toBe("cancelled"),
      { timeout: 4_000 },
    );
    expect(screen.queryByTestId("evaluation-cancel")).toBeNull();
  });

  // 18.6: real service-side pagination for the experiment list, wired end to end through the page.
  // Both pages use terminal-only outcomes deliberately, so the reconcile-poll effect's own gate
  // (`arenas.some(non-terminal)`) never opens and this test is not racing that timer.
  it("loads the next page of arenas on demand and appends it without duplicating what is already shown", async () => {
    await i18n.changeLanguage("zh-CN");
    const list = vi.spyOn(agentService, "listEvaluationArenas");
    list.mockResolvedValueOnce({ items: [arena("cancelled")], nextCursor: "1" });
    render(<EvaluationCenter />);
    const loadMore = await screen.findByTestId("evaluation-arena-load-more");
    expect(screen.getAllByTestId("evaluation-arena-row")).toHaveLength(1);

    list.mockResolvedValueOnce({ items: [arenaWithArtifacts([])], nextCursor: null });
    fireEvent.click(loadMore);
    await waitFor(() => expect(screen.getAllByTestId("evaluation-arena-row")).toHaveLength(2));
    expect(list).toHaveBeenLastCalledWith({ cursor: "1" });
    // No further page: the control that fetched it is gone, not just disabled.
    expect(screen.queryByTestId("evaluation-arena-load-more")).toBeNull();
  });

  it("pauses polling while the document is hidden, and reconciles immediately once visible again", async () => {
    await i18n.changeLanguage("zh-CN");
    const list = vi.spyOn(agentService, "listEvaluationArenas").mockResolvedValue({ items: [], nextCursor: null });
    vi.spyOn(agentService, "startEvaluation").mockResolvedValue(arena("queued"));
    render(<EvaluationCenter />);
    await openWizardAndRun();
    await waitFor(() => expect(screen.getByTestId("evaluation-detail").dataset.selectedOutcome).toBe("queued"));

    const callsBeforeHiding = list.mock.calls.length;
    const visibility = vi.spyOn(document, "visibilityState", "get").mockReturnValue("hidden");
    await new Promise((resolve) => setTimeout(resolve, 1_500));
    expect(list.mock.calls.length).toBe(callsBeforeHiding);

    visibility.mockReturnValue("visible");
    document.dispatchEvent(new Event("visibilitychange"));
    await waitFor(() => expect(list.mock.calls.length).toBeGreaterThan(callsBeforeHiding));
  });

  it("stops polling once unmounted", async () => {
    await i18n.changeLanguage("zh-CN");
    const list = vi.spyOn(agentService, "listEvaluationArenas").mockResolvedValue({ items: [], nextCursor: null });
    vi.spyOn(agentService, "startEvaluation").mockResolvedValue(arena("queued"));
    const { unmount } = render(<EvaluationCenter />);
    await openWizardAndRun();
    await waitFor(() => expect(screen.getByTestId("evaluation-detail").dataset.selectedOutcome).toBe("queued"));

    unmount();
    const callsAtUnmount = list.mock.calls.length;
    await new Promise((resolve) => setTimeout(resolve, 1_500));
    expect(list.mock.calls.length).toBe(callsAtUnmount);
  });

  it("provides every evaluation label in all registered locales", () => {
    for (const locale of ["en", "zh-CN", "zh-TW", "ja", "ko"]) {
      for (const key of [
        "agents", "filter", "cancel", "diff", "metrics", "loadError", "runError", "cancelError", "artifactPreviewUnavailable",
        "loadMore", "loadingMore",
        "configure", "wizard.step", "wizard.back", "wizard.next", "wizard.cancel", "wizard.review",
        "agentSelection.searchLabel", "agentSelection.searchPlaceholder", "agentSelection.statusLabel", "agentSelection.statusAll",
        "agentSelection.capabilityLabel", "agentSelection.capabilityAll", "agentSelection.resultCount", "agentSelection.selectVisible",
        "agentSelection.maxAgents", "agentSelection.maxAgentsExceeded", "agentSelection.empty",
        "agentStatus.available", "agentStatus.unavailable", "agentStatus.needs-auth", "agentStatus.unknown",
        // 18.12: per-outcome failure-classification explanations, the task-level timeout threshold, and the judge-role disclosure.
        "outcomeExplanation.queued", "outcomeExplanation.running", "outcomeExplanation.succeeded", "outcomeExplanation.task_failed",
        "outcomeExplanation.agent_failed", "outcomeExplanation.timed_out", "outcomeExplanation.stuck", "outcomeExplanation.cancelled",
        "outcomeExplanation.benchmark_error", "timeoutLabel", "judgeUnavailable",
      ]) {
        expect(i18n.getFixedT(locale)(`evaluation.${key}`)).not.toBe(`evaluation.${key}`);
      }
      for (const key of ["selectedCount_one", "selectedCount_other"]) {
        expect(i18n.getFixedT(locale)(`evaluation.${key}`, { count: 1 })).not.toBe(`evaluation.${key}`);
      }
      expect(i18n.getFixedT(locale)("evaluation.timeoutValue", { seconds: 120 })).not.toBe("evaluation.timeoutValue");
    }
  });

  // 18.12: the detail pane's own outcome badge, per-outcome explanation, task timeout threshold,
  // and honest judge-role disclosure -- distinct from the results table's own outcome column.
  // zh-CN throughout, matching this file's own convention: `openWizardAndRun` clicks
  // hardcoded-Chinese button labels ("配置评测"/"下一步"/"运行竞技场"), the same as every other
  // test in this file that renders a full run.
  it("surfaces failure classification, the task timeout threshold, and an honest judge-role disclosure in the detail pane", async () => {
    await i18n.changeLanguage("zh-CN");
    vi.spyOn(agentService, "listEvaluationArenas").mockResolvedValue({ items: [], nextCursor: null });
    vi.spyOn(agentService, "startEvaluation").mockResolvedValue(arenaWithArtifacts([]));
    render(<EvaluationCenter />);
    await openWizardAndRun();
    fireEvent.click(await screen.findByTestId("evaluation-row"));
    const outcomeDetail = within(screen.getByTestId("evaluation-outcome-detail"));
    expect(outcomeDetail.getByText("通过")).toBeTruthy();
    expect(outcomeDetail.getByText("所有确定性检查均已通过。")).toBeTruthy();
    // fix-null-auth-token v1's own real catalog timeout (web-evaluation-client.ts), not a fabricated value.
    expect(outcomeDetail.getByText(/120/)).toBeTruthy();
    expect(outcomeDetail.getByText("此次尝试未记录裁判角色。")).toBeTruthy();
  });

  // 18.12 "bounded reason": the checks list must stay reachable in full (scroll-bound), never
  // silently cut down to a fixed count the way a "+N more" cap would.
  it("keeps the checks list scroll-bound rather than unbounded", async () => {
    await i18n.changeLanguage("zh-CN");
    vi.spyOn(agentService, "listEvaluationArenas").mockResolvedValue({ items: [], nextCursor: null });
    vi.spyOn(agentService, "startEvaluation").mockResolvedValue(arenaWithChecks([{ checkId: "a", passed: true, summary: "42/42" }]));
    render(<EvaluationCenter />);
    await openWizardAndRun();
    fireEvent.click(await screen.findByTestId("evaluation-row"));
    const checksList = within(screen.getByTestId("evaluation-detail")).getByText("PASS").closest("ul");
    expect(checksList?.className).toContain("overflow-auto");
  });

  // 18.13: raw artifact ids used to render inside a copyable <pre> block. There is no
  // navigation/preview target for them anywhere yet, so the honest replacement is a typed,
  // explicitly-unavailable EvidenceLink per id -- never a fabricated working link.
  it("renders artifact ids as unavailable evidence links, not a raw copyable block", async () => {
    await i18n.changeLanguage("zh-CN");
    // Isolates from web-evaluation-client.ts's own module-level `webEvaluationArenas`, which an
    // earlier test in this file (the only one that calls the real, unmocked `startEvaluation`)
    // leaves populated -- without this, the initial load can render leftover rows alongside this
    // test's own, and `findByTestId("evaluation-row")` (singular) fails against the full suite.
    vi.spyOn(agentService, "listEvaluationArenas").mockResolvedValue({ items: [], nextCursor: null });
    vi.spyOn(agentService, "startEvaluation").mockResolvedValue(arenaWithArtifacts(["diff-alpha", "diff-beta"]));
    render(<EvaluationCenter />);
    await openWizardAndRun();
    fireEvent.click(await screen.findByTestId("evaluation-row"));
    const detail = screen.getByTestId("evaluation-detail");
    expect(within(detail).getByText("diff-alpha")).toBeTruthy();
    expect(within(detail).getByText("diff-beta")).toBeTruthy();
    expect(within(detail).queryByRole("link")).toBeNull();
    expect(within(detail).getAllByText("不可用")).toHaveLength(2);
  });

  it("falls back to the unavailable message when an attempt carries no artifacts at all", async () => {
    await i18n.changeLanguage("zh-CN");
    vi.spyOn(agentService, "listEvaluationArenas").mockResolvedValue({ items: [], nextCursor: null });
    vi.spyOn(agentService, "startEvaluation").mockResolvedValue(arenaWithArtifacts([]));
    render(<EvaluationCenter />);
    await openWizardAndRun();
    fireEvent.click(await screen.findByTestId("evaluation-row"));
    expect(within(screen.getByTestId("evaluation-detail")).getByText("不可用")).toBeTruthy();
  });

  // 18.8/18.9/18.10: wiring-level proof that `EvaluationComparisonPanel` actually receives every
  // attempt across all arenas (not the results table's own filtered `visible` subset) and that
  // picking a real baseline/candidate pair renders a real comparison -- the panel's own behavior is
  // already covered thoroughly in isolation (`evaluation-comparison-panel.test.tsx`), this only
  // proves the page composes it correctly.
  it("wires the comparison panel to every loaded attempt and renders a real comparison once two are chosen", async () => {
    await i18n.changeLanguage("zh-CN");
    vi.spyOn(agentService, "listEvaluationArenas").mockResolvedValue({ items: [], nextCursor: null });
    vi.spyOn(agentService, "startEvaluation").mockResolvedValue(arenaWithTwoAttempts());
    render(<EvaluationCenter />);
    await openWizardAndRun();
    await screen.findByTestId("evaluation-comparison");
    fireEvent.change(screen.getByTestId("evaluation-comparison-baseline"), { target: { value: "attempt-one" } });
    fireEvent.change(screen.getByTestId("evaluation-comparison-candidate"), { target: { value: "attempt-two" } });
    expect(screen.getByTestId("evaluation-comparison-result")).toBeTruthy();
  });

  it("provides every evaluation.comparison label in all registered locales", () => {
    for (const locale of ["en", "zh-CN", "zh-TW", "ja", "ko"]) {
      for (const key of [
        "title", "description", "baselineLabel", "candidateLabel", "choosePlaceholder", "attemptOption",
        "needsTwoResults", "selectBoth", "notComparable", "outcomeTier", "outcomeTierReason", "metricDeltas",
        "uncomparedMetrics", "reliability", "reliabilityUnavailable", "evidence", "checksCount", "artifactsCount",
        "sameConfiguration", "differentConfiguration",
        "reason.sameAttempt", "reason.differentTask", "reason.differentVersion", "reason.inProgress",
        "verdict.improved", "verdict.regressed", "verdict.unchanged", "verdict.notRankable",
        "uncomparedReason.missingOnBaseline", "uncomparedReason.missingOnCandidate", "uncomparedReason.unavailableQuality", "uncomparedReason.unitMismatch",
      ]) {
        expect(i18n.getFixedT(locale)(`evaluation.comparison.${key}`)).not.toBe(`evaluation.comparison.${key}`);
      }
    }
  });
});

function arena(outcome: "queued" | "cancelled"): EvaluationArena {
  return { id: "arena-cancel", operationId: "operation-cancel", taskId: "fix-null-auth-token", taskVersion: 1, rankingVersion: "deterministic-v2", attempts: [{ id: "attempt-cancel", arenaId: "arena-cancel", canonicalRunId: "run-cancel", taskId: "fix-null-auth-token", taskVersion: 1, agent: { agentId: "onepiece", providerId: "onepiece", modelId: null, interactionMode: "api", configurationFingerprint: "safe" }, outcome, checks: [], metrics: [{ name: "input_tokens", value: null, unit: "tokens", quality: "unavailable", source: "provider" }], contextEvidenceManifestId: null, artifactIds: [], timeline: [] }] };
}

function arenaWithArtifacts(artifactIds: string[]): EvaluationArena {
  return { id: "arena-artifacts", operationId: "operation-artifacts", taskId: "fix-null-auth-token", taskVersion: 1, rankingVersion: "deterministic-v2", attempts: [{ id: "attempt-artifacts", arenaId: "arena-artifacts", canonicalRunId: "run-artifacts", taskId: "fix-null-auth-token", taskVersion: 1, agent: { agentId: "onepiece", providerId: "onepiece", modelId: null, interactionMode: "api", configurationFingerprint: "safe" }, outcome: "succeeded", checks: [], metrics: [], contextEvidenceManifestId: null, artifactIds, timeline: [] }] };
}

function arenaWithChecks(checks: EvaluationCheck[]): EvaluationArena {
  return { id: "arena-checks", operationId: "operation-checks", taskId: "fix-null-auth-token", taskVersion: 1, rankingVersion: "deterministic-v2", attempts: [{ id: "attempt-checks", arenaId: "arena-checks", canonicalRunId: "run-checks", taskId: "fix-null-auth-token", taskVersion: 1, agent: { agentId: "onepiece", providerId: "onepiece", modelId: null, interactionMode: "api", configurationFingerprint: "safe" }, outcome: "succeeded", checks, metrics: [], contextEvidenceManifestId: null, artifactIds: [], timeline: [] }] };
}

// Two attempts sharing task+version, both terminal with different outcomes -- the minimal real
// shape `checkEligibility` (18.8) reports as comparable.
function arenaWithTwoAttempts(): EvaluationArena {
  const base = { arenaId: "arena-compare", canonicalRunId: "run-compare", taskId: "fix-null-auth-token", taskVersion: 1, checks: [], metrics: [], contextEvidenceManifestId: null, artifactIds: [], timeline: [] };
  return {
    id: "arena-compare", operationId: "operation-compare", taskId: "fix-null-auth-token", taskVersion: 1, rankingVersion: "deterministic-v2",
    attempts: [
      { ...base, id: "attempt-one", agent: { agentId: "onepiece", providerId: "onepiece", modelId: null, interactionMode: "api", configurationFingerprint: "safe" }, outcome: "task_failed" },
      { ...base, id: "attempt-two", agent: { agentId: "codex-cli", providerId: "openai", modelId: null, interactionMode: "cli", configurationFingerprint: "other" }, outcome: "succeeded" },
    ],
  };
}
