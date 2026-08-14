import { describe, expect, it, vi } from "vitest";
import type { Session } from "../../types/agent";
import type { ChatConfig } from "../../types/chat";
import { RUNTIME_COMMANDS } from "./runtime-commands";
import type { CommandContext, SlashCommand } from "./types";

const session = (agentId = "onepiece"): Session =>
  ({ id: "s", title: "S", agentId, interactionMode: "api" } as Session);

const config = (overrides: Partial<ChatConfig> = {}): ChatConfig => ({
  agentId: "onepiece", interactionMode: "api", executionMode: "inherit",
  streaming: true, thinking: false, longContext: false, ...overrides,
});

function context(overrides: Partial<ChatConfig> = {}) {
  const chat = {
    setSessionExecutionMode: vi.fn(), setReasoningDepth: vi.fn(),
    setStreaming: vi.fn(), setThinking: vi.fn(), setLongContext: vi.fn(),
  };
  const ctx = {
    session: session(), config: config(overrides), isStreaming: false, chat,
    actions: { exportSession: vi.fn(), stop: vi.fn(), loadUsageSummary: vi.fn() },
    navigate: { openAssociatedPlan: null, openDestination: vi.fn(), openSessionTab: vi.fn() },
    listAvailableCommands: () => [],
  } as unknown as CommandContext;
  return { ctx, chat };
}

const byName = (name: string): SlashCommand => {
  const command = RUNTIME_COMMANDS.find((entry) => entry.name === name);
  if (!command) throw new Error(`missing command: ${name}`);
  return command;
};

describe("runtime commands", () => {
  it("applies only to OnePiece sessions in this phase", () => {
    // Runtime commands never read capabilities, but appliesTo's signature requires it
    // (types.ts), so every call site — including this generic sweep — must supply one.
    const capabilities = { hasAssociatedPlan: false };
    for (const command of RUNTIME_COMMANDS) {
      expect(command.appliesTo(session("onepiece"), capabilities)).toBe(true);
      expect(command.appliesTo(session("claude-code"), capabilities)).toBe(false);
    }
  });

  it("does not expose model, provider or agent switching", () => {
    const names = RUNTIME_COMMANDS.map((command) => command.name);
    expect(names).not.toContain("model");
    expect(names).not.toContain("provider");
    expect(names).not.toContain("agent");
  });

  it("/mode sets a valid execution mode and reports it", async () => {
    const { ctx, chat } = context();
    const outcome = await byName("mode").run(ctx, ["plan"]);
    expect(chat.setSessionExecutionMode).toHaveBeenCalledWith("plan");
    expect(outcome).toEqual({
      kind: "output",
      output: { titleKey: "slash.output.applied", tone: "info",
        messages: [{ key: "slash.output.mode", params: { value: "plan" } }] },
    });
  });

  it("/mode rejects an unknown value without touching config", async () => {
    const { ctx, chat } = context();
    const outcome = await byName("mode").run(ctx, ["nonsense"]);
    expect(chat.setSessionExecutionMode).not.toHaveBeenCalled();
    expect(outcome).toEqual({
      kind: "output",
      output: { titleKey: "slash.error.title", tone: "error",
        messages: [{ key: "slash.error.badArgument", params: { command: "mode", allowed: "inherit, plan, execute" } }] },
    });
  });

  it("/reasoning accepts every supported depth", async () => {
    for (const depth of ["low", "medium", "high", "max"]) {
      const { ctx, chat } = context();
      await byName("reasoning").run(ctx, [depth]);
      expect(chat.setReasoningDepth).toHaveBeenCalledWith(depth);
    }
  });

  it("/thinking toggles when given no argument", async () => {
    const { ctx, chat } = context({ thinking: false });
    await byName("thinking").run(ctx, []);
    expect(chat.setThinking).toHaveBeenCalledWith(true);
  });

  it("/thinking honours an explicit on or off", async () => {
    const enabled = context({ thinking: true });
    await byName("thinking").run(enabled.ctx, ["on"]);
    expect(enabled.chat.setThinking).toHaveBeenCalledWith(true);

    const disabled = context({ thinking: true });
    await byName("thinking").run(disabled.ctx, ["off"]);
    expect(disabled.chat.setThinking).toHaveBeenCalledWith(false);
  });

  it("/streaming and /longcontext toggle their own switches", async () => {
    const streaming = context({ streaming: true });
    await byName("streaming").run(streaming.ctx, []);
    expect(streaming.chat.setStreaming).toHaveBeenCalledWith(false);

    const longContext = context({ longContext: false });
    await byName("longcontext").run(longContext.ctx, []);
    expect(longContext.chat.setLongContext).toHaveBeenCalledWith(true);
  });
});
