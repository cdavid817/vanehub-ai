import { CheckCircle2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../components/agent-brand-icon";
import { getAgentVisualIdentity } from "../lib/agent-visual-identity";
import { cn } from "../lib/utils";
import { groupSessionAgents, isSessionAgentSelectable } from "./create-session-agents";
import type { AgentRegistryEntry } from "../types/agent";

export function CreateSessionAgentSection({
  agents,
  disabled = false,
  onConfigureOnePiece = () => undefined,
  onAgentSelect,
  selectedAgent,
}: {
  agents: AgentRegistryEntry[];
  disabled?: boolean;
  onConfigureOnePiece?: () => void;
  onAgentSelect: (agent: AgentRegistryEntry) => void;
  selectedAgent: AgentRegistryEntry | null;
}) {
  const { t } = useTranslation();
  return (
    <section className="grid gap-2">
      <span className="text-xs font-medium text-muted-foreground">{t("createSession.agent")}</span>
      <div className="grid gap-3">
        {groupSessionAgents(agents).map((group) => (
          <div className="grid min-w-0 gap-2" key={group.id}>
            <span className="text-xs font-medium text-muted-foreground">
              {t(group.labelKey)}
            </span>
            <div
              className={cn(
                "grid min-w-0 grid-cols-1 gap-2",
                group.agents.length > 1 && "sm:grid-cols-2",
              )}
            >
              {group.agents.map((agent) => {
                const identity = getAgentVisualIdentity(agent.id);
                const selected = selectedAgent?.id === agent.id;
                const unavailable = !isSessionAgentSelectable(agent);
                return (
                  <div className="grid min-w-0 gap-1" key={agent.id}>
                    <button
                      aria-disabled={disabled || unavailable ? "true" : undefined}
                      aria-pressed={selected}
                      className={cn(
                        "ucd-list-row flex min-h-12 w-full min-w-0 items-center gap-2 rounded-md p-2 text-left text-sm transition",
                        selected && "border-primary bg-[hsl(var(--nav-active-soft))] text-foreground shadow-[0_0_0_1px_hsl(var(--primary))]",
                        (disabled || unavailable) && "cursor-not-allowed opacity-60",
                      )}
                      onClick={() => {
                        if (!disabled && !unavailable) onAgentSelect(agent);
                      }}
                      type="button"
                    >
                      <span className={cn("flex h-8 w-8 shrink-0 items-center justify-center rounded border", identity.tone)}>
                        <AgentBrandIcon agentId={agent.id} className="h-4 w-4" />
                      </span>
                      <span className="min-w-0 flex-1">
                        <span className="block truncate font-medium">{agent.displayName}</span>
                        <span className="block truncate text-xs text-muted-foreground">{agent.id}</span>
                        {unavailable ? (
                          <span className="block truncate text-xs text-amber-600">
                            {agent.unavailableReason}
                          </span>
                        ) : null}
                      </span>
                      {selected ? <CheckCircle2 className="h-4 w-4 shrink-0 text-primary" aria-hidden="true" /> : null}
                    </button>
                    {unavailable && agent.id === "onepiece" ? (
                      <button
                        className="justify-self-start text-left text-xs text-primary hover:underline"
                        onClick={onConfigureOnePiece}
                        type="button"
                      >
                        {t("onepiece.configureAction")}
                      </button>
                    ) : null}
                  </div>
                );
              })}
            </div>
          </div>
        ))}
      </div>
      {selectedAgent ? (
        <div className="flex min-w-0 items-center gap-2 rounded-md border border-primary/40 bg-[hsl(var(--nav-active-soft))] px-2 py-1.5 text-xs">
          <span className={cn("flex h-6 w-6 shrink-0 items-center justify-center rounded border", getAgentVisualIdentity(selectedAgent.id).tone)}>
            <AgentBrandIcon agentId={selectedAgent.id} className="h-3.5 w-3.5" />
          </span>
          <span className="min-w-0 truncate">{t("createSession.selectedAgent", { agent: selectedAgent.displayName })}</span>
        </div>
      ) : null}
    </section>
  );
}
