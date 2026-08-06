import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { ShieldCheck } from "lucide-react";
import { agentService } from "../../services/runtime-agent-client";
import { permissionsService } from "../../services/runtime-permissions-client";
import { Button } from "../../components/ui/button";
import { ApplicationDialog } from "../../components/ui/application-dialog";
import { PageHeader, SectionPanel, SettingsRow } from "./page-parts";
import {
  CLAUDE_CODE_AGENT_ID,
  MANAGED_CLI_AGENT_IDS,
  type PolicyTemplateName,
  type PrincipalEntry,
} from "../../types/permissions";

const templateOptions: PolicyTemplateName[] = ["readonly", "standard", "trusted", "yolo"];

// Translation-key suffix for each managed CLI principal's display name — parallels how
// `settings.agentPolicies.claudeCode` was already named before the other three tools existed.
const managedCliDisplayNameKeys: Record<(typeof MANAGED_CLI_AGENT_IDS)[number], string> = {
  "claude-code": "claudeCode",
  "codex-cli": "codexCli",
  "gemini-cli": "geminiCli",
  opencode: "opencode",
};

function requiresConfirmationToAssign(template: PolicyTemplateName): boolean {
  return template === "trusted" || template === "yolo";
}

function AgentPolicyRow({
  description,
  disabled,
  onSelect,
  principal,
  title,
}: {
  description: string;
  disabled: boolean;
  onSelect: (template: PolicyTemplateName) => void;
  principal: PrincipalEntry | undefined;
  title: string;
}) {
  const { t } = useTranslation();
  return (
    <SettingsRow description={description} title={title}>
      <div className="flex flex-wrap justify-end gap-1">
        {templateOptions.map((option) => (
          <Button
            aria-pressed={principal?.template === option}
            disabled={disabled || !principal}
            key={option}
            onClick={() => onSelect(option)}
            size="sm"
            variant={principal?.template === option ? "default" : "outline"}
          >
            {t(`settings.agentPolicies.template.${option}`)}
          </Button>
        ))}
      </div>
    </SettingsRow>
  );
}

export function AgentPoliciesPage({ searchTerm }: { searchTerm: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [pendingInstallConfirm, setPendingInstallConfirm] = useState<{ agentId: string; template: PolicyTemplateName } | null>(null);
  const [pendingConfirm, setPendingConfirm] = useState<{ agentId: string; template: PolicyTemplateName } | null>(null);

  const agentsQuery = useQuery({
    queryKey: ["agent-policies", "agents"],
    queryFn: () => agentService.listAgents(),
  });

  const policyEligibleAgents = useMemo(
    () => (agentsQuery.data ?? []).filter((agent) => agent.agentOrigin === "user" || agent.id === "onepiece"),
    [agentsQuery.data],
  );

  const query = searchTerm.trim().toLowerCase();

  const visibleAgents = useMemo(() => {
    if (!query) return policyEligibleAgents;
    return policyEligibleAgents.filter(
      (agent) => agent.displayName.toLowerCase().includes(query) || agent.id.toLowerCase().includes(query),
    );
  }, [policyEligibleAgents, query]);

  // The four managed CLI ids are stable CLI principals, not registered AgentRegistryEntry rows,
  // so they're rendered independently of agentService.listAgents()'s results — but they still
  // participate in the same search filter and empty-state logic as every other row.
  const visibleManagedCliAgents = useMemo(
    () =>
      MANAGED_CLI_AGENT_IDS.map((agentId) => ({
        agentId,
        displayName: t(`settings.agentPolicies.${managedCliDisplayNameKeys[agentId]}`),
      })).filter(
        ({ agentId, displayName }) =>
          !query || displayName.toLowerCase().includes(query) || agentId.includes(query),
      ),
    [query, t],
  );

  const agentIds = useMemo(() => {
    const ids = visibleAgents.map((agent) => agent.id);
    return [...ids, ...visibleManagedCliAgents.map(({ agentId }) => agentId)];
  }, [visibleAgents, visibleManagedCliAgents]);

  const principalsQuery = useQuery({
    queryKey: ["agent-policies", "principals", agentIds],
    queryFn: async () => {
      const entries = await Promise.all(agentIds.map((agentId) => permissionsService.getAgentPolicyPrincipal(agentId)));
      return new Map(entries.map((entry) => [entry.agentId, entry]));
    },
    enabled: agentIds.length > 0,
  });

  const assignMutation = useMutation({
    mutationFn: ({ agentId, template }: { agentId: string; template: PolicyTemplateName }) =>
      permissionsService.applyPolicyTemplate(agentId, template),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["agent-policies", "principals"] });
    },
  });

  const principals = principalsQuery.data;

  function assignOrConfirmTrust(agentId: string, template: PolicyTemplateName) {
    if (requiresConfirmationToAssign(template)) {
      setPendingConfirm({ agentId, template });
      return;
    }
    assignMutation.mutate({ agentId, template });
  }

  // The first-use hook-installation confirmation (claude-code only) and the existing
  // increase-trust confirmation (trusted/yolo, any agent) are independent steps that can both
  // apply to the same click — installing is checked first, and if the chosen template also
  // needs the trust confirmation, that one follows once installation is confirmed.
  function selectTemplate(agentId: string, template: PolicyTemplateName) {
    const principal = principals?.get(agentId);
    if (agentId === CLAUDE_CODE_AGENT_ID && principal && !principal.hasExplicitAssignment) {
      setPendingInstallConfirm({ agentId, template });
      return;
    }
    assignOrConfirmTrust(agentId, template);
  }

  function confirmInstall() {
    if (!pendingInstallConfirm) return;
    const { agentId, template } = pendingInstallConfirm;
    setPendingInstallConfirm(null);
    assignOrConfirmTrust(agentId, template);
  }

  function confirmAssign() {
    if (!pendingConfirm) return;
    assignMutation.mutate(pendingConfirm);
    setPendingConfirm(null);
  }

  const hasAnyVisibleRow = visibleAgents.length > 0 || visibleManagedCliAgents.length > 0;

  return (
    <div>
      <PageHeader
        description={t("settings.agentPolicies.description")}
        icon={ShieldCheck}
        title={t("settings.pages.agentPolicies")}
      />
      <SectionPanel title={t("settings.agentPolicies.listTitle")} variant="settings">
        {!hasAnyVisibleRow ? (
          <div className="px-5 py-6 text-sm text-muted-foreground sm:px-6">{t("settings.agentPolicies.empty")}</div>
        ) : (
          <>
            {visibleAgents.map((agent) => (
              <AgentPolicyRow
                description={agent.id}
                disabled={assignMutation.isPending}
                key={agent.id}
                onSelect={(template) => selectTemplate(agent.id, template)}
                principal={principals?.get(agent.id)}
                title={agent.displayName}
              />
            ))}
            {visibleManagedCliAgents.map(({ agentId, displayName }) => (
              <AgentPolicyRow
                description={agentId}
                disabled={assignMutation.isPending}
                key={agentId}
                onSelect={(template) => selectTemplate(agentId, template)}
                principal={principals?.get(agentId)}
                title={displayName}
              />
            ))}
          </>
        )}
      </SectionPanel>
      {pendingInstallConfirm ? (
        <ApplicationDialog
          description={t("settings.agentPolicies.installConfirmDescription")}
          onClose={() => setPendingInstallConfirm(null)}
          title={t("settings.agentPolicies.installConfirmTitle")}
        >
          <div className="flex justify-end gap-2">
            <Button onClick={() => setPendingInstallConfirm(null)} variant="outline">
              {t("settings.agentPolicies.confirmCancel")}
            </Button>
            <Button onClick={confirmInstall}>{t("settings.agentPolicies.confirmAccept")}</Button>
          </div>
        </ApplicationDialog>
      ) : null}
      {pendingConfirm ? (
        <ApplicationDialog
          description={t(`settings.agentPolicies.confirmDescription.${pendingConfirm.template}`)}
          onClose={() => setPendingConfirm(null)}
          title={t("settings.agentPolicies.confirmTitle")}
        >
          <div className="flex justify-end gap-2">
            <Button onClick={() => setPendingConfirm(null)} variant="outline">
              {t("settings.agentPolicies.confirmCancel")}
            </Button>
            <Button onClick={confirmAssign}>{t("settings.agentPolicies.confirmAccept")}</Button>
          </div>
        </ApplicationDialog>
      ) : null}
    </div>
  );
}
