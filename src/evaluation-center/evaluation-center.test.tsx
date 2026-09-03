// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { i18n } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import type { EvaluationArena } from "../types/evaluation";
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
    fireEvent.click((await screen.findByText("已排队")).closest("tr")!);
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    await waitFor(() => expect(screen.getByText("已取消")).toBeTruthy());
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
    const list = vi.spyOn(agentService, "listEvaluationArenas").mockResolvedValue([]);
    vi.spyOn(agentService, "startEvaluation").mockResolvedValue(arena("queued"));
    render(<EvaluationCenter />);
    await openWizardAndRun();
    await waitFor(() => expect(screen.getByTestId("evaluation-detail").dataset.selectedOutcome).toBe("queued"));
    expect(screen.getByTestId("evaluation-cancel")).toBeTruthy();

    list.mockResolvedValue([arena("cancelled")]);
    await waitFor(
      () => expect(screen.getByTestId("evaluation-detail").dataset.selectedOutcome).toBe("cancelled"),
      { timeout: 4_000 },
    );
    expect(screen.queryByTestId("evaluation-cancel")).toBeNull();
  });

  it("pauses polling while the document is hidden, and reconciles immediately once visible again", async () => {
    await i18n.changeLanguage("zh-CN");
    const list = vi.spyOn(agentService, "listEvaluationArenas").mockResolvedValue([]);
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
    const list = vi.spyOn(agentService, "listEvaluationArenas").mockResolvedValue([]);
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
        "configure", "wizard.step", "wizard.back", "wizard.next", "wizard.cancel", "wizard.review",
        "agentSelection.searchLabel", "agentSelection.searchPlaceholder", "agentSelection.statusLabel", "agentSelection.statusAll",
        "agentSelection.capabilityLabel", "agentSelection.capabilityAll", "agentSelection.resultCount", "agentSelection.selectVisible",
        "agentSelection.maxAgents", "agentSelection.maxAgentsExceeded", "agentSelection.empty",
        "agentStatus.available", "agentStatus.unavailable", "agentStatus.needs-auth", "agentStatus.unknown",
      ]) {
        expect(i18n.getFixedT(locale)(`evaluation.${key}`)).not.toBe(`evaluation.${key}`);
      }
      for (const key of ["selectedCount_one", "selectedCount_other"]) {
        expect(i18n.getFixedT(locale)(`evaluation.${key}`, { count: 1 })).not.toBe(`evaluation.${key}`);
      }
    }
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
    vi.spyOn(agentService, "listEvaluationArenas").mockResolvedValue([]);
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
    vi.spyOn(agentService, "listEvaluationArenas").mockResolvedValue([]);
    vi.spyOn(agentService, "startEvaluation").mockResolvedValue(arenaWithArtifacts([]));
    render(<EvaluationCenter />);
    await openWizardAndRun();
    fireEvent.click(await screen.findByTestId("evaluation-row"));
    expect(within(screen.getByTestId("evaluation-detail")).getByText("不可用")).toBeTruthy();
  });
});

function arena(outcome: "queued" | "cancelled"): EvaluationArena {
  return { id: "arena-cancel", operationId: "operation-cancel", taskId: "fix-null-auth-token", taskVersion: 1, rankingVersion: "deterministic-v2", attempts: [{ id: "attempt-cancel", arenaId: "arena-cancel", canonicalRunId: "run-cancel", taskId: "fix-null-auth-token", taskVersion: 1, agent: { agentId: "onepiece", providerId: "onepiece", modelId: null, interactionMode: "api", configurationFingerprint: "safe" }, outcome, checks: [], metrics: [{ name: "input_tokens", value: null, unit: "tokens", quality: "unavailable", source: "provider" }], contextEvidenceManifestId: null, artifactIds: [], timeline: [] }] };
}

function arenaWithArtifacts(artifactIds: string[]): EvaluationArena {
  return { id: "arena-artifacts", operationId: "operation-artifacts", taskId: "fix-null-auth-token", taskVersion: 1, rankingVersion: "deterministic-v2", attempts: [{ id: "attempt-artifacts", arenaId: "arena-artifacts", canonicalRunId: "run-artifacts", taskId: "fix-null-auth-token", taskVersion: 1, agent: { agentId: "onepiece", providerId: "onepiece", modelId: null, interactionMode: "api", configurationFingerprint: "safe" }, outcome: "succeeded", checks: [], metrics: [], contextEvidenceManifestId: null, artifactIds, timeline: [] }] };
}
