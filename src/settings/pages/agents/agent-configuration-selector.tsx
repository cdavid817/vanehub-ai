import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../../../components/agent-brand-icon";
import { cliConfigAgentIds, type CliConfigAgentId } from "../../../types/cli-agent-config";

export type ConfigurableAgentId = CliConfigAgentId | "onepiece";

const groups: Array<{ id: "cli" | "native"; agents: ConfigurableAgentId[] }> = [
  { id: "cli", agents: [...cliConfigAgentIds] },
  { id: "native", agents: ["onepiece"] },
];

export const agentNameKeys: Record<ConfigurableAgentId, string> = {
  onepiece: "onepiece.title",
  "claude-code": "agentConfigurations.agent.claudeCode",
  opencode: "agentConfigurations.agent.openCode",
  "codex-cli": "agentConfigurations.agent.codex",
  "antigravity-cli": "agentConfigurations.agent.antigravity",
  "gemini-cli": "agentConfigurations.agent.gemini",
};

export function AgentConfigurationSelector({ selected, onSelect }: {
  selected: ConfigurableAgentId;
  onSelect: (agentId: ConfigurableAgentId) => void;
}) {
  const { t } = useTranslation();

  return (
    <nav aria-label={t("agentConfigurations.agentTabs")} className="flex min-w-0 max-w-full gap-3 overflow-x-auto pb-1 lg:sticky lg:top-0 lg:block lg:self-start lg:overflow-visible lg:pb-0">
      {groups.map((group) => (
        <section className="min-w-max lg:mb-5 lg:min-w-0" key={group.id}>
          <h2 className="mb-1.5 px-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
            {t(`agentConfigurations.group.${group.id}`)}
          </h2>
          <div className="flex gap-1 rounded-lg border border-border bg-muted/25 p-1 lg:grid">
            {group.agents.map((agentId) => {
              const active = agentId === selected;
              return (
                <button
                  aria-current={active ? "page" : undefined}
                  className={`flex min-h-10 items-center gap-2 rounded-md px-3 py-2 text-left text-sm transition-colors lg:w-full ${active ? "bg-background font-semibold text-primary shadow-xs" : "text-muted-foreground hover:bg-background/70 hover:text-foreground"}`}
                  data-testid={`agent-config-target-${agentId}`}
                  key={agentId}
                  onClick={() => onSelect(agentId)}
                  type="button"
                >
                  <AgentBrandIcon agentId={agentId} className="h-4 w-4 shrink-0" />
                  <span className="whitespace-nowrap">{t(agentNameKeys[agentId])}</span>
                </button>
              );
            })}
          </div>
        </section>
      ))}
    </nav>
  );
}
