import { isOnePieceSession } from "./command-availability";
import type { SessionTabId } from "../../session-workspace/session-tab-bar";
import type { SlashCommand, SlashCommandDestination } from "./types";

const TAB_COMMANDS: SessionTabId[] = [
  "logs", "files", "changes", "documents", "terminal", "shell", "traces", "report",
];

const DESTINATION_COMMANDS: Array<{ name: string; destination: SlashCommandDestination }> = [
  { name: "todo", destination: "todo-board" },
  { name: "plans", destination: "plans" },
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
  {
    name: "plan", category: "navigation",
    // Availability comes in as an argument rather than module state so the predicate stays pure
    // and the test suite cannot leak one case's setup into the next.
    appliesTo: (session, capabilities) => isOnePieceSession(session) && capabilities.hasAssociatedPlan,
    run: async (context) => {
      context.navigate.openAssociatedPlan?.();
      return { kind: "handled" };
    },
  },
];
