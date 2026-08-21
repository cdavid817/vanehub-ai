// @vitest-environment jsdom

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AgentRegistryEntry, Session } from "../../../types/agent";

const { agentService } = vi.hoisted(() => ({
  agentService: { getSessionChatConfig: vi.fn(), saveSessionChatConfig: vi.fn() },
}));
vi.mock("../../../services/runtime-agent-client", () => ({ agentService }));

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

describe("OnePiece session Plan mode", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    agentService.getSessionChatConfig.mockResolvedValue({
      agentId: "onepiece", interactionMode: "api", executionMode: "plan",
      providerId: "onepiece", modelId: "onepiece-default", streaming: true,
      thinking: true, longContext: true,
    });
    agentService.saveSessionChatConfig.mockImplementation(async (_sessionId: string, config: object) => config);
  });

  it("keeps a declined exit in Plan and applies approval only to a later persisted turn", async () => {
    const { result, rerender } = renderHook(
      ({ approvedPlanExit }: { approvedPlanExit: string | null }) => useChatConfig({
        activeSession: session, agents: [agent], approvedPlanExit,
      }),
      { initialProps: { approvedPlanExit: null as string | null } },
    );
    await waitFor(() => expect(result.current.config.executionMode).toBe("plan"));

    rerender({ approvedPlanExit: null });
    expect(result.current.config.executionMode).toBe("plan");

    rerender({ approvedPlanExit: "call-approved" });
    await waitFor(() => expect(result.current.config.executionMode).toBe("execute"));
    await waitFor(() => expect(agentService.saveSessionChatConfig).toHaveBeenCalledWith(
      "session-1", expect.objectContaining({ executionMode: "execute" }),
    ));

    act(() => result.current.setSessionExecutionMode("plan"));
    await waitFor(() => expect(result.current.config.executionMode).toBe("plan"));
  });
});
