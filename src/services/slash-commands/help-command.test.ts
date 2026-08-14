import { describe, expect, it } from "vitest";
import type { Session } from "../../types/agent";
import { HELP_COMMAND } from "./help-command";
import type { CommandContext, SlashCommand } from "./types";

const session = (agentId = "onepiece"): Session =>
  ({ id: "s", title: "S", agentId, interactionMode: "api" } as Session);

const stub = (name: string, argumentHint?: string): SlashCommand => ({
  name, category: "runtime", argumentHint, appliesTo: () => true,
  run: async () => ({ kind: "handled" }),
});

describe("/help", () => {
  it("lists the commands the dispatcher says are available", async () => {
    const context = {
      session: session(),
      listAvailableCommands: () => [stub("mode", "<inherit|plan|execute>"), stub("status")],
    } as unknown as CommandContext;

    const outcome = await HELP_COMMAND.run(context, []);
    expect(outcome).toEqual({
      kind: "output",
      output: {
        titleKey: "slash.output.helpTitle", tone: "info",
        messages: [
          { key: "slash.output.helpEntry", params: { invocation: "/mode <inherit|plan|execute>", description: "slash.command.mode.description" } },
          { key: "slash.output.helpEntry", params: { invocation: "/status", description: "slash.command.status.description" } },
        ],
      },
    });
  });

  it("is available in any OnePiece session", () => {
    const capabilities = { hasAssociatedPlan: false };
    expect(HELP_COMMAND.appliesTo(session("onepiece"), capabilities)).toBe(true);
    expect(HELP_COMMAND.appliesTo(session("claude-code"), capabilities)).toBe(false);
  });
});
