import { Settings2 } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../../components/agent-brand-icon";
import { cliConfigAgentIds, type CliConfigAgentId } from "../../types/cli-agent-config";
import type { SettingsPageContext } from "../settings-pages";
import { PageHeader } from "./page-parts";
import { AgentGlobalConfigPanel } from "./agents/agent-global-config-panel";
import { OnePieceConfigurationPanel } from "./agents/onepiece-configuration-panel";
import { useEffect, useState } from "react";
import type { AgentService } from "../../services/agent-service";
import { LspConfigurationSection } from "./agents/lsp-configuration-section";
import { LspRuntimeStatusPanel } from "./agents/lsp-runtime-status-panel";
import { LspServerTestPanel } from "./agents/lsp-server-test-panel";
import { LspWorkspaceTrustPanel } from "./agents/lsp-workspace-trust-panel";

type ConfigurableAgentId = CliConfigAgentId | "onepiece";

const configurableAgentIds: ConfigurableAgentId[] = [...cliConfigAgentIds, "onepiece"];

const agentNameKeys: Record<ConfigurableAgentId, string> = {
  onepiece: "onepiece.title",
  "claude-code": "agentConfigurations.agent.claudeCode",
  opencode: "agentConfigurations.agent.openCode",
  "codex-cli": "agentConfigurations.agent.codex",
  "antigravity-cli": "agentConfigurations.agent.antigravity",
  "gemini-cli": "agentConfigurations.agent.gemini",
};

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
    <div className="space-y-5">
      <PageHeader
        description={t("agentConfigurations.description")}
        icon={Settings2}
        title={t("agentConfigurations.title")}
      />
      <div className="flex justify-center sm:justify-start">
      {/* Column count follows the item count instead of being hard-coded: a fixed `sm:grid-cols-4`
          silently wrapped the last tab onto a second row the moment a fifth Agent was added. */}
      <div aria-label={t("agentConfigurations.agentTabs")} className="grid w-full grid-cols-2 gap-1 rounded-xl bg-muted p-1 sm:w-auto sm:auto-cols-fr sm:grid-flow-col sm:grid-cols-none" role="tablist">
        {configurableAgentIds.map((candidate) => (
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
        {agentId === "onepiece" ? (
          <OnePieceConfigurationPanel
            onChanged={async () => {
              await queryClient.invalidateQueries({ queryKey: ["agents"] });
            }}
            searchTerm={searchTerm}
            service={service}
          />
        ) : (
          <AgentGlobalConfigPanel agentId={agentId} searchTerm={searchTerm} service={service} />
        )}
      </div>
      <LspConfigurationSection service={service} />
      <LspWorkspaceTrustPanel service={service} />
      <LspServerTestPanel service={service} />
      <LspRuntimeStatusPanel service={service} />
    </div>
  );
}
