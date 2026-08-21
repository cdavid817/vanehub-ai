import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ChatConfig } from "../types/chat";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

import { tauriAgentClient } from "./tauri-agent-client";

const planConfig: ChatConfig = {
  agentId: "onepiece",
  interactionMode: "api",
  executionMode: "plan",
  providerId: "anthropic",
  modelId: "claude-opus-4-8",
  streaming: true,
  thinking: false,
  longContext: false,
};

describe("session Plan-mode adapter", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(planConfig);
  });

  it("persists OnePiece Plan mode through the Tauri AgentService boundary", async () => {
    await expect(tauriAgentClient.saveSessionChatConfig("session-1", planConfig)).resolves.toEqual(planConfig);
    expect(invokeMock).toHaveBeenCalledWith("save_session_chat_config", {
      sessionId: "session-1",
      config: planConfig,
    });
  });
});
