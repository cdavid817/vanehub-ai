import { isOnePieceSession } from "./command-availability";
import type { WorkbenchLocation } from "../../main-layout/workbench-route";
import type { LegacySessionTabId } from "../../session-workspace/legacy-session-surface-adapter";
import type { SlashCommand } from "./types";

const TAB_COMMANDS: LegacySessionTabId[] = [
  "logs", "files", "changes", "documents", "terminal", "shell", "traces", "report",
];

const DESTINATION_COMMANDS: Array<{ name: string; location: WorkbenchLocation }> = [
  { name: "todo", location: { destination: "plan", section: "board", viewId: undefined, workItemId: undefined } },
  { name: "loops", location: { destination: "runs", section: "loops", definitionId: undefined, loopRunId: undefined } },
];

export const NAVIGATION_COMMANDS: SlashCommand[] = [
  ...DESTINATION_COMMANDS.map(({ name, location }): SlashCommand => ({
    name, category: "navigation", appliesTo: isOnePieceSession,
    run: async (context) => {
      context.navigate.openDestination(location);
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
