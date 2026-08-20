import { Braces } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { AgentService } from "../../services/agent-service";
import type { SettingsPageContext } from "../settings-pages";
import { LspConfigurationSection } from "./agents/lsp-configuration-section";
import { LspRuntimeStatusPanel } from "./agents/lsp-runtime-status-panel";
import { LspServerTestPanel } from "./agents/lsp-server-test-panel";
import { LspWorkspaceTrustPanel } from "./agents/lsp-workspace-trust-panel";
import { PageHeader } from "./page-parts";

export function CodeIntelligencePage({ service }: SettingsPageContext & { service?: AgentService }) {
  const { t } = useTranslation();

  return (
    <div className="space-y-5" data-testid="code-intelligence-page">
      <PageHeader description={t("codeIntelligence.description")} icon={Braces} title={t("settings.pages.codeIntelligence")} />
      <LspConfigurationSection service={service} />
      <LspWorkspaceTrustPanel service={service} />
      <LspServerTestPanel service={service} />
      <LspRuntimeStatusPanel service={service} />
    </div>
  );
}
