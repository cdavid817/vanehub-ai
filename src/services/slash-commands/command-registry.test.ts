import { describe, expect, it } from "vitest";
import type { Session } from "../../types/agent";
import { findCommand, listCommands } from "./command-registry";
import type { SlashCommand } from "./types";

const session = (agentId: string): Session =>
  ({ id: "s", title: "S", agentId, interactionMode: "api" } as Session);

const command = (name: string, overrides: Partial<SlashCommand> = {}): SlashCommand => ({
  name, category: "info", appliesTo: () => true,
  run: async () => ({ kind: "handled" }), ...overrides,
});

const capabilities = () => ({});

describe("command registry", () => {
  const commands = [
    command("help"),
    command("status", { aliases: ["st"] }),
    command("compact", { appliesTo: (target) => target.agentId === "onepiece" }),
  ];

  it("finds a command by name", () => {
    expect(findCommand(commands, "help", session("onepiece"), capabilities())?.name).toBe("help");
  });

  it("finds a command by alias", () => {
    expect(findCommand(commands, "st", session("onepiece"), capabilities())?.name).toBe("status");
  });

  it("returns null for an unknown name", () => {
    expect(findCommand(commands, "nope", session("onepiece"), capabilities())).toBeNull();
  });

  it("returns null when the command does not apply to the session", () => {
    expect(findCommand(commands, "compact", session("claude-code"), capabilities())).toBeNull();
    expect(findCommand(commands, "compact", session("onepiece"), capabilities())?.name).toBe("compact");
  });

  it("lists only applicable commands, sorted by name", () => {
    expect(listCommands(commands, session("claude-code"), capabilities()).map((entry) => entry.name)).toEqual(["help", "status"]);
    expect(listCommands(commands, session("onepiece"), capabilities()).map((entry) => entry.name)).toEqual(["compact", "help", "status"]);
  });
});
