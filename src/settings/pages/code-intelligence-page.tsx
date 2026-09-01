import { useQuery } from "@tanstack/react-query";
import { Braces } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { AgentService } from "../../services/agent-service";
import { agentService as defaultAgentService } from "../../services/runtime-agent-client";
import { CopyDiagnosticsButton } from "../../ui/diagnostics/CopyDiagnosticsButton";
import type { SettingsPageContext } from "../settings-pages";
import {
  LspConfigurationSection,
  lspConfigurationQueryKey,
  lspDiscoveryQueryKey,
  lspServerStatusQueryKey,
} from "./agents/lsp-configuration-section";
import { buildLspDiagnosticFields } from "./agents/lsp-diagnostic-summary";
import { LspRuntimeStatusPanel } from "./agents/lsp-runtime-status-panel";
import { LspServerTestPanel } from "./agents/lsp-server-test-panel";
import { LspWorkspaceTrustPanel, lspWorkspaceTrustQueryKey } from "./agents/lsp-workspace-trust-panel";
import { PageHeader } from "./page-parts";

export function CodeIntelligencePage({
  service = defaultAgentService,
}: SettingsPageContext & { service?: AgentService }) {
  const { t } = useTranslation();
  // Unlike CLI Management/Local Media, no single section here already holds every field the
  // page-level diagnostics summary needs as a prop -- each of the 4 sections below owns its own
  // query. Re-subscribing to the same exported keys is the same cross-component sharing this page's
  // own sections already rely on (`LspServerTestPanel` reuses `lspConfigurationQueryKey` rather than
  // fetching its own copy), so this adds no new fetch, just another reader of the same cache entry.
  const configurationQuery = useQuery({ queryKey: lspConfigurationQueryKey, queryFn: () => service.getLspConfiguration() });
  const discoveryQuery = useQuery({ queryKey: lspDiscoveryQueryKey, queryFn: () => service.discoverLspServers() });
  const trustQuery = useQuery({ queryKey: lspWorkspaceTrustQueryKey, queryFn: () => service.listLspWorkspaceTrust() });
  const statusQuery = useQuery({ queryKey: lspServerStatusQueryKey, queryFn: () => service.getLspServerStatus() });
  const diagnosticFields = buildLspDiagnosticFields(
    configurationQuery.data,
    discoveryQuery.data ?? [],
    trustQuery.data ?? [],
    statusQuery.data ?? [],
    t,
  );

  return (
    <div className="space-y-5" data-testid="code-intelligence-page">
      <PageHeader
        actions={<CopyDiagnosticsButton fields={diagnosticFields} />}
        description={t("codeIntelligence.description")}
        icon={Braces}
        title={t("settings.pages.codeIntelligence")}
      />
      <LspConfigurationSection service={service} />
      <LspWorkspaceTrustPanel service={service} />
      <LspServerTestPanel service={service} />
      <LspRuntimeStatusPanel service={service} />
    </div>
  );
}
