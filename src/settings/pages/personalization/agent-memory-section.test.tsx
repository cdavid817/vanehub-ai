// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { SettingsProvider } from "../../settings-provider";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { AgentMemory } from "../../../types/agent";
import { AgentMemorySection } from "./agent-memory-section";

function renderSection(service: ReturnType<typeof createAgentServiceDouble>) {
  return renderWithAppProviders(
    <SettingsProvider>
      <AgentMemorySection service={service} />
    </SettingsProvider>,
  );
}

describe("AgentMemorySection", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("shows an empty state when the shared pool has no memories", async () => {
    const service = createAgentServiceDouble({ listAllMemories: async () => [] });
    renderSection(service);
    expect(await screen.findByText("暂无已保存的记忆。")).toBeTruthy();
  });

  it("lists memories from every agent with source, agent, and folder, and deletes one on confirm", async () => {
    // `add-cli-memory-support`: the shared pool holds memories from more than one agent —
    // deliberately mixing `onepiece` and `codex-cli` here proves the list isn't still scoped to
    // a single hardcoded agent id.
    let memories: AgentMemory[] = [
      { id: "memory-1", agentId: "onepiece", folder: "D:/project", content: "Uses pnpm.", source: "explicit", createdAt: "2026-01-01T00:00:00.000Z" },
      { id: "memory-2", agentId: "codex-cli", folder: null, content: "Prefers concise responses.", source: "automatic", createdAt: "2026-01-02T00:00:00.000Z" },
    ];
    const deleteAgentMemory = vi.fn(async (memoryId: string) => {
      memories = memories.filter((memory) => memory.id !== memoryId);
    });
    const service = createAgentServiceDouble({
      listAllMemories: async () => memories,
      deleteAgentMemory,
    });
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const { user } = renderSection(service);

    expect(await screen.findByText("Uses pnpm.")).toBeTruthy();
    expect(screen.getByText("Prefers concise responses.")).toBeTruthy();
    expect(screen.getByText("主动要求记住")).toBeTruthy();
    expect(screen.getByText("自动提取")).toBeTruthy();
    expect(screen.getByText("所有项目")).toBeTruthy();
    expect(screen.getByText("onepiece")).toBeTruthy();
    expect(screen.getByText("codex-cli")).toBeTruthy();

    const deleteButtons = screen.getAllByRole("button", { name: "删除" });
    await user.click(deleteButtons[0]!);

    expect(window.confirm).toHaveBeenCalled();
    expect(deleteAgentMemory).toHaveBeenCalledWith("memory-1");
    await waitFor(() => expect(screen.queryByText("Uses pnpm.")).toBeNull());
    expect(screen.getByText("Prefers concise responses.")).toBeTruthy();
  });

  it("does not delete when the confirmation is dismissed", async () => {
    const memories: AgentMemory[] = [
      { id: "memory-1", agentId: "onepiece", folder: null, content: "Uses pnpm.", source: "explicit", createdAt: "2026-01-01T00:00:00.000Z" },
    ];
    const deleteAgentMemory = vi.fn(async () => undefined);
    const service = createAgentServiceDouble({ listAllMemories: async () => memories, deleteAgentMemory });
    vi.spyOn(window, "confirm").mockReturnValue(false);
    const { user } = renderSection(service);

    await screen.findByText("Uses pnpm.");
    await user.click(screen.getByRole("button", { name: "删除" }));

    expect(window.confirm).toHaveBeenCalled();
    expect(deleteAgentMemory).not.toHaveBeenCalled();
    expect(screen.getByText("Uses pnpm.")).toBeTruthy();
  });

  it("resets every agent's memories on confirm", async () => {
    const memories: AgentMemory[] = [
      { id: "memory-1", agentId: "onepiece", folder: null, content: "Uses pnpm.", source: "explicit", createdAt: "2026-01-01T00:00:00.000Z" },
    ];
    const resetAllMemories = vi.fn(async () => undefined);
    const service = createAgentServiceDouble({ listAllMemories: async () => memories, resetAllMemories });
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const { user } = renderSection(service);

    await screen.findByText("Uses pnpm.");
    await user.click(screen.getByRole("button", { name: "重置全部" }));

    expect(window.confirm).toHaveBeenCalled();
    expect(resetAllMemories).toHaveBeenCalledWith();
  });

  it("does not reset when the confirmation is dismissed", async () => {
    const memories: AgentMemory[] = [
      { id: "memory-1", agentId: "onepiece", folder: null, content: "Uses pnpm.", source: "explicit", createdAt: "2026-01-01T00:00:00.000Z" },
    ];
    const resetAllMemories = vi.fn(async () => undefined);
    const service = createAgentServiceDouble({ listAllMemories: async () => memories, resetAllMemories });
    vi.spyOn(window, "confirm").mockReturnValue(false);
    const { user } = renderSection(service);

    await screen.findByText("Uses pnpm.");
    await user.click(screen.getByRole("button", { name: "重置全部" }));

    expect(resetAllMemories).not.toHaveBeenCalled();
  });

  it("disables the tool-assisted toggle when the memory master switch is off", async () => {
    const service = createAgentServiceDouble({ listAllMemories: async () => [] });
    window.localStorage.setItem(
      "vanehub.appSettings",
      JSON.stringify({ memoryEnabled: false, memoryToolAssistedChatsEnabled: true }),
    );
    renderSection(service);

    await screen.findByText("暂无已保存的记忆。");
    const toolAssistedToggle = screen.getByRole("switch", { name: "从工具辅助的会话中记忆" });
    expect(toolAssistedToggle).toHaveProperty("disabled", true);
  });
});
