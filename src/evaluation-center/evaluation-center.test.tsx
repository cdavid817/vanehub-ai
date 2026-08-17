// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { i18n } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import type { EvaluationArena } from "../types/evaluation";
import { EvaluationCenter } from "./evaluation-center";

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

describe("EvaluationCenter", () => {
  it("configures, compares, filters, inspects, and exports a mock arena", async () => {
    await i18n.changeLanguage("zh-CN");
    const exportSpy = vi.spyOn(agentService, "exportEvaluation");
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => undefined);
    vi.stubGlobal("URL", { createObjectURL: () => "blob:evaluation", revokeObjectURL: vi.fn() });
    render(<EvaluationCenter />);
    const run = await screen.findByRole("button", { name: "运行竞技场" });
    await waitFor(() => expect((run as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(run);
    expect(await screen.findByText("onepiece")).toBeTruthy();
    expect(screen.getAllByText("codex-cli").length).toBeGreaterThan(1);
    fireEvent.change(screen.getByLabelText("筛选结果"), { target: { value: "codex-cli" } });
    expect(document.querySelectorAll("tbody tr")).toHaveLength(1);
    fireEvent.click(screen.getByText("任务失败").closest("tr")!);
    expect(screen.getByText("验证")).toBeTruthy();
    expect(screen.getByText("指标与来源")).toBeTruthy();
    expect(screen.getByText(/unavailable · provider/)).toBeTruthy();
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
    const run = await screen.findByRole("button", { name: "运行竞技场" });
    await waitFor(() => expect((run as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(run);
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

  it("provides every evaluation label in all registered locales", () => {
    for (const locale of ["en", "zh-CN", "zh-TW", "ja", "ko"]) {
      for (const key of ["agents", "filter", "cancel", "diff", "metrics", "loadError", "runError", "cancelError"]) {
        expect(i18n.getFixedT(locale)(`evaluation.${key}`)).not.toBe(`evaluation.${key}`);
      }
    }
  });
});

function arena(outcome: "queued" | "cancelled"): EvaluationArena {
  return { id: "arena-cancel", operationId: "operation-cancel", taskId: "fix-null-auth-token", taskVersion: 1, rankingVersion: "deterministic-v1", attempts: [{ id: "attempt-cancel", arenaId: "arena-cancel", canonicalRunId: "run-cancel", taskId: "fix-null-auth-token", taskVersion: 1, agent: { agentId: "onepiece", providerId: "onepiece", modelId: null, interactionMode: "api", configurationFingerprint: "safe" }, outcome, checks: [], metrics: [{ name: "input_tokens", value: null, unit: "tokens", quality: "unavailable", source: "provider" }], contextEvidenceManifestId: null, artifactIds: [], timeline: [] }] };
}
