import { ExternalLink } from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../../components/agent-brand-icon";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { getAgentVisualIdentity } from "../../lib/agent-visual-identity";
import type { ManagedCliAgentId } from "../../types/agent";
import type { CliInstallationSnapshot, CliParameterProfile } from "../../types/cli-parameter-profile";
import { SectionPanel } from "../pages/page-parts";

function installationLabel(
  installation: CliInstallationSnapshot,
  translate: (key: string, values?: Record<string, string>) => string,
): string {
  if (!installation.installed) return translate("cliParameters.lifecycle.notInstalled");
  if (!installation.runnable) return translate("cliParameters.lifecycle.notRunnable");
  if (installation.conflict) return translate("cliParameters.lifecycle.conflict");
  return installation.version
    ? translate("cliParameters.lifecycle.version", { version: installation.version })
    : translate("cliParameters.lifecycle.unknownVersion");
}

export interface CliParameterRailProps {
  profiles: readonly CliParameterProfile[];
  activeAgentId: ManagedCliAgentId | null;
  dirtyCountFor: (agentId: ManagedCliAgentId) => number;
  onSelect: (agentId: ManagedCliAgentId) => void;
  onOpenCliManagement: () => void;
}

/**
 * External CLIs only. OnePiece is not a CLI whose argv this page owns, so it gets a link to the
 * page that does own it rather than a tab that pretends otherwise.
 */
export function CliParameterRail({
  profiles,
  activeAgentId,
  dirtyCountFor,
  onSelect,
  onOpenCliManagement,
}: CliParameterRailProps) {
  const { t } = useTranslation();

  return (
    <SectionPanel
      className="sticky top-4 self-start"
      description={t("cliParameters.agents.description")}
      title={t("cliParameters.agents.title")}
    >
      <nav aria-label={t("cliParameters.agents.title")} className="space-y-2">
        {profiles.map((profile) => {
          const dirty = dirtyCountFor(profile.agentId);
          const errors = profile.diagnostics.filter((entry) => entry.severity === "error").length;
          const warnings = profile.diagnostics.filter(
            (entry) => entry.severity === "warning",
          ).length;
          return (
            <Button
              aria-current={activeAgentId === profile.agentId ? "page" : undefined}
              className="h-auto w-full flex-col items-start gap-1 py-2"
              key={profile.agentId}
              onClick={() => onSelect(profile.agentId)}
              variant={activeAgentId === profile.agentId ? "default" : "ghost"}
            >
              <span className="flex w-full items-center gap-2">
                <span
                  className={`flex h-6 w-6 shrink-0 items-center justify-center rounded border ${getAgentVisualIdentity(profile.agentId).tone}`}
                >
                  <AgentBrandIcon agentId={profile.agentId} className="h-3.5 w-3.5" />
                </span>
                <span className="truncate">{t(`cliParameters.agents.${profile.agentId}`)}</span>
              </span>
              <span className="w-full truncate text-left text-xs font-normal opacity-80">
                {installationLabel(profile.installation, t)}
              </span>
              {profile.installation.activePath ? (
                <span
                  className="w-full truncate text-left text-xs font-normal opacity-60"
                  title={profile.installation.activePath}
                >
                  {profile.installation.activePath}
                </span>
              ) : null}
              {dirty || warnings || errors ? (
                <span className="flex flex-wrap gap-1">
                  {dirty ? (
                    <Badge tone="warning">
                      {t("cliParameters.badge.dirty", { count: String(dirty) })}
                    </Badge>
                  ) : null}
                  {warnings ? (
                    <Badge tone="warning">
                      {t("cliParameters.badge.warnings", { count: String(warnings) })}
                    </Badge>
                  ) : null}
                  {errors ? (
                    <Badge tone="danger">
                      {t("cliParameters.badge.errors", { count: String(errors) })}
                    </Badge>
                  ) : null}
                </span>
              ) : null}
            </Button>
          );
        })}
      </nav>
      <p className="mt-4 text-xs leading-5 text-muted-foreground">
        {t("cliParameters.onepieceLink")}
      </p>
      <Button
        className="mt-2 w-full justify-start gap-2"
        onClick={onOpenCliManagement}
        variant="outline"
      >
        <ExternalLink aria-hidden="true" /> {t("cliParameters.lifecycle.manage")}
      </Button>
    </SectionPanel>
  );
}
