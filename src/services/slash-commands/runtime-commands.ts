import { isOnePieceSession } from "./command-availability";
import type { SessionExecutionMode } from "../../types/chat";
import type { CommandContext, CommandOutcome, SlashCommand } from "./types";

const EXECUTION_MODES: SessionExecutionMode[] = ["inherit", "plan", "execute"];

function applied(key: string, value: string | number): CommandOutcome {
  return { kind: "output", output: { titleKey: "slash.output.applied", tone: "info", messages: [{ key, params: { value } }] } };
}

function badArgument(command: string, allowed: string[]): CommandOutcome {
  return {
    kind: "output",
    output: {
      titleKey: "slash.error.title", tone: "error",
      messages: [{ key: "slash.error.badArgument", params: { command, allowed: allowed.join(", ") } }],
    },
  };
}

/** No argument means "flip it", which is what a bare `/thinking` reads as. */
function resolveToggle(args: string[], current: boolean): boolean | null {
  if (args.length === 0) return !current;
  if (args[0] === "on") return true;
  if (args[0] === "off") return false;
  return null;
}

function toggleCommand(
  name: string,
  read: (context: CommandContext) => boolean,
  write: (context: CommandContext, value: boolean) => void,
  outputKey: string,
): SlashCommand {
  return {
    name, category: "runtime", argumentHint: "[on|off]", appliesTo: isOnePieceSession,
    run: async (context, args) => {
      const next = resolveToggle(args, read(context));
      if (next === null) return badArgument(name, ["on", "off"]);
      write(context, next);
      return applied(outputKey, next ? "on" : "off");
    },
  };
}

export const RUNTIME_COMMANDS: SlashCommand[] = [
  {
    name: "mode", category: "runtime", argumentHint: "<inherit|plan|execute>", appliesTo: isOnePieceSession,
    run: async (context, args) => {
      const value = args[0] as SessionExecutionMode | undefined;
      if (!value || !EXECUTION_MODES.includes(value)) return badArgument("mode", EXECUTION_MODES);
      context.chat.setSessionExecutionMode(value);
      return applied("slash.output.mode", value);
    },
  },
  toggleCommand("thinking", (context) => context.config.thinking, (context, value) => context.chat.setThinking(value), "slash.output.thinking"),
  toggleCommand("streaming", (context) => context.config.streaming, (context, value) => context.chat.setStreaming(value), "slash.output.streaming"),
  toggleCommand("longcontext", (context) => context.config.longContext, (context, value) => context.chat.setLongContext(value), "slash.output.longcontext"),
];
