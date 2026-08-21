import { isOnePieceSession } from "./command-availability";
import type { SessionTabId } from "../../session-workspace/session-tab-bar";
import type { SlashCommand, SlashCommandDestination } from "./types";

const TAB_COMMANDS: SessionTabId[] = [
  "logs", "files", "changes", "documents", "terminal", "shell", "traces", "report",
];

const DESTINATION_COMMANDS: Array<{ name: string; destination: SlashCommandDestination }> = [
  { name: "todo", destination: "work-board" },
  { name: "loops", destination: "loops" },
];

export const NAVIGATION_COMMANDS: SlashCommand[] = [
  ...DESTINATION_COMMANDS.map(({ name, destination }): SlashCommand => ({
    name, category: "navigation", appliesTo: isOnePieceSession,
    run: async (context) => {
      context.navigate.openDestination(destination);
      return { kind: "handled" };
    },
  })),
  ...TAB_COMMANDS.map((tab): SlashCommand => ({
    name: tab, category: "navigation", appliesTo: isOnePieceSession,
    run: async (context) => {
      context.navigate.openSessionTab(tab);
      return { kind: "handled" };
    },
  })),
];
