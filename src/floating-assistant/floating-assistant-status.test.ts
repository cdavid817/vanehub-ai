import { describe, expect, it } from "vitest";
import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import { resolveFloatingAssistantStatus } from "./floating-assistant-status";

const session: Session = {
  id: "floating-status",
  title: "Status",
  agentId: "codex-cli",
  interactionMode: "cli",
  lifecycleState: "idle",
  folder: null,
  projectPath: null,
  worktreePath: null,
  worktreeName: null,
  worktreeBranch: null,
  remoteWorkspace: null,
  remoteSshConnectionId: null,
  remoteSshConnectionRevision: null,
  runtimeSessionId: null,
  categoryId: null,
  pinned: false,
  archived: false,
  createdAt: "2026-07-17T00:00:00.000Z",
  updatedAt: "2026-07-17T00:00:00.000Z",
};

function assistant(status: ChatMessage["status"]): ChatMessage {
  return {
    id: `message-${status}`,
    sessionId: session.id,
    role: "assistant",
    content: "",
    status,
    createdAt: session.createdAt,
    updatedAt: session.updatedAt,
  };
}

describe("floating assistant lifecycle status", () => {
  it("covers unavailable, idle, running, failed, and stopped states", () => {
    expect(resolveFloatingAssistantStatus(null, [])).toBe("unavailable");
    expect(resolveFloatingAssistantStatus(session, [])).toBe("idle");
    expect(resolveFloatingAssistantStatus(session, [assistant("streaming")])).toBe("running");
    expect(resolveFloatingAssistantStatus(session, [assistant("failed")])).toBe("failed");
    expect(resolveFloatingAssistantStatus(session, [assistant("cancelled")])).toBe("stopped");
  });

  it("preserves the explicit starting lifecycle while a stream is being prepared", () => {
    expect(resolveFloatingAssistantStatus({ ...session, lifecycleState: "starting" }, [assistant("streaming")]))
      .toBe("starting");
  });
});
