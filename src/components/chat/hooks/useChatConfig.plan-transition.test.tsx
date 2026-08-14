// @vitest-environment jsdom

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AgentRegistryEntry, Session } from "../../../types/agent";

const { agentService, planService } = vi.hoisted(() => ({
  agentService: { getSessionChatConfig: vi.fn(), saveSessionChatConfig: vi.fn() },
  planService: { getPlanRunForSession: vi.fn(), getPlanRun: vi.fn(), requestPlanControl: vi.fn() },
}));
vi.mock("../../../services/runtime-agent-client", () => ({ agentService }));
vi.mock("../../../services/runtime-plan-client", () => ({ planService }));

import { useChatConfig } from "./useChatConfig";

const agent: AgentRegistryEntry = {
  id: "onepiece", displayName: "OnePiece", provider: "OnePiece", launch: { kind: "api" },
  supportedInteractionModes: ["api"], availabilityState: "available", capabilityTags: [], agentOrigin: "builtin",
};
const session: Session = {
  id: "session-1", title: "Plan", agentId: "onepiece", interactionMode: "api", lifecycleState: "running",
  recoveryStatus: "clean", recoveryRevision: 0, stateRevision: 0, historyRevision: 0,
  activeExecutionRunId: null, folder: null, projectPath: "D:/app", worktreePath: null,
  worktreeName: null, worktreeBranch: null, remoteWorkspace: null, remoteSshConnectionId: null,
  remoteSshConnectionRevision: null, runtimeSessionId: null, categoryId: null, pinned: false,
  archived: false, createdAt: "2026-08-13T00:00:00Z", updatedAt: "2026-08-13T00:00:00Z",
};
const summary = { id: "run-1", planId: "plan-1", status: "running" as const, completedTasks: 0, totalTasks: 1, simulated: false, createdAt: "2026-08-13T00:00:00Z", updatedAt: "2026-08-13T00:00:00Z" };

describe("OnePiece active Plan mode transition", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    agentService.getSessionChatConfig.mockResolvedValue({ agentId: "onepiece", interactionMode: "api", executionMode: "execute", providerId: "onepiece", modelId: "onepiece-default", streaming: true, thinking: true, longContext: true });
    agentService.saveSessionChatConfig.mockImplementation(async (_sessionId: string, config: object) => config);
    planService.getPlanRunForSession.mockResolvedValue(summary);
    planService.requestPlanControl.mockResolvedValue({ requestId: "pause-1", run: { ...summary, status: "pause_requested" } });
  });

  it("keeps Agent mode visible until the associated run reaches a safe paused boundary", async () => {
    planService.getPlanRun
      .mockResolvedValueOnce({ ...summary, status: "pause_requested" })
      .mockResolvedValueOnce({ ...summary, status: "paused" });
    const report = vi.fn();
    const { result } = renderHook(() => useChatConfig({ activeSession: session, agents: [agent], onPersistError: report }));
    await waitFor(() => expect(result.current.config.executionMode).toBe("execute"));
    await waitFor(() => expect(result.current.associatedPlanRun?.id).toBe("run-1"));

    act(() => result.current.setSessionExecutionMode("plan"));
    expect(result.current.config.executionMode).toBe("execute");
    await waitFor(() => expect(planService.requestPlanControl).toHaveBeenCalledWith("run-1", "pause"));
    await waitFor(() => expect(result.current.config.executionMode).toBe("plan"), { timeout: 2_000 });
    expect(report).not.toHaveBeenCalled();
  });
});
