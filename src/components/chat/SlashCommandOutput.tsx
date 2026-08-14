import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { CommandMessage, CommandOutput } from "../../services/slash-commands/types";

/**
 * Command output lives outside the message list on purpose: `invalidateRuntime` refetches
 * `["messages", sessionId]` after every send, which would wipe anything injected locally.
 */
export function SlashCommandOutput({
  onDismiss,
  output,
}: {
  onDismiss: () => void;
  output: CommandOutput | null;
}) {
  const { t } = useTranslation();
  if (!output) return null;

  function render(message: CommandMessage): string {
    const params = message.params ?? {};
    // `/help` ships its per-command description as a key, so it has to be resolved before the
    // outer string interpolates it.
    const resolved = typeof params.description === "string"
      ? { ...params, description: t(params.description) }
      : params;
    return t(message.key, resolved);
  }

  return (
    <div
      className="ucd-panel absolute bottom-full left-0 z-20 mb-2 grid max-h-56 w-full gap-1 overflow-y-auto rounded-md p-2 text-xs shadow-lg"
      data-testid="slash-command-output"
      data-tone={output.tone}
      role={output.tone === "error" ? "alert" : "status"}
    >
      <div className="flex items-center gap-2">
        <span className="min-w-0 flex-1 truncate font-semibold">{t(output.titleKey)}</span>
        <button
          aria-label={t("slash.output.dismiss")}
          className="rounded text-muted-foreground hover:text-foreground"
          onClick={onDismiss}
          type="button"
        >
          <X aria-hidden="true" className="h-3.5 w-3.5" />
        </button>
      </div>
      {output.messages.map((message, index) => (
        <p className="text-muted-foreground" key={`${message.key}-${index}`}>{render(message)}</p>
      ))}
    </div>
  );
}
