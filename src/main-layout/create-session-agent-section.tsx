import { Bot } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { AgentRegistryEntry, InteractionMode } from "../types/agent";

export function CreateSessionAgentSection({
  agents,
  interactionMode,
  onAgentSelect,
  onInteractionModeChange,
  selectedAgent,
}: {
  agents: AgentRegistryEntry[];
  interactionMode: InteractionMode;
  onAgentSelect: (agent: AgentRegistryEntry) => void;
  onInteractionModeChange: (mode: InteractionMode) => void;
  selectedAgent: AgentRegistryEntry | null;
}) {
  const { t } = useTranslation();
  return (
    <section className="grid gap-2">
      <span className="text-xs font-medium text-muted-foreground">{t("createSession.agent")}</span>
      <div className="grid grid-cols-2 gap-2">
        {agents.map((agent) => (
          <button
            className={cn(
              "ucd-list-row flex min-h-12 items-center gap-2 rounded-md p-2 text-left text-sm",
              selectedAgent?.id === agent.id && "border-primary bg-[hsl(var(--nav-active-soft))]",
            )}
            key={agent.id}
            onClick={() => onAgentSelect(agent)}
            type="button"
          >
            <Bot className="h-4 w-4 text-primary" aria-hidden="true" />
            <span className="min-w-0">
              <span className="block truncate font-medium">{agent.displayName}</span>
              <span className="block truncate text-xs text-muted-foreground">{agent.id}</span>
            </span>
          </button>
        ))}
      </div>
      <div className="flex flex-wrap gap-2">
        {selectedAgent?.supportedInteractionModes.map((mode) => (
          <button
            className={cn(
              "h-7 rounded-md border border-border px-2 text-xs hover:bg-muted",
              interactionMode === mode && "border-primary bg-primary text-primary-foreground",
            )}
            key={mode}
            onClick={() => onInteractionModeChange(mode)}
            type="button"
          >
            {mode}
          </button>
        ))}
      </div>
    </section>
  );
}
