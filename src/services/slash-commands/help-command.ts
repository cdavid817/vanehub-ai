import { isOnePieceSession } from "./command-availability";
import type { SlashCommand } from "./types";

function invocation(command: SlashCommand): string {
  return command.argumentHint ? `/${command.name} ${command.argumentHint}` : `/${command.name}`;
}

export const HELP_COMMAND: SlashCommand = {
  name: "help", aliases: ["?"], category: "info", appliesTo: isOnePieceSession,
  run: async (context) => ({
    kind: "output",
    output: {
      titleKey: "slash.output.helpTitle", tone: "info",
      // The description is passed through as a key so the output panel translates it in one place,
      // the same way every other command message is handled.
      messages: context.listAvailableCommands().map((command) => ({
        key: "slash.output.helpEntry",
        params: { invocation: invocation(command), description: `slash.command.${command.name}.description` },
      })),
    },
  }),
};
