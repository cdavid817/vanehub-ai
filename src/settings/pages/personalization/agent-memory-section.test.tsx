// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
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
      { id: "npm-only.md", agentId: "onepiece", folder: "D:/project", name: "npm-only", description: "Uses pnpm", memoryType: "feedback", content: "Uses pnpm.", source: "explicit", createdAt: "2026-01-01T00:00:00.000Z" },
      // Untyped on purpose: a migrated or hand-written memory declares no type, and the row must
      // still render rather than showing an "unknown" placeholder.
      { id: "concise.md", agentId: "codex-cli", folder: null, name: "concise", description: "Prefers concise responses", memoryType: null, content: "Prefers concise responses.", source: "automatic", createdAt: "2026-01-02T00:00:00.000Z" },
    ];
    const deleteAgentMemory = vi.fn(async (memoryId: string) => {
      memories = memories.filter((memory) => memory.id !== memoryId);
    });
    const service = createAgentServiceDouble({
      listAllMemories: async () => memories,
      deleteAgentMemory,
    });
    const { user } = renderSection(service);

    expect(await screen.findByText("Uses pnpm.")).toBeTruthy();
    expect(screen.getByText("Prefers concise responses.")).toBeTruthy();
    expect(screen.getByText("主动要求记住")).toBeTruthy();
    expect(screen.getByText("自动提取")).toBeTruthy();
    expect(screen.getByText("所有项目")).toBeTruthy();
    expect(screen.getByText("onepiece")).toBeTruthy();
    expect(screen.getByText("codex-cli")).toBeTruthy();

    // `migrate-agent-memory-to-file-store`: name and description let a user tell entries apart
    // without reading every body, and the type badge appears only when one is declared.
    expect(screen.getByText("npm-only")).toBeTruthy();
    expect(screen.getByText("Uses pnpm")).toBeTruthy();
    expect(screen.getByText("concise")).toBeTruthy();
    expect(screen.getByText("反馈")).toBeTruthy();
    expect(screen.queryByText("参考")).toBeNull();

    const deleteButtons = screen.getAllByRole("button", { name: "删除" });
    await user.click(deleteButtons[0]!);
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "确认" }));
    // The id is the memory file's path now, not a generated row id.
    expect(deleteAgentMemory).toHaveBeenCalledWith("npm-only.md");
    await waitFor(() => expect(screen.queryByText("Uses pnpm.")).toBeNull());
    expect(screen.getByText("Prefers concise responses.")).toBeTruthy();
  });

  it("does not delete when the confirmation is dismissed", async () => {
    const memories: AgentMemory[] = [
      { id: "memory-1", agentId: "onepiece", folder: null, name: "memory-1", description: "Uses pnpm", memoryType: null, content: "Uses pnpm.", source: "explicit", createdAt: "2026-01-01T00:00:00.000Z" },
    ];
    const deleteAgentMemory = vi.fn(async () => undefined);
    const service = createAgentServiceDouble({ listAllMemories: async () => memories, deleteAgentMemory });
    const { user } = renderSection(service);

    await screen.findByText("Uses pnpm.");
    await user.click(screen.getByRole("button", { name: "删除" }));
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "取消" }));
    expect(deleteAgentMemory).not.toHaveBeenCalled();
    expect(screen.getByText("Uses pnpm.")).toBeTruthy();
  });

  it("resets every agent's memories on confirm", async () => {
    const memories: AgentMemory[] = [
      { id: "memory-1", agentId: "onepiece", folder: null, name: "memory-1", description: "Uses pnpm", memoryType: null, content: "Uses pnpm.", source: "explicit", createdAt: "2026-01-01T00:00:00.000Z" },
    ];
    const resetAllMemories = vi.fn(async () => undefined);
    const service = createAgentServiceDouble({ listAllMemories: async () => memories, resetAllMemories });
    const { user } = renderSection(service);

    await screen.findByText("Uses pnpm.");
    await user.click(screen.getByRole("button", { name: "重置全部" }));
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "确认" }));
    expect(resetAllMemories).toHaveBeenCalledWith();
  });

  it("does not reset when the confirmation is dismissed", async () => {
    const memories: AgentMemory[] = [
      { id: "memory-1", agentId: "onepiece", folder: null, name: "memory-1", description: "Uses pnpm", memoryType: null, content: "Uses pnpm.", source: "explicit", createdAt: "2026-01-01T00:00:00.000Z" },
    ];
    const resetAllMemories = vi.fn(async () => undefined);
    const service = createAgentServiceDouble({ listAllMemories: async () => memories, resetAllMemories });
    const { user } = renderSection(service);

    await screen.findByText("Uses pnpm.");
    await user.click(screen.getByRole("button", { name: "重置全部" }));
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "取消" }));

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
