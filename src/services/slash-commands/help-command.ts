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
      // The description is passed through as a key, not a translated string. This is the only
      // message that nests a key inside another key's params, so the output panel has to
      // resolve it before the outer interpolation — default i18next would not.
      messages: context.listAvailableCommands().map((command) => ({
        key: "slash.output.helpEntry",
        params: { invocation: invocation(command), descriptionKey: `slash.command.${command.name}.description` },
      })),
    },
  }),
};
