import { Settings2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../../components/agent-brand-icon";
import { cliConfigAgentIds, type CliConfigAgentId } from "../../types/cli-agent-config";
import type { SettingsPageContext } from "../settings-pages";
import { PageHeader } from "./page-parts";
import { AgentGlobalConfigPanel } from "./agents/agent-global-config-panel";
import { useEffect, useState } from "react";
import type { AgentService } from "../../services/agent-service";

const agentNameKeys: Record<CliConfigAgentId, string> = {
  "claude-code": "agentConfigurations.agent.claudeCode",
  opencode: "agentConfigurations.agent.openCode",
  "codex-cli": "agentConfigurations.agent.codex",
};

export function AgentConfigurationsPage({ navigationTarget, searchTerm, service }: SettingsPageContext & { service?: AgentService }) {
  const { t } = useTranslation();
  const [agentId, setAgentId] = useState<CliConfigAgentId>(navigationTarget?.cliConfigAgentId ?? "claude-code");

  useEffect(() => {
    if (navigationTarget?.cliConfigAgentId) setAgentId(navigationTarget.cliConfigAgentId);
  }, [navigationTarget?.cliConfigAgentId]);

  return (
    <div className="space-y-5">
      <PageHeader
        description={t("agentConfigurations.description")}
        icon={Settings2}
        title={t("agentConfigurations.title")}
      />
      <div className="flex justify-center sm:justify-start">
      <div aria-label={t("agentConfigurations.agentTabs")} className="grid w-full grid-cols-3 gap-1 rounded-xl bg-muted p-1 sm:w-auto" role="tablist">
        {cliConfigAgentIds.map((candidate) => (
          <button
            aria-selected={candidate === agentId}
            className={`flex min-h-10 min-w-0 items-center justify-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-colors sm:min-w-36 ${candidate === agentId ? "bg-background text-foreground shadow-sm" : "text-muted-foreground hover:bg-background/60 hover:text-foreground"}`}
            key={candidate}
            onClick={() => setAgentId(candidate)}
            role="tab"
            type="button"
          >
            <AgentBrandIcon agentId={candidate} className="h-4 w-4" />
            {t(agentNameKeys[candidate])}
          </button>
        ))}
      </div>
      </div>
      <div aria-label={t(agentNameKeys[agentId])} role="tabpanel">
        <AgentGlobalConfigPanel agentId={agentId} searchTerm={searchTerm} service={service} />
      </div>
    </div>
  );
}
