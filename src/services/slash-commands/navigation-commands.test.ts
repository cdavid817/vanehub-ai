import { describe, expect, it, vi } from "vitest";
import type { Session } from "../../types/agent";
import { NAVIGATION_COMMANDS } from "./navigation-commands";
import type { CommandContext, SlashCommand } from "./types";

const session = (agentId = "onepiece"): Session =>
  ({ id: "s", title: "S", agentId, interactionMode: "api" } as Session);

function context(openAssociatedPlan: (() => void) | null = null) {
  const navigate = { openAssociatedPlan, openDestination: vi.fn(), openSessionTab: vi.fn() };
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
  it("/todo, /plans and /loops switch destination", async () => {
    for (const [name, destination] of [["todo", "todo-board"], ["plans", "plans"], ["loops", "loops"]] as const) {
      const { ctx, navigate } = context();
      await byName(name).run(ctx, []);
      expect(navigate.openDestination).toHaveBeenCalledWith(destination);
    }
  });

  it("exposes one command per workspace tab except chat", async () => {
    for (const tab of ["logs", "files", "changes", "documents", "terminal", "shell", "traces", "report"] as const) {
      const { ctx, navigate } = context();
      await byName(tab).run(ctx, []);
      expect(navigate.openSessionTab).toHaveBeenCalledWith(tab);
    }
  });

  it("/plan is unavailable without an associated plan run", () => {
    expect(byName("plan").appliesTo(session(), { hasAssociatedPlan: false })).toBe(false);
    expect(byName("plan").appliesTo(session(), { hasAssociatedPlan: true })).toBe(true);
    expect(byName("plan").appliesTo(session("claude-code"), { hasAssociatedPlan: true })).toBe(false);
  });

  it("/plan opens the associated run when one exists", async () => {
    const open = vi.fn();
    const { ctx } = context(open);
    await byName("plan").run(ctx, []);
    expect(open).toHaveBeenCalled();
  });

  it("/plan and /plans are distinct commands", () => {
    expect(byName("plan").name).not.toBe(byName("plans").name);
    expect(byName("plan").category).toBe("navigation");
    expect(byName("plans").category).toBe("navigation");
  });
});
