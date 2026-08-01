import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type { AgentRegistryEntry } from "../../../types/agent";

export function AgentToolTrustToggle({
  agent,
  service = defaultAgentService,
}: {
  agent: AgentRegistryEntry;
  service?: AgentService;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const configQuery = useQuery({
    queryKey: ["agents", "provider-config", agent.id],
    queryFn: () => service.getApiAgentProviderConfig(agent.id),
  });

  const mutation = useMutation({
    mutationFn: (enabled: boolean) => service.setAgentToolTrust(agent.id, enabled),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["agents", "provider-config", agent.id] });
    },
  });

  const trusted = configQuery.data?.autoApproveTools ?? false;

  function handleToggle() {
    if (trusted) {
      void mutation.mutateAsync(false);
      return;
    }
    if (!window.confirm(t("agents.trust.confirmEnable", { agent: agent.displayName }))) return;
    void mutation.mutateAsync(true);
  }

  return (
    <div className="mt-2 flex items-center justify-between gap-2 text-xs">
      <span className={trusted ? "ucd-status-warning" : "text-muted-foreground"}>
        {trusted ? t("agents.trust.statusTrusted") : t("agents.trust.statusUntrusted")}
      </span>
      <Button
        className="h-7 px-2 text-xs"
        disabled={mutation.isPending || configQuery.isLoading}
        onClick={handleToggle}
        type="button"
        variant="outline"
      >
        {trusted ? t("agents.trust.disable") : t("agents.trust.enable")}
      </Button>
    </div>
  );
}
