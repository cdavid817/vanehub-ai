import { describe, expect, it, vi } from "vitest";
import type { Session } from "../../types/agent";
import type { ChatConfig } from "../../types/chat";
import { SESSION_COMMANDS } from "./session-commands";
import type { CommandContext, SlashCommand } from "./types";

const session = (agentId = "onepiece"): Session =>
  ({ id: "session-1", title: "S", agentId, interactionMode: "api" } as Session);

function context(overrides: { config?: Partial<ChatConfig>; isStreaming?: boolean } = {}) {
  const actions = {
    exportSession: vi.fn(),
    loadUsageSummary: vi.fn().mockResolvedValue({
      totalTokens: 1234, inputTokens: 1000, outputTokens: 234, responseCount: 7,
    }),
  };
  const reportFailure = vi.fn();
  const ctx = {
    session: session(),
    config: {
      agentId: "onepiece", interactionMode: "api", executionMode: "plan",
      streaming: true, thinking: false, longContext: false,
      ...overrides.config,
    } as ChatConfig,
    isStreaming: overrides.isStreaming ?? false,
    chat: {
      setSessionExecutionMode: vi.fn(),
      setStreaming: vi.fn(), setThinking: vi.fn(), setLongContext: vi.fn(),
    },
    actions,
    navigate: { openDestination: vi.fn(), openSessionTab: vi.fn() },
    reportFailure,
    listAvailableCommands: () => [],
  } as unknown as CommandContext;
  return { ctx, actions, reportFailure };
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

  // Regression guard for the final-review removal: while streaming, the composer replaces the
  // submit affordance with the Stop button entirely, so `slash.dispatch` can never run with
  // `isStreaming` true. Re-adding this needs the composer to accept command input mid-stream first.
  it("does not expose /stop", () => {
    expect(SESSION_COMMANDS.map((command) => command.name)).not.toContain("stop");
  });

  it("/export defaults to markdown", async () => {
    const { ctx, actions } = context();
    const outcome = await byName("export").run(ctx, []);
    expect(actions.exportSession).toHaveBeenCalledWith(ctx.session, "markdown");
    expect(outcome).toEqual({
      kind: "output",
      output: { titleKey: "slash.output.applied", tone: "info",
        messages: [{ key: "slash.output.export", params: { value: "markdown" } }] },
    });
  });

  it("/export accepts json and the md alias", async () => {
    const json = context();
    const jsonOutcome = await byName("export").run(json.ctx, ["json"]);
    expect(json.actions.exportSession).toHaveBeenCalledWith(json.ctx.session, "json");
    expect(jsonOutcome).toEqual({
      kind: "output",
      output: { titleKey: "slash.output.applied", tone: "info",
        messages: [{ key: "slash.output.export", params: { value: "json" } }] },
    });

    const md = context();
    const mdOutcome = await byName("export").run(md.ctx, ["md"]);
    expect(md.actions.exportSession).toHaveBeenCalledWith(md.ctx.session, "markdown");
    expect(mdOutcome).toEqual({
      kind: "output",
      output: { titleKey: "slash.output.applied", tone: "info",
        messages: [{ key: "slash.output.export", params: { value: "markdown" } }] },
    });
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

  it("/export rejects prototype property names instead of walking the prototype chain", async () => {
    for (const requested of ["constructor", "toString", "hasOwnProperty", "__proto__"]) {
      const { ctx, actions } = context();
      const outcome = await byName("export").run(ctx, [requested]);
      expect(actions.exportSession).not.toHaveBeenCalled();
      expect(outcome).toEqual({
        kind: "output",
        output: { titleKey: "slash.error.title", tone: "error",
          messages: [{ key: "slash.error.badArgument", params: { command: "export", allowed: "md, markdown, json" } }] },
      });
    }
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

  it("/usage surfaces a service failure instead of throwing, and reports it", async () => {
    const { ctx, actions, reportFailure } = context();
    const reason = new Error("backend down");
    actions.loadUsageSummary.mockRejectedValue(reason);
    const outcome = await byName("usage").run(ctx, []);
    expect(reportFailure).toHaveBeenCalledWith("SlashCommands.usage", reason);
    expect(outcome).toEqual({
      kind: "output",
      output: { titleKey: "slash.error.title", tone: "error",
        messages: [{ key: "slash.error.usageUnavailable" }] },
    });
  });
});
