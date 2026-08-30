import { describe, expect, it, vi } from "vitest";
import type { Session } from "../../types/agent";
import { NAVIGATION_COMMANDS } from "./navigation-commands";
import type { CommandContext, SlashCommand } from "./types";

const session = (agentId = "onepiece"): Session =>
  ({ id: "s", title: "S", agentId, interactionMode: "api" } as Session);

function context() {
  const navigate = { openDestination: vi.fn(), openSessionTab: vi.fn() };
  const ctx = {
    session: session(), config: {}, isStreaming: false,
    chat: {}, actions: {}, navigate, listAvailableCommands: () => [],
  } as unknown as CommandContext;
  return { ctx, navigate };
}

const byName = (name: string): SlashCommand => {
  const command = NAVIGATION_COMMANDS.find((entry) => entry.name === name);
  if (!command) throw new Error(`missing command: ${name}`);
  return command;
};

describe("navigation commands", () => {
  it("/todo and /loops switch to their exact section, not just their domain's default", async () => {
    const cases = [
      ["todo", { destination: "plan", section: "board", viewId: undefined, workItemId: undefined }],
      ["loops", { destination: "runs", section: "loops", definitionId: undefined, loopRunId: undefined }],
    ] as const;
    for (const [name, location] of cases) {
      const { ctx, navigate } = context();
      await byName(name).run(ctx, []);
      expect(navigate.openDestination).toHaveBeenCalledWith(location);
    }
  });

  it("exposes one command per workspace tab except chat", async () => {
    for (const tab of ["logs", "files", "changes", "documents", "terminal", "shell", "traces", "report"] as const) {
      const { ctx, navigate } = context();
      await byName(tab).run(ctx, []);
      expect(navigate.openSessionTab).toHaveBeenCalledWith(tab);
    }
  });

  it("omits the retired Plan navigation commands", () => {
    expect(NAVIGATION_COMMANDS.some((command) => command.name === "plan" || command.name === "plans")).toBe(false);
  });
});
