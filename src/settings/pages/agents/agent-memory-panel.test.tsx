// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { AgentMemory } from "../../../types/agent";
import { AgentMemoryPanel } from "./agent-memory-panel";

describe("AgentMemoryPanel", () => {
  afterEach(() => vi.restoreAllMocks());

  it("prompts to select an agent when none is active", () => {
    const service = createAgentServiceDouble({});
    renderWithAppProviders(<AgentMemoryPanel agentId={null} service={service} />);
    expect(screen.getByText("选择一个 Agent 以查看其记忆。")).toBeTruthy();
  });

  it("shows an empty state for an agent with no memories", async () => {
    const service = createAgentServiceDouble({ listAgentMemories: async () => [] });
    renderWithAppProviders(<AgentMemoryPanel agentId="my-agent" service={service} />);
    expect(await screen.findByText("暂无保存的记忆。")).toBeTruthy();
  });

  it("lists memories with source and folder, and deletes one on confirm", async () => {
    let memories: AgentMemory[] = [
      { id: "memory-1", agentId: "my-agent", folder: "D:/project", content: "Uses pnpm.", source: "explicit", createdAt: "2026-01-01T00:00:00.000Z" },
      { id: "memory-2", agentId: "my-agent", folder: null, content: "Prefers concise responses.", source: "automatic", createdAt: "2026-01-02T00:00:00.000Z" },
    ];
    const deleteAgentMemory = vi.fn(async (memoryId: string) => {
      memories = memories.filter((memory) => memory.id !== memoryId);
    });
    const service = createAgentServiceDouble({
      listAgentMemories: async () => memories,
      deleteAgentMemory,
    });
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const { user } = renderWithAppProviders(<AgentMemoryPanel agentId="my-agent" service={service} />);

    expect(await screen.findByText("Uses pnpm.")).toBeTruthy();
    expect(screen.getByText("Prefers concise responses.")).toBeTruthy();
    expect(screen.getByText("主动记住")).toBeTruthy();
    expect(screen.getByText("自动提取")).toBeTruthy();
    expect(screen.getByText("Agent 全局")).toBeTruthy();

    const deleteButtons = screen.getAllByRole("button", { name: "删除" });
    await user.click(deleteButtons[0]!);

    expect(window.confirm).toHaveBeenCalled();
    expect(deleteAgentMemory).toHaveBeenCalledWith("memory-1");
    await waitFor(() => expect(screen.queryByText("Uses pnpm.")).toBeNull());
    expect(screen.getByText("Prefers concise responses.")).toBeTruthy();
  });

  it("does not delete when the confirmation is dismissed", async () => {
    const memories: AgentMemory[] = [
      { id: "memory-1", agentId: "my-agent", folder: null, content: "Uses pnpm.", source: "explicit", createdAt: "2026-01-01T00:00:00.000Z" },
    ];
    const deleteAgentMemory = vi.fn(async () => undefined);
    const service = createAgentServiceDouble({ listAgentMemories: async () => memories, deleteAgentMemory });
    vi.spyOn(window, "confirm").mockReturnValue(false);
    const { user } = renderWithAppProviders(<AgentMemoryPanel agentId="my-agent" service={service} />);

    await screen.findByText("Uses pnpm.");
    await user.click(screen.getByRole("button", { name: "删除" }));

    expect(window.confirm).toHaveBeenCalled();
    expect(deleteAgentMemory).not.toHaveBeenCalled();
    expect(screen.getByText("Uses pnpm.")).toBeTruthy();
  });
});
