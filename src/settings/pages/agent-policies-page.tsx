import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { ShieldCheck } from "lucide-react";
import { agentService } from "../../services/runtime-agent-client";
import { permissionsService } from "../../services/runtime-permissions-client";
import { Button } from "../../components/ui/button";
import { ApplicationDialog } from "../../components/ui/application-dialog";
import { PageHeader, SectionPanel, SettingsRow } from "./page-parts";
import type { PolicyTemplateName } from "../../types/permissions";

const templateOptions: PolicyTemplateName[] = ["readonly", "standard", "trusted", "yolo"];

function requiresConfirmationToAssign(template: PolicyTemplateName): boolean {
  return template === "trusted" || template === "yolo";
}

export function AgentPoliciesPage({ searchTerm }: { searchTerm: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [pendingConfirm, setPendingConfirm] = useState<{ agentId: string; template: PolicyTemplateName } | null>(null);

  const agentsQuery = useQuery({
    queryKey: ["agent-policies", "agents"],
    queryFn: () => agentService.listAgents(),
  });

  const policyEligibleAgents = useMemo(
    () => (agentsQuery.data ?? []).filter((agent) => agent.agentOrigin === "user" || agent.id === "onepiece"),
    [agentsQuery.data],
  );

  const visibleAgents = useMemo(() => {
    const query = searchTerm.trim().toLowerCase();
    if (!query) return policyEligibleAgents;
    return policyEligibleAgents.filter(
      (agent) => agent.displayName.toLowerCase().includes(query) || agent.id.toLowerCase().includes(query),
    );
  }, [policyEligibleAgents, searchTerm]);

  const agentIds = useMemo(() => visibleAgents.map((agent) => agent.id), [visibleAgents]);

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

  function selectTemplate(agentId: string, template: PolicyTemplateName) {
    if (requiresConfirmationToAssign(template)) {
      setPendingConfirm({ agentId, template });
      return;
    }
    assignMutation.mutate({ agentId, template });
  }

  function confirmAssign() {
    if (!pendingConfirm) return;
    assignMutation.mutate(pendingConfirm);
    setPendingConfirm(null);
  }

  const principals = principalsQuery.data;

  return (
    <div>
      <PageHeader
        description={t("settings.agentPolicies.description")}
        icon={ShieldCheck}
        title={t("settings.pages.agentPolicies")}
      />
      <SectionPanel title={t("settings.agentPolicies.listTitle")} variant="settings">
        {visibleAgents.length === 0 ? (
          <div className="px-5 py-6 text-sm text-muted-foreground sm:px-6">{t("settings.agentPolicies.empty")}</div>
        ) : (
          visibleAgents.map((agent) => {
            const principal = principals?.get(agent.id);
            return (
              <SettingsRow description={agent.id} key={agent.id} title={agent.displayName}>
                <div className="flex flex-wrap justify-end gap-1">
                  {templateOptions.map((option) => (
                    <Button
                      aria-pressed={principal?.template === option}
                      disabled={assignMutation.isPending || !principal}
                      key={option}
                      onClick={() => selectTemplate(agent.id, option)}
                      size="sm"
                      variant={principal?.template === option ? "default" : "outline"}
                    >
                      {t(`settings.agentPolicies.template.${option}`)}
                    </Button>
                  ))}
                </div>
              </SettingsRow>
            );
          })
        )}
      </SectionPanel>
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
