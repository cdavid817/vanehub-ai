import { useCallback, useMemo, useState } from "react";
import { SLASH_COMMANDS } from "./command-catalog";
import { findCommand, listCommands } from "./command-registry";
import { parseCommandInput } from "./parse-command";
import { slashCommandsEnabled } from "./command-availability";
import type { Session } from "../../types/agent";
import type { ChatConfig } from "../../types/chat";
import type { CommandCapabilities, CommandContext, CommandOutput, SlashCommandNavigation } from "./types";

export type DispatchResult =
  | { kind: "message" }
  | { kind: "literal"; content: string }
  | { kind: "handled" };

const COMPLETION_PATTERN = /^\/([a-zA-Z][a-zA-Z0-9-]*)?$/;

function errorOutput(key: string, params?: Record<string, string | number>): CommandOutput {
  return { titleKey: "slash.error.title", tone: "error", messages: [{ key, params }] };
}

export function useSlashCommands(input: {
  session: Session | null;
  config: ChatConfig;
  isStreaming: boolean;
  actions: CommandContext["actions"];
  chat: CommandContext["chat"];
  navigate: SlashCommandNavigation;
  onError: (source: string, reason: unknown) => void;
}) {
  const { actions, chat, config, isStreaming, navigate, onError, session } = input;
  const [output, setOutput] = useState<CommandOutput | null>(null);
  const [suggestionQuery, setSuggestionQuery] = useState<string | null>(null);

  const enabled = slashCommandsEnabled(session);
  const capabilities = useMemo<CommandCapabilities>(
    () => ({ hasAssociatedPlan: navigate.openAssociatedPlan !== null }),
    [navigate.openAssociatedPlan],
  );

  const available = useMemo(
    () => (session && enabled ? listCommands(SLASH_COMMANDS, session, capabilities) : []),
    [capabilities, enabled, session],
  );

  const suggestions = useMemo(() => {
    if (suggestionQuery === null) return [];
    return available.filter((command) => command.name.startsWith(suggestionQuery)).slice(0, 8);
  }, [available, suggestionQuery]);

  /**
   * Called on every keystroke. It must never run anything — `dispatch` is the only entry point
   * allowed to have side effects, and conflating the two would fire a command per character.
   */
  const updateSuggestions = useCallback((draft: string) => {
    const completion = COMPLETION_PATTERN.exec(draft.trim());
    setSuggestionQuery(completion ? (completion[1] ?? "").toLowerCase() : null);
  }, []);

  const dispatch = useCallback((draft: string): DispatchResult => {
    setSuggestionQuery(null);

    const parsed = parseCommandInput(draft);
    if (parsed.kind === "literal") return parsed;
    if (parsed.kind === "message" || !session || !enabled) return { kind: "message" };

    const command = findCommand(SLASH_COMMANDS, parsed.name, session, capabilities);
    if (!command) {
      setOutput(errorOutput("slash.error.unknown", { command: parsed.name }));
      return { kind: "handled" };
    }

    const context: CommandContext = {
      session, config, isStreaming, chat, actions, navigate,
      // Same channel the dispatcher's own catch uses, so a failure a command chose to absorb
      // lands in the log alongside one it let escape.
      reportFailure: onError,
      listAvailableCommands: () => available,
    };

    // The caller needs a synchronous answer about whether the model should see this input, so the
    // handler's own result lands in state afterwards rather than being awaited here. `run` starts
    // executing the moment it is called (its synchronous prefix runs before this line returns);
    // only the settling of its promise is deferred, which is what stays out of this call stack.
    void command
      .run(context, parsed.args)
      .then((outcome) => setOutput(outcome.kind === "output" ? outcome.output : null))
      .catch((reason) => {
        onError(`SlashCommands.${command.name}`, reason);
        setOutput(errorOutput("slash.error.failed", { command: command.name }));
      });

    return { kind: "handled" };
  }, [actions, available, capabilities, chat, config, enabled, isStreaming, navigate, onError, session]);

  /** A command occupies the whole draft, so completing one replaces it rather than editing it. */
  const completeDraft = useCallback((name: string): string => `/${name} `, []);

  const dismissOutput = useCallback(() => setOutput(null), []);

  return { completeDraft, dismissOutput, dispatch, output, suggestionQuery, suggestions, updateSuggestions };
}
