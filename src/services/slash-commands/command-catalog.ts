import { HELP_COMMAND } from "./help-command";
import { NAVIGATION_COMMANDS } from "./navigation-commands";
import { RUNTIME_COMMANDS } from "./runtime-commands";
import { SESSION_COMMANDS } from "./session-commands";
import type { SlashCommand } from "./types";

export const SLASH_COMMANDS: SlashCommand[] = [
  ...RUNTIME_COMMANDS, ...SESSION_COMMANDS, ...NAVIGATION_COMMANDS, HELP_COMMAND,
];
