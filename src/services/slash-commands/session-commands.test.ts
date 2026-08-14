import { describe, expect, it, vi } from "vitest";
import type { Session } from "../../types/agent";
import type { ChatConfig } from "../../types/chat";
import { SESSION_COMMANDS } from "./session-commands";
import type { CommandContext, SlashCommand } from "./types";

const session = (agentId = "onepiece"): Session =>
  ({ id: "session-1", title: "S", agentId, interactionMode: "api" } as Session);

function context(overrides: { config?: Partial<ChatConfig>; isStreaming?: boolean } = {}) {
  const actions = {
    exportSession: vi.fn(), stop: vi.fn(),
    loadUsageSummary: vi.fn().mockResolvedValue({
      totalTokens: 1234, inputTokens: 1000, outputTokens: 234, responseCount: 7,
    }),
  };
  const ctx = {
    session: session(),
    config: {
      agentId: "onepiece", interactionMode: "api", executionMode: "plan",
      streaming: true, thinking: false, longContext: false,
      reasoningDepth: "medium", ...overrides.config,
    } as ChatConfig,
    isStreaming: overrides.isStreaming ?? false,
    chat: {
      setSessionExecutionMode: vi.fn(), setReasoningDepth: vi.fn(),
      setStreaming: vi.fn(), setThinking: vi.fn(), setLongContext: vi.fn(),
    },
    actions,
    navigate: { openAssociatedPlan: null, openDestination: vi.fn(), openSessionTab: vi.fn() },
    listAvailableCommands: () => [],
  } as unknown as CommandContext;
  return { ctx, actions };
}

const byName = (name: string): SlashCommand => {
  const command = SESSION_COMMANDS.find((entry) => entry.name === name);
  if (!command) throw new Error(`missing command: ${name}`);
  return command;
};

describe("session commands", () => {
  it("defers /clear and /compact to phase two", () => {
    const names = SESSION_COMMANDS.map((command) => command.name);
    expect(names).not.toContain("clear");
    expect(names).not.toContain("compact");
  });

  it("/export defaults to markdown", async () => {
    const { ctx, actions } = context();
    await byName("export").run(ctx, []);
    expect(actions.exportSession).toHaveBeenCalledWith(ctx.session, "markdown");
  });

  it("/export accepts json and the md alias", async () => {
    const json = context();
    await byName("export").run(json.ctx, ["json"]);
    expect(json.actions.exportSession).toHaveBeenCalledWith(json.ctx.session, "json");

    const md = context();
    await byName("export").run(md.ctx, ["md"]);
    expect(md.actions.exportSession).toHaveBeenCalledWith(md.ctx.session, "markdown");
  });

  it("/export rejects an unknown format", async () => {
    const { ctx, actions } = context();
    const outcome = await byName("export").run(ctx, ["pdf"]);
    expect(actions.exportSession).not.toHaveBeenCalled();
    expect(outcome).toEqual({
      kind: "output",
      output: { titleKey: "slash.error.title", tone: "error",
        messages: [{ key: "slash.error.badArgument", params: { command: "export", allowed: "md, markdown, json" } }] },
    });
  });

  it("/stop only acts while streaming", async () => {
    const idle = context({ isStreaming: false });
    const outcome = await byName("stop").run(idle.ctx, []);
    expect(idle.actions.stop).not.toHaveBeenCalled();
    expect(outcome).toEqual({
      kind: "output",
      output: { titleKey: "slash.error.title", tone: "error",
        messages: [{ key: "slash.error.notStreaming" }] },
    });

    const busy = context({ isStreaming: true });
    await byName("stop").run(busy.ctx, []);
    expect(busy.actions.stop).toHaveBeenCalled();
  });

  it("/status reports the current runtime switches", async () => {
    const { ctx } = context();
    const outcome = await byName("status").run(ctx, []);
    expect(outcome).toEqual({
      kind: "output",
      output: {
        titleKey: "slash.output.statusTitle", tone: "info",
        messages: [
          { key: "slash.output.mode", params: { value: "plan" } },
          { key: "slash.output.reasoning", params: { value: "medium" } },
          { key: "slash.output.thinking", params: { value: "off" } },
          { key: "slash.output.streaming", params: { value: "on" } },
          { key: "slash.output.longcontext", params: { value: "off" } },
        ],
      },
    });
  });

  it("/usage reports token totals from the service", async () => {
    const { ctx, actions } = context();
    const outcome = await byName("usage").run(ctx, []);
    expect(actions.loadUsageSummary).toHaveBeenCalledWith("session-1");
    expect(outcome).toEqual({
      kind: "output",
      output: {
        titleKey: "slash.output.usageTitle", tone: "info",
        messages: [
          { key: "slash.output.usageTotal", params: { value: 1234 } },
          { key: "slash.output.usageInput", params: { value: 1000 } },
          { key: "slash.output.usageOutput", params: { value: 234 } },
          { key: "slash.output.usageResponses", params: { value: 7 } },
        ],
      },
    });
  });

  it("/usage surfaces a service failure instead of throwing", async () => {
    const { ctx, actions } = context();
    actions.loadUsageSummary.mockRejectedValue(new Error("backend down"));
    const outcome = await byName("usage").run(ctx, []);
    expect(outcome).toEqual({
      kind: "output",
      output: { titleKey: "slash.error.title", tone: "error",
        messages: [{ key: "slash.error.usageUnavailable" }] },
    });
  });
});
