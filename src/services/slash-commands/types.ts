import type { Session, SessionExportFormat } from "../../types/agent";
import type { ChatConfig, SessionExecutionMode } from "../../types/chat";
import type { SessionTabId } from "../../session-workspace/session-tab-bar";
import type { WorkspaceDestination } from "../../main-layout/workspace-route";

export type SlashCommandCategory = "session" | "runtime" | "navigation" | "info";

/**
 * Aliased rather than redeclared: a second copy of this union would silently drift the day a
 * destination is renamed, and the navigation commands would keep compiling while pointing at a
 * route that no longer exists.
 */
export type SlashCommandDestination = WorkspaceDestination;

/**
 * Facts a command needs for its availability decision that do not live on the session row.
 * Passing them explicitly is what keeps `appliesTo` a pure function of its arguments.
 */
export type CommandCapabilities = object;

/**
 * Commands emit translation keys rather than finished strings so their unit tests stay free of
 * i18n, and so the output panel remains the single place that renders copy.
 */
export interface CommandMessage {
  key: string;
  params?: Record<string, string | number>;
}

export interface CommandOutput {
  titleKey: string;
  messages: CommandMessage[];
  tone: "info" | "error";
}

export type CommandOutcome =
  | { kind: "handled" }
  | { kind: "output"; output: CommandOutput };

export interface SlashCommandNavigation {
  openDestination: (destination: SlashCommandDestination) => void;
  openSessionTab: (tab: SessionTabId) => void;
}

export interface CommandContext {
  session: Session;
  config: ChatConfig;
  isStreaming: boolean;
  chat: {
    setSessionExecutionMode: (value: SessionExecutionMode) => void;
    setStreaming: (value: boolean) => void;
    setThinking: (value: boolean) => void;
    setLongContext: (value: boolean) => void;
  };
  actions: {
    exportSession: (session: Session, format: SessionExportFormat) => void;
    loadUsageSummary: (sessionId: string) => Promise<{
      totalTokens: number; inputTokens: number; outputTokens: number; responseCount: number;
    }>;
  };
  navigate: SlashCommandNavigation;
  /**
   * Commands that absorb an infrastructure failure to show a specific message must still
   * report it — a swallowed rejection is invisible to the dispatcher's error path, and a
   * backend outage would otherwise leave no trace anywhere but the user's screen.
   */
  reportFailure: (source: string, reason: unknown) => void;
  /** Supplied by the dispatcher so `/help` can enumerate siblings without a circular import. */
  listAvailableCommands: () => SlashCommand[];
}

export interface SlashCommand {
  name: string;
  aliases?: string[];
  category: SlashCommandCategory;
  /** Rendered in `/help` and the completion dropdown, e.g. "<plan|execute|inherit>". */
  argumentHint?: string;
  /** Commands that only care about the session may declare a one-parameter function. */
  appliesTo: (session: Session, capabilities: CommandCapabilities) => boolean;
  run: (context: CommandContext, args: string[]) => Promise<CommandOutcome>;
}
