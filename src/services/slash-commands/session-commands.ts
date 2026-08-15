import { isOnePieceSession } from "./command-availability";
import type { SessionExportFormat } from "../../types/agent";
import type { CommandOutcome, SlashCommand } from "./types";

const EXPORT_FORMATS: Record<string, SessionExportFormat> = {
  md: "markdown", markdown: "markdown", json: "json",
};

function error(key: string, params?: Record<string, string | number>): CommandOutcome {
  return { kind: "output", output: { titleKey: "slash.error.title", tone: "error", messages: [{ key, params }] } };
}

const onOff = (value: boolean): string => (value ? "on" : "off");

export const SESSION_COMMANDS: SlashCommand[] = [
  {
    name: "export", category: "session", argumentHint: "[md|json]", appliesTo: isOnePieceSession,
    run: async (context, args) => {
      const requested = args[0] ?? "md";
      // Bracket indexing on a plain object walks Object.prototype, so an unguarded lookup would
      // let names like "constructor" or "toString" resolve to a truthy, non-format value.
      if (!Object.hasOwn(EXPORT_FORMATS, requested)) {
        return error("slash.error.badArgument", { command: "export", allowed: Object.keys(EXPORT_FORMATS).join(", ") });
      }
      const format = EXPORT_FORMATS[requested];
      context.actions.exportSession(context.session, format);
      return { kind: "output", output: { titleKey: "slash.output.applied", tone: "info", messages: [{ key: "slash.output.export", params: { value: format } }] } };
    },
  },
  {
    name: "status", category: "info", appliesTo: isOnePieceSession,
    run: async (context) => ({
      kind: "output",
      output: {
        titleKey: "slash.output.statusTitle", tone: "info",
        messages: [
          { key: "slash.output.mode", params: { value: context.config.executionMode } },
          { key: "slash.output.thinking", params: { value: onOff(context.config.thinking) } },
          { key: "slash.output.streaming", params: { value: onOff(context.config.streaming) } },
          { key: "slash.output.longcontext", params: { value: onOff(context.config.longContext) } },
        ],
      },
    }),
  },
  {
    name: "usage", category: "info", appliesTo: isOnePieceSession,
    run: async (context) => {
      try {
        const summary = await context.actions.loadUsageSummary(context.session.id);
        return {
          kind: "output",
          output: {
            titleKey: "slash.output.usageTitle", tone: "info",
            messages: [
              { key: "slash.output.usageTotal", params: { value: summary.totalTokens } },
              { key: "slash.output.usageInput", params: { value: summary.inputTokens } },
              { key: "slash.output.usageOutput", params: { value: summary.outputTokens } },
              { key: "slash.output.usageResponses", params: { value: summary.responseCount } },
            ],
          },
        };
      } catch (reason) {
        // The panel is the only feedback channel a command has, so a failed lookup has to be
        // reported here rather than thrown into a boundary the user never sees; reportFailure is
        // what keeps the failure from vanishing entirely once this catch has absorbed it.
        context.reportFailure("SlashCommands.usage", reason);
        return error("slash.error.usageUnavailable");
      }
    },
  },
];
