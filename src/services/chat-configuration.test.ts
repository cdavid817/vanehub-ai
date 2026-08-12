import { describe, expect, it } from "vitest";
import type { Session } from "../types/agent";
import type { ChatConfig } from "../types/chat";
import { normalizeChatConfigForSession } from "./chat-configuration";

const session: Session = {
  id: "session-config-boundary",
  title: "Boundary",
  agentId: "gemini-cli",
  interactionMode: "browser",
  lifecycleState: "idle",
  recoveryStatus: "clean",
  recoveryRevision: 0,
  stateRevision: 0,
  historyRevision: 0,
  activeExecutionRunId: null,
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

describe("session chat configuration boundary", () => {
  it("restores session identity and normalizes an incompatible provider/model", () => {
    const input: ChatConfig = {
      agentId: "codex-cli",
      interactionMode: "cli",
      executionMode: "execute",
      providerId: "openai",
      modelId: "gpt-5-5",
      reasoningDepth: "max",
      streaming: true,
      thinking: true,
      longContext: true,
    };

    expect(normalizeChatConfigForSession(session, input)).toMatchObject({
      agentId: "gemini-cli",
      interactionMode: "browser",
      providerId: "google",
      modelId: "gemini-2-5-pro",
      executionMode: "execute",
      reasoningDepth: "high",
    });
  });

  it("keeps OnePiece on its native provider identity", () => {
    const onePieceSession = { ...session, agentId: "onepiece", interactionMode: "api" as const };
    const input: ChatConfig = {
      agentId: "onepiece",
      interactionMode: "api",
      executionMode: "inherit",
      providerId: "onepiece",
      modelId: "deepseek-chat",
      streaming: true,
      thinking: true,
      longContext: true,
    };

    expect(normalizeChatConfigForSession(onePieceSession, input)).toMatchObject({
      agentId: "onepiece",
      interactionMode: "api",
      providerId: "onepiece",
      modelId: "deepseek-chat",
    });
  });
});
