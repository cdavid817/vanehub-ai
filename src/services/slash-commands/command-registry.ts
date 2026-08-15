import type { Session } from "../../types/agent";
import type { CommandCapabilities, SlashCommand } from "./types";

function matches(command: SlashCommand, name: string): boolean {
  return command.name === name || (command.aliases?.includes(name) ?? false);
}

export function findCommand(
  commands: SlashCommand[], name: string, session: Session, capabilities: CommandCapabilities,
): SlashCommand | null {
  const command = commands.find((entry) => matches(entry, name));
  if (!command || !command.appliesTo(session, capabilities)) return null;
  return command;
}

export function listCommands(
  commands: SlashCommand[], session: Session, capabilities: CommandCapabilities,
): SlashCommand[] {
  return commands
    .filter((command) => command.appliesTo(session, capabilities))
    .sort((left, right) => left.name.localeCompare(right.name));
}
