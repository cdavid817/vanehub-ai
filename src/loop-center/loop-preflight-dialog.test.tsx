// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { loopQueryKeys } from "../hooks/loop-query";
import { agentService } from "../services/runtime-agent-client";
import { loopAssessmentFixture, loopDefinitionFixture, loopRunFixture } from "../test/loop-fixtures";
import type { LoopAdmission, LoopReadinessReport } from "../types/loop";
import { LoopPreflightDialog } from "./loop-preflight-dialog";

const readyReport: LoopReadinessReport = {
  definitionId: "definition-1", ready: true, simulated: true, checkedAt: "2026-08-21T00:00:00Z",
  checks: [{ code: "definition-enabled", category: "definition", status: "passed", blocking: true, detail: null, remediationTarget: null }],
  requestedMode: "preventive-required", scopeState: "verified", assessment: loopAssessmentFixture(), acknowledgementRequired: false, definitionRevision: 1,
};

const strictAdmission: LoopAdmission = {
  action: "start", targetId: "definition-1", expectedRevision: 1, requestedMode: "preventive-required", scopeDigest: "sha256:scope",
  allowedPaths: ["src"], protectedPaths: [".git"], assessment: loopAssessmentFixture(), acknowledgementRequired: false, challengeId: null, expiresAt: null, simulated: false,
};

describe("LoopPreflightDialog", () => {
  afterEach(() => vi.restoreAllMocks());

  it("starts only after a passing readiness report and a fresh admission", async () => {
    vi.spyOn(agentService, "prepareLoopAdmission").mockResolvedValue(strictAdmission);
    const start = vi.spyOn(agentService, "startLoop").mockResolvedValue({ run: loopRunFixture("queued"), operationId: "operation-1" });
    const onStarted = vi.fn();
    renderPreflight(readyReport, onStarted);
    expect(screen.getByText("模拟运行")).toBeTruthy();
    expect(screen.getByText("执行覆盖度")).toBeTruthy();
    expect(screen.getByText(/verification:tests: artifact-validation-only/)).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "启动循环" }));
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith("run-1"));
    expect(start).toHaveBeenCalledWith("definition-1", { expectedRevision: 1 });
  });

  it("shows remediation and disables start for a blocked report", () => {
    renderPreflight({ ...readyReport, ready: false, checks: [{ ...readyReport.checks[0], status: "blocked", remediationTarget: "definition" }] }, vi.fn());
    expect(screen.getByText("请启用或编辑此定义。")).toBeTruthy();
    expect((screen.getByRole("button", { name: "启动循环" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("refreshes readiness and preserves the dialog after an authoritative start rejection", async () => {
    const blocked = { ...readyReport, ready: false, checks: [{ ...readyReport.checks[0], status: "blocked" as const, remediationTarget: "runs" as const }] };
    vi.spyOn(agentService, "prepareLoopAdmission").mockResolvedValue(strictAdmission);
    const start = vi.spyOn(agentService, "startLoop").mockRejectedValue(new Error("active run race"));
    vi.spyOn(agentService, "checkLoopReadiness").mockResolvedValueOnce(readyReport).mockResolvedValue(blocked);
    renderPreflight(readyReport, vi.fn());

    await userEvent.click(screen.getByRole("button", { name: "启动循环" }));
    await waitFor(() => expect(screen.getByText("请先完成或停止活动运行。")).toBeTruthy());
    expect(start).toHaveBeenCalledWith("definition-1", { expectedRevision: 1 });
    expect(screen.getByRole("dialog", { name: "运行就绪检查" })).toBeTruthy();
  });

  it("requires an explicit acknowledgement before an artifact-audited start and binds its receipt to the start", async () => {
    const audited = loopAssessmentFixture({ requestedMode: "artifact-audited", acknowledgementRequired: true, limitations: ["worker: artifact-validation-only — codex executes with no host mediation."] });
    vi.spyOn(agentService, "prepareLoopAdmission").mockResolvedValue({ ...strictAdmission, requestedMode: "artifact-audited", assessment: audited, acknowledgementRequired: true, challengeId: "challenge-1", expiresAt: "2026-08-21T00:05:00Z" });
    const acknowledge = vi.spyOn(agentService, "acknowledgeLoopAudit").mockResolvedValue({ acknowledgementId: "receipt-1", action: "start", targetId: "definition-1", expectedRevision: 1, expiresAt: "2026-08-21T00:05:00Z" });
    const start = vi.spyOn(agentService, "startLoop").mockResolvedValue({ run: loopRunFixture("queued"), operationId: "operation-1" });
    const onStarted = vi.fn();
    renderPreflight({ ...readyReport, requestedMode: "artifact-audited", assessment: audited, acknowledgementRequired: true }, onStarted);

    await userEvent.click(screen.getByRole("button", { name: "启动循环" }));
    await waitFor(() => expect(screen.getByRole("alertdialog", { name: "在启动前确认局限" })).toBeTruthy());
    expect(start).not.toHaveBeenCalled();
    expect(screen.getAllByText("worker: artifact-validation-only — codex executes with no host mediation.")).toHaveLength(2);
    await userEvent.click(screen.getByRole("button", { name: "确认局限并启动" }));
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith("run-1"));
    expect(acknowledge).toHaveBeenCalledWith("challenge-1");
    expect(start).toHaveBeenCalledWith("definition-1", { expectedRevision: 1, auditAcknowledgementId: "receipt-1" });
  });

  it("surfaces admission blockers instead of starting when coverage no longer satisfies the mode", async () => {
    vi.spyOn(agentService, "prepareLoopAdmission").mockResolvedValue({ ...strictAdmission, assessment: loopAssessmentFixture({ satisfiesRequestedMode: false, blockers: ["worker: artifact-validation-only does not satisfy preventive-required"] }) });
    const start = vi.spyOn(agentService, "startLoop");
    renderPreflight(readyReport, vi.fn());
    await userEvent.click(screen.getByRole("button", { name: "启动循环" }));
    await waitFor(() => expect(screen.getByText("worker: artifact-validation-only does not satisfy preventive-required")).toBeTruthy());
    expect(start).not.toHaveBeenCalled();
  });
});

function renderPreflight(report: LoopReadinessReport, onStarted: (runId: string) => void) {
  const client = new QueryClient({ defaultOptions: { mutations: { retry: false }, queries: { retry: false } } });
  client.setQueryData(loopQueryKeys.readiness("definition-1"), report);
  render(<QueryClientProvider client={client}><LoopPreflightDialog definition={loopDefinitionFixture()} onClose={() => undefined} onEdit={() => undefined} onStarted={onStarted} /></QueryClientProvider>);
}
