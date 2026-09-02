import { Settings2 } from "lucide-react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import type { SettingsPageContext } from "../settings-pages";
import { PageHeader } from "./page-parts";
import { AgentGlobalConfigPanel, agentGlobalConfigQueryKey } from "./agents/agent-global-config-panel";
import { OnePieceConfigurationPanel, onePieceProviderProfilesQueryKey } from "./agents/onepiece-configuration-panel";
import { OnePieceParametersPanel } from "./onepiece-parameters-panel";
import { useEffect, useState } from "react";
import type { AgentService } from "../../services/agent-service";
import { agentService as defaultAgentService } from "../../services/runtime-agent-client";
import { AgentConfigurationSelector, agentNameKeys, type ConfigurableAgentId } from "./agents/agent-configuration-selector";
import { buildCliAgentConfigDiagnosticFields, buildOnePieceConfigDiagnosticFields } from "./agents/agent-configuration-diagnostic-summary";
import { CopyDiagnosticsButton } from "../../ui/diagnostics/CopyDiagnosticsButton";
import type { CliConfigAgentId } from "../../types/cli-agent-config";

export function AgentConfigurationsPage({ navigationTarget, searchTerm, service = defaultAgentService }: SettingsPageContext & { service?: AgentService }) {
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

  const isOnePiece = agentId === "onepiece";
  // A page-level, currently-selected-agent-scoped diagnostics summary (spec.md "Copyable safe
  // settings diagnostics"): only one branch is ever visible at a time, so only that branch's own
  // query is enabled. Both queries below re-subscribe to the exact cache entry their own panel
  // already owns (`agentGlobalConfigQueryKey`/`onePieceProviderProfilesQueryKey`, both exported for
  // this purpose) rather than refetching -- the same mechanism `code-intelligence-page.tsx` already
  // uses to read its own sections' cache without refactoring them.
  //
  // `cliConfigAgentId` must still be a real `CliConfigAgentId` even while this branch is inactive,
  // since hooks must run unconditionally on every render -- `enabled: false` guarantees the
  // placeholder is never actually fetched, so it can never diverge from the real per-agent entry.
  const cliConfigAgentId: CliConfigAgentId = isOnePiece ? "claude-code" : agentId;
  const cliConfigQuery = useQuery({
    queryKey: agentGlobalConfigQueryKey(cliConfigAgentId),
    queryFn: async () => {
      const [presets, profiles, status] = await Promise.all([
        service.listCliConfigPresets(cliConfigAgentId),
        service.listCliConfigProfiles(cliConfigAgentId),
        service.getCliConfigStatus(cliConfigAgentId),
      ]);
      return { presets, profiles, status };
    },
    enabled: !isOnePiece,
  });
  const onePieceQuery = useQuery({
    queryKey: onePieceProviderProfilesQueryKey,
    queryFn: async () => {
      const [overview, presets] = await Promise.all([
        service.listOnePieceProviderProfiles(),
        service.listOnePieceProviderPresets(),
      ]);
      return { overview, presets };
    },
    enabled: isOnePiece,
  });
  const diagnosticFields = isOnePiece
    ? buildOnePieceConfigDiagnosticFields(onePieceQuery.data?.overview, t)
    : buildCliAgentConfigDiagnosticFields(cliConfigAgentId, cliConfigQuery.data?.status, cliConfigQuery.data?.profiles ?? [], t);

  return (
    <div className="space-y-5" data-testid="agent-configurations-page">
      <PageHeader
        actions={<CopyDiagnosticsButton fields={diagnosticFields} />}
        description={t("agentConfigurations.description")}
        icon={Settings2}
        title={t("agentConfigurations.title")}
      />
      <div className="grid min-w-0 gap-5 lg:grid-cols-[13rem_minmax(0,1fr)]">
        <AgentConfigurationSelector onSelect={setAgentId} selected={agentId} />
        <div aria-label={t(agentNameKeys[agentId])} className="min-w-0" data-agent-id={agentId} data-testid="agent-configuration-content" role="region">
          {isOnePiece ? (
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
