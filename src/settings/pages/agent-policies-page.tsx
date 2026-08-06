import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { ShieldCheck } from "lucide-react";
import { agentService } from "../../services/runtime-agent-client";
import { permissionsService } from "../../services/runtime-permissions-client";
import { Button } from "../../components/ui/button";
import { ApplicationDialog } from "../../components/ui/application-dialog";
import { PageHeader, SectionPanel, SettingsRow } from "./page-parts";
import { CLAUDE_CODE_AGENT_ID, type PolicyTemplateName, type PrincipalEntry } from "../../types/permissions";

const templateOptions: PolicyTemplateName[] = ["readonly", "standard", "trusted", "yolo"];

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

  // claude-code is a stable CLI principal, not a registered AgentRegistryEntry, so it's rendered
  // as its own row rather than folded into agentService.listAgents()'s results — but it still
  // participates in the same search filter and empty-state logic as every other row.
  const claudeCodeDisplayName = t("settings.agentPolicies.claudeCode");
  const claudeCodeVisible =
    !query || claudeCodeDisplayName.toLowerCase().includes(query) || CLAUDE_CODE_AGENT_ID.includes(query);

  const agentIds = useMemo(() => {
    const ids = visibleAgents.map((agent) => agent.id);
    return claudeCodeVisible ? [...ids, CLAUDE_CODE_AGENT_ID] : ids;
  }, [visibleAgents, claudeCodeVisible]);

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

  const hasAnyVisibleRow = visibleAgents.length > 0 || claudeCodeVisible;

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
            {claudeCodeVisible ? (
              <AgentPolicyRow
                description={CLAUDE_CODE_AGENT_ID}
                disabled={assignMutation.isPending}
                key={CLAUDE_CODE_AGENT_ID}
                onSelect={(template) => selectTemplate(CLAUDE_CODE_AGENT_ID, template)}
                principal={principals?.get(CLAUDE_CODE_AGENT_ID)}
                title={claudeCodeDisplayName}
              />
            ) : null}
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
