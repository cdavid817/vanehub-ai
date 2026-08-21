// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { agentService } from "../services/runtime-agent-client";
import { loopEvidenceFixture, loopIterationFixture, loopRunFixture } from "../test/loop-fixtures";
import { LoopRunControls } from "./loop-run-controls";
import { LoopTimeline } from "./loop-timeline";

describe("Loop run workspace", () => {
  afterEach(() => vi.restoreAllMocks());

  it("shows comparison summaries and keeps raw evidence collapsed", () => {
    const previous = loopIterationFixture({ sequence: 1, evidence: [
      loopEvidenceFixture({ id: "old-check", kind: "verification", commandId: "lint", status: "failed", details: null }),
      loopEvidenceFixture({ id: "old-change", details: { changedFiles: 1, additions: 2, deletions: 0 } }),
    ] });
    const current = loopIterationFixture({ id: "iteration-2", sequence: 2, evidence: [
      loopEvidenceFixture({ id: "new-check", kind: "verification", commandId: "tests", status: "failed", details: null }),
      loopEvidenceFixture({ id: "new-change", details: { changedFiles: 3, additions: 8, deletions: 1 } }),
    ] });
    const { container } = renderWithClient(<LoopTimeline run={loopRunFixture("awaiting-acceptance", { currentIteration: 2, iterations: [previous, current] })} />);
    expect(screen.getByText("已解决的失败检查：lint")).toBeTruthy();
    expect(screen.getByText("新增失败检查：tests")).toBeTruthy();
    const rawEvidence = container.querySelector("details details") as HTMLDetailsElement;
    expect(rawEvidence.open).toBe(false);
  });

  it("shows exhausted continuation and recovery/no-progress guidance", () => {
    const exhausted = loopRunFixture("awaiting-acceptance", { currentIteration: 3 });
    exhausted.definitionSnapshot.limits.maxIterations = 3;
    const first = renderWithClient(<LoopTimeline run={exhausted} />);
    expect(screen.getByText("已达到最大迭代次数，请接受或拒绝此结果。")).toBeTruthy();
    first.unmount();

    const recovery = renderWithClient(<LoopTimeline run={loopRunFixture("paused", { terminalReason: "recovery-required" })} />);
    expect(screen.getByText(/请先检查已保留的证据/)).toBeTruthy();
    recovery.unmount();
    renderWithClient(<LoopTimeline run={loopRunFixture("failed", { terminalReason: "no-progress" })} />);
    expect(screen.getByText("无有效进展")).toBeTruthy();
  });

  it("recovers the mutation controls after an action failure", async () => {
    vi.spyOn(agentService, "pauseLoop").mockRejectedValue(new Error("pause failed"));
    renderWithClient(<LoopRunControls run={loopRunFixture("running")} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "暂停" }));
    await user.click(screen.getByRole("button", { name: "确认" }));
    await waitFor(() => expect(screen.getByText("pause failed")).toBeTruthy());
    expect((screen.getByRole("button", { name: "暂停" }) as HTMLButtonElement).disabled).toBe(false);
  });
});

function renderWithClient(node: React.ReactNode) {
  const client = new QueryClient({ defaultOptions: { mutations: { retry: false }, queries: { retry: false } } });
  return render(<QueryClientProvider client={client}>{node}</QueryClientProvider>);
}
