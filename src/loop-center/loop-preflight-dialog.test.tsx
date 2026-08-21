// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { loopQueryKeys } from "../hooks/loop-query";
import { agentService } from "../services/runtime-agent-client";
import { loopDefinitionFixture, loopRunFixture } from "../test/loop-fixtures";
import type { LoopReadinessReport } from "../types/loop";
import { LoopPreflightDialog } from "./loop-preflight-dialog";

const readyReport: LoopReadinessReport = {
  definitionId: "definition-1", ready: true, simulated: true, checkedAt: "2026-08-21T00:00:00Z",
  checks: [{ code: "definition-enabled", category: "definition", status: "passed", blocking: true, detail: null, remediationTarget: null }],
};

describe("LoopPreflightDialog", () => {
  afterEach(() => vi.restoreAllMocks());

  it("starts only after a passing readiness report", async () => {
    const start = vi.spyOn(agentService, "startLoop").mockResolvedValue({ run: loopRunFixture("queued"), operationId: "operation-1" });
    const onStarted = vi.fn();
    renderPreflight(readyReport, onStarted);
    expect(screen.getByText("模拟运行")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "启动循环" }));
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith("run-1"));
    expect(start).toHaveBeenCalledWith("definition-1");
  });

  it("shows remediation and disables start for a blocked report", () => {
    renderPreflight({ ...readyReport, ready: false, checks: [{ ...readyReport.checks[0], status: "blocked", remediationTarget: "definition" }] }, vi.fn());
    expect(screen.getByText("请启用或编辑此定义。")).toBeTruthy();
    expect((screen.getByRole("button", { name: "启动循环" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("refreshes readiness and preserves the dialog after an authoritative start rejection", async () => {
    const blocked = { ...readyReport, ready: false, checks: [{ ...readyReport.checks[0], status: "blocked" as const, remediationTarget: "runs" as const }] };
    const start = vi.spyOn(agentService, "startLoop").mockRejectedValue(new Error("active run race"));
    vi.spyOn(agentService, "checkLoopReadiness").mockResolvedValueOnce(readyReport).mockResolvedValue(blocked);
    renderPreflight(readyReport, vi.fn());

    await userEvent.click(screen.getByRole("button", { name: "启动循环" }));
    await waitFor(() => expect(screen.getByText("请先完成或停止活动运行。")).toBeTruthy());
    expect(start).toHaveBeenCalledWith("definition-1");
    expect(screen.getByRole("dialog", { name: "运行就绪检查" })).toBeTruthy();
  });
});

function renderPreflight(report: LoopReadinessReport, onStarted: (runId: string) => void) {
  const client = new QueryClient({ defaultOptions: { mutations: { retry: false }, queries: { retry: false } } });
  client.setQueryData(loopQueryKeys.readiness("definition-1"), report);
  render(<QueryClientProvider client={client}><LoopPreflightDialog definition={loopDefinitionFixture()} onClose={() => undefined} onEdit={() => undefined} onStarted={onStarted} /></QueryClientProvider>);
}
