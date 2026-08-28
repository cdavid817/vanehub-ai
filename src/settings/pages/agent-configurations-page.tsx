import { Settings2 } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import type { SettingsPageContext } from "../settings-pages";
import { PageHeader } from "./page-parts";
import { AgentGlobalConfigPanel } from "./agents/agent-global-config-panel";
import { OnePieceConfigurationPanel } from "./agents/onepiece-configuration-panel";
import { OnePieceParametersPanel } from "./onepiece-parameters-panel";
import { useEffect, useState } from "react";
import type { AgentService } from "../../services/agent-service";
import { AgentConfigurationSelector, agentNameKeys, type ConfigurableAgentId } from "./agents/agent-configuration-selector";

export function AgentConfigurationsPage({ navigationTarget, searchTerm, service }: SettingsPageContext & { service?: AgentService }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const targetAgentId = navigationTarget?.agentConfigAgentId
    ?? navigationTarget?.cliConfigAgentId
    ?? "claude-code";
  const [agentId, setAgentId] = useState<ConfigurableAgentId>(targetAgentId);

  useEffect(() => {
    const target = navigationTarget?.agentConfigAgentId ?? navigationTarget?.cliConfigAgentId;
    if (target) setAgentId(target);
  }, [navigationTarget?.agentConfigAgentId, navigationTarget?.cliConfigAgentId]);

  return (
    <div className="space-y-5" data-testid="agent-configurations-page">
      <PageHeader
        description={t("agentConfigurations.description")}
        icon={Settings2}
        title={t("agentConfigurations.title")}
      />
      <div className="grid min-w-0 gap-5 lg:grid-cols-[13rem_minmax(0,1fr)]">
        <AgentConfigurationSelector onSelect={setAgentId} selected={agentId} />
        <div aria-label={t(agentNameKeys[agentId])} className="min-w-0" data-agent-id={agentId} data-testid="agent-configuration-content" role="region">
          {agentId === "onepiece" ? (
            <div className="space-y-5">
              <OnePieceConfigurationPanel
                onChanged={async () => {
                  await queryClient.invalidateQueries({ queryKey: ["agents"] });
                }}
                searchTerm={searchTerm}
                service={service}
              />
              {/* OnePiece has no CLI and therefore no launch flags. Its retrieval, compaction and
                  context-health parameters used to sit on the CLI Parameters page next to five
                  things that are argv; they belong here, with the rest of OnePiece. */}
              <OnePieceParametersPanel />
            </div>
          ) : (
            <AgentGlobalConfigPanel agentId={agentId} searchTerm={searchTerm} service={service} />
          )}
        </div>
      </div>
    </div>
  );
}
